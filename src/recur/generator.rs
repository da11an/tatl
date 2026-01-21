//! Recurrence generation logic
//!
//! Generates recurring task instances from seed tasks based on recurrence rules.
//! The generation is idempotent - running multiple times produces the same results.
//!
//! # Algorithm
//!
//! 1. Find all seed tasks (tasks with `recur` field)
//! 2. For each seed task:
//!    - Parse the recurrence rule
//!    - Generate occurrence timestamps up to the `until` date
//!    - For each occurrence:
//!      - Check if it already exists (via `recur_occurrences` table)
//!      - If not, create a new task instance
//!      - Record the occurrence in `recur_occurrences` table
//!
//! # Attribute Precedence
//!
//! When creating instances, attributes are merged in this order (highest to lowest priority):
//! 1. Task-specific values (from seed task)
//! 2. Template values (if template is specified)
//! 3. Computed/default values
//!
//! # Idempotency
//!
//! The `recur_occurrences` table ensures idempotency by recording each generated occurrence.
//! If a seed task and occurrence timestamp combination already exists, no new instance is created.

use chrono::{DateTime, Datelike, Duration};
use rusqlite::Connection;
use anyhow::Result;
use crate::recur::parser::{RecurRule, RecurFrequency};
use crate::repo::{TaskRepo, TemplateRepo};
use crate::models::Task;
use std::collections::HashMap;

/// Generate recurring task instances
///
/// # Example
///
/// ```no_run
/// use tatl::db::DbConnection;
/// use tatl::recur::RecurGenerator;
/// use chrono::Utc;
///
/// let conn = DbConnection::connect().unwrap();
/// let until_ts = (Utc::now() + chrono::Duration::days(30)).timestamp();
/// let count = RecurGenerator::run(&conn, until_ts).unwrap();
/// println!("Generated {} recurring task instances", count);
/// ```
pub struct RecurGenerator;

impl RecurGenerator {
    /// Run recurrence generation for all seed tasks
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `until_ts` - Unix timestamp (UTC) until which to generate occurrences
    ///
    /// # Returns
    /// Number of new instances created
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tatl::db::DbConnection;
    /// use tatl::recur::RecurGenerator;
    /// use chrono::Utc;
    ///
    /// let conn = DbConnection::connect().unwrap();
    /// let until_ts = (Utc::now() + chrono::Duration::days(30)).timestamp();
    /// let count = RecurGenerator::run(&conn, until_ts).unwrap();
    /// ```
    pub fn run(conn: &Connection, until_ts: i64) -> Result<usize> {
        let now = chrono::Utc::now().timestamp();
        
        // Get all seed tasks (tasks with recur field)
        let seed_tasks = Self::get_seed_tasks(conn)?;
        
        let mut total_generated = 0;
        
        for seed_task in seed_tasks {
            let seed_id = seed_task.id.unwrap();
            let recur_str = seed_task.recur.as_ref()
                .ok_or_else(|| anyhow::anyhow!("Seed task {} has no recur field", seed_id))?;
            
            // Parse recurrence rule
            let rule = RecurRule::parse(recur_str)?;
            
            // Generate occurrences
            let occurrences = Self::generate_occurrences(&rule, now, until_ts)?;
            
            // Create instances for each occurrence
            for occurrence_ts in occurrences {
                // Check if this occurrence already exists
                let exists: bool = conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM recur_occurrences WHERE seed_task_id = ?1 AND occurrence_ts = ?2)",
                    rusqlite::params![seed_id, occurrence_ts],
                    |row| row.get(0),
                )?;
                
                if !exists {
                    // Create instance
                    Self::create_instance(conn, &seed_task, occurrence_ts)?;
                    total_generated += 1;
                }
            }
        }
        
        Ok(total_generated)
    }
    
    fn get_seed_tasks(conn: &Connection) -> Result<Vec<Task>> {
        let mut stmt = conn.prepare(
            "SELECT id, uuid, description, status, project_id, due_ts, scheduled_ts, 
                    wait_ts, alloc_secs, template, recur, udas_json, created_ts, modified_ts 
             FROM tasks WHERE recur IS NOT NULL AND recur != ''"
        )?;
        
        let rows = stmt.query_map([], |row| {
            let udas_json: Option<String> = row.get(11)?;
            let mut udas = HashMap::new();
            if let Some(json) = udas_json {
                if let Ok(parsed) = serde_json::from_str::<HashMap<String, String>>(&json) {
                    udas = parsed;
                }
            }
            
            let status_str: String = row.get(3)?;
            let status = crate::models::TaskStatus::from_str(&status_str)
                .unwrap_or(crate::models::TaskStatus::Pending);
            Ok(Task {
                id: Some(row.get(0)?),
                uuid: row.get(1)?,
                description: row.get(2)?,
                status,
                project_id: row.get(4)?,
                due_ts: row.get(5)?,
                scheduled_ts: row.get(6)?,
                wait_ts: row.get(7)?,
                alloc_secs: row.get(8)?,
                template: row.get(9)?,
                recur: row.get(10)?,
                udas,
                created_ts: row.get(12)?,
                modified_ts: row.get(13)?,
            })
        })?;
        
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }
        Ok(tasks)
    }
    
    fn generate_occurrences(rule: &RecurRule, start_ts: i64, end_ts: i64) -> Result<Vec<i64>> {
        let mut occurrences = Vec::new();
        let start_dt = DateTime::<chrono::Utc>::from_timestamp(start_ts, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid start timestamp"))?;
        let end_dt = DateTime::<chrono::Utc>::from_timestamp(end_ts, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid end timestamp"))?;
        
        // For patterns with weekday/monthday filters, we need to check each day
        // For simple patterns, we can advance by the frequency interval
        let needs_daily_check = rule.byweekday.is_some() || rule.bymonthday.is_some();
        
        if needs_daily_check {
            // Check each day in the range
            let mut current = start_dt.date_naive().and_hms_opt(0, 0, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
            let end_date = end_dt.date_naive();
            
            while current.date() <= end_date {
                let current_dt = DateTime::from_naive_utc_and_offset(current, chrono::Utc);
                let current_ts = current_dt.timestamp();
                
                if current_ts > start_ts && current_ts <= end_ts {
                    if Self::matches_rule(rule, &current_dt)? {
                        occurrences.push(current_ts);
                    }
                }
                
                current = current + Duration::days(1);
            }
        } else {
            // Simple pattern - advance by frequency interval
            let mut current = start_dt;
            
            while current <= end_dt {
                let current_ts = current.timestamp();
                
                if current_ts > start_ts && current_ts <= end_ts {
                    occurrences.push(current_ts);
                }
                
                // Advance to next potential occurrence
                current = Self::next_occurrence(rule, &current)?;
            }
        }
        
        Ok(occurrences)
    }
    
    fn matches_rule(rule: &RecurRule, dt: &DateTime<chrono::Utc>) -> Result<bool> {
        // Check weekday filter
        if let Some(ref weekdays) = rule.byweekday {
            let weekday = dt.weekday().num_days_from_monday() as u32;
            if !weekdays.contains(&weekday) {
                return Ok(false);
            }
        }
        
        // Check monthday filter
        if let Some(ref monthdays) = rule.bymonthday {
            let day = dt.day();
            if !monthdays.contains(&day) {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    fn next_occurrence(rule: &RecurRule, current: &DateTime<chrono::Utc>) -> Result<DateTime<chrono::Utc>> {
        match &rule.frequency {
            RecurFrequency::Daily => Ok(*current + Duration::days(1)),
            RecurFrequency::Weekly => Ok(*current + Duration::weeks(1)),
            RecurFrequency::Monthly => {
                // Add one month (approximate - use 30 days)
                Ok(*current + Duration::days(30))
            }
            RecurFrequency::Yearly => Ok(*current + Duration::days(365)),
            RecurFrequency::EveryDays(n) => Ok(*current + Duration::days(*n as i64)),
            RecurFrequency::EveryWeeks(n) => Ok(*current + Duration::weeks(*n as i64)),
            RecurFrequency::EveryMonths(n) => Ok(*current + Duration::days(30 * *n as i64)),
            RecurFrequency::EveryYears(n) => Ok(*current + Duration::days(365 * *n as i64)),
        }
    }
    
    fn create_instance(conn: &Connection, seed: &Task, occurrence_ts: i64) -> Result<i64> {
        // Load template if specified and merge attributes
        // Attribute precedence: Template → Seed → Computed dates
        let (final_project_id, final_due_ts, final_scheduled_ts, final_wait_ts, final_alloc_secs, final_udas, final_tags) = 
            if let Some(template_name) = &seed.template {
                // Load template
                if let Some(template) = TemplateRepo::get_by_name(conn, template_name)? {
                    // Get seed task tags
                    let mut stmt = conn.prepare("SELECT tag FROM task_tags WHERE task_id = ?1")?;
                    let tag_rows = stmt.query_map([seed.id.unwrap()], |row| {
                        Ok(row.get::<_, String>(0)?)
                    })?;
                    
                    let mut seed_tags = Vec::new();
                    for tag_row in tag_rows {
                        seed_tags.push(tag_row?);
                    }
                    
                    // Merge template with seed (seed overrides template)
                    TemplateRepo::merge_attributes(
                        &template,
                        seed.project_id,
                        seed.due_ts,
                        seed.scheduled_ts,
                        seed.wait_ts,
                        seed.alloc_secs,
                        &seed.udas,
                        &seed_tags,
                    )
                } else {
                    // Template not found - use seed attributes as-is
                    let mut stmt = conn.prepare("SELECT tag FROM task_tags WHERE task_id = ?1")?;
                    let tag_rows = stmt.query_map([seed.id.unwrap()], |row| {
                        Ok(row.get::<_, String>(0)?)
                    })?;
                    
                    let mut tags = Vec::new();
                    for tag_row in tag_rows {
                        tags.push(tag_row?);
                    }
                    (seed.project_id, seed.due_ts, seed.scheduled_ts, seed.wait_ts, seed.alloc_secs, seed.udas.clone(), tags)
                }
            } else {
                // No template - use seed attributes as-is
                let mut stmt = conn.prepare("SELECT tag FROM task_tags WHERE task_id = ?1")?;
                let tag_rows = stmt.query_map([seed.id.unwrap()], |row| {
                    Ok(row.get::<_, String>(0)?)
                })?;
                
                let mut tags = Vec::new();
                for tag_row in tag_rows {
                    tags.push(tag_row?);
                }
                (seed.project_id, seed.due_ts, seed.scheduled_ts, seed.wait_ts, seed.alloc_secs, seed.udas.clone(), tags)
            };
        
        // TODO: Evaluate relative dates relative to occurrence_ts
        // For now, dates are used as-is (relative date evaluation deferred)
        
        // Create instance task (no recur field, no template field)
        let instance = TaskRepo::create_full(
            conn,
            &seed.description,
            final_project_id,
            final_due_ts,
            final_scheduled_ts,
            final_wait_ts,
            final_alloc_secs,
            None, // No template field in instance
            None, // No recur field in instance
            &final_udas,
            &final_tags,
        )?;
        
        let instance_id = instance.id.unwrap();
        
        // Record occurrence
        conn.execute(
            "INSERT INTO recur_occurrences (seed_task_id, occurrence_ts, instance_task_id) VALUES (?1, ?2, ?3)",
            rusqlite::params![seed.id.unwrap(), occurrence_ts, instance_id],
        )?;
        
        Ok(instance_id)
    }
}

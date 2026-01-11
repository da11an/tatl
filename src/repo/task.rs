use rusqlite::{Connection, OptionalExtension};
use crate::models::Task;
use anyhow::{Context, Result};
use std::collections::HashMap;

/// Task repository for database operations
pub struct TaskRepo;

impl TaskRepo {
    /// Create a new task with full field support
    pub fn create_full(
        conn: &Connection,
        description: &str,
        project_id: Option<i64>,
        due_ts: Option<i64>,
        scheduled_ts: Option<i64>,
        wait_ts: Option<i64>,
        alloc_secs: Option<i64>,
        template: Option<String>,
        recur: Option<String>,
        udas: &HashMap<String, String>,
        tags: &[String],
    ) -> Result<Task> {
        let mut task = Task::new(description.to_string());
        task.project_id = project_id;
        task.due_ts = due_ts;
        task.scheduled_ts = scheduled_ts;
        task.wait_ts = wait_ts;
        task.alloc_secs = alloc_secs;
        task.template = template.clone();
        task.recur = recur.clone();
        task.udas = udas.clone();
        
        let now = chrono::Utc::now().timestamp();
        
        // Serialize UDAs to JSON
        let udas_json = if udas.is_empty() {
            None
        } else {
            Some(serde_json::to_string(udas)?)
        };
        
        conn.execute(
            "INSERT INTO tasks (uuid, description, status, project_id, due_ts, scheduled_ts, 
                    wait_ts, alloc_secs, template, recur, udas_json, created_ts, modified_ts) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                task.uuid,
                task.description,
                task.status.as_str(),
                task.project_id,
                task.due_ts,
                task.scheduled_ts,
                task.wait_ts,
                task.alloc_secs,
                task.template,
                task.recur,
                udas_json,
                now,
                now
            ],
        )
        .with_context(|| format!("Failed to create task: {}", description))?;
        
        let id = conn.last_insert_rowid();
        
        // Add tags
        for tag in tags {
            conn.execute(
                "INSERT INTO task_tags (task_id, tag) VALUES (?1, ?2)",
                rusqlite::params![id, tag],
            )?;
        }
        
        Ok(Task {
            id: Some(id),
            ..task
        })
    }

    /// Create a new task (simplified version for backward compatibility)
    pub fn create(conn: &Connection, description: &str, project_id: Option<i64>) -> Result<Task> {
        Self::create_full(
            conn,
            description,
            project_id,
            None,
            None,
            None,
            None,
            None,
            None,
            &HashMap::new(),
            &[],
        )
    }

    /// Get task by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> Result<Option<Task>> {
        let mut stmt = conn.prepare(
            "SELECT id, uuid, description, status, project_id, due_ts, scheduled_ts, 
                    wait_ts, alloc_secs, template, recur, udas_json, created_ts, modified_ts 
             FROM tasks WHERE id = ?1"
        )?;
        
        let task = stmt.query_row([id], |row| {
            let udas_json: Option<String> = row.get(11)?;
            let mut udas = HashMap::new();
            if let Some(json) = udas_json {
                if let Ok(parsed) = serde_json::from_str::<HashMap<String, String>>(&json) {
                    udas = parsed;
                }
            }
            
            Ok(Task {
                id: Some(row.get(0)?),
                uuid: row.get(1)?,
                description: row.get(2)?,
                status: crate::models::TaskStatus::from_str(&row.get::<_, String>(3)?)
                    .unwrap_or(crate::models::TaskStatus::Pending),
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
        }).optional()?;
        
        Ok(task)
    }

    /// Get tags for a task
    pub fn get_tags(conn: &Connection, task_id: i64) -> Result<Vec<String>> {
        let mut stmt = conn.prepare("SELECT tag FROM task_tags WHERE task_id = ?1 ORDER BY tag")?;
        let rows = stmt.query_map([task_id], |row| {
            Ok(row.get::<_, String>(0)?)
        })?;
        
        let mut tags = Vec::new();
        for row in rows {
            tags.push(row?);
        }
        Ok(tags)
    }

    /// List all tasks (basic - no filtering yet)
    pub fn list_all(conn: &Connection) -> Result<Vec<(Task, Vec<String>)>> {
        let mut stmt = conn.prepare(
            "SELECT id, uuid, description, status, project_id, due_ts, scheduled_ts, 
                    wait_ts, alloc_secs, template, recur, udas_json, created_ts, modified_ts 
             FROM tasks WHERE status != 'deleted' ORDER BY id"
        )?;
        
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let udas_json: Option<String> = row.get(11)?;
            let mut udas = HashMap::new();
            if let Some(json) = udas_json {
                if let Ok(parsed) = serde_json::from_str::<HashMap<String, String>>(&json) {
                    udas = parsed;
                }
            }
            
            Ok(Task {
                id: Some(id),
                uuid: row.get(1)?,
                description: row.get(2)?,
                status: crate::models::TaskStatus::from_str(&row.get::<_, String>(3)?)
                    .unwrap_or(crate::models::TaskStatus::Pending),
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
        for task_result in rows {
            let task = task_result?;
            let tags = Self::get_tags(conn, task.id.unwrap())?;
            tasks.push((task, tags));
        }
        
        Ok(tasks)
    }
}

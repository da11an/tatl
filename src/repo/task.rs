use rusqlite::{Connection, OptionalExtension};
use crate::models::Task;
use crate::repo::EventRepo;
use anyhow::{Context, Result};
use std::collections::HashMap;

/// Task repository for database operations
///
/// Provides CRUD operations for tasks, including:
/// - Creating tasks with full attribute support
/// - Querying and filtering tasks
/// - Modifying task attributes
/// - Completing tasks
/// - Managing task tags and UDAs
///
/// # Example
///
/// ```no_run
/// use task_ninja::db::DbConnection;
/// use task_ninja::repo::TaskRepo;
///
/// let conn = DbConnection::connect().unwrap();
/// let task = TaskRepo::create(&conn, "Fix bug", None).unwrap();
/// println!("Created task {}", task.id.unwrap());
/// ```
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
            // Record tag_added event
            EventRepo::record_tag_added(conn, id, tag)?;
        }
        
        // Record created event
        EventRepo::record_created(conn, id, description, project_id)?;
        
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

    /// Modify a task
    pub fn modify(
        conn: &Connection,
        task_id: i64,
        description: Option<String>,
        project_id: Option<Option<i64>>, // Some(None) means clear, None means don't change
        due_ts: Option<Option<i64>>,
        scheduled_ts: Option<Option<i64>>,
        wait_ts: Option<Option<i64>>,
        alloc_secs: Option<Option<i64>>,
        template: Option<Option<String>>,
        recur: Option<Option<String>>,
        udas_to_add: &HashMap<String, String>,
        udas_to_remove: &[String],
        tags_to_add: &[String],
        tags_to_remove: &[String],
    ) -> Result<()> {
        // Get current task (for event recording)
        let old_task = Self::get_by_id(conn, task_id)?
            .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;
        let mut task = old_task.clone();
        
        let now = chrono::Utc::now().timestamp();
        
        // Update description if provided
        if let Some(desc) = description {
            if desc != task.description {
                EventRepo::record_modified(
                    conn,
                    task_id,
                    "description",
                    Some(serde_json::Value::String(task.description.clone())),
                    Some(serde_json::Value::String(desc.clone())),
                )?;
            }
            task.description = desc;
        }
        
        // Update project
        if let Some(proj_id) = project_id {
            if proj_id != task.project_id {
                EventRepo::record_modified(
                    conn,
                    task_id,
                    "project_id",
                    task.project_id.map(|id| serde_json::Value::Number(id.into())),
                    proj_id.map(|id| serde_json::Value::Number(id.into())),
                )?;
            }
            task.project_id = proj_id;
        }
        
        // Update dates
        if let Some(due) = due_ts {
            if due != task.due_ts {
                EventRepo::record_modified(
                    conn,
                    task_id,
                    "due_ts",
                    task.due_ts.map(|ts| serde_json::Value::Number(ts.into())),
                    due.map(|ts| serde_json::Value::Number(ts.into())),
                )?;
            }
            task.due_ts = due;
        }
        if let Some(scheduled) = scheduled_ts {
            if scheduled != task.scheduled_ts {
                EventRepo::record_modified(
                    conn,
                    task_id,
                    "scheduled_ts",
                    task.scheduled_ts.map(|ts| serde_json::Value::Number(ts.into())),
                    scheduled.map(|ts| serde_json::Value::Number(ts.into())),
                )?;
            }
            task.scheduled_ts = scheduled;
        }
        if let Some(wait) = wait_ts {
            if wait != task.wait_ts {
                EventRepo::record_modified(
                    conn,
                    task_id,
                    "wait_ts",
                    task.wait_ts.map(|ts| serde_json::Value::Number(ts.into())),
                    wait.map(|ts| serde_json::Value::Number(ts.into())),
                )?;
            }
            task.wait_ts = wait;
        }
        if let Some(alloc) = alloc_secs {
            if alloc != task.alloc_secs {
                EventRepo::record_modified(
                    conn,
                    task_id,
                    "alloc_secs",
                    task.alloc_secs.map(|ts| serde_json::Value::Number(ts.into())),
                    alloc.map(|ts| serde_json::Value::Number(ts.into())),
                )?;
            }
            task.alloc_secs = alloc;
        }
        if let Some(tmpl) = template {
            if tmpl != task.template {
                EventRepo::record_modified(
                    conn,
                    task_id,
                    "template",
                    task.template.as_ref().map(|s| serde_json::Value::String(s.clone())),
                    tmpl.as_ref().map(|s| serde_json::Value::String(s.clone())),
                )?;
            }
            task.template = tmpl;
        }
        if let Some(rec) = recur {
            if rec != task.recur {
                EventRepo::record_modified(
                    conn,
                    task_id,
                    "recur",
                    task.recur.as_ref().map(|s| serde_json::Value::String(s.clone())),
                    rec.as_ref().map(|s| serde_json::Value::String(s.clone())),
                )?;
            }
            task.recur = rec;
        }
        
        // Update UDAs
        for (key, value) in udas_to_add {
            task.udas.insert(key.clone(), value.clone());
        }
        for key in udas_to_remove {
            task.udas.remove(key);
        }
        
        // Serialize UDAs
        let udas_json = if task.udas.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&task.udas)?)
        };
        
        // Update task in database
        conn.execute(
            "UPDATE tasks SET description = ?1, project_id = ?2, due_ts = ?3, scheduled_ts = ?4,
                    wait_ts = ?5, alloc_secs = ?6, template = ?7, recur = ?8, udas_json = ?9,
                    modified_ts = ?10 WHERE id = ?11",
            rusqlite::params![
                task.description,
                task.project_id,
                task.due_ts,
                task.scheduled_ts,
                task.wait_ts,
                task.alloc_secs,
                task.template,
                task.recur,
                udas_json,
                now,
                task_id
            ],
        )?;
        
        // Update tags
        for tag in tags_to_add {
            // Check if tag already exists
            let exists: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM task_tags WHERE task_id = ?1 AND tag = ?2)",
                rusqlite::params![task_id, tag],
                |row| row.get(0),
            )?;
            
            if !exists {
                conn.execute(
                    "INSERT INTO task_tags (task_id, tag) VALUES (?1, ?2)",
                    rusqlite::params![task_id, tag],
                )?;
                // Record tag_added event
                EventRepo::record_tag_added(conn, task_id, tag)?;
            }
        }
        
        for tag in tags_to_remove {
            let existed: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM task_tags WHERE task_id = ?1 AND tag = ?2)",
                rusqlite::params![task_id, tag],
                |row| row.get(0),
            )?;
            
            if existed {
                conn.execute(
                    "DELETE FROM task_tags WHERE task_id = ?1 AND tag = ?2",
                    rusqlite::params![task_id, tag],
                )?;
                // Record tag_removed event
                EventRepo::record_tag_removed(conn, task_id, tag)?;
            }
        }
        
        Ok(())
    }

    /// Get tasks by IDs (for multi-task modification)
    pub fn get_by_ids(conn: &Connection, ids: &[i64]) -> Result<Vec<Task>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT id, uuid, description, status, project_id, due_ts, scheduled_ts, 
                    wait_ts, alloc_secs, template, recur, udas_json, created_ts, modified_ts 
             FROM tasks WHERE id IN ({})",
            placeholders
        );
        
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(ids.iter()), |row| {
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
        })?;
        
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }
        Ok(tasks)
    }

    /// Mark a task as completed
    pub fn complete(conn: &Connection, task_id: i64) -> Result<()> {
        // Get current status for event recording
        let old_task = Self::get_by_id(conn, task_id)?
            .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;
        let old_status = old_task.status.as_str();
        
        let now = chrono::Utc::now().timestamp();
        
        let rows_affected = conn.execute(
            "UPDATE tasks SET status = 'completed', modified_ts = ?1 WHERE id = ?2",
            rusqlite::params![now, task_id],
        )?;
        
        if rows_affected == 0 {
            anyhow::bail!("Task {} not found", task_id);
        }
        
        // Record status_changed event
        if old_status != "completed" {
            EventRepo::record_status_changed(conn, task_id, old_status, "completed")?;
        }
        
        Ok(())
    }

    /// Permanently delete a task and all related data
    /// 
    /// This operation is atomic - all related data is deleted in a transaction.
    /// Related data includes:
    /// - Task tags (CASCADE)
    /// - Task annotations (CASCADE)
    /// - Task sessions (CASCADE)
    /// - Stack items (CASCADE)
    /// - Task events (CASCADE)
    /// - Recurrence occurrences (CASCADE)
    /// 
    /// # Example
    /// 
    /// ```no_run
    /// use task_ninja::db::DbConnection;
    /// use task_ninja::repo::TaskRepo;
    /// 
    /// let conn = DbConnection::connect().unwrap();
    /// TaskRepo::delete(&conn, 5).unwrap();
    /// ```
    pub fn delete(conn: &Connection, task_id: i64) -> Result<()> {
        // Verify task exists
        let task = Self::get_by_id(conn, task_id)?
            .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;
        
        // Use transaction to ensure atomicity
        let tx = conn.unchecked_transaction()?;
        
        // Delete the task - CASCADE will handle related data automatically
        // But we'll also explicitly clean up to be safe and clear
        let rows_affected = tx.execute(
            "DELETE FROM tasks WHERE id = ?1",
            rusqlite::params![task_id],
        )?;
        
        if rows_affected == 0 {
            anyhow::bail!("Task {} not found", task_id);
        }
        
        // Commit transaction
        tx.commit()?;
        
        Ok(())
    }
}

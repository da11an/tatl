use rusqlite::{Connection, OptionalExtension};
use crate::models::External;
use crate::repo::TaskRepo;
use anyhow::{Context, Result};

pub struct ExternalRepo;

impl ExternalRepo {
    /// Create a new external record
    pub fn create(conn: &Connection, task_id: i64, recipient: String, request: Option<String>) -> Result<External> {
        let now = chrono::Utc::now().timestamp();
        let external = External {
            id: None,
            task_id,
            recipient: recipient.clone(),
            request: request.clone(),
            sent_ts: now,
            returned_ts: None,
            created_ts: now,
            modified_ts: now,
        };
        
        conn.execute(
            "INSERT INTO externals (task_id, recipient, request, sent_ts, returned_ts, created_ts, modified_ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                external.task_id,
                external.recipient,
                external.request,
                external.sent_ts,
                external.returned_ts,
                external.created_ts,
                external.modified_ts
            ],
        )
        .with_context(|| format!("Failed to create external record for task {} to {}", task_id, recipient))?;
        
        let id = conn.last_insert_rowid();

        // Touch activity_ts on the task
        TaskRepo::touch_activity(conn, task_id)?;

        let mut result = external;
        result.id = Some(id);
        Ok(result)
    }
    
    /// Get all active (unreturned) externals for a task
    pub fn get_active_for_task(conn: &Connection, task_id: i64) -> Result<Vec<External>> {
        let mut stmt = conn.prepare(
            "SELECT id, task_id, recipient, request, sent_ts, returned_ts, created_ts, modified_ts
             FROM externals
             WHERE task_id = ?1 AND returned_ts IS NULL
             ORDER BY sent_ts"
        )?;
        
        let externals = stmt.query_map([task_id], |row| {
            Ok(External {
                id: Some(row.get(0)?),
                task_id: row.get(1)?,
                recipient: row.get(2)?,
                request: row.get(3)?,
                sent_ts: row.get(4)?,
                returned_ts: row.get(5)?,
                created_ts: row.get(6)?,
                modified_ts: row.get(7)?,
            })
        })?;
        
        let mut result = Vec::new();
        for external in externals {
            result.push(external?);
        }
        Ok(result)
    }
    
    /// Get all active externals (across all tasks)
    pub fn get_all_active(conn: &Connection) -> Result<Vec<External>> {
        let mut stmt = conn.prepare(
            "SELECT id, task_id, recipient, request, sent_ts, returned_ts, created_ts, modified_ts
             FROM externals
             WHERE returned_ts IS NULL
             ORDER BY sent_ts"
        )?;
        
        let externals = stmt.query_map([], |row| {
            Ok(External {
                id: Some(row.get(0)?),
                task_id: row.get(1)?,
                recipient: row.get(2)?,
                request: row.get(3)?,
                sent_ts: row.get(4)?,
                returned_ts: row.get(5)?,
                created_ts: row.get(6)?,
                modified_ts: row.get(7)?,
            })
        })?;
        
        let mut result = Vec::new();
        for external in externals {
            result.push(external?);
        }
        Ok(result)
    }
    
    /// Get externals by recipient
    pub fn get_by_recipient(conn: &Connection, recipient: &str) -> Result<Vec<External>> {
        let mut stmt = conn.prepare(
            "SELECT id, task_id, recipient, request, sent_ts, returned_ts, created_ts, modified_ts
             FROM externals
             WHERE recipient = ?1 AND returned_ts IS NULL
             ORDER BY sent_ts"
        )?;
        
        let externals = stmt.query_map([recipient], |row| {
            Ok(External {
                id: Some(row.get(0)?),
                task_id: row.get(1)?,
                recipient: row.get(2)?,
                request: row.get(3)?,
                sent_ts: row.get(4)?,
                returned_ts: row.get(5)?,
                created_ts: row.get(6)?,
                modified_ts: row.get(7)?,
            })
        })?;
        
        let mut result = Vec::new();
        for external in externals {
            result.push(external?);
        }
        Ok(result)
    }
    
    /// Mark an external as returned
    pub fn mark_returned(conn: &Connection, external_id: i64) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let rows_affected = conn.execute(
            "UPDATE externals SET returned_ts = ?1, modified_ts = ?2 WHERE id = ?3",
            rusqlite::params![now, now, external_id],
        )
        .with_context(|| format!("Failed to mark external {} as returned", external_id))?;
        
        if rows_affected == 0 {
            anyhow::bail!("External {} not found", external_id);
        }
        
        Ok(())
    }
    
    /// Mark all externals for a task as returned
    pub fn mark_all_returned_for_task(conn: &Connection, task_id: i64) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "UPDATE externals SET returned_ts = ?1, modified_ts = ?2 WHERE task_id = ?3 AND returned_ts IS NULL",
            rusqlite::params![now, now, task_id],
        )
        .with_context(|| format!("Failed to mark externals for task {} as returned", task_id))?;

        // Touch activity_ts on the task
        TaskRepo::touch_activity(conn, task_id)?;

        Ok(())
    }

    /// Check if a task has any active externals
    pub fn has_active_externals(conn: &Connection, task_id: i64) -> Result<bool> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM externals WHERE task_id = ?1 AND returned_ts IS NULL",
            [task_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
    
    /// Get external by ID
    pub fn get_by_id(conn: &Connection, external_id: i64) -> Result<Option<External>> {
        let mut stmt = conn.prepare(
            "SELECT id, task_id, recipient, request, sent_ts, returned_ts, created_ts, modified_ts
             FROM externals
             WHERE id = ?1"
        )?;
        
        let external = stmt.query_row([external_id], |row| {
            Ok(External {
                id: Some(row.get(0)?),
                task_id: row.get(1)?,
                recipient: row.get(2)?,
                request: row.get(3)?,
                sent_ts: row.get(4)?,
                returned_ts: row.get(5)?,
                created_ts: row.get(6)?,
                modified_ts: row.get(7)?,
            })
        }).optional()?;
        
        Ok(external)
    }
}

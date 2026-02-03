use rusqlite::{Connection, OptionalExtension};
use crate::models::Annotation;
use crate::repo::{EventRepo, TaskRepo};
use anyhow::{Context, Result};

/// Annotation repository for database operations
///
/// Manages task annotations (timestamped notes) with support for:
/// - Creating annotations linked to tasks
/// - Linking annotations to sessions (optional)
/// - Querying annotations by task or session
/// - Deleting annotations
///
/// # Session Linking
///
/// Annotations can be linked to the session during which they were created.
/// This allows tracking work notes during specific time periods.
///
/// # Example
///
/// ```no_run
/// use tatl::db::DbConnection;
/// use tatl::repo::AnnotationRepo;
///
/// let conn = DbConnection::connect().unwrap();
/// let task_id = 1;
/// let session_id = Some(5);
/// let annotation = AnnotationRepo::create(
///     &conn,
///     task_id,
///     "Found the bug".to_string(),
///     session_id,
/// ).unwrap();
/// ```
pub struct AnnotationRepo;

impl AnnotationRepo {
    /// Create a new annotation
    pub fn create(conn: &Connection, task_id: i64, note: String, session_id: Option<i64>) -> Result<Annotation> {
        let now = chrono::Utc::now().timestamp();
        
        conn.execute(
            "INSERT INTO task_annotations (task_id, session_id, note, entry_ts, created_ts) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![task_id, session_id, note, now, now],
        )?;
        
        let id = conn.last_insert_rowid();
        
        // Record annotation_added event
        EventRepo::record_annotation_added(conn, task_id, id, session_id)?;

        // Touch activity_ts on the task
        TaskRepo::touch_activity(conn, task_id)?;

        Ok(Annotation {
            id: Some(id),
            task_id,
            session_id,
            note,
            entry_ts: now,
            created_ts: now,
        })
    }

    /// Get all annotations for a session, ordered by entry_ts (oldest first)
    pub fn get_by_session(conn: &Connection, session_id: i64) -> Result<Vec<Annotation>> {
        let mut stmt = conn.prepare(
            "SELECT id, task_id, session_id, note, entry_ts, created_ts 
             FROM task_annotations 
             WHERE session_id = ?1 
             ORDER BY entry_ts ASC"
        )?;
        
        let rows = stmt.query_map([session_id], |row| {
            Ok(Annotation {
                id: Some(row.get(0)?),
                task_id: row.get(1)?,
                session_id: row.get(2)?,
                note: row.get(3)?,
                entry_ts: row.get(4)?,
                created_ts: row.get(5)?,
            })
        })?;
        
        let mut annotations = Vec::new();
        for row in rows {
            annotations.push(row?);
        }
        Ok(annotations)
    }
    
    /// Get all annotations for a task, ordered by entry_ts (oldest first)
    pub fn get_by_task(conn: &Connection, task_id: i64) -> Result<Vec<Annotation>> {
        let mut stmt = conn.prepare(
            "SELECT id, task_id, session_id, note, entry_ts, created_ts 
             FROM task_annotations 
             WHERE task_id = ?1 
             ORDER BY entry_ts ASC"
        )?;
        
        let rows = stmt.query_map([task_id], |row| {
            Ok(Annotation {
                id: Some(row.get(0)?),
                task_id: row.get(1)?,
                session_id: row.get(2)?,
                note: row.get(3)?,
                entry_ts: row.get(4)?,
                created_ts: row.get(5)?,
            })
        })?;
        
        let mut annotations = Vec::new();
        for row in rows {
            annotations.push(row?);
        }
        Ok(annotations)
    }

    /// Get annotation by ID
    pub fn get_by_id(conn: &Connection, annotation_id: i64) -> Result<Option<Annotation>> {
        let mut stmt = conn.prepare(
            "SELECT id, task_id, session_id, note, entry_ts, created_ts 
             FROM task_annotations 
             WHERE id = ?1"
        )?;
        
        stmt.query_row([annotation_id], |row| {
            Ok(Annotation {
                id: Some(row.get(0)?),
                task_id: row.get(1)?,
                session_id: row.get(2)?,
                note: row.get(3)?,
                entry_ts: row.get(4)?,
                created_ts: row.get(5)?,
            })
        })
        .optional()
        .context("Failed to query annotation")
    }

    /// Delete an annotation
    pub fn delete(conn: &Connection, annotation_id: i64) -> Result<()> {
        let rows_affected = conn.execute(
            "DELETE FROM task_annotations WHERE id = ?1",
            [annotation_id],
        )?;
        
        if rows_affected == 0 {
            anyhow::bail!("Annotation {} not found", annotation_id);
        }
        
        Ok(())
    }

    /// Delete an annotation, verifying it belongs to the specified task
    pub fn delete_for_task(conn: &Connection, task_id: i64, annotation_id: i64) -> Result<()> {
        // Record event before deletion
        EventRepo::record_annotation_deleted(conn, task_id, annotation_id)?;
        
        let rows_affected = conn.execute(
            "DELETE FROM task_annotations WHERE id = ?1 AND task_id = ?2",
            rusqlite::params![annotation_id, task_id],
        )?;
        
        if rows_affected == 0 {
            // Check if annotation exists at all
            if Self::get_by_id(conn, annotation_id)?.is_some() {
                anyhow::bail!("Annotation {} does not belong to task {}", annotation_id, task_id);
            } else {
                anyhow::bail!("Annotation {} not found", annotation_id);
            }
        }

        // Touch activity_ts on the task
        TaskRepo::touch_activity(conn, task_id)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbConnection;
    use crate::repo::TaskRepo;

    #[test]
    fn test_create_annotation() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let task = TaskRepo::create(&conn, "Test task", None).unwrap();
        let task_id = task.id.unwrap();
        
        let annotation = AnnotationRepo::create(&conn, task_id, "Test note".to_string(), None).unwrap();
        
        assert_eq!(annotation.task_id, task_id);
        assert_eq!(annotation.note, "Test note");
        assert!(annotation.session_id.is_none());
        assert!(annotation.id.is_some());
    }

    #[test]
    fn test_create_annotation_with_session() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let task = TaskRepo::create(&conn, "Test task", None).unwrap();
        let task_id = task.id.unwrap();
        
        // Create a session first
        use crate::repo::SessionRepo;
        let start_ts = chrono::Utc::now().timestamp();
        let session = SessionRepo::create(&conn, task_id, start_ts).unwrap();
        let session_id = session.id.unwrap();
        
        // Create annotation linked to session
        let annotation = AnnotationRepo::create(&conn, task_id, "Note during session".to_string(), Some(session_id)).unwrap();
        
        assert_eq!(annotation.session_id, Some(session_id));
    }

    #[test]
    fn test_get_by_task() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let task = TaskRepo::create(&conn, "Test task", None).unwrap();
        let task_id = task.id.unwrap();
        
        AnnotationRepo::create(&conn, task_id, "First note".to_string(), None).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        AnnotationRepo::create(&conn, task_id, "Second note".to_string(), None).unwrap();
        
        let annotations = AnnotationRepo::get_by_task(&conn, task_id).unwrap();
        assert_eq!(annotations.len(), 2);
        assert_eq!(annotations[0].note, "First note");
        assert_eq!(annotations[1].note, "Second note");
    }

    #[test]
    fn test_delete_annotation() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let task = TaskRepo::create(&conn, "Test task", None).unwrap();
        let task_id = task.id.unwrap();
        
        let annotation = AnnotationRepo::create(&conn, task_id, "Test note".to_string(), None).unwrap();
        let annotation_id = annotation.id.unwrap();
        
        AnnotationRepo::delete(&conn, annotation_id).unwrap();
        
        let annotations = AnnotationRepo::get_by_task(&conn, task_id).unwrap();
        assert_eq!(annotations.len(), 0);
    }

    #[test]
    fn test_delete_for_task() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let task1 = TaskRepo::create(&conn, "Task 1", None).unwrap();
        let task2 = TaskRepo::create(&conn, "Task 2", None).unwrap();
        
        let annotation = AnnotationRepo::create(&conn, task1.id.unwrap(), "Note for task 1".to_string(), None).unwrap();
        let annotation_id = annotation.id.unwrap();
        
        // Should succeed
        AnnotationRepo::delete_for_task(&conn, task1.id.unwrap(), annotation_id).unwrap();
        
        // Try to delete with wrong task - should fail
        let annotation2 = AnnotationRepo::create(&conn, task1.id.unwrap(), "Another note".to_string(), None).unwrap();
        let result = AnnotationRepo::delete_for_task(&conn, task2.id.unwrap(), annotation2.id.unwrap());
        assert!(result.is_err());
    }
}

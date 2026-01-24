use rusqlite::{Connection, OptionalExtension};
use crate::models::Session;
use crate::repo::EventRepo;
use anyhow::{Context, Result};

/// Micro-session threshold (30 seconds)
/// Sessions shorter than this duration may be merged or purged based on subsequent activity
const MICRO_SECONDS: i64 = 30;

/// Session repository for database operations
///
/// Manages timing sessions for tasks, including:
/// - Creating and closing sessions
/// - Micro-session merge/purge rules
/// - Querying session history
pub struct SessionRepo;

impl SessionRepo {
    /// Create a new open session for a task
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `task_id` - ID of the task to start timing
    /// * `start_ts` - Start timestamp (Unix timestamp in UTC)
    ///
    /// # Returns
    /// The created `Session` object
    ///
    /// # Errors
    /// * Returns error if a session is already open (enforced by unique constraint)
    /// * Applies micro-session merge/purge rules if applicable
    ///
    /// # Micro-Session Rules
    ///
    /// When creating a new session, the system checks for recent micro-sessions (closed sessions
    /// shorter than 30 seconds that ended within the last 30 seconds):
    ///
    /// 1. **Merge**: If a micro-session exists for the same task, it is merged into the new session
    ///    (the new session's start time is set to the micro-session's original start time)
    ///
    /// 2. **Purge**: If a micro-session exists for a different task, it is deleted
    ///
    /// 3. **Preserve**: If no micro-session exists or it's outside the 30-second window, the
    ///    micro-session is preserved
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tatl::db::DbConnection;
    /// use tatl::repo::SessionRepo;
    /// use chrono::Utc;
    ///
    /// let conn = DbConnection::connect().unwrap();
    /// let task_id = 1;
    /// let start_ts = Utc::now().timestamp();
    /// let session = SessionRepo::create(&conn, task_id, start_ts).unwrap();
    /// ```
    pub fn create(conn: &Connection, task_id: i64, start_ts: i64) -> Result<Session> {
        let now = chrono::Utc::now().timestamp();
        
        // Check for recent micro-session that might need merge/purge
        if let Some(micro_session) = Self::get_recent_micro_session(conn, start_ts)? {
            let micro_end_ts = micro_session.end_ts.unwrap();
            let time_since_micro_end = start_ts - micro_end_ts;
            
            // Check if within MICRO seconds of micro-session end
            if time_since_micro_end >= 0 && time_since_micro_end <= MICRO_SECONDS {
                if micro_session.task_id == task_id {
                    // Merge: same task - merge micro-session into new session
                    let new_session_id = {
                        // Create the new session first
                        conn.execute(
                            "INSERT INTO sessions (task_id, start_ts, end_ts, created_ts) VALUES (?1, ?2, NULL, ?3)",
                            rusqlite::params![task_id, start_ts, now],
                        )
                        .map_err(|e| {
                            if e.to_string().contains("UNIQUE constraint") {
                                anyhow::anyhow!("A session is already running. Please clock out first.")
                            } else {
                                anyhow::anyhow!("Failed to create session: {}", e)
                            }
                        })?;
                        conn.last_insert_rowid()
                    };
                    
                    // Merge: update new session to start at micro-session's start time
                    Self::merge_micro_session(conn, &micro_session, new_session_id)?;
                    
                    println!("Merged micro-session (task {}, {}s) into new session (task {}, started at {}).", 
                        micro_session.task_id, 
                        micro_end_ts - micro_session.start_ts,
                        task_id,
                        micro_session.start_ts);
                    
                    return Ok(Session {
                        id: Some(new_session_id),
                        task_id,
                        start_ts: micro_session.start_ts, // Merged start time
                        end_ts: None,
                        created_ts: now,
                    });
                } else {
                    // Purge: different task - delete micro-session
                    Self::purge_micro_session(conn, micro_session.id.unwrap())?;
                    
                    println!("Purged micro-session (task {}, {}s) - different task (task {}) started within {} seconds.", 
                        micro_session.task_id,
                        micro_end_ts - micro_session.start_ts,
                        task_id,
                        MICRO_SECONDS);
                }
            }
        }
        
        // Normal session creation
        conn.execute(
            "INSERT INTO sessions (task_id, start_ts, end_ts, created_ts) VALUES (?1, ?2, NULL, ?3)",
            rusqlite::params![task_id, start_ts, now],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint") {
                anyhow::anyhow!("A session is already running. Please clock out first.")
            } else {
                anyhow::anyhow!("Failed to create session: {}", e)
            }
        })?;
        
        let id = conn.last_insert_rowid();
        
        // Record session_started event
        EventRepo::record_session_started(conn, task_id, id, start_ts)?;
        
        Ok(Session {
            id: Some(id),
            task_id,
            start_ts,
            end_ts: None,
            created_ts: now,
        })
    }

    /// Create a closed session (with both start and end times)
    ///
    /// # Errors
    /// Returns an error if end_ts <= start_ts (session must have positive duration)
    pub fn create_closed(conn: &Connection, task_id: i64, start_ts: i64, end_ts: i64) -> Result<Session> {
        if end_ts <= start_ts {
            return Err(anyhow::anyhow!(
                "Session end time ({}) must be after start time ({})",
                end_ts, start_ts
            ));
        }

        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO sessions (task_id, start_ts, end_ts, created_ts) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![task_id, start_ts, end_ts, now],
        )?;
        
        let id = conn.last_insert_rowid();
        Ok(Session {
            id: Some(id),
            task_id,
            start_ts,
            end_ts: Some(end_ts),
            created_ts: now,
        })
    }

    /// Get the currently open session (if any)
    pub fn get_open(conn: &Connection) -> Result<Option<Session>> {
        let mut stmt = conn.prepare(
            "SELECT id, task_id, start_ts, end_ts, created_ts FROM sessions WHERE end_ts IS NULL"
        )?;
        
        stmt.query_row([], |row| {
            Ok(Session {
                id: Some(row.get(0)?),
                task_id: row.get(1)?,
                start_ts: row.get(2)?,
                end_ts: row.get(3)?,
                created_ts: row.get(4)?,
            })
        })
        .optional()
        .context("Failed to query open session")
    }

    /// Close the currently open session
    /// Returns the closed session and whether it was a micro-session
    ///
    /// # Errors
    /// Returns an error if end_ts <= start_ts (session must have positive duration)
    pub fn close_open(conn: &Connection, end_ts: i64) -> Result<Option<Session>> {
        // Get the open session first
        let session_opt = Self::get_open(conn)?;

        if let Some(session) = session_opt {
            let session_id = session.id.unwrap();
            let duration = end_ts - session.start_ts;

            // Validate end time is after start time
            if duration <= 0 {
                return Err(anyhow::anyhow!(
                    "Session end time must be after start time (start: {}, end: {})",
                    session.start_ts, end_ts
                ));
            }

            // Update the session
            conn.execute(
                "UPDATE sessions SET end_ts = ?1 WHERE id = ?2",
                rusqlite::params![end_ts, session_id],
            )?;
            
            let closed_session = Session {
                id: Some(session_id),
                task_id: session.task_id,
                start_ts: session.start_ts,
                end_ts: Some(end_ts),
                created_ts: session.created_ts,
            };
            
            // Check if this is a micro-session and warn
            if duration < MICRO_SECONDS {
                eprintln!("Warning: Micro-session detected ({}s). This session may be merged or purged if another session starts within {} seconds.", duration, MICRO_SECONDS);
            }
            
            // Return the closed session
            Ok(Some(closed_session))
        } else {
            Ok(None)
        }
    }
    
    /// Get the most recent micro-session (closed within MICRO seconds)
    /// Returns the most recent closed session that ended within MICRO seconds of the given timestamp
    pub fn get_recent_micro_session(conn: &Connection, before_ts: i64) -> Result<Option<Session>> {
        // Look for sessions that ended within MICRO seconds before before_ts
        let cutoff_ts = before_ts - MICRO_SECONDS;
        
        let mut stmt = conn.prepare(
            "SELECT id, task_id, start_ts, end_ts, created_ts 
             FROM sessions 
             WHERE end_ts IS NOT NULL 
             AND end_ts >= ?1 
             AND end_ts <= ?2
             AND (end_ts - start_ts) < ?3
             ORDER BY end_ts DESC 
             LIMIT 1"
        )?;
        
        stmt.query_row(rusqlite::params![cutoff_ts, before_ts, MICRO_SECONDS], |row| {
            Ok(Session {
                id: Some(row.get(0)?),
                task_id: row.get(1)?,
                start_ts: row.get(2)?,
                end_ts: Some(row.get(3)?),
                created_ts: row.get(4)?,
            })
        })
        .optional()
        .context("Failed to query recent micro-session")
    }
    
    /// Merge a micro-session into an adjacent session
    /// The micro-session's start time becomes the start time of the adjacent session
    pub fn merge_micro_session(conn: &Connection, micro_session: &Session, adjacent_session_id: i64) -> Result<()> {
        // Update the adjacent session to start at the micro-session's start time
        conn.execute(
            "UPDATE sessions SET start_ts = ?1 WHERE id = ?2",
            rusqlite::params![micro_session.start_ts, adjacent_session_id],
        )?;
        
        // Delete the micro-session
        conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            rusqlite::params![micro_session.id.unwrap()],
        )?;
        
        Ok(())
    }
    
    /// Purge (delete) a micro-session
    pub fn purge_micro_session(conn: &Connection, micro_session_id: i64) -> Result<()> {
        conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            rusqlite::params![micro_session_id],
        )?;
        Ok(())
    }

    /// Get all sessions for a task, ordered by start time (newest first)
    pub fn get_by_task(conn: &Connection, task_id: i64) -> Result<Vec<Session>> {
        let mut stmt = conn.prepare(
            "SELECT id, task_id, start_ts, end_ts, created_ts 
             FROM sessions 
             WHERE task_id = ?1 
             ORDER BY start_ts DESC"
        )?;
        
        let rows = stmt.query_map([task_id], |row| {
            Ok(Session {
                id: Some(row.get(0)?),
                task_id: row.get(1)?,
                start_ts: row.get(2)?,
                end_ts: row.get(3)?,
                created_ts: row.get(4)?,
            })
        })?;
        
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    /// Amend the end time of a closed session (for overlap prevention)
    pub fn amend_end_time(conn: &Connection, session_id: i64, new_end_ts: i64) -> Result<()> {
        conn.execute(
            "UPDATE sessions SET end_ts = ?1 WHERE id = ?2 AND end_ts IS NOT NULL",
            rusqlite::params![new_end_ts, session_id],
        )?;
        Ok(())
    }

    /// Get the most recent closed session that ends at or after the given timestamp
    /// Used for overlap prevention - find sessions that might need end time amendment
    /// Get all sessions, ordered by start time (newest first)
    pub fn list_all(conn: &Connection) -> Result<Vec<Session>> {
        let mut stmt = conn.prepare(
            "SELECT id, task_id, start_ts, end_ts, created_ts FROM sessions ORDER BY start_ts DESC"
        )?;
        
        let rows = stmt.query_map([], |row| {
            Ok(Session {
                id: Some(row.get(0)?),
                task_id: row.get(1)?,
                start_ts: row.get(2)?,
                end_ts: row.get(3)?,
                created_ts: row.get(4)?,
            })
        })?;
        
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }
    
    /// Get the most recent session for a task (open or closed)
    pub fn get_most_recent_for_task(conn: &Connection, task_id: i64) -> Result<Option<Session>> {
        let mut stmt = conn.prepare(
            "SELECT id, task_id, start_ts, end_ts, created_ts FROM sessions 
             WHERE task_id = ?1 ORDER BY start_ts DESC LIMIT 1"
        )?;
        
        let session = stmt.query_row([task_id], |row| {
            Ok(Session {
                id: Some(row.get(0)?),
                task_id: row.get(1)?,
                start_ts: row.get(2)?,
                end_ts: row.get(3)?,
                created_ts: row.get(4)?,
            })
        }).optional()?;
        
        Ok(session)
    }
    
    pub fn get_recent_closed_after(conn: &Connection, before_ts: i64) -> Result<Vec<Session>> {
        let mut stmt = conn.prepare(
            "SELECT id, task_id, start_ts, end_ts, created_ts 
             FROM sessions 
             WHERE end_ts IS NOT NULL AND end_ts >= ?1 
             ORDER BY end_ts DESC 
             LIMIT 10"
        )?;
        
        let rows = stmt.query_map([before_ts], |row| {
            Ok(Session {
                id: Some(row.get(0)?),
                task_id: row.get(1)?,
                start_ts: row.get(2)?,
                end_ts: row.get(3)?,
                created_ts: row.get(4)?,
            })
        })?;
        
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    /// Get session by ID
    pub fn get_by_id(conn: &Connection, session_id: i64) -> Result<Option<Session>> {
        let mut stmt = conn.prepare(
            "SELECT id, task_id, start_ts, end_ts, created_ts FROM sessions WHERE id = ?1"
        )?;
        
        stmt.query_row([session_id], |row| {
            Ok(Session {
                id: Some(row.get(0)?),
                task_id: row.get(1)?,
                start_ts: row.get(2)?,
                end_ts: row.get(3)?,
                created_ts: row.get(4)?,
            })
        })
        .optional()
        .context("Failed to query session by ID")
    }

    /// Modify session start time
    pub fn modify_start_time(conn: &Connection, session_id: i64, new_start_ts: i64) -> Result<()> {
        conn.execute(
            "UPDATE sessions SET start_ts = ?1 WHERE id = ?2",
            rusqlite::params![new_start_ts, session_id],
        )?;
        Ok(())
    }

    /// Modify session end time
    /// Can set to None (make open) or Some(timestamp) (set end time)
    pub fn modify_end_time(conn: &Connection, session_id: i64, new_end_ts: Option<i64>) -> Result<()> {
        conn.execute(
            "UPDATE sessions SET end_ts = ?1 WHERE id = ?2",
            rusqlite::params![new_end_ts, session_id],
        )?;
        Ok(())
    }

    /// Delete a session
    /// Annotations linked to this session will have their session_id set to NULL (via ON DELETE SET NULL)
    pub fn delete(conn: &Connection, session_id: i64) -> Result<()> {
        conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            rusqlite::params![session_id],
        )?;
        Ok(())
    }

    /// Update both start and end times for a session
    /// 
    /// # Arguments
    /// * `conn` - Database connection
    /// * `session_id` - ID of session to update
    /// * `start_ts` - New start timestamp
    /// * `end_ts` - New end timestamp (None for open session)
    ///
    /// # Errors
    /// Returns an error if end_ts is Some and end_ts <= start_ts
    pub fn update_times(conn: &Connection, session_id: i64, start_ts: i64, end_ts: Option<i64>) -> Result<()> {
        // Validate end time is after start time (for closed sessions)
        if let Some(end) = end_ts {
            if end <= start_ts {
                return Err(anyhow::anyhow!(
                    "Session end time ({}) must be after start time ({})",
                    end, start_ts
                ));
            }
        }

        conn.execute(
            "UPDATE sessions SET start_ts = ?1, end_ts = ?2 WHERE id = ?3",
            rusqlite::params![start_ts, end_ts, session_id],
        )?;
        Ok(())
    }

    /// Find sessions that overlap with the given time range
    /// 
    /// # Arguments
    /// * `conn` - Database connection
    /// * `task_id` - Task ID for the session being checked
    /// * `start_ts` - Start timestamp
    /// * `end_ts` - End timestamp (None for open session)
    /// * `exclude_session_id` - Session ID to exclude from results (for modification checks)
    /// 
    /// # Returns
    /// Vector of overlapping sessions
    /// 
    /// # Overlap Rules
    /// - Two closed sessions overlap if: (start1 < end2) && (end1 > start2)
    /// - An open session conflicts with any session that starts before it would end
    /// - Only one open session allowed at a time
    pub fn find_overlapping_sessions(
        conn: &Connection,
        _task_id: i64,
        start_ts: i64,
        end_ts: Option<i64>,
        exclude_session_id: Option<i64>,
    ) -> Result<Vec<Session>> {
        let mut overlapping = Vec::new();
        
        // Get all sessions (excluding the one being modified if specified)
        let all_sessions = if let Some(exclude_id) = exclude_session_id {
            let mut stmt = conn.prepare(
                "SELECT id, task_id, start_ts, end_ts, created_ts FROM sessions WHERE id != ?1"
            )?;
            let rows = stmt.query_map([exclude_id], |row| {
                Ok(Session {
                    id: Some(row.get(0)?),
                    task_id: row.get(1)?,
                    start_ts: row.get(2)?,
                    end_ts: row.get(3)?,
                    created_ts: row.get(4)?,
                })
            })?;
            let mut sessions = Vec::new();
            for row in rows {
                sessions.push(row?);
            }
            sessions
        } else {
            Self::list_all(conn)?
        };
        
        // Check for overlaps
        for session in all_sessions {
            let overlaps = if let Some(session_end) = session.end_ts {
                // Both sessions are closed
                if let Some(check_end) = end_ts {
                    // Both closed: (start1 < end2) && (end1 > start2)
                    (start_ts < session_end) && (check_end > session.start_ts)
                } else {
                    // Check session is closed, new session is open
                    // Open session conflicts if it starts before the closed session ends
                    start_ts < session_end
                }
            } else {
                // Session is open
                if let Some(check_end) = end_ts {
                    // Session is open, check session is closed
                    // Open session conflicts if it starts before the closed session ends
                    session.start_ts < check_end
                } else {
                    // Both are open - only one open session allowed
                    true
                }
            };
            
            if overlaps {
                overlapping.push(session);
            }
        }
        
        Ok(overlapping)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbConnection;
    use crate::repo::TaskRepo;

    #[test]
    fn test_create_session() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let task = TaskRepo::create(&conn, "Test task", None).unwrap();
        let task_id = task.id.unwrap();
        
        let start_ts = chrono::Utc::now().timestamp();
        let session = SessionRepo::create(&conn, task_id, start_ts).unwrap();
        
        assert_eq!(session.task_id, task_id);
        assert_eq!(session.start_ts, start_ts);
        assert!(session.is_open());
        assert!(session.id.is_some());
    }

    #[test]
    fn test_only_one_open_session() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let task1 = TaskRepo::create(&conn, "Task 1", None).unwrap();
        let task2 = TaskRepo::create(&conn, "Task 2", None).unwrap();
        
        let start_ts = chrono::Utc::now().timestamp();
        SessionRepo::create(&conn, task1.id.unwrap(), start_ts).unwrap();
        
        // Try to create another open session - should fail
        let result = SessionRepo::create(&conn, task2.id.unwrap(), start_ts + 100);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already running"));
    }

    #[test]
    fn test_get_open_session() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let task = TaskRepo::create(&conn, "Test task", None).unwrap();
        
        // No open session initially
        let open = SessionRepo::get_open(&conn).unwrap();
        assert!(open.is_none());
        
        // Create a session
        let start_ts = chrono::Utc::now().timestamp();
        let session = SessionRepo::create(&conn, task.id.unwrap(), start_ts).unwrap();
        
        // Should find the open session
        let open = SessionRepo::get_open(&conn).unwrap();
        assert!(open.is_some());
        assert_eq!(open.unwrap().id, session.id);
    }

    #[test]
    fn test_close_session() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let task = TaskRepo::create(&conn, "Test task", None).unwrap();
        
        let start_ts = chrono::Utc::now().timestamp();
        SessionRepo::create(&conn, task.id.unwrap(), start_ts).unwrap();
        
        // Close the session
        let end_ts = start_ts + 3600;
        let closed = SessionRepo::close_open(&conn, end_ts).unwrap();
        assert!(closed.is_some());
        assert_eq!(closed.unwrap().end_ts, Some(end_ts));
        
        // No open session after closing
        let open = SessionRepo::get_open(&conn).unwrap();
        assert!(open.is_none());
    }

    #[test]
    fn test_create_closed_session() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let task = TaskRepo::create(&conn, "Test task", None).unwrap();
        
        let start_ts = chrono::Utc::now().timestamp();
        let end_ts = start_ts + 3600;
        
        let session = SessionRepo::create_closed(&conn, task.id.unwrap(), start_ts, end_ts).unwrap();
        assert_eq!(session.start_ts, start_ts);
        assert_eq!(session.end_ts, Some(end_ts));
        assert!(!session.is_open());
    }

    #[test]
    fn test_get_by_task() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let task = TaskRepo::create(&conn, "Test task", None).unwrap();
        let task_id = task.id.unwrap();
        
        let start1 = chrono::Utc::now().timestamp();
        let end1 = start1 + 100;
        SessionRepo::create_closed(&conn, task_id, start1, end1).unwrap();
        
        let start2 = start1 + 200;
        let end2 = start2 + 100;
        SessionRepo::create_closed(&conn, task_id, start2, end2).unwrap();
        
        let sessions = SessionRepo::get_by_task(&conn, task_id).unwrap();
        assert_eq!(sessions.len(), 2);
        // Should be ordered newest first
        assert_eq!(sessions[0].start_ts, start2);
        assert_eq!(sessions[1].start_ts, start1);
    }

    #[test]
    fn test_amend_end_time() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let task = TaskRepo::create(&conn, "Test task", None).unwrap();
        
        let start_ts = chrono::Utc::now().timestamp();
        let end_ts = start_ts + 3600;
        let session = SessionRepo::create_closed(&conn, task.id.unwrap(), start_ts, end_ts).unwrap();
        
        // Amend the end time
        let new_end_ts = start_ts + 1800;
        SessionRepo::amend_end_time(&conn, session.id.unwrap(), new_end_ts).unwrap();
        
        // Verify the change
        let sessions = SessionRepo::get_by_task(&conn, task.id.unwrap()).unwrap();
        assert_eq!(sessions[0].end_ts, Some(new_end_ts));
    }

    #[test]
    fn test_utc_storage() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let task = TaskRepo::create(&conn, "Test task", None).unwrap();
        
        // Use a specific UTC timestamp
        let start_ts = 1704067200; // 2024-01-01 00:00:00 UTC
        let session = SessionRepo::create(&conn, task.id.unwrap(), start_ts).unwrap();
        
        // Verify it's stored as-is (UTC)
        let open = SessionRepo::get_open(&conn).unwrap().unwrap();
        assert_eq!(open.start_ts, start_ts);
    }
}

use rusqlite::{Connection, OptionalExtension};
use crate::models::Project;
use anyhow::{Context, Result};

/// Project repository for database operations
///
/// Manages projects with support for:
/// - Creating projects (including nested projects via dot notation)
/// - Querying projects by name or ID
/// - Renaming/merging projects
/// - Archiving projects
///
/// # Nested Projects
///
/// Projects can be nested using dot notation (e.g., `admin.email`, `sales.northamerica`).
/// The hierarchy is implicit - no explicit parent-child relationships are stored.
/// Filtering by `project=admin` matches `admin`, `admin.email`, `admin.other`, etc.
///
/// # Example
///
/// ```no_run
/// use tatl::db::DbConnection;
/// use tatl::repo::ProjectRepo;
///
/// let conn = DbConnection::connect().unwrap();
/// let project = ProjectRepo::create(&conn, "work").unwrap();
/// let nested = ProjectRepo::create(&conn, "work.email").unwrap();
/// ```
pub struct ProjectRepo;

impl ProjectRepo {
    /// Create a new project
    pub fn create(conn: &Connection, name: &str) -> Result<Project> {
        let project = Project::new(name.to_string());
        let now = chrono::Utc::now().timestamp();
        
        conn.execute(
            "INSERT INTO projects (name, is_archived, created_ts, modified_ts) 
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                project.name,
                if project.is_archived { 1 } else { 0 },
                now,
                now
            ],
        )
        .with_context(|| format!("Failed to create project: {}", name))?;
        
        let id = conn.last_insert_rowid();
        Ok(Project {
            id: Some(id),
            ..project
        })
    }

    /// Get project by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> Result<Option<Project>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, is_archived, created_ts, modified_ts 
             FROM projects WHERE id = ?1"
        )?;
        
        let project = stmt.query_row([id], |row| {
            Ok(Project {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                is_archived: row.get::<_, i64>(2)? != 0,
                created_ts: row.get(3)?,
                modified_ts: row.get(4)?,
            })
        }).optional()?;
        
        Ok(project)
    }
    
    /// Get project by name
    pub fn get_by_name(conn: &Connection, name: &str) -> Result<Option<Project>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, is_archived, created_ts, modified_ts 
             FROM projects WHERE name = ?1"
        )?;
        
        let project = stmt.query_row([name], |row| {
            Ok(Project {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                is_archived: row.get::<_, i64>(2)? != 0,
                created_ts: row.get(3)?,
                modified_ts: row.get(4)?,
            })
        }).optional()?;
        
        Ok(project)
    }

    /// List all projects (optionally filtered by archived status)
    pub fn list(conn: &Connection, include_archived: bool) -> Result<Vec<Project>> {
        let query = if include_archived {
            "SELECT id, name, is_archived, created_ts, modified_ts 
             FROM projects ORDER BY name"
        } else {
            "SELECT id, name, is_archived, created_ts, modified_ts 
             FROM projects WHERE is_archived = 0 ORDER BY name"
        };
        
        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map([], |row| {
            Ok(Project {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                is_archived: row.get::<_, i64>(2)? != 0,
                created_ts: row.get(3)?,
                modified_ts: row.get(4)?,
            })
        })?;
        
        let mut projects = Vec::new();
        for row in rows {
            projects.push(row?);
        }
        
        Ok(projects)
    }

    /// Rename a project
    pub fn rename(conn: &Connection, old_name: &str, new_name: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        
        conn.execute(
            "UPDATE projects SET name = ?1, modified_ts = ?2 WHERE name = ?3",
            rusqlite::params![new_name, now, old_name],
        )
        .with_context(|| format!("Failed to rename project from {} to {}", old_name, new_name))?;
        
        // Update all tasks referencing this project
        conn.execute(
            "UPDATE tasks SET modified_ts = ?1 
             WHERE project_id = (SELECT id FROM projects WHERE name = ?2)",
            rusqlite::params![now, new_name],
        )?;
        
        Ok(())
    }

    /// Merge two projects (move tasks from old to new, delete old)
    pub fn merge(conn: &Connection, old_name: &str, new_name: &str) -> Result<()> {
        let tx = conn.unchecked_transaction()?;
        
        // Get project IDs
        let old_id: i64 = tx.query_row(
            "SELECT id FROM projects WHERE name = ?1",
            [old_name],
            |row| row.get(0),
        )
        .with_context(|| format!("Project '{}' not found", old_name))?;
        
        let new_id: i64 = tx.query_row(
            "SELECT id FROM projects WHERE name = ?1",
            [new_name],
            |row| row.get(0),
        )
        .with_context(|| format!("Project '{}' not found", new_name))?;
        
        // Get archive status of both projects
        let old_archived: i64 = tx.query_row(
            "SELECT is_archived FROM projects WHERE id = ?1",
            [old_id],
            |row| row.get(0),
        )?;
        
        let new_archived: i64 = tx.query_row(
            "SELECT is_archived FROM projects WHERE id = ?1",
            [new_id],
            |row| row.get(0),
        )?;
        
        // Move all tasks from old to new project
        let now = chrono::Utc::now().timestamp();
        tx.execute(
            "UPDATE tasks SET project_id = ?1, modified_ts = ?2 WHERE project_id = ?3",
            rusqlite::params![new_id, now, old_id],
        )?;
        
        // If old is active and new is archived, keep new archived
        // If old is archived and new is active, merged project becomes active
        // If both same, keep that status
        let final_archived = if old_archived == 0 && new_archived == 1 {
            1  // Keep archived
        } else {
            0  // Make active (if old was active or both were active)
        };
        
        // Update new project's archive status if needed
        if new_archived != final_archived {
            tx.execute(
                "UPDATE projects SET is_archived = ?1, modified_ts = ?2 WHERE id = ?3",
                rusqlite::params![final_archived, now, new_id],
            )?;
        }
        
        // Delete old project
        tx.execute("DELETE FROM projects WHERE id = ?1", [old_id])?;
        
        tx.commit()?;
        Ok(())
    }

    /// Archive a project
    pub fn archive(conn: &Connection, name: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        
        let rows_affected = conn.execute(
            "UPDATE projects SET is_archived = 1, modified_ts = ?1 WHERE name = ?2",
            rusqlite::params![now, name],
        )?;
        
        if rows_affected == 0 {
            anyhow::bail!("Project '{}' not found", name);
        }
        
        Ok(())
    }

    /// Unarchive a project
    pub fn unarchive(conn: &Connection, name: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        
        let rows_affected = conn.execute(
            "UPDATE projects SET is_archived = 0, modified_ts = ?1 WHERE name = ?2",
            rusqlite::params![now, name],
        )?;
        
        if rows_affected == 0 {
            anyhow::bail!("Project '{}' not found", name);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbConnection;

    #[test]
    fn test_create_project() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let project = ProjectRepo::create(&conn, "work").unwrap();
        
        assert_eq!(project.name, "work");
        assert!(!project.is_archived);
        assert!(project.id.is_some());
    }

    #[test]
    fn test_create_duplicate_project() {
        let conn = DbConnection::connect_in_memory().unwrap();
        ProjectRepo::create(&conn, "work").unwrap();
        
        // Should fail due to unique constraint
        let result = ProjectRepo::create(&conn, "work");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_by_name() {
        let conn = DbConnection::connect_in_memory().unwrap();
        ProjectRepo::create(&conn, "work").unwrap();
        
        let project = ProjectRepo::get_by_name(&conn, "work").unwrap();
        assert!(project.is_some());
        assert_eq!(project.unwrap().name, "work");
        
        let missing = ProjectRepo::get_by_name(&conn, "nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_list_projects() {
        let conn = DbConnection::connect_in_memory().unwrap();
        ProjectRepo::create(&conn, "work").unwrap();
        ProjectRepo::create(&conn, "home").unwrap();
        
        let projects = ProjectRepo::list(&conn, false).unwrap();
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].name, "home");
        assert_eq!(projects[1].name, "work");
    }

    #[test]
    fn test_list_with_archived() {
        let conn = DbConnection::connect_in_memory().unwrap();
        ProjectRepo::create(&conn, "work").unwrap();
        ProjectRepo::create(&conn, "old").unwrap();
        ProjectRepo::archive(&conn, "old").unwrap();
        
        let active = ProjectRepo::list(&conn, false).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "work");
        
        let all = ProjectRepo::list(&conn, true).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_rename_project() {
        let conn = DbConnection::connect_in_memory().unwrap();
        ProjectRepo::create(&conn, "work").unwrap();
        ProjectRepo::rename(&conn, "work", "office").unwrap();
        
        let project = ProjectRepo::get_by_name(&conn, "office").unwrap();
        assert!(project.is_some());
        
        let old = ProjectRepo::get_by_name(&conn, "work").unwrap();
        assert!(old.is_none());
    }

    #[test]
    fn test_merge_projects() {
        let conn = DbConnection::connect_in_memory().unwrap();
        
        // Create projects
        let work = ProjectRepo::create(&conn, "work").unwrap();
        let temp = ProjectRepo::create(&conn, "temp").unwrap();
        
        // Create tasks
        use crate::repo::task::TaskRepo;
        let task1 = TaskRepo::create(&conn, "Task 1", Some(work.id.unwrap())).unwrap();
        let task2 = TaskRepo::create(&conn, "Task 2", Some(temp.id.unwrap())).unwrap();
        
        // Merge temp into work
        ProjectRepo::merge(&conn, "temp", "work").unwrap();
        
        // Verify tasks moved
        let task1_updated = TaskRepo::get_by_id(&conn, task1.id.unwrap()).unwrap().unwrap();
        let task2_updated = TaskRepo::get_by_id(&conn, task2.id.unwrap()).unwrap().unwrap();
        assert_eq!(task1_updated.project_id, work.id);
        assert_eq!(task2_updated.project_id, work.id);
        
        // Verify temp project deleted
        let temp_project = ProjectRepo::get_by_name(&conn, "temp").unwrap();
        assert!(temp_project.is_none());
    }

    #[test]
    fn test_archive_unarchive() {
        let conn = DbConnection::connect_in_memory().unwrap();
        ProjectRepo::create(&conn, "work").unwrap();
        
        ProjectRepo::archive(&conn, "work").unwrap();
        let project = ProjectRepo::get_by_name(&conn, "work").unwrap().unwrap();
        assert!(project.is_archived);
        
        ProjectRepo::unarchive(&conn, "work").unwrap();
        let project = ProjectRepo::get_by_name(&conn, "work").unwrap().unwrap();
        assert!(!project.is_archived);
    }
}

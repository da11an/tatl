use rusqlite::{Connection, Result};
use std::collections::HashMap;

/// Current database schema version
const CURRENT_VERSION: u32 = 5;

/// Migration system for managing database schema versions
pub struct MigrationManager;

impl MigrationManager {
    /// Initialize the database with the current schema
    /// This creates the schema_version table and applies all migrations
    pub fn initialize(conn: &Connection) -> Result<()> {
        // Create schema_version table to track migrations
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY
            )",
            [],
        )?;

        // Get current version
        let current_version: u32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Apply migrations up to current version
        for version in (current_version + 1)..=CURRENT_VERSION {
            Self::apply_migration(conn, version)?;
        }

        Ok(())
    }

    /// Apply a specific migration by version number
    fn apply_migration(conn: &Connection, version: u32) -> Result<()> {
        let migrations = get_migrations();
        if let Some(migration) = migrations.get(&version) {
            // For migrations that need to disable foreign keys (like table recreation),
            // we must set the PRAGMA before starting the transaction
            if version == 5 {
                conn.execute("PRAGMA foreign_keys=OFF", [])?;
            }
            
            // Execute migration in a transaction
            let tx = conn.unchecked_transaction()?;
            migration(&tx)?;
            tx.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                [version],
            )?;
            tx.commit()?;
            
            // Re-enable foreign keys after transaction completes
            if version == 5 {
                conn.execute("PRAGMA foreign_keys=ON", [])?;
            }
            
            Ok(())
        } else {
            Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_MISUSE),
                Some(format!("No migration found for version {}", version)),
            ))
        }
    }

    /// Get the current schema version
    pub fn get_version(conn: &Connection) -> Result<u32> {
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
    }
}

/// Get all migrations indexed by version
fn get_migrations() -> HashMap<u32, fn(&rusqlite::Transaction) -> Result<(), rusqlite::Error>> {
    let mut migrations: HashMap<u32, fn(&rusqlite::Transaction) -> Result<(), rusqlite::Error>> = HashMap::new();
    migrations.insert(1, migration_v1);
    migrations.insert(2, migration_v2);
    migrations.insert(3, migration_v3);
    migrations.insert(4, migration_v4);
    migrations.insert(5, migration_v5);
    migrations
}

/// Migration v1: Initial schema
fn migration_v1(tx: &rusqlite::Transaction) -> Result<(), rusqlite::Error> {
    // Enable foreign keys
    tx.execute("PRAGMA foreign_keys=ON", [])?;

    // Projects table
    tx.execute(
        "CREATE TABLE projects (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            is_archived INTEGER NOT NULL DEFAULT 0,
            created_ts INTEGER NOT NULL,
            modified_ts INTEGER NOT NULL
        )",
        [],
    )?;
    // Note: Nested projects use dot notation in the name field (e.g., 'admin.email', 'sales.northamerica.texas').
    // The hierarchy is implicit - no explicit parent-child relationships are stored.

    // Tasks table
    tx.execute(
        "CREATE TABLE tasks (
            id INTEGER PRIMARY KEY,
            uuid TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('pending','completed','deleted')),
            project_id INTEGER NULL REFERENCES projects(id),
            due_ts INTEGER NULL,
            scheduled_ts INTEGER NULL,
            wait_ts INTEGER NULL,
            alloc_secs INTEGER NULL,
            template TEXT NULL,
            recur TEXT NULL,
            udas_json TEXT NULL,
            created_ts INTEGER NOT NULL,
            modified_ts INTEGER NOT NULL
        )",
        [],
    )?;
    // Note: udas_json stores JSON object: {\"key\": \"value\", ...} - keys stored without \"uda.\" prefix
    
    // Create indexes on commonly queried task columns
    tx.execute(
        "CREATE INDEX idx_tasks_project_id ON tasks(project_id)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_tasks_status ON tasks(status)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_tasks_due_ts ON tasks(due_ts)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_tasks_scheduled_ts ON tasks(scheduled_ts)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_tasks_wait_ts ON tasks(wait_ts)",
        [],
    )?;

    // Task tags table
    tx.execute(
        "CREATE TABLE task_tags (
            task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            tag TEXT NOT NULL,
            PRIMARY KEY(task_id, tag)
        )",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_task_tags_tag ON task_tags(tag)",
        [],
    )?;

    // Task annotations table
    tx.execute(
        "CREATE TABLE task_annotations (
            id INTEGER PRIMARY KEY,
            task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            session_id INTEGER NULL REFERENCES sessions(id) ON DELETE SET NULL,
            note TEXT NOT NULL,
            entry_ts INTEGER NOT NULL,
            created_ts INTEGER NOT NULL
        )",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_task_annotations_task_entry ON task_annotations(task_id, entry_ts)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_task_annotations_session ON task_annotations(session_id)",
        [],
    )?;
    // Note: session_id links annotation to the session during which it was created (if applicable)

    // Stacks table
    tx.execute(
        "CREATE TABLE stacks (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            created_ts INTEGER NOT NULL,
            modified_ts INTEGER NOT NULL
        )",
        [],
    )?;
    // Note: The default stack (name='default') is auto-created on first stack operation.
    // No explicit initialization or migration is required.

    // Stack items table
    tx.execute(
        "CREATE TABLE stack_items (
            stack_id INTEGER NOT NULL REFERENCES stacks(id) ON DELETE CASCADE,
            task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            ordinal INTEGER NOT NULL,
            added_ts INTEGER NOT NULL,
            PRIMARY KEY(stack_id, task_id),
            UNIQUE(stack_id, ordinal)
        )",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_stack_items_stack_ordinal ON stack_items(stack_id, ordinal)",
        [],
    )?;

    // Sessions table
    tx.execute(
        "CREATE TABLE sessions (
            id INTEGER PRIMARY KEY,
            task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            start_ts INTEGER NOT NULL,
            end_ts INTEGER NULL,
            created_ts INTEGER NOT NULL
        )",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_sessions_task_start ON sessions(task_id, start_ts)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_sessions_open ON sessions(end_ts) WHERE end_ts IS NULL",
        [],
    )?;
    // Note: Session notes are handled via task annotations linked to the session

    // Enforce single open session via partial unique index
    // SQLite supports partial unique indexes
    tx.execute(
        "CREATE UNIQUE INDEX ux_sessions_single_open ON sessions(1) WHERE end_ts IS NULL",
        [],
    )?;

    // Task events table (immutable audit log)
    tx.execute(
        "CREATE TABLE task_events (
            id INTEGER PRIMARY KEY,
            task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            ts INTEGER NOT NULL,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL
        )",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_task_events_task_ts ON task_events(task_id, ts)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_task_events_type ON task_events(event_type)",
        [],
    )?;
    // Note: Task events are the immutable history of what happens to tasks.
    // This can be used to reconstruct task history, and analysis.

    // Templates table
    tx.execute(
        "CREATE TABLE templates (
            name TEXT PRIMARY KEY,
            payload_json TEXT NOT NULL,
            created_ts INTEGER NOT NULL,
            modified_ts INTEGER NOT NULL
        )",
        [],
    )?;

    // Recur occurrences table
    tx.execute(
        "CREATE TABLE recur_occurrences (
            seed_task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            occurrence_ts INTEGER NOT NULL,
            instance_task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            PRIMARY KEY(seed_task_id, occurrence_ts)
        )",
        [],
    )?;

    Ok(())
}

/// Migration v2: Add 'closed' task status
fn migration_v2(tx: &rusqlite::Transaction) -> Result<(), rusqlite::Error> {
    // Disable foreign keys temporarily for table rebuild
    tx.execute("PRAGMA foreign_keys=OFF", [])?;
    
    // Recreate tasks table with updated status constraint
    tx.execute(
        "CREATE TABLE tasks_new (
            id INTEGER PRIMARY KEY,
            uuid TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('pending','completed','closed','deleted')),
            project_id INTEGER NULL REFERENCES projects(id),
            due_ts INTEGER NULL,
            scheduled_ts INTEGER NULL,
            wait_ts INTEGER NULL,
            alloc_secs INTEGER NULL,
            template TEXT NULL,
            recur TEXT NULL,
            udas_json TEXT NULL,
            created_ts INTEGER NOT NULL,
            modified_ts INTEGER NOT NULL
        )",
        [],
    )?;
    
    tx.execute(
        "INSERT INTO tasks_new (id, uuid, description, status, project_id, due_ts, scheduled_ts, 
                wait_ts, alloc_secs, template, recur, udas_json, created_ts, modified_ts)
         SELECT id, uuid, description, status, project_id, due_ts, scheduled_ts,
                wait_ts, alloc_secs, template, recur, udas_json, created_ts, modified_ts
         FROM tasks",
        [],
    )?;
    
    tx.execute("DROP TABLE tasks", [])?;
    tx.execute("ALTER TABLE tasks_new RENAME TO tasks", [])?;
    
    // Recreate indexes
    tx.execute(
        "CREATE INDEX idx_tasks_project_id ON tasks(project_id)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_tasks_status ON tasks(status)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_tasks_due_ts ON tasks(due_ts)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_tasks_scheduled_ts ON tasks(scheduled_ts)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX idx_tasks_wait_ts ON tasks(wait_ts)",
        [],
    )?;
    
    // Re-enable foreign keys
    tx.execute("PRAGMA foreign_keys=ON", [])?;
    Ok(())
}

/// Migration v3: Add list views table
fn migration_v3(tx: &rusqlite::Transaction) -> Result<(), rusqlite::Error> {
    tx.execute(
        "CREATE TABLE list_views (
            name TEXT PRIMARY KEY,
            entity TEXT NOT NULL,
            filter_json TEXT NOT NULL,
            sort_json TEXT NOT NULL,
            group_json TEXT NOT NULL,
            created_ts INTEGER NOT NULL,
            modified_ts INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(())
}

/// Migration v4: Add hide_json column to list_views
fn migration_v4(tx: &rusqlite::Transaction) -> Result<(), rusqlite::Error> {
    tx.execute(
        "ALTER TABLE list_views ADD COLUMN hide_json TEXT NOT NULL DEFAULT '[]'",
        [],
    )?;
    Ok(())
}

/// Migration v5: Rename recur to respawn, drop recur_occurrences
/// 
/// This migration transitions from the time-based "recurrence" model to the
/// completion-based "respawn" model. The recur_occurrences table is no longer
/// needed as respawn happens on task completion, not via batch generation.
fn migration_v5(tx: &rusqlite::Transaction) -> Result<(), rusqlite::Error> {
    // Drop the recur_occurrences table (no longer needed for respawn model)
    tx.execute("DROP TABLE IF EXISTS recur_occurrences", [])?;
    
    // SQLite doesn't support direct column rename before 3.25.0,
    // so we recreate the table with the renamed column.
    // Note: PRAGMA foreign_keys=OFF must be set BEFORE the transaction starts
    // (handled in apply_migration), otherwise CASCADE deletes will trigger.
    
    tx.execute(
        "CREATE TABLE tasks_new (
            id INTEGER PRIMARY KEY,
            uuid TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('pending','completed','closed','deleted')),
            project_id INTEGER NULL REFERENCES projects(id),
            due_ts INTEGER NULL,
            scheduled_ts INTEGER NULL,
            wait_ts INTEGER NULL,
            alloc_secs INTEGER NULL,
            template TEXT NULL,
            respawn TEXT NULL,
            udas_json TEXT NULL,
            created_ts INTEGER NOT NULL,
            modified_ts INTEGER NOT NULL
        )",
        [],
    )?;
    
    tx.execute(
        "INSERT INTO tasks_new (id, uuid, description, status, project_id, due_ts, scheduled_ts, 
                wait_ts, alloc_secs, template, respawn, udas_json, created_ts, modified_ts)
         SELECT id, uuid, description, status, project_id, due_ts, scheduled_ts,
                wait_ts, alloc_secs, template, recur, udas_json, created_ts, modified_ts
         FROM tasks",
        [],
    )?;
    
    tx.execute("DROP TABLE tasks", [])?;
    tx.execute("ALTER TABLE tasks_new RENAME TO tasks", [])?;
    
    // Recreate indexes
    tx.execute("CREATE INDEX idx_tasks_project_id ON tasks(project_id)", [])?;
    tx.execute("CREATE INDEX idx_tasks_status ON tasks(status)", [])?;
    tx.execute("CREATE INDEX idx_tasks_due_ts ON tasks(due_ts)", [])?;
    tx.execute("CREATE INDEX idx_tasks_scheduled_ts ON tasks(scheduled_ts)", [])?;
    tx.execute("CREATE INDEX idx_tasks_wait_ts ON tasks(wait_ts)", [])?;
    
    // Note: PRAGMA foreign_keys=ON is restored by apply_migration after commit
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_migration_applies_cleanly() {
        let conn = Connection::open_in_memory().unwrap();
        MigrationManager::initialize(&conn).unwrap();
        
        let version = MigrationManager::get_version(&conn).unwrap();
        assert_eq!(version, CURRENT_VERSION);
    }

    #[test]
    fn test_migration_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        
        // Apply migration twice
        MigrationManager::initialize(&conn).unwrap();
        MigrationManager::initialize(&conn).unwrap();
        
        let version = MigrationManager::get_version(&conn).unwrap();
        assert_eq!(version, CURRENT_VERSION);
    }

    #[test]
    fn test_foreign_key_constraints() {
        let conn = Connection::open_in_memory().unwrap();
        MigrationManager::initialize(&conn).unwrap();
        
        // Try to insert a task with invalid project_id
        let result = conn.execute(
            "INSERT INTO tasks (uuid, description, status, project_id, created_ts, modified_ts) 
             VALUES ('test-uuid', 'Test', 'pending', 999, 1000, 1000)",
            [],
        );
        
        // Should fail due to foreign key constraint
        assert!(result.is_err());
    }

    #[test]
    fn test_single_open_session_constraint() {
        let conn = Connection::open_in_memory().unwrap();
        MigrationManager::initialize(&conn).unwrap();
        
        // Create a task first
        conn.execute(
            "INSERT INTO tasks (uuid, description, status, created_ts, modified_ts) 
             VALUES ('uuid1', 'Task 1', 'pending', 1000, 1000)",
            [],
        ).unwrap();
        
        let task_id: i64 = conn.last_insert_rowid();
        
        // Create first open session
        conn.execute(
            "INSERT INTO sessions (task_id, start_ts, created_ts) 
             VALUES (?1, 1000, 1000)",
            [task_id],
        ).unwrap();
        
        // Try to create second open session - should fail
        let result = conn.execute(
            "INSERT INTO sessions (task_id, start_ts, created_ts)
             VALUES (?1, 2000, 2000)",
            [task_id],
        );
        
        // Should fail due to unique constraint
        assert!(result.is_err());
    }

    #[test]
    fn test_migration_v5_preserves_sessions() {
        // This test verifies that migration_v5 (recur -> respawn rename)
        // does not trigger CASCADE deletes on sessions table.
        let conn = Connection::open_in_memory().unwrap();
        
        // Apply migrations 1-4 only
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY)",
            [],
        ).unwrap();
        
        for version in 1..=4 {
            let migrations = get_migrations();
            if let Some(migration) = migrations.get(&version) {
                let tx = conn.unchecked_transaction().unwrap();
                migration(&tx).unwrap();
                tx.execute("INSERT INTO schema_version (version) VALUES (?1)", [version]).unwrap();
                tx.commit().unwrap();
            }
        }
        
        // Create a task with the old 'recur' column
        conn.execute(
            "INSERT INTO tasks (uuid, description, status, recur, created_ts, modified_ts) 
             VALUES ('uuid1', 'Task 1', 'pending', 'daily', 1000, 1000)",
            [],
        ).unwrap();
        let task_id: i64 = conn.last_insert_rowid();
        
        // Create a session for this task
        conn.execute(
            "INSERT INTO sessions (task_id, start_ts, end_ts, created_ts) 
             VALUES (?1, 1000, 2000, 1000)",
            [task_id],
        ).unwrap();
        
        // Verify session exists
        let session_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(session_count, 1, "Should have 1 session before migration");
        
        // Now apply migration v5 (this is where the bug was - CASCADE deletes)
        // Must disable foreign keys BEFORE starting transaction
        conn.execute("PRAGMA foreign_keys=OFF", []).unwrap();
        let tx = conn.unchecked_transaction().unwrap();
        migration_v5(&tx).unwrap();
        tx.execute("INSERT INTO schema_version (version) VALUES (5)", []).unwrap();
        tx.commit().unwrap();
        conn.execute("PRAGMA foreign_keys=ON", []).unwrap();
        
        // Verify session still exists after migration
        let session_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(session_count, 1, "Session should be preserved after migration v5");
        
        // Verify the respawn column exists and has the migrated value
        let respawn: Option<String> = conn.query_row(
            "SELECT respawn FROM tasks WHERE id = ?1",
            [task_id],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(respawn, Some("daily".to_string()), "Respawn value should be migrated from recur");
    }
}

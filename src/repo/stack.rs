use rusqlite::{Connection, OptionalExtension};
use crate::models::{Stack, StackItem};
use crate::repo::{EventRepo, TaskRepo, ExternalRepo};
use anyhow::Result;

/// Stack repository for database operations
///
/// Manages the work queue (stack) with operations for:
/// - Enqueueing tasks (add to end)
/// - Pushing tasks to top (do it now)
/// - Picking tasks from any position
/// - Rolling/rotating the stack
/// - Dropping tasks from stack
/// - Clearing the stack
///
/// The default stack (name='default') is auto-created on first operation.
///
/// # Example
///
/// ```no_run
/// use tatl::db::DbConnection;
/// use tatl::repo::{StackRepo, TaskRepo};
///
/// let conn = DbConnection::connect().unwrap();
/// let stack = StackRepo::get_or_create_default(&conn).unwrap();
/// let task = TaskRepo::create(&conn, "New task", None).unwrap();
/// StackRepo::enqueue(&conn, stack.id.unwrap(), task.id.unwrap()).unwrap();
/// ```
pub struct StackRepo;

impl StackRepo {
    /// Get or create the default stack
    /// Auto-creates the default stack if it doesn't exist
    pub fn get_or_create_default(conn: &Connection) -> Result<Stack> {
        // Try to get existing default stack
        let mut stmt = conn.prepare("SELECT id, name, created_ts, modified_ts FROM stacks WHERE name = 'default'")?;
        let stack_opt = stmt.query_row([], |row| {
            Ok(Stack {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                created_ts: row.get(2)?,
                modified_ts: row.get(3)?,
            })
        }).optional()?;
        
        if let Some(stack) = stack_opt {
            return Ok(stack);
        }
        
        // Create default stack
        let stack = Stack::default();
        let now = chrono::Utc::now().timestamp();
        
        conn.execute(
            "INSERT INTO stacks (name, created_ts, modified_ts) VALUES (?1, ?2, ?3)",
            rusqlite::params![stack.name, now, now],
        )?;
        
        let id = conn.last_insert_rowid();
        Ok(Stack {
            id: Some(id),
            ..stack
        })
    }

    /// Get stack items ordered by ordinal
    pub fn get_items(conn: &Connection, stack_id: i64) -> Result<Vec<StackItem>> {
        let mut stmt = conn.prepare(
            "SELECT stack_id, task_id, ordinal, added_ts 
             FROM stack_items WHERE stack_id = ?1 ORDER BY ordinal"
        )?;
        
        let rows = stmt.query_map([stack_id], |row| {
            Ok(StackItem {
                stack_id: row.get(0)?,
                task_id: row.get(1)?,
                ordinal: row.get(2)?,
                added_ts: row.get(3)?,
            })
        })?;
        
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    /// Validate that a task is eligible for queue insertion via enqueue.
    /// Rejects terminal tasks and external-waiting tasks.
    /// Note: push_to_top() does NOT call this — it is used by the `on` command
    /// for temporarily working on external tasks.
    fn validate_enqueue_eligibility(conn: &Connection, task_id: i64) -> Result<()> {
        let task = TaskRepo::get_by_id(conn, task_id)?
            .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;

        if task.status.is_terminal() {
            anyhow::bail!("Cannot queue task {}: status is {}", task_id, task.status.as_str());
        }

        if ExternalRepo::has_active_externals(conn, task_id)? {
            anyhow::bail!(
                "Cannot queue task {}: waiting on external party. Use 'collect' first, or 'on {}' to work on it temporarily.",
                task_id, task_id
            );
        }

        Ok(())
    }

    /// Add task to end of stack (enqueue)
    pub fn enqueue(conn: &Connection, stack_id: i64, task_id: i64) -> Result<()> {
        // Validate eligibility (terminal tasks and external-waiting tasks are rejected)
        Self::validate_enqueue_eligibility(conn, task_id)?;

        // Check if task already in stack
        let existing: Option<i32> = conn.query_row(
            "SELECT ordinal FROM stack_items WHERE stack_id = ?1 AND task_id = ?2",
            rusqlite::params![stack_id, task_id],
            |row| row.get(0),
        ).ok();
        
        if let Some(_) = existing {
            // Task already in stack - move to end
            Self::move_to_end(conn, stack_id, task_id)?;
            return Ok(());
        }
        
        // Get current max ordinal
        let max_ordinal: i32 = conn.query_row(
            "SELECT COALESCE(MAX(ordinal), -1) FROM stack_items WHERE stack_id = ?1",
            [stack_id],
            |row| row.get(0),
        ).unwrap_or(-1);
        
        let new_ordinal = max_ordinal + 1;
        let now = chrono::Utc::now().timestamp();
        
        conn.execute(
            "INSERT INTO stack_items (stack_id, task_id, ordinal, added_ts) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![stack_id, task_id, new_ordinal, now],
        )?;
        
        // Record stack_added event
        EventRepo::record_stack_added(conn, task_id, stack_id, new_ordinal)?;
        
        Self::update_modified(conn, stack_id)?;
        Ok(())
    }

    /// Move task to top of stack (push)
    pub fn push_to_top(conn: &Connection, stack_id: i64, task_id: i64) -> Result<()> {
        // Check if task already in stack
        let existing_ordinal: Option<i32> = conn.query_row(
            "SELECT ordinal FROM stack_items WHERE stack_id = ?1 AND task_id = ?2",
            rusqlite::params![stack_id, task_id],
            |row| row.get(0),
        ).ok();
        
        // If task exists, remove it first to avoid constraint issues
        if existing_ordinal.is_some() {
            conn.execute(
                "DELETE FROM stack_items WHERE stack_id = ?1 AND task_id = ?2",
                rusqlite::params![stack_id, task_id],
            )?;
        }
        
        // Shift all items down by 1 (using negative ordinals temporarily to avoid conflicts)
        conn.execute(
            "UPDATE stack_items SET ordinal = -(ordinal + 1) WHERE stack_id = ?1",
            [stack_id],
        )?;
        
        // Insert at position 0
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO stack_items (stack_id, task_id, ordinal, added_ts) VALUES (?1, ?2, 0, ?3)",
            rusqlite::params![stack_id, task_id, now],
        )?;
        
        // Convert negative ordinals back to positive and renumber
        conn.execute(
            "UPDATE stack_items SET ordinal = -ordinal WHERE stack_id = ?1 AND ordinal < 0",
            [stack_id],
        )?;
        
        // Renumber to ensure clean ordinals
        Self::renumber(conn, stack_id)?;
        Self::update_modified(conn, stack_id)?;
        Ok(())
    }

    /// Move task at index to top (pick)
    pub fn pick(conn: &Connection, stack_id: i64, index: i32) -> Result<()> {
        // Clamp index
        let item_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM stack_items WHERE stack_id = ?1",
            [stack_id],
            |row| row.get(0),
        )?;
        
        let clamped_index = if index < 0 {
            (item_count + index).max(0)
        } else {
            index.min(item_count - 1).max(0)
        };
        
        // Get task_id at that position
        let task_id: i64 = conn.query_row(
            "SELECT task_id FROM stack_items WHERE stack_id = ?1 AND ordinal = ?2",
            rusqlite::params![stack_id, clamped_index],
            |row| row.get(0),
        )?;
        
        // Move to top
        Self::push_to_top(conn, stack_id, task_id)?;
        Ok(())
    }

    /// Rotate stack by n positions
    /// If called within a transaction, the transaction should be passed as conn
    pub fn roll(conn: &Connection, stack_id: i64, n: i32) -> Result<()> {
        let items = Self::get_items(conn, stack_id)?;
        if items.len() <= 1 {
            return Ok(()); // Nothing to rotate
        }
        
        let item_count = items.len() as i32;
        let effective_n = n % item_count;
        if effective_n == 0 {
            return Ok(()); // No rotation needed
        }
        
        // Store current ordinals and task_ids
        let mut task_ordinals: Vec<(i64, i32)> = Vec::new();
        for item in &items {
            task_ordinals.push((item.task_id, item.ordinal));
        }
        
        // Clear all items temporarily
        conn.execute("DELETE FROM stack_items WHERE stack_id = ?1", [stack_id])?;
        
        // Reinsert with new ordinals
        // Roll: [a,b,c] with roll 1 becomes [b,c,a]
        // This is a left rotation: each item moves left by n positions
        // Item at position i moves to position (i - n) mod count
        let now = chrono::Utc::now().timestamp();
        for (task_id, old_ordinal) in task_ordinals {
            let new_ordinal = if effective_n > 0 {
                // Left rotation: (i - n) mod count
                (old_ordinal - effective_n + item_count) % item_count
            } else {
                // Right rotation (negative n): (i + |n|) mod count
                let abs_n = (-effective_n) % item_count;
                (old_ordinal + abs_n) % item_count
            };
            
            conn.execute(
                "INSERT INTO stack_items (stack_id, task_id, ordinal, added_ts) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![stack_id, task_id, new_ordinal, now],
            )?;
        }
        
        // Renumber to ensure clean ordinals (within same transaction if applicable)
        Self::renumber(conn, stack_id)?;
        Self::update_modified(conn, stack_id)?;
        Ok(())
    }

    /// Remove task at index (drop)
    pub fn drop(conn: &Connection, stack_id: i64, index: i32) -> Result<()> {
        // Clamp index
        let item_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM stack_items WHERE stack_id = ?1",
            [stack_id],
            |row| row.get(0),
        )?;
        
        let clamped_index = if index < 0 {
            (item_count + index).max(0)
        } else {
            index.min(item_count - 1).max(0)
        };
        
        // Get task_id before deletion for event recording
        let task_id: Option<i64> = conn.query_row(
            "SELECT task_id FROM stack_items WHERE stack_id = ?1 AND ordinal = ?2",
            rusqlite::params![stack_id, clamped_index],
            |row| row.get(0),
        ).ok();
        
        // Delete item
        conn.execute(
            "DELETE FROM stack_items WHERE stack_id = ?1 AND ordinal = ?2",
            rusqlite::params![stack_id, clamped_index],
        )?;
        
        // Record stack_removed event
        if let Some(task_id) = task_id {
            EventRepo::record_stack_removed(conn, task_id, stack_id)?;
        }
        
        // Renumber remaining items
        Self::renumber(conn, stack_id)?;
        Self::update_modified(conn, stack_id)?;
        Ok(())
    }

    /// Clear all items from stack
    pub fn clear(conn: &Connection, stack_id: i64) -> Result<()> {
        conn.execute(
            "DELETE FROM stack_items WHERE stack_id = ?1",
            [stack_id],
        )?;
        Self::update_modified(conn, stack_id)?;
        Ok(())
    }

    /// Remove a specific task from the stack by task_id
    pub fn remove_task(conn: &Connection, stack_id: i64, task_id: i64) -> Result<()> {
        // Delete the item
        conn.execute(
            "DELETE FROM stack_items WHERE stack_id = ?1 AND task_id = ?2",
            rusqlite::params![stack_id, task_id],
        )?;
        
        // Record stack_removed event
        EventRepo::record_stack_removed(conn, task_id, stack_id)?;
        
        // Renumber remaining items
        Self::renumber(conn, stack_id)?;
        Self::update_modified(conn, stack_id)?;
        Ok(())
    }

    /// Move task to end of stack
    fn move_to_end(conn: &Connection, stack_id: i64, task_id: i64) -> Result<()> {
        // Remove from current position
        conn.execute(
            "DELETE FROM stack_items WHERE stack_id = ?1 AND task_id = ?2",
            rusqlite::params![stack_id, task_id],
        )?;
        
        // Get new max ordinal
        let max_ordinal: i32 = conn.query_row(
            "SELECT COALESCE(MAX(ordinal), -1) FROM stack_items WHERE stack_id = ?1",
            [stack_id],
            |row| row.get(0),
        ).unwrap_or(-1);
        
        // Add at end
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO stack_items (stack_id, task_id, ordinal, added_ts) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![stack_id, task_id, max_ordinal + 1, now],
        )?;
        
        Self::renumber(conn, stack_id)?;
        Ok(())
    }

    /// Renumber stack items to ensure clean ordinals (0, 1, 2, ...)
    fn renumber(conn: &Connection, stack_id: i64) -> Result<()> {
        let items = Self::get_items(conn, stack_id)?;
        
        for (new_ordinal, item) in items.iter().enumerate() {
            conn.execute(
                "UPDATE stack_items SET ordinal = ?1 WHERE stack_id = ?2 AND task_id = ?3",
                rusqlite::params![new_ordinal as i32, stack_id, item.task_id],
            )?;
        }
        Ok(())
    }

    /// Update stack modified timestamp
    fn update_modified(conn: &Connection, stack_id: i64) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "UPDATE stacks SET modified_ts = ?1 WHERE id = ?2",
            rusqlite::params![now, stack_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbConnection;
    use crate::repo::TaskRepo;

    #[test]
    fn test_get_or_create_default() {
        let conn = DbConnection::connect_in_memory().unwrap();
        
        // First call creates stack
        let stack1 = StackRepo::get_or_create_default(&conn).unwrap();
        assert_eq!(stack1.name, "default");
        assert!(stack1.id.is_some());
        
        // Second call returns same stack
        let stack2 = StackRepo::get_or_create_default(&conn).unwrap();
        assert_eq!(stack1.id, stack2.id);
    }

    #[test]
    fn test_enqueue() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let stack = StackRepo::get_or_create_default(&conn).unwrap();
        let stack_id = stack.id.unwrap();
        
        let task1 = TaskRepo::create(&conn, "Task 1", None).unwrap();
        let task2 = TaskRepo::create(&conn, "Task 2", None).unwrap();
        
        StackRepo::enqueue(&conn, stack_id, task1.id.unwrap()).unwrap();
        StackRepo::enqueue(&conn, stack_id, task2.id.unwrap()).unwrap();
        
        let items = StackRepo::get_items(&conn, stack_id).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].task_id, task1.id.unwrap());
        assert_eq!(items[1].task_id, task2.id.unwrap());
    }

    #[test]
    fn test_push_to_top() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let stack = StackRepo::get_or_create_default(&conn).unwrap();
        let stack_id = stack.id.unwrap();
        
        let task1 = TaskRepo::create(&conn, "Task 1", None).unwrap();
        let task2 = TaskRepo::create(&conn, "Task 2", None).unwrap();
        let task3 = TaskRepo::create(&conn, "Task 3", None).unwrap();
        
        StackRepo::enqueue(&conn, stack_id, task1.id.unwrap()).unwrap();
        StackRepo::enqueue(&conn, stack_id, task2.id.unwrap()).unwrap();
        StackRepo::push_to_top(&conn, stack_id, task3.id.unwrap()).unwrap();
        
        let items = StackRepo::get_items(&conn, stack_id).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].task_id, task3.id.unwrap());
        assert_eq!(items[1].task_id, task1.id.unwrap());
        assert_eq!(items[2].task_id, task2.id.unwrap());
    }

    #[test]
    fn test_pick() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let stack = StackRepo::get_or_create_default(&conn).unwrap();
        let stack_id = stack.id.unwrap();
        
        let task1 = TaskRepo::create(&conn, "Task 1", None).unwrap();
        let task2 = TaskRepo::create(&conn, "Task 2", None).unwrap();
        let task3 = TaskRepo::create(&conn, "Task 3", None).unwrap();
        
        StackRepo::enqueue(&conn, stack_id, task1.id.unwrap()).unwrap();
        StackRepo::enqueue(&conn, stack_id, task2.id.unwrap()).unwrap();
        StackRepo::enqueue(&conn, stack_id, task3.id.unwrap()).unwrap();
        
        // Pick task at index 2 (task3)
        StackRepo::pick(&conn, stack_id, 2).unwrap();
        
        let items = StackRepo::get_items(&conn, stack_id).unwrap();
        assert_eq!(items[0].task_id, task3.id.unwrap());
    }

    #[test]
    fn test_roll() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let stack = StackRepo::get_or_create_default(&conn).unwrap();
        let stack_id = stack.id.unwrap();
        
        let task1 = TaskRepo::create(&conn, "Task 1", None).unwrap();
        let task2 = TaskRepo::create(&conn, "Task 2", None).unwrap();
        let task3 = TaskRepo::create(&conn, "Task 3", None).unwrap();
        
        StackRepo::enqueue(&conn, stack_id, task1.id.unwrap()).unwrap();
        StackRepo::enqueue(&conn, stack_id, task2.id.unwrap()).unwrap();
        StackRepo::enqueue(&conn, stack_id, task3.id.unwrap()).unwrap();
        
        // Roll once
        StackRepo::roll(&conn, stack_id, 1).unwrap();
        
        let items = StackRepo::get_items(&conn, stack_id).unwrap();
        assert_eq!(items[0].task_id, task2.id.unwrap());
        assert_eq!(items[1].task_id, task3.id.unwrap());
        assert_eq!(items[2].task_id, task1.id.unwrap());
    }

    #[test]
    fn test_drop() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let stack = StackRepo::get_or_create_default(&conn).unwrap();
        let stack_id = stack.id.unwrap();
        
        let task1 = TaskRepo::create(&conn, "Task 1", None).unwrap();
        let task2 = TaskRepo::create(&conn, "Task 2", None).unwrap();
        let task3 = TaskRepo::create(&conn, "Task 3", None).unwrap();
        
        StackRepo::enqueue(&conn, stack_id, task1.id.unwrap()).unwrap();
        StackRepo::enqueue(&conn, stack_id, task2.id.unwrap()).unwrap();
        StackRepo::enqueue(&conn, stack_id, task3.id.unwrap()).unwrap();
        
        // Drop task at index 1
        StackRepo::drop(&conn, stack_id, 1).unwrap();
        
        let items = StackRepo::get_items(&conn, stack_id).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].task_id, task1.id.unwrap());
        assert_eq!(items[1].task_id, task3.id.unwrap());
    }

    #[test]
    fn test_clear() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let stack = StackRepo::get_or_create_default(&conn).unwrap();
        let stack_id = stack.id.unwrap();
        
        let task1 = TaskRepo::create(&conn, "Task 1", None).unwrap();
        StackRepo::enqueue(&conn, stack_id, task1.id.unwrap()).unwrap();
        
        StackRepo::clear(&conn, stack_id).unwrap();
        
        let items = StackRepo::get_items(&conn, stack_id).unwrap();
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_index_clamping() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let stack = StackRepo::get_or_create_default(&conn).unwrap();
        let stack_id = stack.id.unwrap();
        
        let task1 = TaskRepo::create(&conn, "Task 1", None).unwrap();
        StackRepo::enqueue(&conn, stack_id, task1.id.unwrap()).unwrap();
        
        // Test out-of-range index (should clamp to 0)
        StackRepo::pick(&conn, stack_id, 10).unwrap();
        let items = StackRepo::get_items(&conn, stack_id).unwrap();
        assert_eq!(items[0].task_id, task1.id.unwrap());
        
        // Test negative index (should clamp to 0)
        StackRepo::pick(&conn, stack_id, -5).unwrap();
        let items = StackRepo::get_items(&conn, stack_id).unwrap();
        assert_eq!(items[0].task_id, task1.id.unwrap());
    }
}

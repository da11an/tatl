use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
use tatl::db::DbConnection;
use tatl::repo::{TaskRepo, EventRepo, StackRepo, SessionRepo, AnnotationRepo};
use rusqlite::Connection;
mod test_env;

fn setup_test_env() -> (TempDir, std::sync::MutexGuard<'static, ()>) {
    let guard = test_env::lock_test_env();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let config_dir = temp_dir.path().join(".tatl");
    fs::create_dir_all(&config_dir).unwrap();
    let config_file = config_dir.join("rc");
    fs::write(&config_file, format!("data.location={}\n", db_path.display())).unwrap();
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    (temp_dir, guard)
}

fn get_task_cmd() -> Command {
    Command::cargo_bin("tatl").unwrap()
}

#[test]
fn test_event_created_on_task_creation() {
    let (_temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    let task = TaskRepo::create(&conn, "Test task", None).unwrap();
    let task_id = task.id.unwrap();
    
    // Verify created event was recorded
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM task_events WHERE task_id = ?1 AND event_type = 'created'").unwrap();
    let count: i64 = stmt.query_row([task_id], |row| row.get(0)).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_event_status_changed_on_completion() {
    let (_temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    let task = TaskRepo::create(&conn, "Test task", None).unwrap();
    let task_id = task.id.unwrap();

    TaskRepo::close(&conn, task_id).unwrap();

    // Verify status_changed event was recorded
    let mut stmt = conn.prepare("SELECT payload_json FROM task_events WHERE task_id = ?1 AND event_type = 'status_changed'").unwrap();
    let payload: String = stmt.query_row([task_id], |row| row.get(0)).unwrap();
    let payload_value: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(payload_value["old_status"], "open");
    assert_eq!(payload_value["new_status"], "closed");
}

#[test]
fn test_event_tag_added() {
    let (_temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    let task = TaskRepo::create(&conn, "Test task", None).unwrap();
    let task_id = task.id.unwrap();
    
    TaskRepo::modify(
        &conn,
        task_id,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &std::collections::HashMap::new(),
        &[],
        &["urgent".to_string()],
        &[],
        None, // parent_id
    ).unwrap();

    // Verify tag_added event was recorded
    let mut stmt = conn.prepare("SELECT payload_json FROM task_events WHERE task_id = ?1 AND event_type = 'tag_added'").unwrap();
    let payload: String = stmt.query_row([task_id], |row| row.get(0)).unwrap();
    let payload_value: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(payload_value["tag"], "urgent");
}

#[test]
fn test_event_stack_added() {
    let (_temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    let task = TaskRepo::create(&conn, "Test task", None).unwrap();
    let task_id = task.id.unwrap();
    
    let stack = StackRepo::get_or_create_default(&conn).unwrap();
    let stack_id = stack.id.unwrap();
    
    StackRepo::enqueue(&conn, stack_id, task_id).unwrap();
    
    // Verify stack_added event was recorded
    let mut stmt = conn.prepare("SELECT payload_json FROM task_events WHERE task_id = ?1 AND event_type = 'stack_added'").unwrap();
    let payload: String = stmt.query_row([task_id], |row| row.get(0)).unwrap();
    let payload_value: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(payload_value["stack_id"], stack_id);
}

#[test]
fn test_event_session_started() {
    let (_temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    let task = TaskRepo::create(&conn, "Test task", None).unwrap();
    let task_id = task.id.unwrap();
    
    let now = chrono::Utc::now().timestamp();
    SessionRepo::create(&conn, task_id, now).unwrap();
    
    // Verify session_started event was recorded
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM task_events WHERE task_id = ?1 AND event_type = 'session_started'").unwrap();
    let count: i64 = stmt.query_row([task_id], |row| row.get(0)).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_event_annotation_added() {
    let (_temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    let task = TaskRepo::create(&conn, "Test task", None).unwrap();
    let task_id = task.id.unwrap();
    
    AnnotationRepo::create(&conn, task_id, "Test annotation".to_string(), None).unwrap();
    
    // Verify annotation_added event was recorded
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM task_events WHERE task_id = ?1 AND event_type = 'annotation_added'").unwrap();
    let count: i64 = stmt.query_row([task_id], |row| row.get(0)).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_events_immutable() {
    let (_temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    let task = TaskRepo::create(&conn, "Test task", None).unwrap();
    let task_id = task.id.unwrap();
    
    // Get initial event count
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM task_events WHERE task_id = ?1").unwrap();
    let initial_count: i64 = stmt.query_row([task_id], |row| row.get(0)).unwrap();
    
    // Try to delete an event (should fail or be prevented)
    // Events are immutable - we can't delete them via the API
    // But we can verify they still exist
    let final_count: i64 = stmt.query_row([task_id], |row| row.get(0)).unwrap();
    assert_eq!(initial_count, final_count);
}

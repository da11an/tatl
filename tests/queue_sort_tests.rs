use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
use tatl::db::DbConnection;
use tatl::repo::{TaskRepo, StackRepo};
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

fn get_task_cmd(temp_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("tatl").unwrap();
    cmd.env("HOME", temp_dir.path());
    cmd
}

#[test]
fn test_queue_sort_by_due_date() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks with different due dates
    get_task_cmd(&temp_dir)
        .args(&["add", "Task 1", "due=2026-01-25"])
        .assert()
        .success();
    get_task_cmd(&temp_dir)
        .args(&["add", "Task 2", "due=2026-01-20"])
        .assert()
        .success();
    get_task_cmd(&temp_dir)
        .args(&["add", "Task 3", "due=2026-01-30"])
        .assert()
        .success();
    
    // Enqueue all tasks
    get_task_cmd(&temp_dir)
        .args(&["enqueue", "1,2,3"])
        .assert()
        .success();
    
    // Sort by due date (ascending - earliest first)
    get_task_cmd(&temp_dir)
        .args(&["queue", "sort", "due"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Queue sorted by due"));
    
    // Verify order (should be 2, 1, 3)
    let conn = DbConnection::connect().unwrap();
    let stack = StackRepo::get_or_create_default(&conn).unwrap();
    let items = StackRepo::get_items(&conn, stack.id.unwrap()).unwrap();
    assert_eq!(items[0].task_id, 2, "First task should be task 2 (earliest due)");
    assert_eq!(items[1].task_id, 1, "Second task should be task 1");
    assert_eq!(items[2].task_id, 3, "Third task should be task 3 (latest due)");
}

#[test]
fn test_queue_sort_by_priority_descending() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks with priority UDAs
    get_task_cmd(&temp_dir)
        .args(&["add", "Task 1", "uda.priority=3"])
        .assert()
        .success();
    get_task_cmd(&temp_dir)
        .args(&["add", "Task 2", "uda.priority=1"])
        .assert()
        .success();
    get_task_cmd(&temp_dir)
        .args(&["add", "Task 3", "uda.priority=5"])
        .assert()
        .success();
    
    // Enqueue all tasks
    get_task_cmd(&temp_dir)
        .args(&["enqueue", "1,2,3"])
        .assert()
        .success();
    
    // Sort by priority descending (highest first)
    // Use -- to pass -priority as a value
    get_task_cmd(&temp_dir)
        .args(&["queue", "sort", "--", "-priority"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Queue sorted by priority"));
    
    // Verify order (should be 3, 1, 2)
    let conn = DbConnection::connect().unwrap();
    let stack = StackRepo::get_or_create_default(&conn).unwrap();
    let items = StackRepo::get_items(&conn, stack.id.unwrap()).unwrap();
    assert_eq!(items[0].task_id, 3, "First task should be task 3 (highest priority)");
    assert_eq!(items[1].task_id, 1, "Second task should be task 1");
    assert_eq!(items[2].task_id, 2, "Third task should be task 2 (lowest priority)");
}

#[test]
fn test_queue_sort_rejects_invalid_field() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create and enqueue a task
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    
    // Try to sort by invalid field
    get_task_cmd(&temp_dir)
        .args(&["queue", "sort", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid sort field"));
}

#[test]
fn test_queue_sort_empty_queue() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Try to sort empty queue
    get_task_cmd(&temp_dir)
        .args(&["queue", "sort", "due"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Queue is empty"));
}

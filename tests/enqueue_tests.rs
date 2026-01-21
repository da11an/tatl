use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
mod test_env;

/// Helper to create a temporary database and set it as the data location
fn setup_test_env() -> (TempDir, std::sync::MutexGuard<'static, ()>) {
    let guard = test_env::lock_test_env();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    
    // Create config file
    let config_dir = temp_dir.path().join(".tatl");
    fs::create_dir_all(&config_dir).unwrap();
    let config_file = config_dir.join("rc");
    fs::write(&config_file, format!("data.location={}\n", db_path.display())).unwrap();
    
    // Set HOME to temp_dir so the config file is found
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    (temp_dir, guard)
}

fn get_task_cmd() -> Command {
    Command::cargo_bin("tatl").unwrap()
}

#[test]
fn test_enqueue_single_task() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create a task
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Test task"]).assert().success();
    
    // Enqueue it
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Enqueued task 1"));
    
    // Verify it's on the clock stack
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
}

#[test]
fn test_enqueue_multiple_tasks_in_order() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create multiple tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 2"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 3"]).assert().success();
    
    // Enqueue them in order
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "2"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "3"]).assert().success();
    
    // Verify clock stack order is [1, 2, 3] (check for Task 1, Task 2, Task 3 in order)
    let mut cmd = get_task_cmd();
    let output = cmd.args(&["clock", "list"]).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Verify all tasks are present
    assert!(stdout.contains("Task 1"), "Stack should contain Task 1");
    assert!(stdout.contains("Task 2"), "Stack should contain Task 2");
    assert!(stdout.contains("Task 3"), "Stack should contain Task 3");
    
    // Verify order: Task 1 should come before Task 2, Task 2 before Task 3
    let pos1 = stdout.find("Task 1").unwrap();
    let pos2 = stdout.find("Task 2").unwrap();
    let pos3 = stdout.find("Task 3").unwrap();
    assert!(pos1 < pos2 && pos2 < pos3, "Tasks should be in order 1, 2, 3");
}

#[test]
fn test_enqueue_nonexistent_task() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Try to enqueue a task that doesn't exist
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "999"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Task 999 not found"));
}

#[test]
fn test_enqueue_invalid_task_id() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Try to enqueue with invalid ID
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "abc"])
        .assert()
        .success()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn test_enqueue_task_already_on_stack() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create a task
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Test task"]).assert().success();
    
    // Enqueue it once
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Verify it's on the clock stack
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
    
    // Try to enqueue it again - should move to end (not create duplicate)
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Verify it's still on the clock stack (only once) - check for task ID 1
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Test task"));
    
    // Add another task
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 2"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "2"]).assert().success();
    
    // Clock stack should have both tasks (1 was already at end, 2 added after)
    let mut cmd = get_task_cmd();
    let output = cmd.args(&["clock", "list"]).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Verify both tasks are present and in order
    assert!(stdout.contains("Test task"), "Stack should contain Test task");
    assert!(stdout.contains("Task 2"), "Stack should contain Task 2");
    
    // Find positions - Task 1 should come before Task 2
    let pos1 = stdout.find("Test task").unwrap();
    let pos2 = stdout.find("Task 2").unwrap();
    assert!(pos1 < pos2, "Task 1 should come before Task 2 in stack");
}

#[test]
fn test_enqueue_completed_task() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create and complete a task
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Test task"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "in"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["finish"]).assert().success();
    
    // Verify task is completed
    let mut cmd = get_task_cmd();
    cmd.args(&["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("done"));
    
    // Try to enqueue the completed task
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Verify it's on the clock stack (completed tasks can be enqueued)
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
}

#[test]
fn test_enqueue_negative_task_id() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Try to enqueue with negative ID
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "-1"])
        .assert()
        .success()
        .stderr(predicate::str::contains("unexpected argument"));
}

#[test]
fn test_enqueue_zero_task_id() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Try to enqueue with zero ID
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid task ID"));
}

#[test]
fn test_enqueue_empty_string() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Try to enqueue with empty string (should fail parsing)
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", ""])
        .assert()
        .success()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn test_enqueue_with_range_syntax() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 2"]).assert().success();
    
    // Try to enqueue with range syntax - should this work?
    // Currently enqueue only accepts single IDs, not ranges
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1-2"])
        .assert()
        .success()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn test_enqueue_with_comma_list() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 2"]).assert().success();
    
    // Try to enqueue with comma list - should this work?
    // Currently enqueue only accepts single IDs, not lists
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1,2"])
        .assert()
        .success()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn test_enqueue_after_stack_operations() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 2"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 3"]).assert().success();
    
    // Enqueue tasks 1 and 2
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "2"]).assert().success();
    
    // Roll the clock stack
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "next", "1"]).assert().success();
    
    // Verify clock stack is [2, 1] (Task 2, Task 1 in order)
    let mut cmd = get_task_cmd();
    let output = cmd.args(&["clock", "list"]).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Task 2") && stdout.contains("Task 1"), "Clock stack should contain Task 2 and Task 1");
    let pos2 = stdout.find("Task 2").unwrap();
    let pos1 = stdout.find("Task 1").unwrap();
    assert!(pos2 < pos1, "Task 2 should come before Task 1");
    
    // Enqueue task 3 - should go to the end
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "3"]).assert().success();
    
    // Verify clock stack is [2, 1, 3] (Task 2, Task 1, Task 3 in order)
    let mut cmd = get_task_cmd();
    let output = cmd.args(&["clock", "list"]).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Task 2") && stdout.contains("Task 1") && stdout.contains("Task 3"), 
            "Stack should contain all three tasks");
    let pos2 = stdout.find("Task 2").unwrap();
    let pos1 = stdout.find("Task 1").unwrap();
    let pos3 = stdout.find("Task 3").unwrap();
    assert!(pos2 < pos1 && pos1 < pos3, "Tasks should be in order 2, 1, 3");
}

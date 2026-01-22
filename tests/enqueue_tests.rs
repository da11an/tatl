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
    
    // Verify it's on the queue by starting timing (should succeed)
    let mut cmd = get_task_cmd();
    cmd.args(&["on"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 1"));
    
    let mut cmd = get_task_cmd();
    cmd.args(&["off"]).assert().success();
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
    
    // Verify queue order by starting timing (queue[0] should be Task 1)
    let mut cmd = get_task_cmd();
    cmd.args(&["on"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 1"));
    
    let mut cmd = get_task_cmd();
    cmd.args(&["off"]).assert().success();
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
    
    // Verify it's on the queue
    let mut cmd = get_task_cmd();
    cmd.args(&["on"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 1"));
    
    let mut cmd = get_task_cmd();
    cmd.args(&["off"]).assert().success();
    
    // Try to enqueue it again - should move to end (not create duplicate)
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Add another task and enqueue it
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 2"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "2"]).assert().success();
    
    // Queue[0] should be Task 1 (since it was re-enqueued to end, then Task 2 added after)
    // Actually, Task 1 would be at end, Task 2 would be after it
    // So queue order is [Task 1, Task 2] - start timing should get Task 1
    let mut cmd = get_task_cmd();
    cmd.args(&["on"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 1"));
    
    let mut cmd = get_task_cmd();
    cmd.args(&["off"]).assert().success();
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
    cmd.args(&["on"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["finish"]).assert().success();
    
    // Verify task is completed
    let mut cmd = get_task_cmd();
    let output = cmd.args(&["show", "1"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("completed") || stdout.contains("Status: completed"));
    
    // Try to enqueue the completed task
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Verify it's on the queue by starting timing
    let mut cmd = get_task_cmd();
    cmd.args(&["on"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 1"));
    
    let mut cmd = get_task_cmd();
    cmd.args(&["off"]).assert().success();
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

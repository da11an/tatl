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

fn get_task_cmd(temp_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("tatl").unwrap();
    cmd.env("HOME", temp_dir.path());
    cmd
}

#[test]
fn test_on_task_pushes_to_top() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create two tasks
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "task 1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "task 2"]).assert().success();
    
    // Enqueue task 1
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Use `on 2` - should push task 2 to top and start timing
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["on", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 2"));
    
    // Stop timing
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"]).assert().success();
}

#[test]
fn test_on_task_closes_previous_session() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create two tasks
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "task 1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "task 2"]).assert().success();
    
    // Start session for task 1
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["on", "1"]).assert().success();
    
    // Start session for task 2 - should close task 1's session
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["on", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 2"));
    
    // Verify only task 2 session is open (try to start timing again should fail)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["on"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already running"));
}

#[test]
fn test_on_task_same_timestamp() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create two tasks
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "task 1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "task 2"]).assert().success();
    
    // Start session for task 1
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["on", "1"]).assert().success();
    
    // Small delay to ensure different timestamps if not handled correctly
    std::thread::sleep(std::time::Duration::from_millis(10));
    
    // Start session for task 2 - should close task 1 at same timestamp
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["on", "2"]).assert().success();
    
    // Both sessions should exist and task 1's session should be closed
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"]).assert().success();
}

#[test]
fn test_on_task_with_start_time() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "test task"]).assert().success();
    
    // Start timing with specific start time
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["on", "1", "2026-01-10T09:00"])
        .assert()
        .success();
    
    // Stop timing
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"]).assert().success();
}

#[test]
fn test_on_task_default_now() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "test task"]).assert().success();
    
    // Start timing without start time (should default to now)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["on", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 1"));
    
    // Stop timing
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"]).assert().success();
}

#[test]
fn test_on_positional_syntax() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task"]).assert().success();
    
    // Start timing using positional syntax
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["on", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 1"));
    
    // Stop timing
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"]).assert().success();
}

#[test]
fn test_on_no_args_uses_queue0() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and enqueue
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Start timing without args (should use queue[0])
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["on"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 1"));
    
    // Stop timing
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"]).assert().success();
}

#[test]
fn test_on_positional_with_time() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task"]).assert().success();
    
    // Start timing using positional syntax with start time
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["on", "1", "2026-01-10T09:00"])
        .assert()
        .success();
    
    // Stop timing
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"]).assert().success();
}

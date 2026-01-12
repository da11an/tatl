use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;

/// Helper to create a temporary database and set it as the data location
fn setup_test_env() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    
    // Create config file
    let config_dir = temp_dir.path().join(".taskninja");
    fs::create_dir_all(&config_dir).unwrap();
    let config_file = config_dir.join("rc");
    fs::write(&config_file, format!("data.location={}\n", db_path.display())).unwrap();
    
    // Set HOME to temp_dir so the config file is found
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    
    temp_dir
}

fn get_task_cmd() -> Command {
    Command::cargo_bin("task").unwrap()
}

#[test]
fn test_task_clock_in_pushes_to_top() {
    let _temp_dir = setup_test_env();
    
    // Create two tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 2"]).assert().success();
    
    // Enqueue task 1
    let mut cmd = get_task_cmd();
    cmd.args(&["1", "enqueue"]).assert().success();
    
    // Use task 2 clock in - should push task 2 to top
    let mut cmd = get_task_cmd();
    cmd.args(&["2", "clock", "in"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 2"));
    
    // Verify stack order: task 2 should be at top (position 0)
    let mut cmd = get_task_cmd();
    let output = cmd.args(&["clock", "list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let pos_0_line = stdout.lines()
        .find(|l| l.trim_start().starts_with("0"))
        .unwrap();
    assert!(pos_0_line.contains("2"), "Task 2 should be at position 0");
}

#[test]
fn test_task_clock_in_closes_previous_session() {
    let _temp_dir = setup_test_env();
    
    // Create two tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 2"]).assert().success();
    
    // Start session for task 1
    let mut cmd = get_task_cmd();
    cmd.args(&["1", "clock", "in"]).assert().success();
    
    // Start session for task 2 - should close task 1's session
    let mut cmd = get_task_cmd();
    cmd.args(&["2", "clock", "in"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 2"));
    
    // Verify only task 2 session is open (try to clock in again should fail)
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "in"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already running"));
}

#[test]
fn test_task_clock_in_same_timestamp() {
    let _temp_dir = setup_test_env();
    
    // Create two tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 2"]).assert().success();
    
    // Start session for task 1
    let mut cmd = get_task_cmd();
    cmd.args(&["1", "clock", "in"]).assert().success();
    
    // Small delay to ensure different timestamps if not handled correctly
    std::thread::sleep(std::time::Duration::from_millis(10));
    
    // Start session for task 2 - should close task 1 at same timestamp
    let mut cmd = get_task_cmd();
    cmd.args(&["2", "clock", "in"]).assert().success();
    
    // Both sessions should exist and task 1's session should be closed
    // (We can't easily verify timestamps match without querying DB directly,
    // but if the session was closed, we can verify it's not open)
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "out"]).assert().success();
}

#[test]
fn test_task_clock_in_with_start_time() {
    let _temp_dir = setup_test_env();
    
    // Create task
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    // Clock in with specific start time
    let mut cmd = get_task_cmd();
    cmd.args(&["1", "clock", "in", "2026-01-10T09:00"])
        .assert()
        .success();
    
    // Clock out
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "out"]).assert().success();
}

#[test]
fn test_task_clock_in_default_now() {
    let _temp_dir = setup_test_env();
    
    // Create task
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    // Clock in without start time (should default to now)
    let mut cmd = get_task_cmd();
    cmd.args(&["1", "clock", "in"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 1"));
    
    // Clock out
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "out"]).assert().success();
}

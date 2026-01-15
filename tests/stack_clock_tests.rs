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
    let config_dir = temp_dir.path().join(".taskninja");
    fs::create_dir_all(&config_dir).unwrap();
    let config_file = config_dir.join("rc");
    fs::write(&config_file, format!("data.location={}\n", db_path.display())).unwrap();
    
    // Set HOME to temp_dir so the config file is found
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    (temp_dir, guard)
}

fn get_task_cmd() -> Command {
    Command::cargo_bin("task").unwrap()
}

#[test]
fn test_stack_next_while_clock_running_switches_live_task() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create two tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 2"]).assert().success();
    
    // Enqueue both tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "2"]).assert().success();
    
    // Start clock on task 1 (clock[0])
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "in"]).assert().success();
    
    // Move to next task - should switch to task 2
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "next"]).assert().success();
    
    // Verify task 2 is now at top (check table format)
    let mut cmd = get_task_cmd();
    let output = cmd.args(&["clock", "list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    // Task 2 should be at position 0
    let pos_0_line = stdout.lines()
        .find(|l| l.trim_start().starts_with("0"))
        .unwrap();
    assert!(pos_0_line.contains("2"), "Task 2 should be at position 0");
    
    // Clock out should work (session exists for task 2)
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "out"]).assert().success();
}

#[test]
fn test_stack_pick_while_stopped_does_not_create_sessions() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create three tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 2"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 3"]).assert().success();
    
    // Enqueue all tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "2"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "3"]).assert().success();
    
    // Pick task at position 2 (task 3) - no session should be created
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "pick", "2"]).assert().success();
    
    // Verify stack order changed (task 3 should be at position 0)
    let mut cmd = get_task_cmd();
    let output = cmd.args(&["clock", "list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let pos_0_line = stdout.lines()
        .find(|l| l.trim_start().starts_with("0"))
        .unwrap();
    assert!(pos_0_line.contains("3"), "Task 3 should be at position 0");
    
    // Verify no session is running
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "out"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No session is currently running"));
}

#[test]
fn test_stack_next_with_clock_in_flag() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create two tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 2"]).assert().success();
    
    // Enqueue both tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "2"]).assert().success();
    
    // Move to next task
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "next"]).assert().success();
    
    // Start clock on new clock[0]
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "in"]).assert().success();
    
    // Verify session is running
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "out"]).assert().success();
}

#[test]
fn test_stack_clear_with_clock_out_flag() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create task
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 1"]).assert().success();
    
    // Enqueue and start clock
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "in"]).assert().success();
    
    // Stop clock first
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "out"]).assert().success();
    
    // Clear clock stack
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "clear"]).assert().success();
    
    // Verify no session is running
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "out"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No session is currently running"));
}

#[test]
fn test_stack_pick_while_clock_running_switches_task() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create three tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 2"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 3"]).assert().success();
    
    // Enqueue all tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "2"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "3"]).assert().success();
    
    // Start clock on task 1
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "in"]).assert().success();
    
    // Pick task at position 2 (task 3) - should switch to task 3
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "pick", "2"]).assert().success();
    
    // Verify task 3 is at top (position 0)
    let mut cmd = get_task_cmd();
    let output = cmd.args(&["clock", "list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let pos_0_line = stdout.lines()
        .find(|l| l.trim_start().starts_with("0"))
        .unwrap();
    assert!(pos_0_line.contains("3"), "Task 3 should be at position 0");
    
    // Clock out should work (session exists for task 3)
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "out"]).assert().success();
}

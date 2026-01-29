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

// Note: test_stack_next_while_clock_running_switches_live_task was removed
// because the `next` command was removed per Plan_22_CLI_Syntax_Review.md.
// Use `tatl on <task_id>` to switch to a different task instead.

// Note: test_stack_pick_while_stopped_does_not_create_sessions was removed
// because the `pick` command was removed per Plan_22_CLI_Syntax_Review.md.
// Use `tatl on <task_id>` to switch to a different task instead.

// Note: test_stack_next_with_clock_in_flag was removed
// because the `next` command was removed per Plan_22_CLI_Syntax_Review.md.
// Use `tatl on <task_id>` to switch to a different task instead.

#[test]
fn test_queue_column_shows_position() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create three tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 2"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 3"]).assert().success();
    
    // Enqueue tasks 1 and 3 (not 2)
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "3"]).assert().success();
    
    // List all tasks and verify Q column values
    let mut cmd = get_task_cmd();
    let output = cmd.args(&["list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    // Check header has Q column
    assert!(stdout.contains("Q"), "Output should have Q column header");
    
    // Q column is printed as a leading field; verify lines directly
    assert!(stdout.contains("0    1    task 1"), "Task 1 should be at queue position 0");
    assert!(stdout.contains("?    2    task 2"), "Task 2 should not have a concrete queue position");
    assert!(stdout.contains("1    3    task 3"), "Task 3 should be at queue position 1");
}

#[test]
fn test_switch_task_with_on_command() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create two tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 2"]).assert().success();
    
    // Enqueue and start clock on task 1
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["on"]).assert().success();
    
    // Switch to task 2 using `on 2` (replaces old `pick` behavior)
    let mut cmd = get_task_cmd();
    cmd.args(&["on", "2"]).assert().success();
    
    // Verify task 2 is now active at position 0 in queue
    let mut cmd = get_task_cmd();
    let output = cmd.args(&["list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // Find the task 2 line and verify it has ▶ in the Q column (active at position 0)
    // Note: list output is sorted by ID, so task 2 may not be the first data line
    let task2_line = stdout.lines()
        .find(|l| l.contains("task 2"))
        .expect("Should have a line for task 2");
    assert!(task2_line.trim_start().starts_with("▶"),
        "Task 2 should show ▶ (active at position 0). Line: {}", task2_line);
    
    // Clock out should work
    let mut cmd = get_task_cmd();
    cmd.args(&["off"]).assert().success();
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
    cmd.args(&["on"]).assert().success();
    
    // Stop clock first
    let mut cmd = get_task_cmd();
    cmd.args(&["off"]).assert().success();
    
    // Clear clock stack
    let mut cmd = get_task_cmd();
    cmd.args(&["dequeue", "--all"]).assert().success();
    
    // Verify no session is running
    let mut cmd = get_task_cmd();
    cmd.args(&["off"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No session is currently running"));
}

// Note: test_stack_pick_while_clock_running_switches_task was removed
// because the `pick` command was removed per Plan_22_CLI_Syntax_Review.md.
// Use `tatl on <task_id>` to switch to a different task instead.

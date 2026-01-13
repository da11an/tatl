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

fn get_task_cmd(temp_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("task").unwrap();
    cmd.env("HOME", temp_dir.path());
    cmd
}

#[test]
fn test_add_with_clock_in_flag() {
    let temp_dir = setup_test_env();
    
    // Add task with --clock-in flag (flag must come before args)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "--clock-in", "Test task with clock-in"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Started timing task"));
    
    // Verify task is on clock stack at position 0
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["clock", "list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("0"), "Task should be at position 0");
    assert!(stdout.contains("Test task with clock-in"), "Task description should be visible");
    
    // Verify session is running
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "out"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stopped timing task"));
}

#[test]
fn test_add_without_clock_in_flag() {
    let temp_dir = setup_test_env();
    
    // Add task without --clock-in flag
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task without clock-in"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Started timing task").not());
    
    // Verify task is NOT on clock stack
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["clock", "list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Clock stack is empty"), "Task should not be on clock stack");
    
    // Verify no session is running
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "out"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No session is currently running"));
}

#[test]
fn test_add_clock_in_pushes_to_top() {
    let temp_dir = setup_test_env();
    
    // Create first task and enqueue it
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "First task"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "enqueue", "1"]).assert().success();
    
    // Add second task with --clock-in (should push to top)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "--clock-in", "Second task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 2"));
    
    // Verify second task is at position 0
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["clock", "list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let pos_0_line = stdout.lines()
        .find(|l| l.trim_start().starts_with("0"))
        .unwrap();
    assert!(pos_0_line.contains("2"), "Task 2 should be at position 0");
}

#[test]
fn test_add_clock_in_closes_existing_session() {
    let temp_dir = setup_test_env();
    
    // Create first task and clock in
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "First task"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "in"]).assert().success();
    
    // Add second task with --clock-in (should close first task's session)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "--clock-in", "Second task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 2"));
    
    // Verify only task 2 session is running
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "out"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Stopped timing task 2"));
}

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
fn test_done_with_filter_single_match() {
    let _temp_dir = setup_test_env();
    
    // Create tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 1", "+urgent"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 2"]).assert().success();
    
    // Clock in Task 1
    let mut cmd = get_task_cmd();
    cmd.args(&["1", "clock", "in"]).assert().success();
    
    // Complete Task 1 using filter
    let mut cmd = get_task_cmd();
    cmd.args(&["+urgent", "done"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed task 1"));
}

#[test]
fn test_done_with_yes_flag() {
    let _temp_dir = setup_test_env();
    
    // Create tasks with same tag
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 1", "+urgent"]).assert().success();
    
    // Clock in Task 1
    let mut cmd = get_task_cmd();
    cmd.args(&["1", "clock", "in"]).assert().success();
    
    // Complete with --yes flag (should work even for single task)
    let mut cmd = get_task_cmd();
    cmd.args(&["+urgent", "done", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed task 1"));
}

#[test]
fn test_done_with_next_flag() {
    let _temp_dir = setup_test_env();
    
    // Create tasks and enqueue
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 2"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["1", "enqueue"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["2", "enqueue"]).assert().success();
    
    // Clock in Task 1
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "in"]).assert().success();
    
    // Complete Task 1 with --next flag
    let mut cmd = get_task_cmd();
    cmd.args(&["done", "--next"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed task 1"))
        .stdout(predicate::str::contains("Started timing task 2"));
}

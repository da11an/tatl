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
fn test_finish_with_filter_single_match() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 1", "+urgent"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 2"]).assert().success();
    
    // Start timing Task 1
    let mut cmd = get_task_cmd();
    cmd.args(&["on", "1"]).assert().success();
    
    // Finish Task 1 using filter
    let mut cmd = get_task_cmd();
    cmd.args(&["finish", "+urgent"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Finished task 1"));
}

#[test]
fn test_finish_with_yes_flag() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create tasks with same tag
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 1", "+urgent"]).assert().success();
    
    // Start timing Task 1
    let mut cmd = get_task_cmd();
    cmd.args(&["on", "1"]).assert().success();
    
    // Finish with --yes flag (should work even for single task)
    let mut cmd = get_task_cmd();
    cmd.args(&["finish", "+urgent", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Finished task 1"));
}

#[test]
fn test_finish_with_next_flag() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create tasks and enqueue
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 2"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["1", "enqueue"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["2", "enqueue"]).assert().success();
    
    // Start timing Task 1
    let mut cmd = get_task_cmd();
    cmd.args(&["on"]).assert().success();
    
    // Finish Task 1 with --next flag
    let mut cmd = get_task_cmd();
    cmd.args(&["finish", "--next"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Finished task 1"))
        .stdout(predicate::str::contains("Started timing task 2"));
}

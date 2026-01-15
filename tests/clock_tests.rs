use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
use std::path::PathBuf;
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
fn test_clock_in_empty_stack() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Initialize database (create a task first to ensure DB is set up)
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    // Clear stack
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "clear"]).assert().success();
    
    // Try to clock in with empty stack - should fail
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "in"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Stack is empty"));
}

#[test]
fn test_clock_in_already_running() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create task and add to stack
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Clock in
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "in"]).assert().success();
    
    // Try to clock in again - should fail
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "in"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already running"));
}

#[test]
fn test_clock_out_no_session() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Initialize database (but don't start a session)
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    // Try to clock out with no session - should fail
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "out"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No session is currently running"));
}

#[test]
fn test_clock_in_out_workflow() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create task and add to stack
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Clock in
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "in"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task"));
    
    // Clock out
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "out"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stopped timing task"));
}

#[test]
fn test_clock_in_default_now() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Initialize database
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    // Clear any existing stack state
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "clear"]).assert().success();
    
    // Add task to stack
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Clock in without arguments (should default to "now")
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "in"])
        .assert()
        .success();
    
    // Clock out
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "out"]).assert().success();
}

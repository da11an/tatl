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
fn test_on_empty_queue() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Initialize database (create a task but don't enqueue it)
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    // Try to start timing with empty queue - should fail
    let mut cmd = get_task_cmd();
    cmd.args(&["on"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Queue is empty"));
}

#[test]
fn test_on_already_running() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create task and add to queue
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Start timing
    let mut cmd = get_task_cmd();
    cmd.args(&["on"]).assert().success();
    
    // Try to start timing again - should fail
    let mut cmd = get_task_cmd();
    cmd.args(&["on"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already running"));
}

#[test]
fn test_off_no_session() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Initialize database (but don't start a session)
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    // Try to stop timing with no session - should fail
    let mut cmd = get_task_cmd();
    cmd.args(&["off"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No session is currently running"));
}

#[test]
fn test_on_off_workflow() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create task and add to queue
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Start timing
    let mut cmd = get_task_cmd();
    cmd.args(&["on"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task"));
    
    // Stop timing
    let mut cmd = get_task_cmd();
    cmd.args(&["off"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stopped timing task"));
}

#[test]
fn test_on_default_now() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Initialize database
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    // Add task to queue
    let mut cmd = get_task_cmd();
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Start timing without arguments (should default to "now")
    let mut cmd = get_task_cmd();
    cmd.args(&["on"])
        .assert()
        .success();
    
    // Stop timing
    let mut cmd = get_task_cmd();
    cmd.args(&["off"]).assert().success();
}

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
fn test_status_empty_state() {
    let (temp_dir, _guard) = setup_test_env();
    let mut cmd = get_task_cmd(&temp_dir);
    
    cmd.args(&["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Clock Status ==="))
        .stdout(predicate::str::contains("Clocked OUT"))
        .stdout(predicate::str::contains("=== Clock Stack (Top 3) ==="))
        .stdout(predicate::str::contains("Stack is empty"))
        .stdout(predicate::str::contains("=== Today's Sessions ==="))
        .stdout(predicate::str::contains("=== Overdue Tasks ==="));
}

#[test]
fn test_status_with_clocked_in_task() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task and clock in
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task"])
        .assert()
        .success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "in", "1"])
        .assert()
        .success();
    
    // Wait a moment to ensure session is created
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["status"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    
    let stdout = String::from_utf8(output).unwrap();
    // Should show either "Clocked IN" or "Clocked OUT" with task in stack
    assert!(stdout.contains("=== Clock Status ==="));
    assert!(stdout.contains("1") || stdout.contains("Test task"));
}

#[test]
fn test_status_with_clock_stack() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create multiple tasks and add to stack
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 1"])
        .assert()
        .success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 2"])
        .assert()
        .success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "1"])
        .assert()
        .success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "2"])
        .assert()
        .success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Clock Stack (Top 3) ==="))
        .stdout(predicate::str::contains("Task 1"))
        .stdout(predicate::str::contains("Task 2"));
}

#[test]
fn test_status_with_overdue_tasks() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task with past due date
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Overdue task", "due:2020-01-01"])
        .assert()
        .success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Overdue Tasks ==="))
        .stdout(predicate::str::contains("1 task(s) overdue"));
}

#[test]
fn test_status_with_today_sessions() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task, clock in, and clock out
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task"])
        .assert()
        .success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "in", "1"])
        .assert()
        .success();
    
    // Wait a moment
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "out"])
        .assert()
        .success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Today's Sessions ==="))
        .stdout(predicate::str::contains("session"));
}

#[test]
fn test_status_json_output() {
    let (temp_dir, _guard) = setup_test_env();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["status", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"clock\""))
        .stdout(predicate::str::contains("\"clock_stack\""))
        .stdout(predicate::str::contains("\"today_sessions\""))
        .stdout(predicate::str::contains("\"overdue\""));
}

#[test]
fn test_status_json_with_data() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task and clock in
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task"])
        .assert()
        .success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "in", "1"])
        .assert()
        .success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["status", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    
    let json_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    
    assert!(json.get("clock").is_some());
    assert_eq!(json["clock"]["state"], "in");
    assert_eq!(json["clock"]["task_id"], 1);
    assert_eq!(json["clock"]["task_description"], "Test task");
}

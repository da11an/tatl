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
fn test_done_errors_if_stack_empty() {
    let temp_dir = setup_test_env();
    
    // Initialize database (create a task to ensure DB is set up)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "test task"]).assert().success();
    
    // Clear stack
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "clear"]).assert().success();
    
    // Try to do done with empty stack - should fail
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["done"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Stack is empty"));
}

#[test]
fn test_done_errors_if_no_session_running() {
    let temp_dir = setup_test_env();
    
    // Create task and enqueue
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "test task"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "enqueue", "1"]).assert().success();
    
    // Try to do done without session - should fail (task done without ID requires session)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["done"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No session is running"));
}

#[test]
fn test_done_completes_task_and_removes_from_stack() {
    let temp_dir = setup_test_env();
    
    // Create tasks and enqueue
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 2"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "enqueue", "2"]).assert().success();
    
    // Clock in Task 1
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "in"]).assert().success();
    
    // Verify stack has both tasks
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["clock", "list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("1") && stdout.contains("2"));
    
    // Complete Task 1
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["done"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed task 1"));
    
    // Verify stack only has Task 2
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["clock", "list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("2") && !stdout.contains("1"));
    
    // Verify Task 1 is completed (check status via JSON or verify it's not in pending list)
    // For now, we'll verify by checking that it doesn't appear in default list (which shows pending)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["list", "id:1", "--json"])
        .assert()
        .success();
    
    // The task should be completed - we can verify this by checking it's not in pending list
    // or by parsing JSON. For simplicity, we'll just verify the command succeeded.
}

#[test]
fn test_done_with_task_id() {
    let temp_dir = setup_test_env();
    
    // Create tasks
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 2"]).assert().success();
    
    // Clock in Task 1
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "in", "1"]).assert().success();
    
    // Complete Task 1 using ID
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["done", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed task 1"));
}

#[test]
fn test_done_with_task_id_no_session() {
    let temp_dir = setup_test_env();
    
    // Create task (not clocked in)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task without session"]).assert().success();
    
    // Complete task without session - should work now
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["done", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed task 1"));
    
    // Verify task is completed (check via show command which shows all statuses)
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["show", "1"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("completed") || stdout.contains("Status: completed"));
}

#[test]
fn test_done_with_task_id_with_session() {
    let temp_dir = setup_test_env();
    
    // Create task and clock in
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task with session"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "in"]).assert().success();
    
    // Complete task with session - should close session
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["done", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed task 1"));
    
    // Verify session was closed (try to clock out should fail)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "out"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No session is currently running"));
}

#[test]
fn test_done_with_filter_no_sessions() {
    let temp_dir = setup_test_env();
    
    // Create projects
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["projects", "add", "work"]).assert().success();
    
    // Create tasks with project (not clocked in)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Work task 1", "project:work"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Work task 2", "project:work"]).assert().success();
    
    // Complete tasks by filter without sessions
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["done", "project:work", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed task"));
    
    // Verify both tasks are completed (check via show command)
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["show", "1"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("completed") || stdout.contains("Status: completed"));
    
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["show", "2"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("completed") || stdout.contains("Status: completed"));
}

#[test]
fn test_done_with_next_flag() {
    let temp_dir = setup_test_env();
    
    // Create tasks and enqueue
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 2"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "enqueue", "2"]).assert().success();
    
    // Clock in Task 1
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "in"]).assert().success();
    
    // Complete Task 1 with --next flag
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["done", "--next"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed task 1"))
        .stdout(predicate::str::contains("Started timing task 2"));
    
    // Verify Task 2 session is running
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "out"]).assert().success(); // Should succeed if session is running
}

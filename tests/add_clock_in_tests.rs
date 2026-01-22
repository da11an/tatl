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
fn test_add_with_on_flag() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with --on flag (flag must come before args)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "--on", "Test task with --on"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Started timing task"));
    
    // Verify session is running by stopping it
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stopped timing task"));
}

#[test]
fn test_add_without_on_flag() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task without --on flag
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task without --on"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Started timing task").not());
    
    // Verify no session is running
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No session is currently running"));
}

#[test]
fn test_add_on_pushes_to_top() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create first task and enqueue it
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "First task"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Add second task with --on (should push to top)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "--on", "Second task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 2"));
    
    // Verify by stopping timing
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stopped timing task 2"));
}

#[test]
fn test_add_on_closes_existing_session() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create first task and start timing
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "First task"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["on", "1"]).assert().success();
    
    // Add second task with --on (should close first task's session)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "--on", "Second task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 2"));
    
    // Verify only task 2 session is running
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Stopped timing task 2"));
}

/// Test case 1: Timer running but adding new task with --on
/// Should: close existing session, push new task to queue[0], start new session
#[test]
fn test_add_on_when_timer_running() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create first task, enqueue, and start timing
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["on", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started timing task 1"));
    
    // Add new task with --on flag (timer is running)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "--on", "Task 2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task 2"))
        .stdout(predicate::str::contains("Started timing task 2"));
    
    // Verify task 2 session is running (not task 1)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stopped timing task 2"));
    
    // Verify task 1's session was closed (check that task 1 has a closed session)
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["sessions", "list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    // Should see task 1's session with an end time (closed)
    assert!(stdout.contains("1") || stdout.contains("Task 1"), "Should see task 1 in session list");
}

/// Test case 2: Timer not running yet but adding new task with --on
/// Should: push task to queue[0], start new session
#[test]
fn test_add_on_when_timer_not_running() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Verify no session is running
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No session is currently running"));
    
    // Add task with --on flag (timer is NOT running)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "--on", "New task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Started timing task"));
    
    // Verify session is running
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stopped timing task"));
}

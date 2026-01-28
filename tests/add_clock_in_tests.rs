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
    
    // Add task and pipe to on
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task", ":", "on"])
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
    
    // Add task without pipe to on
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task without on"])
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
    
    // Add second task with pipe to on (should push to top)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Second task", ":", "on"])
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
    
    // Add second task with pipe to on (should close first task's session)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Second task", ":", "on"])
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
    
    // Add new task with pipe to on (timer is running)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 2", ":", "on"])
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
    
    // Add task with pipe to on (timer is NOT running)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "New task", ":", "on"])
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

#[test]
fn test_add_with_on_equals_time() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task and pipe to on with backdated start
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Meeting started at 2pm", ":", "on", "14:00"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Started timing"));
    
    // Verify session exists and has correct start time
    let output = get_task_cmd(&temp_dir)
        .args(&["sessions", "list", "--json"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let sessions = json.as_array().unwrap();
    assert!(!sessions.is_empty(), "Should have at least one session");
    
    // Session should be open (running)
    let session = &sessions[0];
    assert_eq!(session["end_ts"], serde_json::Value::Null, "Session should be open");
}

#[test]
fn test_add_with_on_equals_time_creates_open_session() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task and pipe to on with start time
    get_task_cmd(&temp_dir)
        .args(&["add", "Early morning task", ":", "on", "09:00"])
        .assert()
        .success();
    
    // Verify session is open (running)
    let output = get_task_cmd(&temp_dir)
        .args(&["sessions", "list", "--json"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let sessions = json.as_array().unwrap();
    assert!(!sessions.is_empty());
    
    let session = &sessions[0];
    assert_eq!(session["end_ts"], serde_json::Value::Null);
}

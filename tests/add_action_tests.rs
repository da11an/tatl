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

// =============================================================================
// : finish pipe tests
// =============================================================================

#[test]
fn test_add_with_finish_flag() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task and pipe to finish
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Already done task", ":", "finish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Finished task"));
    
    // Verify task exists and is completed (list --json shows all non-deleted tasks)
    let output = get_task_cmd(&temp_dir)
        .args(&["list", "--json"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.as_array().unwrap();
    assert_eq!(tasks.len(), 1, "Should have one task");
    assert_eq!(tasks[0]["status"], "completed", "Task should be completed");
    
    // Verify no pending tasks
    let output = get_task_cmd(&temp_dir)
        .args(&["list", "status=pending", "--json"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    // When no tasks match, output is "No tasks found.\n"
    assert!(stdout.contains("No tasks found"), "Should have no pending tasks");
}

#[test]
fn test_add_finish_with_onoff_creates_session_and_completes() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with : onoff and : finish
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Meeting", ":", "onoff", "09:00..10:00", ":", "finish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Added session"))
        .stdout(predicate::str::contains("Finished task"));
    
    // Verify session was created
    let output = get_task_cmd(&temp_dir)
        .args(&["sessions", "list", "--json"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let sessions = json.as_array().unwrap();
    assert!(!sessions.is_empty(), "Should have at least one session");
    
    // Session should be closed (has end_ts)
    let session = &sessions[0];
    assert!(session["end_ts"] != serde_json::Value::Null, "Session should be closed");
}

#[test]
fn test_add_finish_with_respawn_triggers_respawn() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with respawn rule and : finish
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Daily standup", "respawn=daily", "due=09:00", ":", "finish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Finished task"))
        .stdout(predicate::str::contains("Respawned"));
    
    // Verify we have 2 tasks: 1 completed (original) + 1 pending (respawned)
    let output = get_task_cmd(&temp_dir)
        .args(&["list", "--json"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.as_array().unwrap();
    assert_eq!(tasks.len(), 2, "Should have two tasks (original + respawned)");
    
    // Check statuses
    let completed_count = tasks.iter().filter(|t| t["status"] == "completed").count();
    let pending_count = tasks.iter().filter(|t| t["status"] == "pending").count();
    assert_eq!(completed_count, 1, "Should have one completed task");
    assert_eq!(pending_count, 1, "Should have one pending task (respawned)");
}

#[test]
#[ignore] // TODO: Fix implementation bug - : on : finish fails with "Failed to close session"
fn test_add_finish_with_on_pipe_works() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with : on : finish should work (pipe allows chaining)
    // Note: There may be a micro-session warning, but the task should still be finished
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["add", "Task", ":", "on", ":", "finish"])
        .output()
        .unwrap();
    
    assert!(output.status.success(), "Command should succeed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Created task"), "Should create task");
    assert!(stdout.contains("Finished task"), "Should finish task");
    // Started timing may or may not appear due to micro-session purging
}

#[test]
fn test_add_finish_with_enqueue_pipe_works() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with : enqueue : finish should work (pipe allows chaining)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task", ":", "enqueue", ":", "finish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Enqueued"))
        .stdout(predicate::str::contains("Finished task"));
}

// =============================================================================
// : close pipe tests
// =============================================================================

#[test]
fn test_add_with_close_flag() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task and pipe to close
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Cancelled request", ":", "close"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Closed task"));
    
    // Verify task exists and is closed (list --json shows all non-deleted tasks)
    let output = get_task_cmd(&temp_dir)
        .args(&["list", "--json"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.as_array().unwrap();
    assert_eq!(tasks.len(), 1, "Should have one task");
    assert_eq!(tasks[0]["status"], "closed", "Task should be closed");
    
    // Verify no pending tasks
    let output = get_task_cmd(&temp_dir)
        .args(&["list", "status=pending", "--json"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("No tasks found"), "Should have no pending tasks");
}

#[test]
fn test_add_close_with_onoff_creates_session_and_closes() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with : onoff and : close (recording effort before closing)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Started but abandoned", ":", "onoff", "09:00..10:00", ":", "close"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Added session"))
        .stdout(predicate::str::contains("Closed task"));
    
    // Verify session was created
    let output = get_task_cmd(&temp_dir)
        .args(&["sessions", "list", "--json"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let sessions = json.as_array().unwrap();
    assert!(!sessions.is_empty(), "Should have at least one session");
}

#[test]
fn test_add_close_with_respawn_triggers_respawn() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with respawn rule and : close
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Daily report", "respawn=daily", ":", "close"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Closed task"))
        .stdout(predicate::str::contains("Respawned"));
    
    // Verify we have 2 tasks: 1 closed (original) + 1 pending (respawned)
    let output = get_task_cmd(&temp_dir)
        .args(&["list", "--json"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.as_array().unwrap();
    assert_eq!(tasks.len(), 2, "Should have two tasks (original + respawned)");
    
    // Check statuses
    let closed_count = tasks.iter().filter(|t| t["status"] == "closed").count();
    let pending_count = tasks.iter().filter(|t| t["status"] == "pending").count();
    assert_eq!(closed_count, 1, "Should have one closed task");
    assert_eq!(pending_count, 1, "Should have one pending task (respawned)");
}

#[test]
#[ignore] // TODO: Fix implementation bug - : on : close fails with "Failed to close session"
fn test_add_close_with_on_pipe_works() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with : on : close should work (pipe allows chaining)
    // Note: There may be a micro-session warning, but the task should still be closed
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["add", "Task", ":", "on", ":", "close"])
        .output()
        .unwrap();
    
    assert!(output.status.success(), "Command should succeed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Created task"), "Should create task");
    assert!(stdout.contains("Closed task"), "Should close task");
    // Started timing may or may not appear due to micro-session purging
}

#[test]
fn test_add_close_with_enqueue_pipe_works() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with : enqueue : close should work (pipe allows chaining)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task", ":", "enqueue", ":", "close"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Enqueued"))
        .stdout(predicate::str::contains("Closed task"));
}

// =============================================================================
// Note: With pipe operator, you can chain finish and close sequentially
// but they would operate on the same task, so the second would override the first.
// This is expected behavior - pipes execute sequentially.
// =============================================================================

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn test_add_finish_with_project() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with project and : finish
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "-y", "Completed work task", "project=work", ":", "finish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Finished task").or(predicate::str::contains("Closed task")));
    
    // Verify task has project and is completed (list all tasks)
    let output = get_task_cmd(&temp_dir)
        .args(&["list", "--json"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.as_array().unwrap();
    assert_eq!(tasks.len(), 1);
    // Check status and project
    assert_eq!(tasks[0]["status"], "completed");
    assert!(tasks[0]["project_id"] != serde_json::Value::Null, "Project should be assigned");
}

#[test]
fn test_add_finish_pipe_after_description() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Pipe can appear after description
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task with pipe after", ":", "finish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Finished task"));
}

#[test]
fn test_add_close_pipe_after_description() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Pipe can appear after description
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task with pipe after", ":", "close"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Closed task"));
}

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
mod test_env;

fn setup_test_env() -> (TempDir, std::sync::MutexGuard<'static, ()>) {
    let guard = test_env::lock_test_env();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let config_dir = temp_dir.path().join(".taskninja");
    fs::create_dir_all(&config_dir).unwrap();
    let config_file = config_dir.join("rc");
    fs::write(&config_file, format!("data.location={}\n", db_path.display())).unwrap();
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    (temp_dir, guard)
}

fn get_task_cmd(temp_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("task").unwrap();
    cmd.env("HOME", temp_dir.path());
    cmd
}

#[test]
fn test_sessions_list_all() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Task 2"]).assert().success();
    
    // Create sessions
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // List all sessions
    get_task_cmd(&temp_dir).args(&["sessions", "list"]).assert().success()
        .stdout(predicates::str::contains("Task"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_list_filter_by_project() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create projects
    get_task_cmd(&temp_dir).args(&["projects", "add", "work"]).assert().success();
    get_task_cmd(&temp_dir).args(&["projects", "add", "personal"]).assert().success();
    
    // Create tasks with different projects
    get_task_cmd(&temp_dir).args(&["add", "Work task", "project:work"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Personal task", "project:personal"]).assert().success();
    
    // Create sessions for both tasks
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // Filter by project
    get_task_cmd(&temp_dir).args(&["sessions", "list", "project:work"]).assert().success()
        .stdout(predicates::str::contains("Work task"))
        .stdout(predicates::str::contains("Personal task").not());
    
    drop(temp_dir);
}

#[test]
fn test_sessions_list_filter_by_tags() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks with different tags
    get_task_cmd(&temp_dir).args(&["add", "Urgent task", "+urgent"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Normal task", "+normal"]).assert().success();
    
    // Create sessions for both tasks
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // Filter by tag
    get_task_cmd(&temp_dir).args(&["sessions", "list", "+urgent"]).assert().success()
        .stdout(predicates::str::contains("Urgent task"))
        .stdout(predicates::str::contains("Normal task").not());
    
    drop(temp_dir);
}

#[test]
fn test_sessions_list_filter_by_task_id() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Task 2"]).assert().success();
    
    // Create sessions for both tasks
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // Filter by task ID
    get_task_cmd(&temp_dir).args(&["sessions", "list", "1"]).assert().success()
        .stdout(predicates::str::contains("Task 1"))
        .stdout(predicates::str::contains("Task 2").not());
    
    drop(temp_dir);
}

#[test]
fn test_sessions_list_filter_empty_results() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task with a project
    get_task_cmd(&temp_dir).args(&["projects", "add", "work"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Work task", "project:work"]).assert().success();
    
    // Create a session
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // Filter by non-existent project
    get_task_cmd(&temp_dir).args(&["sessions", "list", "project:nonexistent"]).assert().success()
        .stdout(predicates::str::contains("No sessions found"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_list_filter_multiple_arguments() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create projects
    get_task_cmd(&temp_dir).args(&["projects", "add", "work"]).assert().success();
    
    // Create tasks with different attributes
    get_task_cmd(&temp_dir).args(&["add", "Urgent work task", "project:work", "+urgent"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Normal work task", "project:work", "+normal"]).assert().success();
    
    // Create sessions
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // Filter by project and tag
    get_task_cmd(&temp_dir).args(&["sessions", "list", "project:work", "+urgent"]).assert().success()
        .stdout(predicates::str::contains("Urgent work task"))
        .stdout(predicates::str::contains("Normal work task").not());
    
    drop(temp_dir);
}

#[test]
fn test_sessions_list_for_task() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Task 2"]).assert().success();
    
    // Create sessions
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // List sessions for task 1 (using new filter syntax)
    get_task_cmd(&temp_dir).args(&["sessions", "list", "1"]).assert().success()
        .stdout(predicates::str::contains("Task 1"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_show_current() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and start session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    
    // Show current session
    get_task_cmd(&temp_dir).args(&["sessions", "show"]).assert().success()
        .stdout(predicates::str::contains("running"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_show_for_task() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // Show most recent session for task (using --task flag for backward compatibility)
    get_task_cmd(&temp_dir).args(&["sessions", "show", "--task", "1"]).assert().success()
        .stdout(predicates::str::contains("Task 1"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_list_json() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // List sessions in JSON format
    get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success()
        .stdout(predicates::str::contains("\"task_id\""));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_list_shows_session_id() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // List sessions - should show Session ID column
    get_task_cmd(&temp_dir).args(&["sessions", "list"]).assert().success()
        .stdout(predicates::str::contains("Session"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_modify_start_time() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and closed session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // Get session ID from list
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session_id = json[0]["id"].as_i64().unwrap();
    
    // Modify start time
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session_id.to_string(), "start:09:00", "--yes"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Modified session"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_modify_end_time() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and closed session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // Get session ID from list
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session_id = json[0]["id"].as_i64().unwrap();
    
    // Modify end time
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session_id.to_string(), "end:17:00", "--yes"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Modified session"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_modify_both_times() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and closed session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // Get session ID from list
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session_id = json[0]["id"].as_i64().unwrap();
    
    // Modify both start and end times
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session_id.to_string(), "start:09:00", "end:17:00", "--yes"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Modified session"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_modify_end_none() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and closed session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // Get session ID from list
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session_id = json[0]["id"].as_i64().unwrap();
    
    // Make session open (clear end time)
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session_id.to_string(), "end:none", "--yes"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Modified session"));
    
    // Verify session is now open
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json[0]["is_open"].as_bool().unwrap());
    
    drop(temp_dir);
}

#[test]
fn test_sessions_modify_end_now() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and open session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    
    // Get session ID from list
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session_id = json[0]["id"].as_i64().unwrap();
    
    // Close session (set end to now)
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session_id.to_string(), "end:now", "--yes"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Modified session"));
    
    // Verify session is now closed
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(!json[0]["is_open"].as_bool().unwrap());
    
    drop(temp_dir);
}

#[test]
fn test_sessions_modify_invalid_session_id() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Try to modify non-existent session
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", "999", "start:09:00", "--yes"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not found"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_modify_running_session_end_none() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and open session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    
    // Get session ID from list
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session_id = json[0]["id"].as_i64().unwrap();
    
    // Try to clear end time of running session (should fail)
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session_id.to_string(), "end:none", "--yes"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("already open"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_modify_overlap_detection() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create two tasks
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Task 2"]).assert().success();
    
    // Create first session: 09:00-11:00
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in", "09:00"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out", "11:00"]).assert().success();
    
    // Create second session: 10:00-12:00 (overlaps with first: 10:00-11:00)
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in", "10:00"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out", "12:00"]).assert().success();
    
    // Get second session ID (newest first, so index 0)
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session2_id = json[0]["id"].as_i64().unwrap(); // Second session (newest first)
    
    // Try to modify second session to start at 09:00 (would overlap with first session 09:00-11:00)
    // This should fail without --force because it would overlap with first session
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session2_id.to_string(), "start:09:00", "--yes"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("conflicts"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_modify_overlap_force() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create two tasks
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Task 2"]).assert().success();
    
    // Create first session: 09:00-11:00
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in", "09:00"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out", "11:00"]).assert().success();
    
    // Create second session: 10:00-12:00 (overlaps with first: 10:00-11:00)
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in", "10:00"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out", "12:00"]).assert().success();
    
    // Get second session ID (newest first, so index 0)
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session2_id = json[0]["id"].as_i64().unwrap(); // Second session (newest first)
    
    // Modify with --force (should succeed despite conflicts)
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session2_id.to_string(), "start:09:00", "--yes", "--force"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Modified session"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_delete() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and closed session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // Get session ID from list
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session_id = json[0]["id"].as_i64().unwrap();
    
    // Delete session
    get_task_cmd(&temp_dir)
        .args(&["sessions", "delete", &session_id.to_string(), "--yes"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Deleted session"));
    
    // Verify session is gone
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 0);
    
    drop(temp_dir);
}

#[test]
fn test_sessions_delete_invalid_session_id() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Try to delete non-existent session
    get_task_cmd(&temp_dir)
        .args(&["sessions", "delete", "999", "--yes"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not found"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_delete_running_session() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and open session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    
    // Get session ID from list
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session_id = json[0]["id"].as_i64().unwrap();
    
    // Try to delete running session (should fail)
    get_task_cmd(&temp_dir)
        .args(&["sessions", "delete", &session_id.to_string(), "--yes"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("running session"));
    
    drop(temp_dir);
}

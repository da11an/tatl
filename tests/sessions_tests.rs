use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
mod test_env;

fn setup_test_env() -> (TempDir, std::sync::MutexGuard<'static, ()>) {
    let guard = test_env::lock_test_env();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let config_dir = temp_dir.path().join(".tatl");
    fs::create_dir_all(&config_dir).unwrap();
    let config_file = config_dir.join("rc");
    fs::write(&config_file, format!("data.location={}\n", db_path.display())).unwrap();
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    (temp_dir, guard)
}

fn get_task_cmd(temp_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("tatl").unwrap();
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
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
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
    get_task_cmd(&temp_dir).args(&["add", "Work task", "project=work"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Personal task", "project=personal"]).assert().success();
    
    // Create sessions for both tasks
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
    // Filter by project
    get_task_cmd(&temp_dir).args(&["sessions", "list", "project=work"]).assert().success()
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
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
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
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
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
    get_task_cmd(&temp_dir).args(&["add", "Work task", "project=work"]).assert().success();
    
    // Create a session
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
    // Filter by non-existent project
    get_task_cmd(&temp_dir).args(&["sessions", "list", "project=nonexistent"]).assert().success()
        .stdout(predicates::str::contains("No sessions found"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_list_filter_multiple_arguments() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create projects
    get_task_cmd(&temp_dir).args(&["projects", "add", "work"]).assert().success();
    
    // Create tasks with different attributes
    get_task_cmd(&temp_dir).args(&["add", "Urgent work task", "project=work", "+urgent"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Normal work task", "project=work", "+normal"]).assert().success();
    
    // Create sessions
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
    // Filter by project and tag
    get_task_cmd(&temp_dir).args(&["sessions", "list", "project=work", "+urgent"]).assert().success()
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
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
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
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    
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
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
    // Show most recent session for task (using --task flag for backward compatibility)
    get_task_cmd(&temp_dir).args(&["sessions", "--task", "1", "show"]).assert().success()
        .stdout(predicates::str::contains("Task 1"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_list_json() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
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
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
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
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
    // Get session ID from list
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session_id = json[0]["id"].as_i64().unwrap();
    
    // Modify start time
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session_id.to_string(), "--yes", "start=09:00"])
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
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
    // Get session ID from list
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session_id = json[0]["id"].as_i64().unwrap();
    
    // Modify end time
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session_id.to_string(), "--yes", "end=17:00"])
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
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
    // Get session ID from list
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session_id = json[0]["id"].as_i64().unwrap();
    
    // Modify both start and end times
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session_id.to_string(), "--yes", "start=09:00", "end=17:00"])
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
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
    // Get session ID from list
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session_id = json[0]["id"].as_i64().unwrap();
    
    // Make session open (clear end time)
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session_id.to_string(), "--yes", "end=none"])
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
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    
    // Get session ID from list
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session_id = json[0]["id"].as_i64().unwrap();
    
    // Close session (set end to now)
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session_id.to_string(), "--yes", "end:now"])
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
        .args(&["sessions", "modify", "999", "--yes", "start:09:00"])
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
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    
    // Get session ID from list
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session_id = json[0]["id"].as_i64().unwrap();
    
    // Try to clear end time of running session (should fail)
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session_id.to_string(), "--yes", "end:none"])
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
    get_task_cmd(&temp_dir).args(&["on", "09:00"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off", "11:00"]).assert().success();
    
    // Create second session: 10:00-12:00 (overlaps with first: 10:00-11:00)
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on", "10:00"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off", "12:00"]).assert().success();
    
    // Get second session ID (newest first, so index 0)
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session2_id = json[0]["id"].as_i64().unwrap(); // Second session (newest first)
    
    // Try to modify second session to start at 09:00 (would overlap with first session 09:00-11:00)
    // This should fail without --force because it would overlap with first session
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session2_id.to_string(), "--yes", "start:09:00"])
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
    get_task_cmd(&temp_dir).args(&["on", "09:00"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off", "11:00"]).assert().success();
    
    // Create second session: 10:00-12:00 (overlaps with first: 10:00-11:00)
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on", "10:00"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off", "12:00"]).assert().success();
    
    // Get second session ID (newest first, so index 0)
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let session2_id = json[0]["id"].as_i64().unwrap(); // Second session (newest first)
    
    // Modify with --force (should succeed despite conflicts)
    get_task_cmd(&temp_dir)
        .args(&["sessions", "modify", &session2_id.to_string(), "--yes", "--force", "start:09:00"])
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
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
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
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    
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

// ============================================================================
// Time Report Tests
// ============================================================================

#[test]
fn test_sessions_report_all_time() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and add a session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["onoff", "2026-01-14T10:00..2026-01-14T12:00", "1", "-y"]).assert().success();
    
    // Run report (all time)
    let output = get_task_cmd(&temp_dir).args(&["sessions", "report"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    // Should show time report header and totals
    assert!(stdout.contains("Project") && stdout.contains("Time"), "Should have report table header");
    assert!(stdout.contains("TOTAL"), "Should have TOTAL line");
    assert!(stdout.contains("2h 00m"), "Should show 2 hours logged");
    assert!(stdout.contains("100.0%"), "Should show 100%");
    
    drop(temp_dir);
}

#[test]
fn test_sessions_report_date_range() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and add sessions on different days
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["onoff", "2026-01-10T10:00..2026-01-10T12:00", "1", "-y"]).assert().success();
    get_task_cmd(&temp_dir).args(&["onoff", "2026-01-15T10:00..2026-01-15T11:00", "1", "-y"]).assert().success();
    
    // Run report for a specific date range that includes only the first session
    let output = get_task_cmd(&temp_dir).args(&["sessions", "report", "2026-01-10", "2026-01-11"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    // Should show only the first session (2h)
    assert!(stdout.contains("2h 00m"), "Should show 2 hours");
    assert!(stdout.contains("Sessions: 1"), "Should show 1 session");
    
    drop(temp_dir);
}

#[test]
fn test_sessions_report_nested_projects() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks with nested projects
    get_task_cmd(&temp_dir).args(&["add", "Frontend work", "project=client.web.frontend"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Backend work", "project=client.web.backend"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Mobile work", "project=client.mobile"]).assert().success();
    
    // Add sessions
    get_task_cmd(&temp_dir).args(&["enqueue", "1,2,3"]).assert().success();
    get_task_cmd(&temp_dir).args(&["onoff", "2026-01-14T10:00..2026-01-14T12:00", "1", "-y"]).assert().success();  // 2h frontend
    get_task_cmd(&temp_dir).args(&["onoff", "2026-01-14T13:00..2026-01-14T14:00", "2", "-y"]).assert().success();  // 1h backend
    get_task_cmd(&temp_dir).args(&["onoff", "2026-01-14T14:00..2026-01-14T15:00", "3", "-y"]).assert().success();  // 1h mobile
    
    // Run report
    let output = get_task_cmd(&temp_dir).args(&["sessions", "report"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    // Should show hierarchical structure
    assert!(stdout.contains("client"), "Should show client top-level");
    assert!(stdout.contains("web"), "Should show web sub-project");
    assert!(stdout.contains("frontend"), "Should show frontend");
    assert!(stdout.contains("backend"), "Should show backend");
    assert!(stdout.contains("mobile"), "Should show mobile");
    
    // Total should be 4 hours
    assert!(stdout.contains("4h 00m"), "Total should be 4 hours");
    
    drop(temp_dir);
}

#[test]
fn test_sessions_report_no_sessions() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Just create a task with no sessions
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    
    // Run report
    let output = get_task_cmd(&temp_dir).args(&["sessions", "report"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    // Should indicate no sessions
    assert!(stdout.contains("No sessions found"), "Should indicate no sessions");
    
    drop(temp_dir);
}

#[test]
fn test_sessions_report_with_relative_dates() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["onoff", "2026-01-14T10:00..2026-01-14T12:00", "1", "-y"]).assert().success();
    
    // Run report with relative dates
    let output = get_task_cmd(&temp_dir).args(&["sessions", "report", "-30d", "today"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    // Should show the session
    assert!(stdout.contains("Project") && stdout.contains("Time"), "Should have report header");
    assert!(stdout.contains("TOTAL"), "Should have total");
    
    drop(temp_dir);
}

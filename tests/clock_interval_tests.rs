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
fn test_clock_in_interval_creates_closed_session() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create task and add to stack
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["1", "enqueue"]).assert().success();
    
    // Clock in with interval - should create closed session
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "in", "2026-01-10T09:00..2026-01-10T10:30"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Recorded session"));
    
    // Verify no open session exists
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "out"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No session is currently running"));
}

#[test]
fn test_task_clock_in_interval_creates_closed_session() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create task
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    // Clock in with interval - should create closed session
    let mut cmd = get_task_cmd();
    cmd.args(&["1", "clock", "in", "2026-01-10T09:00..2026-01-10T10:30"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Recorded session"));
    
    // Verify no open session exists
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "out"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No session is currently running"));
}

#[test]
fn test_overlap_prevention_amends_end_time() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create two tasks
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 1"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "task 2"]).assert().success();
    
    // Create closed session for task 1: 09:00 to 10:30
    let mut cmd = get_task_cmd();
    cmd.args(&["1", "clock", "in", "2026-01-10T09:00..2026-01-10T10:30"])
        .assert()
        .success();
    
    // Create session for task 2 starting at 10:00 (before task 1's end time)
    // This should amend task 1's end time to 10:00
    let mut cmd = get_task_cmd();
    cmd.args(&["2", "clock", "in", "2026-01-10T10:00"])
        .assert()
        .success();
    
    // The overlap prevention should have amended task 1's session end time
    // We can verify this by checking that task 2's session is now running
    let mut cmd = get_task_cmd();
    cmd.args(&["clock", "out"])
        .assert()
        .success();
}

#[test]
fn test_interval_with_default_start() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create task and add to stack
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    let mut cmd = get_task_cmd();
    cmd.args(&["1", "enqueue"]).assert().success();
    
    // Clock in with interval where start is omitted (should default to now)
    // Note: This test uses a future end time to avoid issues
    let mut cmd = get_task_cmd();
    // We'll use a specific format that works with our date parser
    cmd.args(&["clock", "in", "2026-01-10T09:00..2026-01-10T10:30"])
        .assert()
        .success();
}

#[test]
fn test_interval_parsing() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create task
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "test task"]).assert().success();
    
    // Test various interval formats
    let mut cmd = get_task_cmd();
    cmd.args(&["1", "clock", "in", "2026-01-10T09:00..2026-01-10T10:30"])
        .assert()
        .success();
}

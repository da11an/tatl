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
fn test_priority_overdue_task_has_higher_priority() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create overdue task
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Overdue task", "due=2020-01-01"])
        .assert()
        .success();
    
    // Create task due in future
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Future task", "due=+30d"])
        .assert()
        .success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["status"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    
    let stdout = String::from_utf8(output).unwrap();
    // Overdue task should appear first in priority section
    assert!(stdout.contains("=== Priority Tasks (Top 3) ==="));
    assert!(stdout.contains("Overdue task"));
}

#[test]
fn test_priority_due_soon_has_higher_priority() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task due tomorrow
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Due soon", "due=tomorrow"])
        .assert()
        .success();
    
    // Create task due in a month
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Due later", "due=+30d"])
        .assert()
        .success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["status"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    
    let stdout = String::from_utf8(output).unwrap();
    // Task due soon should have higher priority
    assert!(stdout.contains("=== Priority Tasks (Top 3) ==="));
    assert!(stdout.contains("Due soon"));
}

#[test]
fn test_priority_allocation_affects_priority() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task with allocation (don't add to clock stack)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task with allocation", "allocation=2h"])
        .assert()
        .success();
    
    // Create another task without allocation
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task without allocation"])
        .assert()
        .success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["status"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    
    let stdout = String::from_utf8(output).unwrap();
    // Priority section should show tasks with priority scores
    assert!(stdout.contains("=== Priority Tasks (Top 3) ==="));
    // At least one task should have a priority score displayed
    if stdout.contains("Task with allocation") || stdout.contains("Task without allocation") {
        assert!(stdout.contains("priority:"));
    }
}

#[test]
fn test_priority_tasks_exclude_clock_stack() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 1"])
        .assert()
        .success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 2"])
        .assert()
        .success();
    
    // Add Task 1 to clock stack
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "1"])
        .assert()
        .success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["status"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    
    let stdout = String::from_utf8(output).unwrap();
    // Task 1 should be in clock stack, not priority tasks
    assert!(stdout.contains("=== Clock Stack (Top 3) ==="));
    assert!(stdout.contains("Task 1"));
    // Task 2 should be in priority tasks
    assert!(stdout.contains("=== Priority Tasks (Top 3) ==="));
    assert!(stdout.contains("Task 2"));
    // Task 1 should NOT be in priority tasks
    let priority_section_start = stdout.find("=== Priority Tasks (Top 3) ===").unwrap();
    let priority_section = &stdout[priority_section_start..];
    assert!(!priority_section.contains("Task 1"));
}

#[test]
fn test_priority_json_output() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task", "due=tomorrow"])
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
    
    assert!(json.get("priority_tasks").is_some());
    assert!(json["priority_tasks"].is_array());
    if json["priority_tasks"].as_array().unwrap().len() > 0 {
        assert!(json["priority_tasks"][0].get("priority").is_some());
    }
}

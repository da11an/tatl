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
fn test_task_list_table_formatting() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create project first
    get_task_cmd(&temp_dir).args(&["projects", "add", "work"]).assert().success();
    
    // Create tasks
    get_task_cmd(&temp_dir).args(&["add", "Task 1", "project:work", "+urgent"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Task 2"]).assert().success();
    
    // List tasks - should show table format with Kanban column (replaced Status)
    get_task_cmd(&temp_dir).args(&["list"]).assert().success()
        .stdout(predicates::str::contains("ID"))
        .stdout(predicates::str::contains("Description"))
        .stdout(predicates::str::contains("Kanban"));
    
    drop(temp_dir);
}

#[test]
fn test_task_list_json_format() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    
    // List tasks in JSON format
    get_task_cmd(&temp_dir).args(&["list", "--json"]).assert().success()
        .stdout(predicates::str::contains("\"id\""))
        .stdout(predicates::str::contains("\"description\""))
        .stdout(predicates::str::contains("\"status\""));
    
    drop(temp_dir);
}

#[test]
fn test_task_list_allocation_column() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task with allocation
    get_task_cmd(&temp_dir).args(&["add", "Task with allocation", "allocation:2h30m"]).assert().success();
    
    // Create task without allocation
    get_task_cmd(&temp_dir).args(&["add", "Task without allocation"]).assert().success();
    
    // List tasks - should show allocation column (header is now "Alloc")
    get_task_cmd(&temp_dir).args(&["list"]).assert().success()
        .stdout(predicates::str::contains("Alloc"))  // Header (changed from "Allocation")
        .stdout(predicates::str::contains("2h30m0s"))  // Formatted allocation
        .stdout(predicates::str::contains("1"));  // Task ID
    
    drop(temp_dir);
}

#[test]
fn test_task_list_allocation_column_empty() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task without allocation
    get_task_cmd(&temp_dir).args(&["add", "Task without allocation"]).assert().success();
    
    // List tasks - allocation column should be present but empty
    let output = get_task_cmd(&temp_dir).args(&["list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    // Verify allocation column header exists (now "Alloc")
    assert!(stdout.contains("Alloc"), "Should have Alloc column header");
    
    // Verify task row exists (allocation should be empty/blank)
    assert!(stdout.contains("1"), "Should show task ID");
    assert!(stdout.contains("Task without allocation"), "Should show task description");
    
    drop(temp_dir);
}

#[test]
fn test_task_list_allocation_various_formats() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks with different allocation formats
    get_task_cmd(&temp_dir).args(&["add", "Task 1", "allocation:1h"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Task 2", "allocation:30m"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Task 3", "allocation:45s"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Task 4", "allocation:2h15m30s"]).assert().success();
    
    // List tasks - should show all allocations correctly formatted
    let output = get_task_cmd(&temp_dir).args(&["list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    // Verify all allocation formats are displayed
    assert!(stdout.contains("1h0m0s") || stdout.contains("1h"), "Should show 1 hour allocation");
    assert!(stdout.contains("30m0s") || stdout.contains("30m"), "Should show 30 minutes allocation");
    assert!(stdout.contains("45s"), "Should show 45 seconds allocation");
    assert!(stdout.contains("2h15m30s"), "Should show complex allocation");
    
    drop(temp_dir);
}

#[test]
fn test_task_list_allocation_column_position() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task with allocation
    get_task_cmd(&temp_dir).args(&["add", "Task with allocation", "allocation:1h"]).assert().success();
    
    // List tasks and verify allocation column is after Due column
    let output = get_task_cmd(&temp_dir).args(&["list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    // Find header line (header is now "Alloc")
    let header_line = stdout.lines()
        .find(|l| l.contains("ID") && l.contains("Alloc"))
        .unwrap();
    
    // Verify column order: Due comes before alloc
    let due_pos = header_line.find("Due").unwrap();
    let alloc_pos = header_line.find("Alloc").unwrap();
    assert!(due_pos < alloc_pos, "Due column should come before alloc column");
    
    drop(temp_dir);
}

#[test]
fn test_task_list_priority_column() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task with due date (affects priority)
    get_task_cmd(&temp_dir).args(&["add", "Urgent task", "due:tomorrow"]).assert().success();
    
    // Create task without due date
    get_task_cmd(&temp_dir).args(&["add", "Normal task"]).assert().success();
    
    // List tasks - should show Priority column
    let output = get_task_cmd(&temp_dir).args(&["list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    // Verify Priority column header exists
    assert!(stdout.contains("Priority"), "Should have Priority column header");
    
    // Verify priority values are shown for pending tasks
    // Priority should be a decimal number like "1.0", "11.0", etc.
    // Just verify the urgent task shows some priority value (a number)
    assert!(stdout.lines().any(|l| {
        l.contains("Urgent task") && l.split_whitespace().any(|w| w.parse::<f64>().is_ok())
    }), "Urgent task should have priority value");
    
    drop(temp_dir);
}

#[test]
fn test_task_list_priority_empty_for_completed() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create and complete a task
    get_task_cmd(&temp_dir).args(&["add", "Task to complete"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["finish", "1", "--yes"]).assert().success();
    
    // List tasks - completed tasks should not show priority
    let output = get_task_cmd(&temp_dir).args(&["list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    // Find the completed task line
    let completed_line = stdout.lines()
        .find(|l| l.contains("Task to complete") && l.contains("completed"));
    
    if let Some(line) = completed_line {
        // Priority column should be empty for completed tasks
        // The line should have spaces where priority would be
        assert!(!line.contains("1.0"), "Completed task should not show priority 1.0");
    }
    
    drop(temp_dir);
}

#[test]
fn test_stack_display_formatting() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks and add to stack
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Task 2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    
    // Show stack - should show formatted display
    get_task_cmd(&temp_dir).args(&["list"]).assert().success()
        .stdout(predicates::str::contains("ID"));
    
    drop(temp_dir);
}

#[test]
fn test_stack_json_format() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and add to stack
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    
    // Show task list in JSON format
    get_task_cmd(&temp_dir).args(&["list", "--json"]).assert().success()
        .stdout(predicates::str::contains("\"id\""))
        .stdout(predicates::str::contains("\"description\""));
    
    drop(temp_dir);
}

#[test]
fn test_projects_list_table_formatting() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create projects
    get_task_cmd(&temp_dir).args(&["projects", "add", "work"]).assert().success();
    get_task_cmd(&temp_dir).args(&["projects", "add", "home"]).assert().success();
    
    // List projects - should show table format
    get_task_cmd(&temp_dir).args(&["projects", "list"]).assert().success()
        .stdout(predicates::str::contains("ID"))
        .stdout(predicates::str::contains("Name"));
    
    drop(temp_dir);
}

#[test]
fn test_projects_list_json_format() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create project
    get_task_cmd(&temp_dir).args(&["projects", "add", "work"]).assert().success();
    
    // List projects in JSON format
    get_task_cmd(&temp_dir).args(&["projects", "list", "--json"]).assert().success()
        .stdout(predicates::str::contains("\"id\""))
        .stdout(predicates::str::contains("\"name\""));
    
    drop(temp_dir);
}

#[test]
fn test_clock_transition_messages() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and start clock
    get_task_cmd(&temp_dir).args(&["add", "Test task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    
    // On - should show explicit message
    get_task_cmd(&temp_dir).args(&["on"]).assert().success()
        .stdout(predicates::str::contains("Started timing"));
    
    // Off - should show explicit message
    get_task_cmd(&temp_dir).args(&["off"]).assert().success()
        .stdout(predicates::str::contains("Stopped timing"));
    
    drop(temp_dir);
}

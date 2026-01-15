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
    let config_dir = temp_dir.path().join(".taskninja");
    fs::create_dir_all(&config_dir).unwrap();
    let config_file = config_dir.join("rc");
    fs::write(&config_file, format!("data.location={}\n", db_path.display())).unwrap();
    
    // Set HOME to temp_dir so the config file is found
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    (temp_dir, guard)
}

fn get_task_cmd(temp_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("task").unwrap();
    cmd.env("HOME", temp_dir.path());
    cmd
}

#[test]
fn test_clock_show_no_longer_exists() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Initialize database
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "test task"]).assert().success();
    
    // Verify clock show command no longer exists
    // Note: clap returns exit code 0 even for unrecognized commands, so we check stderr
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["clock", "show"]).output().unwrap();
    assert!(output.stderr.len() > 0, "Should have error output");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unrecognized subcommand") && stderr.contains("show"), 
            "Should show unrecognized subcommand error");
}

#[test]
fn test_clock_list_empty_stack() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Initialize database
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "test task"]).assert().success();
    
    // Clear stack
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "clear"]).assert().success();
    
    // Verify empty stack message
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Clock stack is empty"));
}

#[test]
fn test_clock_list_shows_full_task_details() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create project first
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["projects", "add", "work"]).assert().success();
    
    // Create task with project, tags, and due date
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Complete project documentation", "project:work", "+urgent", "+docs", "due:2026-01-15"])
        .assert()
        .success();
    
    // Add to clock stack
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Verify clock list shows full details
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Pos"))  // Position column
        .stdout(predicate::str::contains("ID"))   // ID column
        .stdout(predicate::str::contains("Description"))  // Description column
        .stdout(predicate::str::contains("Status"))  // Status column
        .stdout(predicate::str::contains("Project"))  // Project column
        .stdout(predicate::str::contains("Tags"))  // Tags column
        .stdout(predicate::str::contains("Due"))  // Due column
        .stdout(predicate::str::contains("0"))  // Position 0
        .stdout(predicate::str::contains("1"))  // Task ID
        .stdout(predicate::str::contains("Complete project documentation"))  // Description
        .stdout(predicate::str::contains("pending"))  // Status
        .stdout(predicate::str::contains("work"))  // Project
        .stdout(predicate::str::contains("+urgent"))  // Tags
        .stdout(predicate::str::contains("2026-01-15"));  // Due date
}

#[test]
fn test_clock_list_shows_multiple_tasks_sorted_by_position() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create projects first
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["projects", "add", "work"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["projects", "add", "personal"]).assert().success();
    
    // Create multiple tasks
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 1", "project:work"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 2", "project:personal"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 3", "+urgent"]).assert().success();
    
    // Add all to clock stack
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "2"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "3"]).assert().success();
    
    // Verify all tasks appear in order
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["clock", "list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    // Check positions are in order
    assert!(stdout.contains("0") && stdout.contains("1") && stdout.contains("2"));
    
    // Check all task IDs appear
    assert!(stdout.contains("1") && stdout.contains("2") && stdout.contains("3"));
    
    // Check all descriptions appear
    assert!(stdout.contains("Task 1") && stdout.contains("Task 2") && stdout.contains("Task 3"));
    
    // Verify position 0 comes before position 1, etc.
    // Find lines that start with position numbers (data rows, not header)
    let pos_0_line_idx = stdout.lines()
        .position(|l| l.trim_start().starts_with("0") && l.contains("1") && !l.contains("ID"))
        .unwrap();
    let pos_1_line_idx = stdout.lines()
        .position(|l| l.trim_start().starts_with("1") && l.contains("2") && !l.contains("ID"))
        .unwrap();
    let pos_2_line_idx = stdout.lines()
        .position(|l| l.trim_start().starts_with("2") && l.contains("3") && !l.contains("ID"))
        .unwrap();
    
    assert!(pos_0_line_idx < pos_1_line_idx);
    assert!(pos_1_line_idx < pos_2_line_idx);
}

#[test]
fn test_clock_list_json_output() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "test task"]).assert().success();
    
    // Add to clock stack
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Verify JSON output
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"index\""))
        .stdout(predicate::str::contains("\"task_id\""))
        .stdout(predicate::str::contains("\"task_description\""))
        .stdout(predicate::str::contains("\"ordinal\""));
}

#[test]
fn test_clock_list_position_column_first() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "test task"]).assert().success();
    
    // Add to clock stack
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "1"]).assert().success();
    
    // Verify position column is first in header
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["clock", "list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    // Find header line
    let header_line = stdout.lines()
        .find(|l| l.contains("Pos") && l.contains("ID"))
        .unwrap();
    
    // Verify "Pos" comes before "ID" in header
    let pos_pos = header_line.find("Pos").unwrap();
    let id_pos = header_line.find("ID").unwrap();
    assert!(pos_pos < id_pos, "Position column should come before ID column");
}

#[test]
fn test_clock_list_after_pick_shows_new_order() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create three tasks
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 2"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 3"]).assert().success();
    
    // Enqueue all tasks
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "1"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "2"]).assert().success();
    
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "3"]).assert().success();
    
    // Pick task at position 2 (task 3) to move to top
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["clock", "pick", "2"]).assert().success();
    
    // Verify task 3 is now at position 0
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["clock", "list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    // Find the line with position 0
    let pos_0_line = stdout.lines()
        .find(|l| l.trim_start().starts_with("0") && !l.contains("ID"))
        .unwrap();
    
    // Verify task 3 is at position 0
    assert!(pos_0_line.contains("3"), "Task 3 should be at position 0 after pick");
}

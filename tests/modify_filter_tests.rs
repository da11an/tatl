use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;

fn setup_test_env() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let config_dir = temp_dir.path().join(".taskninja");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("rc"), format!("data.location={}\n", db_path.display())).unwrap();
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    temp_dir
}

fn get_task_cmd() -> Command {
    Command::cargo_bin("task").unwrap()
}

#[test]
fn test_modify_with_filter_multiple_tasks() {
    let temp_dir = setup_test_env();
    
    // Create multiple tasks with same tag
    get_task_cmd().args(&["add", "Task 1", "+urgent"]).assert().success();
    get_task_cmd().args(&["add", "Task 2", "+urgent"]).assert().success();
    get_task_cmd().args(&["add", "Task 3", "+urgent"]).assert().success();
    
    // Modify with filter - should match multiple tasks
    // Note: This will prompt for confirmation, so we use --yes flag
    get_task_cmd()
        .args(&["+urgent", "modify", "--yes", "+important"])
        .assert()
        .success();
    
    // Verify all tasks have both tags
    let output = get_task_cmd()
        .args(&["list", "+urgent"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_str = String::from_utf8(output).unwrap();
    
    // All tasks should still have +urgent
    assert!(output_str.contains("Task 1"));
    assert!(output_str.contains("Task 2"));
    assert!(output_str.contains("Task 3"));
    
    drop(temp_dir);
}

#[test]
fn test_modify_with_filter_single_match() {
    let temp_dir = setup_test_env();
    
    // Create tasks
    get_task_cmd().args(&["add", "Task 1", "+urgent"]).assert().success();
    get_task_cmd().args(&["add", "Task 2"]).assert().success();
    
    // Modify with filter that matches one task
    get_task_cmd()
        .args(&["+urgent", "modify", "--yes", "description:Updated task"])
        .assert()
        .success();
    
    // Verify the task was updated
    let output = get_task_cmd()
        .args(&["list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_str = String::from_utf8(output).unwrap();
    
    assert!(output_str.contains("Updated task"));
    
    drop(temp_dir);
}

#[test]
fn test_modify_with_filter_no_matches() {
    let temp_dir = setup_test_env();
    
    // Create a task
    get_task_cmd().args(&["add", "Task 1"]).assert().success();
    
    // Try to modify with filter that matches nothing
    get_task_cmd()
        .args(&["+nonexistent", "modify", "--yes", "+tag"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error:"));
    
    drop(temp_dir);
}

#[test]
fn test_modify_with_filter_and_project() {
    let temp_dir = setup_test_env();
    
    // Create project
    get_task_cmd().args(&["projects", "add", "work"]).assert().success();
    
    // Create tasks
    get_task_cmd().args(&["add", "Task 1", "project:work"]).assert().success();
    get_task_cmd().args(&["add", "Task 2", "project:work"]).assert().success();
    
    // Modify with project filter
    get_task_cmd()
        .args(&["project:work", "modify", "--yes", "+important"])
        .assert()
        .success();
    
    // Verify tasks were updated
    let output = get_task_cmd()
        .args(&["list", "+important"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_str = String::from_utf8(output).unwrap();
    
    assert!(output_str.contains("Task 1"));
    assert!(output_str.contains("Task 2"));
    
    drop(temp_dir);
}

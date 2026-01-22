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
    fs::write(config_dir.join("rc"), format!("data.location={}\n", db_path.display())).unwrap();
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    (temp_dir, guard)
}

fn get_task_cmd() -> Command {
    Command::cargo_bin("tatl").unwrap()
}

#[test]
fn test_user_error_format() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Test that user errors have "Error: " prefix and exit code 1
    get_task_cmd().args(&["add"]).assert()
        .failure()
        .code(1)
        .stderr(predicate::str::starts_with("Error:"));
    
    drop(temp_dir);
}

#[test]
fn test_task_not_found_error() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Test error message for non-existent task
    get_task_cmd().args(&["show", "999"]).assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Error:"))
        .stderr(predicate::str::contains("not found"));
    
    drop(temp_dir);
}

#[test]
fn test_project_not_found_error() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Test error message for non-existent project
    get_task_cmd().args(&["projects", "archive", "nonexistent"]).assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("Failed to archive project"));
    
    drop(temp_dir);
}

#[test]
fn test_invalid_task_id_error() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Test error message for invalid task ID
    get_task_cmd().args(&["show", "abc"]).assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Invalid filter token"));
    
    drop(temp_dir);
}

#[test]
fn test_empty_stack_error() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Test error message for empty stack
    get_task_cmd().args(&["on"]).assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Error:"))
        .stderr(predicate::str::contains("empty"));
    
    drop(temp_dir);
}

#[test]
fn test_no_session_running_error() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Test error message for no running session
    get_task_cmd().args(&["off"]).assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Error:"))
        .stderr(predicate::str::contains("No session"));
    
    drop(temp_dir);
}

#[test]
fn test_project_name_validation() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Test invalid project name (contains invalid characters)
    get_task_cmd().args(&["projects", "add", "work@home"]).assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Error:"))
        .stderr(predicate::str::contains("Invalid project name"));
    
    drop(temp_dir);
}

#[test]
fn test_empty_description_error() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Empty description currently allowed (should still succeed)
    get_task_cmd().args(&["add", ""]).assert()
        .success()
        .stdout(predicate::str::contains("Created task"));
    
    drop(temp_dir);
}

#[test]
fn test_duplicate_project_error() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a project
    get_task_cmd().args(&["projects", "add", "work"]).assert().success();
    
    // Try to create duplicate
    get_task_cmd().args(&["projects", "add", "work"]).assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Error:"))
        .stderr(predicate::str::contains("already exists"));
    
    drop(temp_dir);
}

#[test]
fn test_error_messages_go_to_stderr() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Error messages should go to stderr, not stdout
    get_task_cmd().args(&["modify", "999", "description"]).assert()
        .failure()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("Failed to modify task"));
    
    drop(temp_dir);
}

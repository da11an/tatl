mod test_env;
// Tests for task projects list command

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;

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
    (temp_dir, guard)
}

/// Helper to create a new command with test environment
fn new_cmd(temp_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("task").unwrap();
    cmd.env("HOME", temp_dir.path());
    cmd
}

#[test]
fn test_projects_list_basic() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create some projects
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["projects", "add", "home"])
        .assert()
        .success();
    
    // Test: task projects list
    new_cmd(&temp_dir)
        .args(&["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("work"))
        .stdout(predicate::str::contains("home"))
        .stdout(predicate::str::contains("ID"))
        .stdout(predicate::str::contains("Name"))
        .stdout(predicate::str::contains("Status"));
}

#[test]
fn test_projects_list_empty() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Test: task projects list with no projects
    new_cmd(&temp_dir)
        .args(&["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No projects found"));
}

#[test]
fn test_projects_list_json() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a project
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    
    // Test: task projects list --json
    new_cmd(&temp_dir)
        .args(&["projects", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("\"id\""))
        .stdout(predicate::str::contains("\"is_archived\""))
        .stdout(predicate::str::contains("work"));
}

#[test]
fn test_projects_list_archived() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create projects
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["projects", "add", "old"])
        .assert()
        .success();
    
    // Archive one
    new_cmd(&temp_dir)
        .args(&["projects", "archive", "old"])
        .assert()
        .success();
    
    // Test: task projects list (should not show archived)
    new_cmd(&temp_dir)
        .args(&["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("work"))
        .stdout(predicate::str::contains("old").not());
    
    // Test: task projects list --archived (should show archived)
    new_cmd(&temp_dir)
        .args(&["projects", "list", "--archived"])
        .assert()
        .success()
        .stdout(predicate::str::contains("old"));
}

#[test]
fn test_projects_list_not_intercepted_by_filter_handler() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a project
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    
    // This should work - "projects" is a global subcommand, not a filter
    new_cmd(&temp_dir)
        .args(&["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("work"));
    
    // This should also work - "list" is a task subcommand, not intercepted
    new_cmd(&temp_dir)
        .args(&["list"])
        .assert()
        .success();
}

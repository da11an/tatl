mod test_env;
// Integration tests for Task Ninja CLI commands
// These test the full CLI interface end-to-end

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::env;
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
fn test_projects_add() {
    let (temp_dir, _guard) = setup_test_env();
    let mut cmd = new_cmd(&temp_dir);
    
    cmd.args(&["projects", "add", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created project 'work'"));
}

#[test]
fn test_projects_list() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add a project first
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    
    // List projects
    new_cmd(&temp_dir)
        .args(&["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("work"));
}

#[test]
fn test_projects_add_duplicate_fails() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add project
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    
    // Try to add duplicate
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_projects_nested() {
    let (temp_dir, _guard) = setup_test_env();
    
    new_cmd(&temp_dir)
        .args(&["projects", "add", "admin.email"])
        .assert()
        .success();
    
    new_cmd(&temp_dir)
        .args(&["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("admin.email"));
}

#[test]
fn test_projects_rename() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create project
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    
    // Rename it
    new_cmd(&temp_dir)
        .args(&["projects", "rename", "work", "office"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Renamed"));
    
    // Verify new name exists
    new_cmd(&temp_dir)
        .args(&["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("office"));
}

#[test]
fn test_projects_archive() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create and archive project
    new_cmd(&temp_dir)
        .args(&["projects", "add", "old"])
        .assert()
        .success();
    
    new_cmd(&temp_dir)
        .args(&["projects", "archive", "old"])
        .assert()
        .success();
    
    // Should not appear in default list
    new_cmd(&temp_dir)
        .args(&["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("old").not());
    
    // Should appear in archived list
    new_cmd(&temp_dir)
        .args(&["projects", "list", "--archived"])
        .assert()
        .success()
        .stdout(predicate::str::contains("old"));
}

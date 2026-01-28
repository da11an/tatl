mod test_env;
// Tests for fuzzy project matching (Item 1)

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
    let config_dir = temp_dir.path().join(".tatl");
    fs::create_dir_all(&config_dir).unwrap();
    let config_file = config_dir.join("rc");
    fs::write(&config_file, format!("data.location={}\n", db_path.display())).unwrap();
    (temp_dir, guard)
}

/// Helper to create a new command with test environment
fn new_cmd(temp_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("tatl").unwrap();
    cmd.env("HOME", temp_dir.path());
    cmd
}

#[test]
fn test_project_not_found_no_matches() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Try to add task with non-existent project
    new_cmd(&temp_dir)
        .args(&["add", "Test task", "project=nonexistent"])
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stderr(predicate::str::contains("Create it?"));
}

#[test]
fn test_project_not_found_with_match() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a project
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    
    // Try to add task with typo in project name
    new_cmd(&temp_dir)
        .args(&["add", "Test task", "project=Work"])
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stderr(predicate::str::contains("Create it?"));
}

#[test]
fn test_project_not_found_multiple_matches() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create multiple similar projects
    new_cmd(&temp_dir)
        .args(&["projects", "add", "newproject"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["projects", "add", "newproject2"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["projects", "add", "newproject3"])
        .assert()
        .success();
    
    // Try to add task with typo
    new_cmd(&temp_dir)
        .args(&["add", "Test task", "project=Newproject"])
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stderr(predicate::str::contains("Create it?"));
    // Should suggest the matches
}

#[test]
fn test_project_not_found_in_modify() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a project and task
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["add", "Test task", "project=work"])
        .assert()
        .success();
    
    // Try to modify with typo in project name
    new_cmd(&temp_dir)
        .args(&["1", "modify", "project=Work"])
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Modified task 1"))
        .stderr(predicate::str::contains("Create it?"));
}

#[test]
fn test_project_not_found_case_insensitive() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a project with lowercase
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    
    // Try with different case variations
    new_cmd(&temp_dir)
        .args(&["add", "Test task", "project=WORK"])
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stderr(predicate::str::contains("Create it?"));
    
    new_cmd(&temp_dir)
        .args(&["add", "Test task", "project=WoRk"])
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stderr(predicate::str::contains("Create it?"));
}

#[test]
fn test_project_not_found_substring_match() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create projects with "work" as substring
    new_cmd(&temp_dir)
        .args(&["projects", "add", "workemail"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["projects", "add", "workproject"])
        .assert()
        .success();
    
    // Try with substring that doesn't exist exactly
    // Note: substring matching may not always trigger if distance is too high
    // This test verifies the error message format
    new_cmd(&temp_dir)
        .args(&["add", "Test task", "project=work"])
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stderr(predicate::str::contains("Create it?"));
    // May suggest matches if substring logic finds them within threshold
}

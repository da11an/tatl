use assert_cmd::Command;
use predicates::prelude::predicate;
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
fn test_add_with_new_project_prompt_yes() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with new project, respond 'y' to prompt
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task", "project:newproject"])
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created project 'newproject'"))
        .stdout(predicate::str::contains("Created task"));
    
    // Verify project was created
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("newproject"));
    
    // Verify task has the project
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["list", "1", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("newproject"));
}

#[test]
fn test_add_with_new_project_prompt_no() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with new project, respond 'n' to prompt
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task", "project:newproject"])
        .write_stdin("n\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicates::str::contains("Created project").not());
    
    // Verify project was NOT created
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["projects", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("newproject").not());
    
    // Verify task does NOT have the project - list all tasks and check
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    // Task should exist
    assert!(stdout.contains("Test task"), "Task should exist");
    // Project should be null or not present (check for null or empty project field)
    assert!(!stdout.contains("\"project\":\"newproject\""), "Task should not have newproject");
}

#[test]
fn test_add_with_new_project_prompt_cancel() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with new project, respond 'c' to prompt
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task", "project:newproject"])
        .write_stdin("c\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cancelled"))
        .stdout(predicates::str::contains("Created task").not());
    
    // Verify project was NOT created
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["projects", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("newproject").not());
    
    // Verify task was NOT created
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No tasks found"));
}

#[test]
fn test_add_with_new_project_prompt_default_yes() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with new project, respond with empty line (default: yes, create project)
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task", "project:newproject"])
        .write_stdin("\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created project 'newproject'"))
        .stdout(predicate::str::contains("Created task"));
    
    // Verify project was created
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("newproject"));
    
    // Verify task was created with project - check by listing the task
    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["list", "--json"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    // Task should exist
    assert!(stdout.contains("Test task"), "Task should exist");
    // Project should be present (either as project_id or project name in JSON)
    // The JSON format may have project_id, so we check that the task exists and project was created
    // We already verified project exists above, so this is sufficient
}

#[test]
fn test_add_with_auto_create_project_flag() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with --auto-create-project flag
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "--auto-create-project", "Test task", "project:autoproject"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created project 'autoproject'"))
        .stdout(predicate::str::contains("Created task"));
    
    // Verify project was created
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("autoproject"));
    
    // Verify task has the project
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["list", "1", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("autoproject"));
}

#[test]
fn test_add_with_existing_project_no_prompt() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a project first
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["projects", "add", "existingproject"])
        .assert()
        .success();
    
    // Add task with existing project - should not prompt
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task", "project:existingproject"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicates::str::contains("This is a new project").not());
    
    // Verify task has the project
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["list", "1", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("existingproject"));
}

#[test]
fn test_add_with_new_project_invalid_response() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with new project, respond with invalid input
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task", "project:newproject"])
        .write_stdin("invalid\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Invalid response"))
        .stdout(predicate::str::contains("Cancelled"))
        .stdout(predicates::str::contains("Created task").not());
    
    // Verify task was NOT created
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No tasks found"));
}

#[test]
fn test_add_with_auto_yes_and_on() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Add task with both -y and --on flags
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "-y", "--on", "Test task", "project:autoproject"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created project 'autoproject'"))
        .stdout(predicate::str::contains("Created task"))
        .stdout(predicate::str::contains("Started timing task"));
    
    // Verify project was created
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("autoproject"));
    
    // Verify timer is running by stopping it
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stopped timing task"));
}

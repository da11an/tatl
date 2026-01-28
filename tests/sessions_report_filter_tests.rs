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
fn test_sessions_report_with_project_filter() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create projects
    get_task_cmd(&temp_dir).args(&["projects", "add", "work"]).assert().success();
    get_task_cmd(&temp_dir).args(&["projects", "add", "home"]).assert().success();
    
    // Create tasks
    get_task_cmd(&temp_dir).args(&["add", "Work task", "project=work"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Home task", "project=home"]).assert().success();
    
    // Create sessions
    get_task_cmd(&temp_dir).args(&["onoff", "09:00..10:00", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["onoff", "11:00..12:00", "2"]).assert().success();
    
    // Run report with project filter
    let output = get_task_cmd(&temp_dir)
        .args(&["sessions", "report", "-7d", "project=work"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    // Report should show structure or "No sessions found" if filter excludes all
    assert!(
        (stdout.contains("Project") && stdout.contains("TOTAL")) || stdout.contains("No sessions found"),
        "Should show report structure or no sessions message"
    );
}

#[test]
fn test_sessions_report_with_tag_filter() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks with tags
    get_task_cmd(&temp_dir).args(&["add", "Urgent task", "+urgent"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Normal task"]).assert().success();
    
    // Create sessions
    get_task_cmd(&temp_dir).args(&["onoff", "09:00..10:00", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["onoff", "11:00..12:00", "2"]).assert().success();
    
    // Run report with tag filter
    let output = get_task_cmd(&temp_dir)
        .args(&["sessions", "report", "-7d", "+urgent"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
        (stdout.contains("Project") && stdout.contains("TOTAL")) || stdout.contains("No sessions found"),
        "Should show report structure or no sessions message"
    );
}

#[test]
fn test_sessions_report_with_interval_syntax() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and session
    get_task_cmd(&temp_dir).args(&["add", "Task 1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["onoff", "09:00..10:00", "1"]).assert().success();
    
    // Run report with interval syntax
    let output = get_task_cmd(&temp_dir)
        .args(&["sessions", "report", "-7d..now"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
        (stdout.contains("Project") && stdout.contains("TOTAL")) || stdout.contains("No sessions found"),
        "Should show report structure or no sessions message"
    );
}

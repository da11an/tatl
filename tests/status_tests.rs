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
fn test_report_empty_state() {
    let (temp_dir, _guard) = setup_test_env();
    let mut cmd = get_task_cmd(&temp_dir);

    cmd.args(&["report"])
        .assert()
        .success()
        .stdout(predicate::str::contains("TATL DASHBOARD"))
        .stdout(predicate::str::contains("QUEUE"))
        .stdout(predicate::str::contains("no tasks in queue"))
        .stdout(predicate::str::contains("TODAY'S SESSIONS"))
        .stdout(predicate::str::contains("ATTENTION NEEDED"));
}

#[test]
fn test_report_with_clocked_in_task() {
    let (temp_dir, _guard) = setup_test_env();

    // Create a task and clock in
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task"])
        .assert()
        .success();

    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["on", "1"])
        .assert()
        .success();

    // Wait a moment to ensure session is created
    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut cmd = get_task_cmd(&temp_dir);
    let output = cmd.args(&["report"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();
    // Should show the active task with ▶ indicator
    assert!(stdout.contains("QUEUE") && stdout.contains("1 tasks"));
    assert!(stdout.contains("Test task"));
    // Should show current session in today's sessions
    assert!(stdout.contains("[current]"));
}

#[test]
fn test_report_with_queue() {
    let (temp_dir, _guard) = setup_test_env();

    // Create multiple tasks and add to stack
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 1"])
        .assert()
        .success();

    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Task 2"])
        .assert()
        .success();

    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "1"])
        .assert()
        .success();

    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["enqueue", "2"])
        .assert()
        .success();

    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["report"])
        .assert()
        .success()
        .stdout(predicate::str::contains("QUEUE"))
        .stdout(predicate::str::contains("Task 1"))
        .stdout(predicate::str::contains("Task 2"));
}

#[test]
fn test_report_with_overdue_tasks() {
    let (temp_dir, _guard) = setup_test_env();

    // Create a task with past due date
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Overdue task", "due=2020-01-01"])
        .assert()
        .success();

    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["report"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ATTENTION NEEDED"))
        .stdout(predicate::str::contains("Overdue"));
}

#[test]
fn test_report_with_today_sessions() {
    let (temp_dir, _guard) = setup_test_env();

    // Create a task, clock in, and clock out
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["add", "Test task"])
        .assert()
        .success();

    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["on", "1"])
        .assert()
        .success();

    // Wait a moment
    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["off"])
        .assert()
        .success();

    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["report"])
        .assert()
        .success()
        .stdout(predicate::str::contains("TODAY'S SESSIONS"))
        .stdout(predicate::str::contains("Test task"));
}

#[test]
fn test_report_period_option() {
    let (temp_dir, _guard) = setup_test_env();

    // Test with month period
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["report", "--period=month"])
        .assert()
        .success()
        .stdout(predicate::str::contains("THIS MONTH"));

    // Test with year period
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["report", "--period=year"])
        .assert()
        .success()
        .stdout(predicate::str::contains("THIS YEAR"));
}

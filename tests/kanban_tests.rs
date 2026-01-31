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
fn test_stage_column_appears_in_list() {
    let (temp_dir, _guard) = setup_test_env();

    // Create a task
    get_task_cmd(&temp_dir).args(&["add", "Test task"]).assert().success();

    // List tasks - should show Stage column
    get_task_cmd(&temp_dir).args(&["list"]).assert().success()
        .stdout(predicates::str::contains("Stage"));

    drop(temp_dir);
}

#[test]
fn test_stage_status_proposed() {
    let (temp_dir, _guard) = setup_test_env();

    // Create a task (not in stack, no sessions)
    get_task_cmd(&temp_dir).args(&["add", "Proposed task"]).assert().success();

    // Should show as "proposed"
    get_task_cmd(&temp_dir).args(&["list"]).assert().success()
        .stdout(predicates::str::contains("proposed"));

    drop(temp_dir);
}

#[test]
fn test_stage_status_planned() {
    let (temp_dir, _guard) = setup_test_env();

    // Create two tasks
    get_task_cmd(&temp_dir).args(&["add", "First task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Second task"]).assert().success();

    // Add both to stack (first will be at position 0, second at position 1)
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();

    // Second task should be "planned" (position > 0, no sessions)
    let output = get_task_cmd(&temp_dir).args(&["list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // First task at position 0 should be planned (NEXT/LIVE stages were removed)
    assert!(stdout.lines().any(|l| l.contains("First task") && l.contains("planned")),
        "First task should be planned (position 0)");

    // Second task at position 1 should be planned
    assert!(stdout.lines().any(|l| l.contains("Second task") && l.contains("planned")),
        "Second task should be planned (position > 0)");

    drop(temp_dir);
}

// NOTE: NEXT/LIVE stages were removed (folded into planned + active indicator).

#[test]
fn test_stage_status_completed() {
    let (temp_dir, _guard) = setup_test_env();

    // Create a task, add to stack, clock in, complete
    get_task_cmd(&temp_dir).args(&["add", "Done task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["close", "1", "--yes"]).assert().success();

    // Should show as "completed"
    get_task_cmd(&temp_dir).args(&["list"]).assert().success()
        .stdout(predicates::str::contains("completed"));

    drop(temp_dir);
}

#[test]
fn test_stage_filter_proposed() {
    let (temp_dir, _guard) = setup_test_env();

    // Create two tasks - one proposed, one in stack
    get_task_cmd(&temp_dir).args(&["add", "Proposed task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Stack task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();

    // Filter by stage=proposed should only show first task
    let output = get_task_cmd(&temp_dir).args(&["list", "stage=proposed"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Proposed task"), "Should show proposed task");
    assert!(!stdout.contains("Stack task"), "Should not show stack task");

    drop(temp_dir);
}

// NOTE: `stage=next` and `stage=live` filters were removed along with NEXT/LIVE stages.

#[test]
fn test_stage_filter_completed() {
    let (temp_dir, _guard) = setup_test_env();

    // Create two tasks - one completed, one pending
    get_task_cmd(&temp_dir).args(&["add", "Done task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Pending task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();
    get_task_cmd(&temp_dir).args(&["close", "1", "--yes"]).assert().success();

    // Filter by stage=completed should only show first task
    let output = get_task_cmd(&temp_dir).args(&["list", "stage=completed"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Done task"), "Should show done task");
    assert!(!stdout.contains("Pending task"), "Should not show pending task");

    drop(temp_dir);
}

#[test]
fn test_stage_filter_case_insensitive() {
    let (temp_dir, _guard) = setup_test_env();

    // Create a task
    get_task_cmd(&temp_dir).args(&["add", "Test task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();

    // Filter should be case-insensitive
    get_task_cmd(&temp_dir).args(&["list", "stage=PLANNED"]).assert().success()
        .stdout(predicates::str::contains("Test task"));
    get_task_cmd(&temp_dir).args(&["list", "stage=Planned"]).assert().success()
        .stdout(predicates::str::contains("Test task"));
    get_task_cmd(&temp_dir).args(&["list", "stage=planned"]).assert().success()
        .stdout(predicates::str::contains("Test task"));

    drop(temp_dir);
}

#[test]
fn test_stage_status_suspended() {
    let (temp_dir, _guard) = setup_test_env();

    // Create a task, clock in, clock out, then remove from stack
    get_task_cmd(&temp_dir).args(&["add", "Suspended task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();

    // Wait a moment then clock out
    std::thread::sleep(std::time::Duration::from_millis(100));
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();

    // Remove from stack
    get_task_cmd(&temp_dir).args(&["dequeue"]).assert().success();

    // Should show as "suspended" (not in stack, has sessions)
    get_task_cmd(&temp_dir).args(&["list"]).assert().success()
        .stdout(predicates::str::contains("suspended"));

    drop(temp_dir);
}

#[test]
fn test_stage_status_planned_with_sessions() {
    let (temp_dir, _guard) = setup_test_env();

    // Create two tasks
    get_task_cmd(&temp_dir).args(&["add", "Working task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Another task"]).assert().success();

    // Add first task to stack and clock in
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on"]).assert().success();

    // Wait a moment then clock out
    std::thread::sleep(std::time::Duration::from_millis(100));
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();

    // Add another task on top and switch to it
    get_task_cmd(&temp_dir).args(&["on", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();

    // First task should be "planned" (position > 0, regardless of sessions)
    let output = get_task_cmd(&temp_dir).args(&["list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    assert!(stdout.lines().any(|l| l.contains("Working task") && l.contains("planned")),
        "Working task should have 'planned' stage status");

    drop(temp_dir);
}

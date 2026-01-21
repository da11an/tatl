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
fn test_kanban_column_appears_in_list() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task
    get_task_cmd(&temp_dir).args(&["add", "Test task"]).assert().success();
    
    // List tasks - should show Kanban column
    get_task_cmd(&temp_dir).args(&["list"]).assert().success()
        .stdout(predicates::str::contains("Kanban"));
    
    drop(temp_dir);
}

#[test]
fn test_kanban_status_proposed() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task (not in stack, no sessions)
    get_task_cmd(&temp_dir).args(&["add", "Proposed task"]).assert().success();
    
    // Should show as "proposed"
    get_task_cmd(&temp_dir).args(&["list"]).assert().success()
        .stdout(predicates::str::contains("proposed"));
    
    drop(temp_dir);
}

#[test]
fn test_kanban_status_queued() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create two tasks
    get_task_cmd(&temp_dir).args(&["add", "First task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Second task"]).assert().success();
    
    // Add both to stack (first will be at position 0, second at position 1)
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    
    // Second task should be "queued" (position > 0, no sessions)
    let output = get_task_cmd(&temp_dir).args(&["list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    // First task at position 0 should be NEXT
    assert!(stdout.lines().any(|l| l.contains("First task") && l.contains("NEXT")), 
        "First task should be NEXT (position 0)");
    
    // Second task at position 1 should be queued
    assert!(stdout.lines().any(|l| l.contains("Second task") && l.contains("queued")), 
        "Second task should be queued (position > 0)");
    
    drop(temp_dir);
}

#[test]
fn test_kanban_status_next() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task and add to stack (position 0)
    get_task_cmd(&temp_dir).args(&["add", "Next task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    
    // Should show as "NEXT" (position 0, clock out)
    get_task_cmd(&temp_dir).args(&["list"]).assert().success()
        .stdout(predicates::str::contains("NEXT"));
    
    drop(temp_dir);
}

#[test]
fn test_kanban_status_live() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task, add to stack, and clock in
    get_task_cmd(&temp_dir).args(&["add", "Live task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    
    // Should show as "LIVE" (position 0, clock in)
    get_task_cmd(&temp_dir).args(&["list"]).assert().success()
        .stdout(predicates::str::contains("LIVE"));
    
    // Clean up - clock out
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    drop(temp_dir);
}

#[test]
fn test_kanban_status_next_when_live_in_front() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create two tasks, enqueue both, and clock in
    get_task_cmd(&temp_dir).args(&["add", "Live task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Next task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    
    let output = get_task_cmd(&temp_dir).args(&["list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.lines().any(|l| l.contains("Live task") && l.contains("LIVE")));
    assert!(stdout.lines().any(|l| l.contains("Next task") && l.contains("NEXT")));
    
    // Clean up
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    drop(temp_dir);
}

#[test]
fn test_kanban_status_done() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task, add to stack, clock in, complete
    get_task_cmd(&temp_dir).args(&["add", "Done task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["finish", "1", "--yes"]).assert().success();
    
    // Should show as "done"
    get_task_cmd(&temp_dir).args(&["list"]).assert().success()
        .stdout(predicates::str::contains("done"));
    
    drop(temp_dir);
}

#[test]
fn test_kanban_filter_proposed() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create two tasks - one proposed, one in stack
    get_task_cmd(&temp_dir).args(&["add", "Proposed task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Stack task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    
    // Filter by kanban:proposed should only show first task
    let output = get_task_cmd(&temp_dir).args(&["list", "kanban:proposed"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    assert!(stdout.contains("Proposed task"), "Should show proposed task");
    assert!(!stdout.contains("Stack task"), "Should not show stack task");
    
    drop(temp_dir);
}

#[test]
fn test_kanban_filter_next() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task and add to stack
    get_task_cmd(&temp_dir).args(&["add", "Next task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Proposed task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    
    // Filter by kanban:next should only show first task
    let output = get_task_cmd(&temp_dir).args(&["list", "kanban:next"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    assert!(stdout.contains("Next task"), "Should show next task");
    assert!(!stdout.contains("Proposed task"), "Should not show proposed task");
    
    drop(temp_dir);
}

#[test]
fn test_kanban_filter_live() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create two tasks - one live, one proposed
    get_task_cmd(&temp_dir).args(&["add", "Live task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Proposed task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    
    // Filter by kanban:live should only show first task
    let output = get_task_cmd(&temp_dir).args(&["list", "kanban:live"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    assert!(stdout.contains("Live task"), "Should show live task");
    assert!(!stdout.contains("Proposed task"), "Should not show proposed task");
    
    // Clean up
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    drop(temp_dir);
}

#[test]
fn test_kanban_filter_done() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create two tasks - one completed, one pending
    get_task_cmd(&temp_dir).args(&["add", "Done task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Pending task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["finish", "1", "--yes"]).assert().success();
    
    // Filter by kanban:done should only show first task
    let output = get_task_cmd(&temp_dir).args(&["list", "kanban:done"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    assert!(stdout.contains("Done task"), "Should show done task");
    assert!(!stdout.contains("Pending task"), "Should not show pending task");
    
    drop(temp_dir);
}

#[test]
fn test_kanban_filter_case_insensitive() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task
    get_task_cmd(&temp_dir).args(&["add", "Test task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    
    // Filter should be case-insensitive
    get_task_cmd(&temp_dir).args(&["list", "kanban:NEXT"]).assert().success()
        .stdout(predicates::str::contains("Test task"));
    get_task_cmd(&temp_dir).args(&["list", "kanban:Next"]).assert().success()
        .stdout(predicates::str::contains("Test task"));
    get_task_cmd(&temp_dir).args(&["list", "kanban:next"]).assert().success()
        .stdout(predicates::str::contains("Test task"));
    
    drop(temp_dir);
}

#[test]
fn test_kanban_status_paused() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task, clock in, clock out, then remove from stack
    get_task_cmd(&temp_dir).args(&["add", "Paused task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    
    // Wait a moment then clock out
    std::thread::sleep(std::time::Duration::from_millis(100));
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // Remove from stack
    get_task_cmd(&temp_dir).args(&["clock", "drop", "0"]).assert().success();
    
    // Should show as "paused" (not in stack, has sessions)
    get_task_cmd(&temp_dir).args(&["list"]).assert().success()
        .stdout(predicates::str::contains("paused"));
    
    drop(temp_dir);
}

#[test]
fn test_kanban_status_working() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create two tasks
    get_task_cmd(&temp_dir).args(&["add", "Working task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Another task"]).assert().success();
    
    // Add first task to stack and clock in
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    
    // Wait a moment then clock out
    std::thread::sleep(std::time::Duration::from_millis(100));
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    // Add another task on top
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "pick", "1"]).assert().success();
    
    // First task should be "working" (position > 0, has sessions)
    let output = get_task_cmd(&temp_dir).args(&["list"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    assert!(stdout.lines().any(|l| l.contains("Working task") && l.contains("working")), 
        "Working task should have 'working' kanban status");
    
    drop(temp_dir);
}

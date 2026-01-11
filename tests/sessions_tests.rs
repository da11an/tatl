use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;

fn setup_test_env() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let config_dir = temp_dir.path().join(".taskninja");
    fs::create_dir_all(&config_dir).unwrap();
    let config_file = config_dir.join("rc");
    fs::write(&config_file, format!("data.location={}\n", db_path.display())).unwrap();
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    temp_dir
}

fn get_task_cmd() -> Command {
    Command::cargo_bin("task").unwrap()
}

#[test]
fn test_sessions_list_all() {
    let temp_dir = setup_test_env();
    
    // Create tasks
    get_task_cmd().args(&["add", "Task 1"]).assert().success();
    get_task_cmd().args(&["add", "Task 2"]).assert().success();
    
    // Create sessions
    get_task_cmd().args(&["1", "enqueue"]).assert().success();
    get_task_cmd().args(&["clock", "in"]).assert().success();
    get_task_cmd().args(&["clock", "out"]).assert().success();
    
    get_task_cmd().args(&["2", "enqueue"]).assert().success();
    get_task_cmd().args(&["clock", "in"]).assert().success();
    get_task_cmd().args(&["clock", "out"]).assert().success();
    
    // List all sessions
    get_task_cmd().args(&["sessions", "list"]).assert().success()
        .stdout(predicates::str::contains("Task"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_list_for_task() {
    let temp_dir = setup_test_env();
    
    // Create tasks
    get_task_cmd().args(&["add", "Task 1"]).assert().success();
    get_task_cmd().args(&["add", "Task 2"]).assert().success();
    
    // Create sessions
    get_task_cmd().args(&["1", "enqueue"]).assert().success();
    get_task_cmd().args(&["clock", "in"]).assert().success();
    get_task_cmd().args(&["clock", "out"]).assert().success();
    
    get_task_cmd().args(&["2", "enqueue"]).assert().success();
    get_task_cmd().args(&["clock", "in"]).assert().success();
    get_task_cmd().args(&["clock", "out"]).assert().success();
    
    // List sessions for task 1
    get_task_cmd().args(&["1", "sessions", "list"]).assert().success()
        .stdout(predicates::str::contains("Task 1"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_show_current() {
    let temp_dir = setup_test_env();
    
    // Create task and start session
    get_task_cmd().args(&["add", "Task 1"]).assert().success();
    get_task_cmd().args(&["1", "enqueue"]).assert().success();
    get_task_cmd().args(&["clock", "in"]).assert().success();
    
    // Show current session
    get_task_cmd().args(&["sessions", "show"]).assert().success()
        .stdout(predicates::str::contains("running"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_show_for_task() {
    let temp_dir = setup_test_env();
    
    // Create task and session
    get_task_cmd().args(&["add", "Task 1"]).assert().success();
    get_task_cmd().args(&["1", "enqueue"]).assert().success();
    get_task_cmd().args(&["clock", "in"]).assert().success();
    get_task_cmd().args(&["clock", "out"]).assert().success();
    
    // Show most recent session for task
    get_task_cmd().args(&["1", "sessions", "show"]).assert().success()
        .stdout(predicates::str::contains("Task 1"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_list_json() {
    let temp_dir = setup_test_env();
    
    // Create task and session
    get_task_cmd().args(&["add", "Task 1"]).assert().success();
    get_task_cmd().args(&["1", "enqueue"]).assert().success();
    get_task_cmd().args(&["clock", "in"]).assert().success();
    get_task_cmd().args(&["clock", "out"]).assert().success();
    
    // List sessions in JSON format
    get_task_cmd().args(&["sessions", "list", "--json"]).assert().success()
        .stdout(predicates::str::contains("\"task_id\""));
    
    drop(temp_dir);
}

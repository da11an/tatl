use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
use tatl::db::DbConnection;
use tatl::repo::TaskRepo;
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
fn test_modify_rejects_invalid_respawn_rule() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task
    get_task_cmd(&temp_dir)
        .args(&["add", "Test task"])
        .assert()
        .success();
    
    // Try to modify with invalid respawn rule
    get_task_cmd(&temp_dir)
        .args(&["modify", "1", "respawn=every:fridayy"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid respawn rule"));
}

#[test]
fn test_modify_clears_respawn_with_empty_value() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task with respawn
    get_task_cmd(&temp_dir)
        .args(&["add", "Test task", "respawn=daily"])
        .assert()
        .success();
    
    // Empty respawn: value should clear it (same as respawn:none)
    get_task_cmd(&temp_dir)
        .args(&["modify", "1", "respawn="])
        .assert()
        .success();
    
    // Verify respawn was cleared
    let conn = DbConnection::connect().unwrap();
    let task = TaskRepo::get_by_id(&conn, 1).unwrap().unwrap();
    assert_eq!(task.respawn, None);
}

#[test]
fn test_modify_rejects_invalid_weekday() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task
    get_task_cmd(&temp_dir)
        .args(&["add", "Test task"])
        .assert()
        .success();
    
    // Try to modify with invalid weekday
    get_task_cmd(&temp_dir)
        .args(&["modify", "1", "respawn=weekdays:invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid weekday"));
}

#[test]
fn test_modify_rejects_invalid_interval() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task
    get_task_cmd(&temp_dir)
        .args(&["add", "Test task"])
        .assert()
        .success();
    
    // Try to modify with invalid interval (zero days)
    get_task_cmd(&temp_dir)
        .args(&["modify", "1", "respawn=every:0d"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid respawn rule"));
    
    // Try with negative
    get_task_cmd(&temp_dir)
        .args(&["modify", "1", "respawn=every:-1d"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid respawn rule"));
}

#[test]
fn test_modify_accepts_valid_respawn_rule() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task
    get_task_cmd(&temp_dir)
        .args(&["add", "Test task"])
        .assert()
        .success();
    
    // Modify with valid respawn rule
    get_task_cmd(&temp_dir)
        .args(&["modify", "1", "respawn=daily"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Modified task 1"))
        .stdout(predicate::str::contains("↻"));
    
    // Verify the respawn rule was set
    let conn = DbConnection::connect().unwrap();
    let task = TaskRepo::get_by_id(&conn, 1).unwrap().unwrap();
    assert_eq!(task.respawn, Some("daily".to_string()));
}

#[test]
fn test_modify_shows_preview_for_daily() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task
    get_task_cmd(&temp_dir)
        .args(&["add", "Daily task"])
        .assert()
        .success();
    
    // Modify with daily respawn
    get_task_cmd(&temp_dir)
        .args(&["modify", "1", "respawn=daily"])
        .assert()
        .success()
        .stdout(predicate::str::contains("↻"))
        .stdout(predicate::str::contains("next day"));
}

#[test]
fn test_modify_shows_preview_for_weekly() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task
    get_task_cmd(&temp_dir)
        .args(&["add", "Weekly task"])
        .assert()
        .success();
    
    // Modify with weekly respawn
    get_task_cmd(&temp_dir)
        .args(&["modify", "1", "respawn=weekly"])
        .assert()
        .success()
        .stdout(predicate::str::contains("↻"))
        .stdout(predicate::str::contains("next week"));
}

#[test]
fn test_modify_shows_preview_for_weekdays() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task
    get_task_cmd(&temp_dir)
        .args(&["add", "Weekday task"])
        .assert()
        .success();
    
    // Modify with weekdays respawn
    get_task_cmd(&temp_dir)
        .args(&["modify", "1", "respawn=weekdays:mon,wed,fri"])
        .assert()
        .success()
        .stdout(predicate::str::contains("↻"))
        .stdout(predicate::str::contains("Mon").or(predicate::str::contains("Wed")).or(predicate::str::contains("Fri")));
}

#[test]
fn test_modify_shows_preview_for_every_interval() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task
    get_task_cmd(&temp_dir)
        .args(&["add", "Interval task"])
        .assert()
        .success();
    
    // Modify with every:2d respawn
    get_task_cmd(&temp_dir)
        .args(&["modify", "1", "respawn=every:2d"])
        .assert()
        .success()
        .stdout(predicate::str::contains("↻"))
        .stdout(predicate::str::contains("2 days later"));
}

#[test]
fn test_modify_shows_preview_for_monthdays() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task
    get_task_cmd(&temp_dir)
        .args(&["add", "Monthly task"])
        .assert()
        .success();
    
    // Modify with monthdays respawn
    get_task_cmd(&temp_dir)
        .args(&["modify", "1", "respawn=monthdays:1,15"])
        .assert()
        .success()
        .stdout(predicate::str::contains("↻"))
        .stdout(predicate::str::contains("day 1").or(predicate::str::contains("day 15")));
}

#[test]
fn test_modify_shows_preview_for_nth_weekday() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task
    get_task_cmd(&temp_dir)
        .args(&["add", "Nth weekday task"])
        .assert()
        .success();
    
    // Modify with nth weekday respawn
    get_task_cmd(&temp_dir)
        .args(&["modify", "1", "respawn=nth:2:tue"])
        .assert()
        .success()
        .stdout(predicate::str::contains("↻"))
        .stdout(predicate::str::contains("2nd").or(predicate::str::contains("Tuesday")));
}

#[test]
fn test_modify_clears_respawn_with_none() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task with respawn
    get_task_cmd(&temp_dir)
        .args(&["add", "Task with respawn", "respawn=daily"])
        .assert()
        .success();
    
    // Clear respawn rule
    get_task_cmd(&temp_dir)
        .args(&["modify", "1", "respawn=none"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Modified task 1"));
    
    // Verify respawn was cleared
    let conn = DbConnection::connect().unwrap();
    let task = TaskRepo::get_by_id(&conn, 1).unwrap().unwrap();
    assert_eq!(task.respawn, None);
}

#[test]
fn test_modify_respawn_validation_prevents_invalid_storage() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task
    get_task_cmd(&temp_dir)
        .args(&["add", "Test task"])
        .assert()
        .success();
    
    // Try to set invalid respawn rule
    get_task_cmd(&temp_dir)
        .args(&["modify", "1", "respawn=invalid:pattern"])
        .assert()
        .failure();
    
    // Verify task was NOT modified (respawn should still be None)
    let conn = DbConnection::connect().unwrap();
    let task = TaskRepo::get_by_id(&conn, 1).unwrap().unwrap();
    assert_eq!(task.respawn, None, "Task should not have respawn rule after failed modification");
}

#[test]
fn test_modify_respawn_with_multiple_fields() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task
    get_task_cmd(&temp_dir)
        .args(&["add", "Test task"])
        .assert()
        .success();
    
    // Modify with respawn and other fields
    get_task_cmd(&temp_dir)
        .args(&["modify", "1", "respawn=weekly", "project=work", "+urgent"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Modified task 1"))
        .stdout(predicate::str::contains("↻"));
    
    // Verify all fields were set
    let conn = DbConnection::connect().unwrap();
    let task = TaskRepo::get_by_id(&conn, 1).unwrap().unwrap();
    assert_eq!(task.respawn, Some("weekly".to_string()));
}

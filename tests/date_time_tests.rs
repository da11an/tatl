use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
use task_ninja::utils::date::parse_date_expr;
use task_ninja::utils::duration::parse_duration;
mod test_env;

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
    
    // Set HOME to temp_dir so the config file is found
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    (temp_dir, guard)
}

fn get_task_cmd() -> Command {
    Command::cargo_bin("task").unwrap()
}

#[test]
fn test_absolute_date_parsing() {
    // Test absolute date formats
    assert!(parse_date_expr("2026-01-10").is_ok());
    assert!(parse_date_expr("2026-01-10T14:30").is_ok());
}

#[test]
fn test_relative_date_parsing() {
    // Test relative date formats
    assert!(parse_date_expr("today").is_ok());
    assert!(parse_date_expr("tomorrow").is_ok());
    assert!(parse_date_expr("+2d").is_ok());
    assert!(parse_date_expr("+1w").is_ok());
    assert!(parse_date_expr("-3d").is_ok());
}

#[test]
fn test_end_of_period_parsing() {
    // Test end of period formats
    assert!(parse_date_expr("eod").is_ok());
    assert!(parse_date_expr("eow").is_ok());
    assert!(parse_date_expr("eom").is_ok());
}

#[test]
fn test_time_only_parsing() {
    // Test time-only formats
    assert!(parse_date_expr("9am").is_ok());
    assert!(parse_date_expr("2pm").is_ok());
    assert!(parse_date_expr("14:30").is_ok());
    assert!(parse_date_expr("noon").is_ok());
    assert!(parse_date_expr("midnight").is_ok());
}

#[test]
fn test_duration_parsing() {
    // Test valid duration formats
    assert_eq!(parse_duration("30s").unwrap(), 30);
    assert_eq!(parse_duration("10m").unwrap(), 600);
    assert_eq!(parse_duration("2h").unwrap(), 7200);
    assert_eq!(parse_duration("1h30m").unwrap(), 5400);
    assert_eq!(parse_duration("2d5h30m").unwrap(), 2*86400 + 5*3600 + 30*60);
}

#[test]
fn test_duration_ordering_validation() {
    // Test that units must be in order (d, h, m, s)
    assert!(parse_duration("30m10s").is_ok()); // Valid order
    assert!(parse_duration("10s30m").is_err()); // Invalid order
    assert!(parse_duration("1h30m10s").is_ok()); // Valid order
    assert!(parse_duration("30m1h").is_err()); // Invalid order
}

#[test]
fn test_duration_no_duplicates() {
    // Test that each unit can only appear once
    assert!(parse_duration("1h30m").is_ok());
    assert!(parse_duration("1h30m1h").is_err()); // Duplicate hour
    assert!(parse_duration("30m10m").is_err()); // Duplicate minute
}

#[test]
fn test_duration_integration() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create task with duration
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 1", "allocation:1h30m"]).assert().success();
    
    // Create task with relative date
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 2", "due:+2d"]).assert().success();
    
    // Create task with time-only
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Task 3", "due:9am"]).assert().success();
}

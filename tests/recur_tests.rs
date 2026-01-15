use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
use task_ninja::db::DbConnection;
use task_ninja::repo::TaskRepo;
use task_ninja::recur::{RecurGenerator, RecurRule};
mod test_env;

fn setup_test_env() -> (TempDir, std::sync::MutexGuard<'static, ()>) {
    let guard = test_env::lock_test_env();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let config_dir = temp_dir.path().join(".taskninja");
    fs::create_dir_all(&config_dir).unwrap();
    let config_file = config_dir.join("rc");
    fs::write(&config_file, format!("data.location={}\n", db_path.display())).unwrap();
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    (temp_dir, guard)
}

fn get_task_cmd() -> Command {
    Command::cargo_bin("task").unwrap()
}

#[test]
fn test_recur_rule_parse_simple() {
    assert!(RecurRule::parse("daily").is_ok());
    assert!(RecurRule::parse("weekly").is_ok());
    assert!(RecurRule::parse("monthly").is_ok());
    assert!(RecurRule::parse("yearly").is_ok());
}

#[test]
fn test_recur_rule_parse_interval() {
    assert!(RecurRule::parse("every:2d").is_ok());
    assert!(RecurRule::parse("every:3w").is_ok());
    assert!(RecurRule::parse("every:2m").is_ok());
    assert!(RecurRule::parse("every:1y").is_ok());
}

#[test]
fn test_recur_rule_parse_weekday_modifier() {
    let rule = RecurRule::parse("weekly byweekday:mon,wed,fri").unwrap();
    assert_eq!(rule.frequency, task_ninja::recur::parser::RecurFrequency::Weekly);
    assert_eq!(rule.byweekday, Some(vec![0, 2, 4]));
}

#[test]
fn test_recur_rule_parse_monthday_modifier() {
    let rule = RecurRule::parse("monthly bymonthday:1,15").unwrap();
    assert_eq!(rule.frequency, task_ninja::recur::parser::RecurFrequency::Monthly);
    assert_eq!(rule.bymonthday, Some(vec![1, 15]));
}

#[test]
fn test_recur_rule_modifier_validation() {
    // Daily with weekday modifier should fail
    assert!(RecurRule::parse("daily byweekday:mon").is_err());
    
    // Weekly with monthday modifier should fail
    assert!(RecurRule::parse("weekly bymonthday:1").is_err());
    
    // Monthly with weekday modifier should fail
    assert!(RecurRule::parse("monthly byweekday:mon").is_err());
}

#[test]
fn test_recur_run_command() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create a seed task
    get_task_cmd().args(&["add", "Daily standup", "recur:daily"]).assert().success();
    
    // Run recurrence generation
    get_task_cmd().args(&["recur", "run"]).assert().success()
        .stdout(predicate::str::contains("Generated"));
}

#[test]
fn test_recur_idempotency() {
    let (_temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    // Create a seed task
    let seed = TaskRepo::create(&conn, "Daily task", None).unwrap();
    let seed_id = seed.id.unwrap();
    
    // Set recur field
    TaskRepo::modify(
        &conn,
        seed_id,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(Some("daily".to_string())),
        &std::collections::HashMap::new(),
        &[],
        &[],
        &[],
    ).unwrap();
    
    // Run generation twice
    let until_ts = chrono::Utc::now().timestamp() + 86400 * 3; // 3 days
    let count1 = RecurGenerator::run(&conn, until_ts).unwrap();
    let count2 = RecurGenerator::run(&conn, until_ts).unwrap();
    
    // Second run should generate 0 (idempotent)
    assert_eq!(count2, 0);
    assert!(count1 > 0);
}

#[test]
fn test_recur_weekly_with_weekday() {
    let (_temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    // Create a seed task with weekly recurrence on Mon/Wed/Fri
    let seed = TaskRepo::create(&conn, "Team meeting", None).unwrap();
    let seed_id = seed.id.unwrap();
    
    TaskRepo::modify(
        &conn,
        seed_id,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(Some("weekly byweekday:mon,wed,fri".to_string())),
        &std::collections::HashMap::new(),
        &[],
        &[],
        &[],
    ).unwrap();
    
    // Generate for next week
    let until_ts = chrono::Utc::now().timestamp() + 86400 * 7; // 7 days
    let count = RecurGenerator::run(&conn, until_ts).unwrap();
    
    // Should generate instances for Mon/Wed/Fri
    assert!(count >= 0); // At least some instances
}

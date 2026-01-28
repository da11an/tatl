use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
use tatl::db::DbConnection;
use tatl::repo::TaskRepo;
use tatl::respawn::{RespawnRule, RespawnPattern};
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

fn get_task_cmd() -> Command {
    Command::cargo_bin("tatl").unwrap()
}

#[test]
fn test_respawn_rule_parse_simple() {
    assert!(RespawnRule::parse("daily").is_ok());
    assert!(RespawnRule::parse("weekly").is_ok());
    assert!(RespawnRule::parse("monthly").is_ok());
    assert!(RespawnRule::parse("yearly").is_ok());
}

#[test]
fn test_respawn_rule_parse_interval() {
    let rule = RespawnRule::parse("every:2d").unwrap();
    assert_eq!(rule.pattern, RespawnPattern::EveryDays(2));
    
    let rule = RespawnRule::parse("every:3w").unwrap();
    assert_eq!(rule.pattern, RespawnPattern::EveryWeeks(3));
    
    let rule = RespawnRule::parse("every:2m").unwrap();
    assert_eq!(rule.pattern, RespawnPattern::EveryMonths(2));
    
    let rule = RespawnRule::parse("every:1y").unwrap();
    assert_eq!(rule.pattern, RespawnPattern::EveryYears(1));
}

#[test]
fn test_respawn_rule_parse_weekdays() {
    let rule = RespawnRule::parse("weekdays:mon,wed,fri").unwrap();
    assert_eq!(rule.pattern, RespawnPattern::Weekdays(vec![0, 2, 4]));
}

#[test]
fn test_respawn_rule_parse_monthdays() {
    let rule = RespawnRule::parse("monthdays:1,15").unwrap();
    assert_eq!(rule.pattern, RespawnPattern::Monthdays(vec![1, 15]));
}

#[test]
fn test_respawn_rule_parse_nth_weekday() {
    let rule = RespawnRule::parse("nth:2:tue").unwrap();
    assert_eq!(rule.pattern, RespawnPattern::NthWeekday { nth: 2, weekday: 1 });
}

#[test]
fn test_respawn_on_finish() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create a task with respawn rule
    get_task_cmd().args(&["add", "Daily standup", "respawn=daily"]).assert().success();
    
    // Get task ID
    let conn = DbConnection::connect().unwrap();
    let tasks = TaskRepo::list_all(&conn).unwrap();
    let (task, _) = tasks.iter().find(|(t, _)| t.description == "Daily standup").unwrap();
    let task_id = task.id.unwrap();
    
    // Finish the task
    get_task_cmd().args(&["finish", &task_id.to_string(), "-y"]).assert().success()
        .stdout(predicate::str::contains("Respawned"));
    
    // Verify respawned task exists
    let tasks_after = TaskRepo::list_all(&conn).unwrap();
    let respawned: Vec<_> = tasks_after.iter()
        .filter(|(t, _)| t.id != Some(task_id) && t.description == "Daily standup")
        .collect();
    
    assert_eq!(respawned.len(), 1, "Should have one respawned task");
    let (new_task, _) = &respawned[0];
    assert_eq!(new_task.respawn, Some("daily".to_string()));
}

#[test]
fn test_respawn_on_close() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create a task with respawn rule
    get_task_cmd().args(&["add", "Weekly review", "respawn=weekly"]).assert().success();
    
    // Get task ID
    let conn = DbConnection::connect().unwrap();
    let tasks = TaskRepo::list_all(&conn).unwrap();
    let (task, _) = tasks.iter().find(|(t, _)| t.description == "Weekly review").unwrap();
    let task_id = task.id.unwrap();
    
    // Close the task (abandon it)
    get_task_cmd().args(&["close", &task_id.to_string(), "-y"]).assert().success()
        .stdout(predicate::str::contains("Respawned"));
    
    // Verify respawned task exists
    let tasks_after = TaskRepo::list_all(&conn).unwrap();
    let respawned: Vec<_> = tasks_after.iter()
        .filter(|(t, _)| t.id != Some(task_id) && t.description == "Weekly review")
        .collect();
    
    assert_eq!(respawned.len(), 1, "Should have one respawned task");
}

#[test]
fn test_no_respawn_without_rule() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create a task without respawn rule
    get_task_cmd().args(&["add", "One-time task"]).assert().success();
    
    // Get task ID
    let conn = DbConnection::connect().unwrap();
    let tasks = TaskRepo::list_all(&conn).unwrap();
    let (task, _) = tasks.iter().find(|(t, _)| t.description == "One-time task").unwrap();
    let task_id = task.id.unwrap();
    
    // Finish the task
    get_task_cmd().args(&["finish", &task_id.to_string(), "-y"]).assert().success()
        .stdout(predicate::str::contains("Finished"));
    
    // Verify no respawned task exists
    let tasks_after = TaskRepo::list_all(&conn).unwrap();
    let matching: Vec<_> = tasks_after.iter()
        .filter(|(t, _)| t.description == "One-time task")
        .collect();
    
    // Should only have the original (completed) task
    assert_eq!(matching.len(), 1, "Should only have original task");
}

#[test]
fn test_respawn_carries_attributes() {
    let (_temp_dir, _guard) = setup_test_env();
    
    // Create a task with respawn rule and attributes
    get_task_cmd().args(&["add", "Meeting", "respawn=daily", "allocation=1h", "+important", "project=work"]).assert().success();
    
    // Get task ID
    let conn = DbConnection::connect().unwrap();
    let tasks = TaskRepo::list_all(&conn).unwrap();
    let (task, tags) = tasks.iter().find(|(t, _)| t.description == "Meeting").unwrap();
    let task_id = task.id.unwrap();
    let original_alloc = task.alloc_secs;
    let original_project_id = task.project_id;
    
    assert!(tags.contains(&"important".to_string()));
    
    // Finish the task
    get_task_cmd().args(&["finish", &task_id.to_string(), "-y"]).assert().success();
    
    // Verify respawned task has same attributes
    let tasks_after = TaskRepo::list_all(&conn).unwrap();
    let (new_task, new_tags) = tasks_after.iter()
        .find(|(t, _)| t.id != Some(task_id) && t.description == "Meeting")
        .unwrap();
    
    assert_eq!(new_task.alloc_secs, original_alloc);
    assert_eq!(new_task.project_id, original_project_id);
    assert!(new_tags.contains(&"important".to_string()));
    assert_eq!(new_task.respawn, Some("daily".to_string()));
}

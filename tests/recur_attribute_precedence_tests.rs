use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
use tatl::db::DbConnection;
use tatl::repo::{TaskRepo, TemplateRepo, ProjectRepo};
use tatl::recur::RecurGenerator;
mod test_env;

fn setup_test_env() -> (TempDir, std::sync::MutexGuard<'static, ()>) {
    let guard = test_env::lock_test_env();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let config_dir = temp_dir.path().join(".tatl");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("rc"), format!("data.location={}\n", db_path.display())).unwrap();
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    (temp_dir, guard)
}

fn get_task_cmd() -> Command {
    Command::cargo_bin("tatl").unwrap()
}

#[test]
fn test_attribute_precedence_template_base_seed_overrides() {
    let (temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    // Create project
    let project = ProjectRepo::create(&conn, "work").unwrap();
    let project_id = project.id.unwrap();
    
    // Create template with base attributes
    let mut template_payload = std::collections::HashMap::new();
    template_payload.insert("project_id".to_string(), serde_json::json!(project_id));
    template_payload.insert("alloc_secs".to_string(), serde_json::json!(3600)); // 1 hour
    template_payload.insert("tags".to_string(), serde_json::json!(["meeting", "recurring"]));
    TemplateRepo::save(&conn, "standup", &template_payload).unwrap();
    
    // Create seed task with template and overrides
    // Seed overrides: alloc_secs (30m = 1800s), adds tag "urgent"
    // Template provides: project_id, base tags
    get_task_cmd()
        .args(&["add", "Daily standup", "template:standup", "recur:daily", "allocation:30m", "+urgent"])
        .assert()
        .success();
    
    // Get the seed task
    let tasks = TaskRepo::list_all(&conn).unwrap();
    let seed = tasks.iter()
        .find(|(t, _)| t.recur.is_some() && t.description == "Daily standup")
        .map(|(t, _)| t)
        .unwrap();
    
    // Run recurrence generation
    let until_ts = chrono::Utc::now().timestamp() + 86400 * 2; // 2 days
    RecurGenerator::run(&conn, until_ts).unwrap();
    
    // Get generated instances
    let tasks_after = TaskRepo::list_all(&conn).unwrap();
    let instances: Vec<_> = tasks_after.iter()
        .filter(|(t, _)| t.recur.is_none() && t.description == "Daily standup")
        .collect();
    
    assert!(!instances.is_empty(), "Should have generated at least one instance");
    
    // Check attribute precedence: Template (base) → Seed (overrides)
    let (instance, tags) = &instances[0];
    
    // Project ID should come from template (seed didn't override)
    assert_eq!(instance.project_id, Some(project_id), "Project ID should come from template");
    
    // Alloc should come from seed (seed overrides template)
    assert_eq!(instance.alloc_secs, Some(1800), "Alloc should come from seed (30m = 1800s)");
    
    // Tags should be merged: template tags + seed tags
    assert!(tags.contains(&"meeting".to_string()), "Should have template tag 'meeting'");
    assert!(tags.contains(&"recurring".to_string()), "Should have template tag 'recurring'");
    assert!(tags.contains(&"urgent".to_string()), "Should have seed tag 'urgent'");
    
    drop(temp_dir);
}

#[test]
fn test_attribute_precedence_seed_only_no_template() {
    let (temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    // Create seed task without template
    get_task_cmd()
        .args(&["add", "Daily task", "recur:daily", "allocation:1h", "+important"])
        .assert()
        .success();
    
    // Run recurrence generation
    let until_ts = chrono::Utc::now().timestamp() + 86400 * 2; // 2 days
    RecurGenerator::run(&conn, until_ts).unwrap();
    
    // Get generated instances
    let tasks = TaskRepo::list_all(&conn).unwrap();
    let instances: Vec<_> = tasks.iter()
        .filter(|(t, _)| t.recur.is_none() && t.description == "Daily task")
        .collect();
    
    assert!(!instances.is_empty());
    
    // Instance should have seed attributes
    let (instance, tags) = &instances[0];
    assert_eq!(instance.alloc_secs, Some(3600)); // 1h = 3600s
    assert!(tags.contains(&"important".to_string()));
    
    drop(temp_dir);
}

#[test]
fn test_attribute_precedence_template_only_no_seed_overrides() {
    let (temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    // Create project
    let project = ProjectRepo::create(&conn, "work").unwrap();
    let project_id = project.id.unwrap();
    
    // Create template
    let mut template_payload = std::collections::HashMap::new();
    template_payload.insert("project_id".to_string(), serde_json::json!(project_id));
    template_payload.insert("alloc_secs".to_string(), serde_json::json!(1800)); // 30m
    template_payload.insert("tags".to_string(), serde_json::json!(["meeting"]));
    TemplateRepo::save(&conn, "meeting", &template_payload).unwrap();
    
    // Create seed task with template but no overrides
    get_task_cmd()
        .args(&["add", "Team meeting", "template:meeting", "recur:weekly"])
        .assert()
        .success();
    
    // Run recurrence generation
    let until_ts = chrono::Utc::now().timestamp() + 86400 * 7; // 7 days
    RecurGenerator::run(&conn, until_ts).unwrap();
    
    // Get generated instances
    let tasks = TaskRepo::list_all(&conn).unwrap();
    let instances: Vec<_> = tasks.iter()
        .filter(|(t, _)| t.recur.is_none() && t.description == "Team meeting")
        .collect();
    
    assert!(!instances.is_empty());
    
    // Instance should have template attributes (no seed overrides)
    let (instance, tags) = &instances[0];
    assert_eq!(instance.project_id, Some(project_id));
    assert_eq!(instance.alloc_secs, Some(1800));
    assert!(tags.contains(&"meeting".to_string()));
    
    drop(temp_dir);
}

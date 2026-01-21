use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
use tatl::db::DbConnection;
use tatl::repo::{TaskRepo, TemplateRepo};
use std::collections::HashMap;
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
fn test_template_create_and_get() {
    let (temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    let mut payload = HashMap::new();
    payload.insert("project_id".to_string(), serde_json::json!(1));
    payload.insert("alloc_secs".to_string(), serde_json::json!(3600));
    
    TemplateRepo::save(&conn, "meeting", &payload).unwrap();
    
    let template = TemplateRepo::get_by_name(&conn, "meeting").unwrap().unwrap();
    assert_eq!(template.name, "meeting");
    assert_eq!(template.payload.get("project_id").unwrap().as_i64(), Some(1));
    assert_eq!(template.payload.get("alloc_secs").unwrap().as_i64(), Some(3600));
    drop(temp_dir); // Keep temp_dir alive until end
}

#[test]
fn test_template_merge_attributes() {
    let (temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    // Create template
    let mut template_payload = HashMap::new();
    template_payload.insert("project_id".to_string(), serde_json::json!(1));
    template_payload.insert("alloc_secs".to_string(), serde_json::json!(1800));
    template_payload.insert("tags".to_string(), serde_json::json!(["meeting", "recurring"]));
    TemplateRepo::save(&conn, "standup", &template_payload).unwrap();
    
    let template = TemplateRepo::get_by_name(&conn, "standup").unwrap().unwrap();
    
    // Merge with task attributes (task overrides template)
    let (project_id, _due_ts, _scheduled_ts, _wait_ts, alloc_secs, _udas, tags) = 
        TemplateRepo::merge_attributes(
            &template,
            Some(2), // Task overrides project_id
            None,
            None,
            None,
            None, // Task doesn't override alloc_secs
            &HashMap::new(),
            &["urgent".to_string()], // Task adds tag
        );
    
    assert_eq!(project_id, Some(2)); // Task value
    assert_eq!(alloc_secs, Some(1800)); // Template value
    assert!(tags.contains(&"meeting".to_string())); // From template
    assert!(tags.contains(&"urgent".to_string())); // From task
    drop(temp_dir); // Keep temp_dir alive until end
}

#[test]
fn test_task_add_with_template() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a task with template (template will be auto-created from task attributes)
    // Note: We don't need a project for this test
    get_task_cmd().args(&["add", "Daily standup", "template:standup", "allocation:30m"]).assert().success();
    
    // Verify template was created
    let conn = DbConnection::connect().unwrap();
    let template = TemplateRepo::get_by_name(&conn, "standup").unwrap();
    assert!(template.is_some());
    
    // Verify template has the attributes
    let tmpl = template.unwrap();
    assert!(tmpl.payload.contains_key("alloc_secs"));
    assert_eq!(tmpl.payload.get("alloc_secs").unwrap().as_i64(), Some(1800)); // 30m = 1800s
    drop(temp_dir); // Keep temp_dir alive until end
}

#[test]
fn test_recur_with_template() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create a template via task creation (without project to avoid project creation issues)
    get_task_cmd().args(&["add", "Template task", "template:meeting", "allocation:1h", "+meeting"]).assert().success();
    
    // Create a seed task with template and recurrence
    get_task_cmd().args(&["add", "Daily standup", "template:meeting", "recur:daily", "allocation:30m"]).assert().success();
    
    // Run recurrence generation
    get_task_cmd().args(&["recur", "run"]).assert().success();
    
    // Verify instances were created with template attributes
    let conn = DbConnection::connect().unwrap();
    let tasks = TaskRepo::list_all(&conn).unwrap();
    let instances: Vec<_> = tasks.iter()
        .filter(|(t, _)| t.recur.is_none() && t.description == "Daily standup")
        .collect();
    assert!(!instances.is_empty());
    
    // Check that instance has template attributes (+meeting) but seed override (allocation:30m)
    let (instance, tags) = &instances[0];
    assert_eq!(instance.description, "Daily standup");
    // Verify template tag is present (from template)
    // Note: Tags from template should be merged with seed tags
    assert!(tags.contains(&"meeting".to_string()), "Tags: {:?}", tags);
    drop(temp_dir); // Keep temp_dir alive until end
}

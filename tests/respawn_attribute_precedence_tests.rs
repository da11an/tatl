use assert_cmd::Command;
use tempfile::TempDir;
use std::fs;
use tatl::db::DbConnection;
use tatl::repo::{TaskRepo, ProjectRepo};
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
fn test_respawn_carries_project() {
    let (temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    // Create project
    let project = ProjectRepo::create(&conn, "work").unwrap();
    let _project_id = project.id.unwrap();
    
    // Create task with respawn rule and project
    get_task_cmd()
        .args(&["add", "Daily standup", "project:work", "respawn:daily"])
        .assert()
        .success();
    
    // Get the task
    let tasks = TaskRepo::list_all(&conn).unwrap();
    let (task, _) = tasks.iter()
        .find(|(t, _)| t.description == "Daily standup")
        .unwrap();
    let task_id = task.id.unwrap();
    let original_project_id = task.project_id;
    
    // Finish the task to trigger respawn
    get_task_cmd()
        .args(&["finish", &task_id.to_string(), "-y"])
        .assert()
        .success();
    
    // Get respawned task
    let tasks_after = TaskRepo::list_all(&conn).unwrap();
    let (respawned, _) = tasks_after.iter()
        .find(|(t, _)| t.id != Some(task_id) && t.description == "Daily standup")
        .expect("Should have respawned task");
    
    // Project should be carried forward
    assert_eq!(respawned.project_id, original_project_id, "Project should be carried forward");
    
    drop(temp_dir);
}

#[test]
fn test_respawn_carries_allocation() {
    let (temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    // Create task with respawn rule and allocation
    get_task_cmd()
        .args(&["add", "Daily task", "respawn:daily", "allocation:30m"])
        .assert()
        .success();
    
    // Get the task
    let tasks = TaskRepo::list_all(&conn).unwrap();
    let (task, _) = tasks.iter()
        .find(|(t, _)| t.description == "Daily task")
        .unwrap();
    let task_id = task.id.unwrap();
    
    // Finish the task
    get_task_cmd()
        .args(&["finish", &task_id.to_string(), "-y"])
        .assert()
        .success();
    
    // Get respawned task
    let tasks_after = TaskRepo::list_all(&conn).unwrap();
    let (respawned, _) = tasks_after.iter()
        .find(|(t, _)| t.id != Some(task_id) && t.description == "Daily task")
        .expect("Should have respawned task");
    
    // Allocation should be carried forward (30m = 1800s)
    assert_eq!(respawned.alloc_secs, Some(1800), "Allocation should be carried forward");
    
    drop(temp_dir);
}

#[test]
fn test_respawn_carries_tags() {
    let (temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    // Create task with respawn rule and tags
    get_task_cmd()
        .args(&["add", "Meeting", "respawn:daily", "+important", "+recurring"])
        .assert()
        .success();
    
    // Get the task
    let tasks = TaskRepo::list_all(&conn).unwrap();
    let (task, tags) = tasks.iter()
        .find(|(t, _)| t.description == "Meeting")
        .unwrap();
    let task_id = task.id.unwrap();
    
    assert!(tags.contains(&"important".to_string()));
    assert!(tags.contains(&"recurring".to_string()));
    
    // Finish the task
    get_task_cmd()
        .args(&["finish", &task_id.to_string(), "-y"])
        .assert()
        .success();
    
    // Get respawned task
    let tasks_after = TaskRepo::list_all(&conn).unwrap();
    let (_, respawned_tags) = tasks_after.iter()
        .find(|(t, _)| t.id != Some(task_id) && t.description == "Meeting")
        .expect("Should have respawned task");
    
    // Tags should be carried forward
    assert!(respawned_tags.contains(&"important".to_string()), "Should carry 'important' tag");
    assert!(respawned_tags.contains(&"recurring".to_string()), "Should carry 'recurring' tag");
    
    drop(temp_dir);
}

#[test]
fn test_respawn_resets_status() {
    let (temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    // Create task with respawn rule
    get_task_cmd()
        .args(&["add", "Daily check", "respawn:daily"])
        .assert()
        .success();
    
    // Get the task
    let tasks = TaskRepo::list_all(&conn).unwrap();
    let (task, _) = tasks.iter()
        .find(|(t, _)| t.description == "Daily check")
        .unwrap();
    let task_id = task.id.unwrap();
    
    // Finish the task
    get_task_cmd()
        .args(&["finish", &task_id.to_string(), "-y"])
        .assert()
        .success();
    
    // Verify original is completed
    let original = TaskRepo::get_by_id(&conn, task_id).unwrap().unwrap();
    assert_eq!(original.status, tatl::models::TaskStatus::Completed);
    
    // Get respawned task
    let tasks_after = TaskRepo::list_all(&conn).unwrap();
    let (respawned, _) = tasks_after.iter()
        .find(|(t, _)| t.id != Some(task_id) && t.description == "Daily check")
        .expect("Should have respawned task");
    
    // Respawned task should have pending status
    assert_eq!(respawned.status, tatl::models::TaskStatus::Pending, "Respawned task should be pending");
    
    drop(temp_dir);
}

#[test]
fn test_respawn_preserves_respawn_rule() {
    let (temp_dir, _guard) = setup_test_env();
    let conn = DbConnection::connect().unwrap();
    
    // Create task with respawn rule
    get_task_cmd()
        .args(&["add", "Weekly review", "respawn:weekly"])
        .assert()
        .success();
    
    // Get the task
    let tasks = TaskRepo::list_all(&conn).unwrap();
    let (task, _) = tasks.iter()
        .find(|(t, _)| t.description == "Weekly review")
        .unwrap();
    let task_id = task.id.unwrap();
    
    // Finish the task
    get_task_cmd()
        .args(&["finish", &task_id.to_string(), "-y"])
        .assert()
        .success();
    
    // Get respawned task
    let tasks_after = TaskRepo::list_all(&conn).unwrap();
    let (respawned, _) = tasks_after.iter()
        .find(|(t, _)| t.id != Some(task_id) && t.description == "Weekly review")
        .expect("Should have respawned task");
    
    // Respawn rule should be preserved
    assert_eq!(respawned.respawn, Some("weekly".to_string()), "Respawn rule should be preserved");
    
    drop(temp_dir);
}

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
mod test_env;

/// Helper to create a temporary database and set it as the data location
fn setup_test_env() -> (TempDir, std::sync::MutexGuard<'static, ()>) {
    let guard = test_env::lock_test_env();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    
    // Create config file
    let config_dir = temp_dir.path().join(".tatl");
    fs::create_dir_all(&config_dir).unwrap();
    let config_file = config_dir.join("rc");
    fs::write(&config_file, format!("data.location={}\n", db_path.display())).unwrap();
    
    // Set HOME to temp_dir so the config file is found
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    (temp_dir, guard)
}

fn get_task_cmd(temp_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("tatl").unwrap();
    cmd.env("HOME", temp_dir.path());
    cmd
}

// =============================================================================
// color: parsing tests
// =============================================================================

#[test]
fn test_color_column_parsing() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks with different projects
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "Task 1", "project:work"])
        .assert()
        .success();
    
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "Task 2", "project:home"])
        .assert()
        .success();
    
    // List with color:project should succeed and show tasks
    // (colors won't be visible in piped output, but parsing should work)
    get_task_cmd(&temp_dir)
        .args(&["list", "color:project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Task 1"))
        .stdout(predicate::str::contains("Task 2"));
}

#[test]
fn test_fill_column_parsing() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks with different statuses
    get_task_cmd(&temp_dir)
        .args(&["add", "Pending task"])
        .assert()
        .success();
    
    get_task_cmd(&temp_dir)
        .args(&["add", "--finish", "Completed task"])
        .assert()
        .success();
    
    // List with fill:status should succeed
    get_task_cmd(&temp_dir)
        .args(&["list", "fill:status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Pending task"))
        .stdout(predicate::str::contains("Completed task"));
}

#[test]
fn test_color_kanban() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks with different kanban stages
    get_task_cmd(&temp_dir)
        .args(&["add", "Proposed task"])
        .assert()
        .success();
    
    get_task_cmd(&temp_dir)
        .args(&["add", "--enqueue", "Queued task"])
        .assert()
        .success();
    
    // List with color:kanban should succeed
    get_task_cmd(&temp_dir)
        .args(&["list", "color:kanban"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Proposed task"))
        .stdout(predicate::str::contains("Queued task"));
}

#[test]
fn test_color_with_group() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks with different projects
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "Work task 1", "project:work"])
        .assert()
        .success();
    
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "Work task 2", "project:work"])
        .assert()
        .success();
    
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "Home task", "project:home"])
        .assert()
        .success();
    
    // List with group:project color:project should show grouped output
    get_task_cmd(&temp_dir)
        .args(&["list", "group:project", "color:project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[work]"))
        .stdout(predicate::str::contains("[home]"));
}

#[test]
fn test_color_priority_gradient() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks with different priorities (via due dates)
    get_task_cmd(&temp_dir)
        .args(&["add", "Urgent task", "due:today"])
        .assert()
        .success();
    
    get_task_cmd(&temp_dir)
        .args(&["add", "Less urgent", "due:+7d"])
        .assert()
        .success();
    
    // List with color:priority should succeed
    get_task_cmd(&temp_dir)
        .args(&["list", "color:priority"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Urgent task"))
        .stdout(predicate::str::contains("Less urgent"));
}

#[test]
fn test_color_combined_with_filter() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "Work task", "project:work"])
        .assert()
        .success();
    
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "Home task", "project:home"])
        .assert()
        .success();
    
    // Filter and color together
    get_task_cmd(&temp_dir)
        .args(&["list", "project:work", "color:kanban"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Work task"))
        .stdout(predicate::str::contains("Home task").not());
}

#[test]
fn test_color_and_fill_together() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "Task", "project:work"])
        .assert()
        .success();
    
    // Both color and fill
    get_task_cmd(&temp_dir)
        .args(&["list", "color:project", "fill:status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Task"));
}

#[test]
fn test_color_with_sort() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "A task", "project:work"])
        .assert()
        .success();
    
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "B task", "project:home"])
        .assert()
        .success();
    
    // Sort and color
    get_task_cmd(&temp_dir)
        .args(&["list", "sort:project", "color:project"])
        .assert()
        .success();
}

#[test]
fn test_color_with_hide() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "Task", "project:work"])
        .assert()
        .success();
    
    // Hide and color
    get_task_cmd(&temp_dir)
        .args(&["list", "hide:tags", "color:project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Task"));
}

#[test]
fn test_color_matches_group_colors_headers_only() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks with different projects
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "Work task 1", "project:work"])
        .assert()
        .success();
    
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "Work task 2", "project:work"])
        .assert()
        .success();
    
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "Home task", "project:home"])
        .assert()
        .success();
    
    // When color:project matches group:project, only headers should be colored
    // (rows won't be colored, but command should succeed)
    get_task_cmd(&temp_dir)
        .args(&["list", "group:project", "color:project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[work]"))
        .stdout(predicate::str::contains("[home]"));
}

#[test]
fn test_color_does_not_match_group_colors_rows() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks with different priorities (via due dates)
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "Urgent task", "project:work", "due:today"])
        .assert()
        .success();
    
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "Less urgent", "project:work", "due:+7d"])
        .assert()
        .success();
    
    get_task_cmd(&temp_dir)
        .args(&["add", "-y", "Another urgent", "project:home", "due:today"])
        .assert()
        .success();
    
    // When color:priority does NOT match group:project, rows should be colored
    // (group headers won't be colored, but rows will)
    get_task_cmd(&temp_dir)
        .args(&["list", "group:project", "color:priority"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[work]"))
        .stdout(predicate::str::contains("[home]"))
        .stdout(predicate::str::contains("Urgent task"))
        .stdout(predicate::str::contains("Less urgent"));
}

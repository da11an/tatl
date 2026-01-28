mod test_env;
// Tests for filter-before-command pattern (Item 4)

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;

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
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    (temp_dir, guard)
}

/// Helper to create a new command with test environment
fn new_cmd(temp_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("tatl").unwrap();
    cmd.env("HOME", temp_dir.path());
    cmd
}

#[test]
fn test_filter_list_pattern() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create projects first
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["projects", "add", "home"])
        .assert()
        .success();
    
    // Create some tasks
    new_cmd(&temp_dir)
        .args(&["add", "Task 1", "project=work", "+urgent"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["add", "Task 2", "project=home"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["add", "Task 3", "project=work", "+important"])
        .assert()
        .success();
    
    // Test: task list <filter> (CLAP-native pattern)
    new_cmd(&temp_dir)
        .args(&["list", "project=work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Task 1"))
        .stdout(predicate::str::contains("Task 3"));
    
    // Test: task list <filter> (old pattern - backward compatibility)
    new_cmd(&temp_dir)
        .args(&["list", "project=home"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Task 2"));
    
    // Test: task show <id> (CLAP-native)
    new_cmd(&temp_dir)
        .args(&["show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Task 1"));
    
    // Test: task list <filter> --json
    new_cmd(&temp_dir)
        .args(&["list", "--json", "project=work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"project_id\""))
        .stdout(predicate::str::contains("\"description\""));
}

#[test]
fn test_filter_annotate_pattern() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create projects first
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["projects", "add", "home"])
        .assert()
        .success();
    
    // Create some tasks
    new_cmd(&temp_dir)
        .args(&["add", "Task 1", "project=work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["add", "Task 2", "project=work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["add", "Task 3", "project=home"])
        .assert()
        .success();
    
    // Test: task annotate <id> (CLAP-native pattern)
    new_cmd(&temp_dir)
        .args(&["annotate", "1", "Note for task 1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added annotation"));
    
    // Test: task annotate <id> - single match
    new_cmd(&temp_dir)
        .args(&["annotate", "3", "Note for task 3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added annotation"));
    
    // Test: task annotate <filter> with --yes flag (multiple matches)
    new_cmd(&temp_dir)
        .args(&["annotate", "project=work", "--yes", "Note for all work tasks"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added annotation"))
        .stdout(predicate::str::contains("task 1"))
        .stdout(predicate::str::contains("task 2"));
}

#[test]
fn test_filter_sessions_pattern() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create projects first
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["projects", "add", "home"])
        .assert()
        .success();
    
    // Create tasks
    new_cmd(&temp_dir)
        .args(&["add", "Task 1", "project=work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["add", "Task 2", "project=work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["add", "Task 3", "project=home"])
        .assert()
        .success();
    
    // Create sessions for tasks
    new_cmd(&temp_dir)
        .args(&["onoff", "09:00..09:10", "1"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["onoff", "10:00..10:10", "2"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["onoff", "11:00..11:10", "3"])
        .assert()
        .success();
    
    // Verify sessions were created by listing all
    let all_output = new_cmd(&temp_dir)
        .args(&["sessions", "list"])
        .assert()
        .success();
    let all_stdout = String::from_utf8(all_output.get_output().stdout.clone()).unwrap();
    assert!(all_stdout.contains("Task 1") || all_stdout.contains("Task 2") || all_stdout.contains("Task 3"), 
            "Sessions should be created");
    
    // Test: sessions list with task ID (single token parsed as task ID)
    new_cmd(&temp_dir)
        .args(&["sessions", "list", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Task 1"));
    
    // Test: task <filter> sessions list (new pattern)
    new_cmd(&temp_dir)
        .args(&["sessions", "list", "project=work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Task 1"))
        .stdout(predicate::str::contains("Task 2"));
    
    // Test: task <filter> sessions list --json
    new_cmd(&temp_dir)
        .args(&["sessions", "list", "--json", "project=work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"task_id\""))
        .stdout(predicate::str::contains("\"start_ts\""));
    
    // Test: task sessions show --task <filter>
    new_cmd(&temp_dir)
        .args(&["sessions", "--task", "project=work", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Session"))
        .stdout(predicate::str::contains("Task"));
}

#[test]
fn test_backward_compatibility() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create projects first
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["projects", "add", "home"])
        .assert()
        .success();
    
    // Create tasks
    new_cmd(&temp_dir)
        .args(&["add", "Task 1", "project=work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["add", "Task 2", "project=home"])
        .assert()
        .success();
    
    // All CLAP-native patterns should work
    new_cmd(&temp_dir)
        .args(&["list", "project=work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["show", "1"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["annotate", "1", "Note"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["sessions", "list", "--task", "1"])
        .assert()
        .success();
}

#[test]
fn test_filter_list_no_matches() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create project and task
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["add", "Task 1", "project=work"])
        .assert()
        .success();
    
    // Filter that matches nothing
    new_cmd(&temp_dir)
        .args(&["list", "project=nonexistent"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No tasks found"));
}

#[test]
fn test_filter_annotate_no_matches() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create project and task
    new_cmd(&temp_dir)
        .args(&["projects", "add", "work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["add", "Task 1", "project=work"])
        .assert()
        .success();
    
    // Filter that matches nothing
    new_cmd(&temp_dir)
        .args(&["annotate", "project=nonexistent", "Note"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No matching tasks found"));
}

#[test]
fn test_desc_filter_matches_substring() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create tasks with different descriptions
    new_cmd(&temp_dir)
        .args(&["add", "Team meeting at 3pm"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["add", "Code review for PR"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["add", "Weekly standup meeting"])
        .assert()
        .success();
    
    // Filter by description containing "meeting"
    let output = new_cmd(&temp_dir)
        .args(&["list", "desc=meeting"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Team meeting"), "Should find 'Team meeting'");
    assert!(stdout.contains("Weekly standup meeting"), "Should find 'Weekly standup meeting'");
    assert!(!stdout.contains("Code review"), "Should not find 'Code review'");
}

#[test]
fn test_desc_filter_case_insensitive() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task with capital letters
    new_cmd(&temp_dir)
        .args(&["add", "IMPORTANT Task"])
        .assert()
        .success();
    
    // Filter with lowercase
    let output = new_cmd(&temp_dir)
        .args(&["list", "desc=important"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("IMPORTANT"), "Should find case-insensitive match");
}

#[test]
fn test_desc_filter_combined_with_other_filters() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create projects
    new_cmd(&temp_dir).args(&["projects", "add", "work"]).assert().success();
    
    // Create tasks
    new_cmd(&temp_dir)
        .args(&["add", "Work meeting", "project=work"])
        .assert()
        .success();
    new_cmd(&temp_dir)
        .args(&["add", "Personal meeting"])
        .assert()
        .success();
    
    // Filter by project AND description
    let output = new_cmd(&temp_dir)
        .args(&["list", "project=work", "desc=meeting"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Work meeting"), "Should find matching task");
    assert!(!stdout.contains("Personal meeting"), "Should not find non-matching task");
}

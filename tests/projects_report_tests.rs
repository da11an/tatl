use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
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
fn test_projects_report_shows_task_counts() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create projects
    get_task_cmd(&temp_dir).args(&["projects", "add", "work"]).assert().success();
    get_task_cmd(&temp_dir).args(&["projects", "add", "home"]).assert().success();
    
    // Create tasks in different projects
    get_task_cmd(&temp_dir).args(&["add", "Work task 1", "project=work"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Work task 2", "project=work"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Home task", "project=home"]).assert().success();
    
    // Run projects report
    let output = get_task_cmd(&temp_dir)
        .args(&["projects", "report"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Project"), "Should have table header");
    assert!(stdout.contains("work"), "Should show work project");
    assert!(stdout.contains("home"), "Should show home project");
    assert!(stdout.contains("TOTAL"), "Should show total row");
}

#[test]
fn test_projects_report_shows_kanban_statuses() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create project
    get_task_cmd(&temp_dir).args(&["projects", "add", "work"]).assert().success();
    
    // Create tasks in different states
    get_task_cmd(&temp_dir).args(&["add", "Pending task", "project=work"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Queued task", "project=work"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "2"]).assert().success();
    
    // Complete a task
    get_task_cmd(&temp_dir).args(&["add", "Done task", "project=work"]).assert().success();
    get_task_cmd(&temp_dir).args(&["finish", "3", "-y"]).assert().success();
    
    // Run report
    let output = get_task_cmd(&temp_dir)
        .args(&["projects", "report"])
        .assert()
        .success();
    
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Proposed"), "Should show Proposed column");
    assert!(stdout.contains("Stalled"), "Should show Stalled column");
    assert!(stdout.contains("Queued"), "Should show Queued column");
    assert!(stdout.contains("External"), "Should show External column");
    assert!(stdout.contains("Done"), "Should show Done column");
}

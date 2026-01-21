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
fn test_task_list_sort_columns_first() {
    let (temp_dir, _guard) = setup_test_env();
    
    get_task_cmd(&temp_dir).args(&["projects", "add", "alpha"]).assert().success();
    get_task_cmd(&temp_dir).args(&["projects", "add", "beta"]).assert().success();
    
    get_task_cmd(&temp_dir).args(&["add", "Alpha task", "project:alpha"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Beta task", "project:beta"]).assert().success();
    
    let output = get_task_cmd(&temp_dir).args(&["list", "sort:project"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    let header_line = stdout.lines()
        .find(|l| l.contains("Project") && l.contains("ID"))
        .expect("Header line not found");
    assert!(header_line.find("Project").unwrap() < header_line.find("ID").unwrap());
    
    let alpha_pos = stdout.lines().position(|l| l.contains("Alpha task")).unwrap();
    let beta_pos = stdout.lines().position(|l| l.contains("Beta task")).unwrap();
    assert!(alpha_pos < beta_pos, "Alpha task should appear before Beta task");
    
    drop(temp_dir);
}

#[test]
fn test_task_list_group_by_project() {
    let (temp_dir, _guard) = setup_test_env();
    
    get_task_cmd(&temp_dir).args(&["projects", "add", "alpha"]).assert().success();
    get_task_cmd(&temp_dir).args(&["projects", "add", "beta"]).assert().success();
    
    get_task_cmd(&temp_dir).args(&["add", "Alpha task", "project:alpha"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Beta task", "project:beta"]).assert().success();
    
    let output = get_task_cmd(&temp_dir).args(&["list", "group:project"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    assert!(stdout.contains("alpha"), "Group header or rows should include alpha");
    assert!(stdout.contains("beta"), "Group header or rows should include beta");
    
    drop(temp_dir);
}

#[test]
fn test_task_list_view_alias() {
    let (temp_dir, _guard) = setup_test_env();
    
    get_task_cmd(&temp_dir).args(&["projects", "add", "alpha"]).assert().success();
    get_task_cmd(&temp_dir).args(&["projects", "add", "beta"]).assert().success();
    
    get_task_cmd(&temp_dir).args(&["add", "Alpha task", "project:alpha"]).assert().success();
    get_task_cmd(&temp_dir).args(&["add", "Beta task", "project:beta"]).assert().success();
    
    get_task_cmd(&temp_dir)
        .args(&["list", "project:alpha", "sort:project", "alias:myview"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Saved view 'myview'"));
    
    let output = get_task_cmd(&temp_dir).args(&["list", "myview"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Alpha task"));
    assert!(!stdout.contains("Beta task"));
    
    drop(temp_dir);
}

#[test]
fn test_sessions_list_view_alias() {
    let (temp_dir, _guard) = setup_test_env();
    
    get_task_cmd(&temp_dir).args(&["add", "Session task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "in"]).assert().success();
    get_task_cmd(&temp_dir).args(&["clock", "out"]).assert().success();
    
    get_task_cmd(&temp_dir)
        .args(&["sessions", "list", "sort:start", "alias:mysessions"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Saved view 'mysessions'"));
    
    let output = get_task_cmd(&temp_dir).args(&["sessions", "list", "mysessions"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Session ID"));
    
    drop(temp_dir);
}

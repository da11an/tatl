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
fn test_task_close_sets_closed_status() {
    let (temp_dir, _guard) = setup_test_env();
    
    get_task_cmd(&temp_dir).args(&["add", "Close me"]).assert().success();
    get_task_cmd(&temp_dir).args(&["close", "1", "--yes"]).assert().success()
        .stdout(predicate::str::contains("Closed task 1"));
    
    let output = get_task_cmd(&temp_dir).args(&["show", "1"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("closed") || stdout.contains("Status: closed"));
    
    let output = get_task_cmd(&temp_dir).args(&["list", "status:closed"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Close me"));
    
    drop(temp_dir);
}

#[test]
fn test_task_done_is_removed() {
    let (temp_dir, _guard) = setup_test_env();
    
    get_task_cmd(&temp_dir).args(&["add", "Test task"]).assert().success();
    
    get_task_cmd(&temp_dir)
        .args(&["done"])
        .assert()
        .success()
        .stderr(predicate::str::contains("unrecognized subcommand").and(predicate::str::contains("done")));
    
    drop(temp_dir);
}

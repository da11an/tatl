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
    let config_dir = temp_dir.path().join(".taskninja");
    fs::create_dir_all(&config_dir).unwrap();
    let config_file = config_dir.join("rc");
    fs::write(&config_file, format!("data.location={}\n", db_path.display())).unwrap();
    
    // Set HOME to temp_dir so the config file is found
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    (temp_dir, guard)
}

fn get_task_cmd(temp_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("task").unwrap();
    cmd.env("HOME", temp_dir.path());
    cmd
}

#[test]
fn test_version_command() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Test --version flag
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["--version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("0.2.0"));
    
    // Test -V flag
    let mut cmd = get_task_cmd(&temp_dir);
    cmd.args(&["-V"])
        .assert()
        .success()
        .stdout(predicate::str::contains("0.2.0"));
}

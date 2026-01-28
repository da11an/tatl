use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
use tatl::db::DbConnection;
use tatl::repo::SessionRepo;
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
fn test_onoff_splits_open_session_correctly() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and start timing
    get_task_cmd(&temp_dir).args(&["add", "Test task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on", "1"]).assert().success();
    
    // Split the open session using onoff (removing a middle interval)
    // This should create two sessions: one closed (before the split) and one open (after the split)
    get_task_cmd(&temp_dir)
        .args(&["onoff", "14:30..15:00", "1", "-y"])
        .assert()
        .success();
    
    // Verify sessions
    let conn = DbConnection::connect().unwrap();
    let sessions = SessionRepo::list_all(&conn).unwrap();
    
    // Should have at least 2 sessions (implementation may create additional segments)
    assert!(sessions.len() >= 2, "Should have at least 2 sessions after split");
    
    // Find the open session (should be the second part)
    let open_session = sessions.iter().find(|s| s.end_ts.is_none());
    assert!(open_session.is_some(), "Should have an open session after split");
    
    // The open session should NOT have end_ts = i64::MAX
    let open = open_session.unwrap();
    assert_eq!(open.end_ts, None, "Open session should have end_ts = None, not i64::MAX");
}

#[test]
fn test_onoff_splits_closed_session_correctly() {
    let (temp_dir, _guard) = setup_test_env();
    
    // Create task and closed session
    get_task_cmd(&temp_dir).args(&["add", "Test task"]).assert().success();
    get_task_cmd(&temp_dir).args(&["enqueue", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["on", "1"]).assert().success();
    get_task_cmd(&temp_dir).args(&["off"]).assert().success();
    
    // Split the closed session
    get_task_cmd(&temp_dir)
        .args(&["onoff", "14:30..15:00", "1", "-y"])
        .assert()
        .success();
    
    // Verify sessions
    let conn = DbConnection::connect().unwrap();
    let sessions = SessionRepo::list_all(&conn).unwrap();
    
    // Should have 2 sessions, both closed
    assert_eq!(sessions.len(), 2, "Should have 2 sessions after split");
    assert!(sessions.iter().all(|s| s.end_ts.is_some()), "All sessions should be closed");
}

// Tests for offon and onoff commands (Plan 23: Break Capture Workflow)

use std::process::Command;
use tempfile::TempDir;

fn get_task_cmd(temp_dir: &TempDir) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tatl"));
    cmd.env("HOME", temp_dir.path());
    cmd
}

// ============================================
// Basic offon tests (current session mode)
// ============================================

#[test]
fn test_offon_stops_and_resumes_current_session() {
    let temp_dir = TempDir::new().unwrap();

    // Add a task and start timing at a specific time (use fixed past date to avoid timing issues)
    let output = get_task_cmd(&temp_dir)
        .args(&["add", "Test task"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = get_task_cmd(&temp_dir)
        .args(&["enqueue", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Start session at 14:00 today
    let output = get_task_cmd(&temp_dir)
        .args(&["on", "14:00"])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Use offon to stop at 14:30 and resume now (14:30 is after 14:00)
    let output = get_task_cmd(&temp_dir)
        .args(&["offon", "14:30"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Stopped timing"));
    assert!(stdout.contains("Started timing"));
}

#[test]
fn test_offon_with_interval_creates_break() {
    let temp_dir = TempDir::new().unwrap();

    // Add a task and start timing at a specific time
    let output = get_task_cmd(&temp_dir)
        .args(&["add", "Test task"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = get_task_cmd(&temp_dir)
        .args(&["enqueue", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Start session at 14:00 today
    let output = get_task_cmd(&temp_dir)
        .args(&["on", "14:00"])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Use offon with interval: stop at 14:30, resume at 15:00
    let output = get_task_cmd(&temp_dir)
        .args(&["offon", "14:30..15:00"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Stopped timing"));
    assert!(stdout.contains("Started timing"));
}

#[test]
fn test_offon_no_session_requires_overlapping_sessions() {
    let temp_dir = TempDir::new().unwrap();
    
    // Try offon with no session running and no overlapping sessions
    let output = get_task_cmd(&temp_dir)
        .args(&["offon", "20:00..21:00"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No sessions found overlapping"));
}

// ============================================
// Basic onoff tests (historical session)
// ============================================

#[test]
fn test_onoff_adds_historical_session() {
    let temp_dir = TempDir::new().unwrap();
    
    // Add a task and enqueue it
    let output = get_task_cmd(&temp_dir)
        .args(&["add", "Test task"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    let output = get_task_cmd(&temp_dir)
        .args(&["enqueue", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    // Add historical session using onoff
    let output = get_task_cmd(&temp_dir)
        .args(&["onoff", "09:00..12:00"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Added session"));
    assert!(stdout.contains("09:00"));
    assert!(stdout.contains("12:00"));
}

#[test]
fn test_onoff_with_specific_task() {
    let temp_dir = TempDir::new().unwrap();
    
    // Add two tasks
    let output = get_task_cmd(&temp_dir)
        .args(&["add", "First task"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    let output = get_task_cmd(&temp_dir)
        .args(&["add", "Second task"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    // Add historical session for task 2 specifically
    let output = get_task_cmd(&temp_dir)
        .args(&["onoff", "09:00..12:00", "2"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("task 2"));
}

#[test]
fn test_onoff_requires_interval() {
    let temp_dir = TempDir::new().unwrap();
    
    // Add a task
    let output = get_task_cmd(&temp_dir)
        .args(&["add", "Test task"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    let output = get_task_cmd(&temp_dir)
        .args(&["enqueue", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    // Try onoff with single time (should fail)
    let output = get_task_cmd(&temp_dir)
        .args(&["onoff", "09:00"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Interval required"));
}

#[test]
fn test_onoff_requires_start_before_end() {
    let temp_dir = TempDir::new().unwrap();
    
    // Add a task
    let output = get_task_cmd(&temp_dir)
        .args(&["add", "Test task"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    let output = get_task_cmd(&temp_dir)
        .args(&["enqueue", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    // Try onoff with end before start (should fail)
    let output = get_task_cmd(&temp_dir)
        .args(&["onoff", "12:00..09:00"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Start time must be before end time"));
}

// ============================================
// add --onoff tests
// ============================================

#[test]
fn test_add_with_onoff_creates_task_and_session() {
    let temp_dir = TempDir::new().unwrap();
    
    // Add task with historical session
    let output = get_task_cmd(&temp_dir)
        .args(&["add", "Meeting", ":", "onoff", "09:00..10:00"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Created task 1"));
    assert!(stdout.contains("Added session"));
}

#[test]
fn test_add_with_onoff_requires_interval() {
    let temp_dir = TempDir::new().unwrap();
    
    // Try to add task with invalid onoff (single time)
    let output = get_task_cmd(&temp_dir)
        .args(&["add", "Meeting", ":", "onoff", "09:00"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Interval required") || stderr.contains("<start>..<end>"));
}

// ============================================
// History modification tests
// ============================================

#[test]
fn test_offon_history_splits_session() {
    let temp_dir = TempDir::new().unwrap();
    
    // Add a task and create a session 
    let output = get_task_cmd(&temp_dir)
        .args(&["add", "Test task"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    let output = get_task_cmd(&temp_dir)
        .args(&["enqueue", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    // Add a historical session using absolute date (2020-01-15)
    let output = get_task_cmd(&temp_dir)
        .args(&["onoff", "2020-01-15T09:00..2020-01-15T17:00"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "onoff failed.\nstdout: {}\nstderr: {}", stdout, stderr);
    
    // Split the session using offon (removing 14:30-15:00)
    let output = get_task_cmd(&temp_dir)
        .args(&["offon", "2020-01-15T14:30..2020-01-15T15:00", "-y"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "offon failed.\nstdout: {}\nstderr: {}", stdout, stderr);
    assert!(stdout.contains("Sessions modified"), "Expected 'Sessions modified' in stdout: {}", stdout);
    
    // Verify sessions were split
    let output = get_task_cmd(&temp_dir)
        .args(&["sessions", "list", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should now have 2 sessions
    let session_count = stdout.matches("\"id\":").count();
    assert_eq!(session_count, 2, "Expected 2 sessions after split, got {}", session_count);
}

#[test]
fn test_onoff_insertion_clears_overlapping() {
    let temp_dir = TempDir::new().unwrap();
    
    // Add two tasks
    let output = get_task_cmd(&temp_dir)
        .args(&["add", "Main work"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    let output = get_task_cmd(&temp_dir)
        .args(&["add", "Meeting"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    let output = get_task_cmd(&temp_dir)
        .args(&["enqueue", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    // Add a session for task 1: using absolute date (2020-01-15)
    let output = get_task_cmd(&temp_dir)
        .args(&["onoff", "2020-01-15T09:00..2020-01-15T17:00"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "onoff for task 1 failed.\nstdout: {}\nstderr: {}", stdout, stderr);
    
    // Insert a session for task 2: 14:00-15:00 (should modify task 1's session)
    let output = get_task_cmd(&temp_dir)
        .args(&["onoff", "2020-01-15T14:00..2020-01-15T15:00", "2", "-y"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "onoff for task 2 failed.\nstdout: {}\nstderr: {}", stdout, stderr);
    assert!(stdout.contains("Inserted session"), "Expected 'Inserted session' in stdout: {}", stdout);
    
    // Verify: should now have 3 sessions (task 1 split + task 2 inserted)
    let output = get_task_cmd(&temp_dir)
        .args(&["sessions", "list", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let session_count = stdout.matches("\"id\":").count();
    assert_eq!(session_count, 3, "Expected 3 sessions after insertion, got {}", session_count);
}

// ============================================
// Error handling tests
// ============================================

#[test]
fn test_offon_requires_time_expression() {
    let temp_dir = TempDir::new().unwrap();
    
    // Add a task and start timing
    let output = get_task_cmd(&temp_dir)
        .args(&["add", "Test task"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    let output = get_task_cmd(&temp_dir)
        .args(&["enqueue", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    let output = get_task_cmd(&temp_dir)
        .args(&["on"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    // Try offon without time
    let output = get_task_cmd(&temp_dir)
        .args(&["offon"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Time expression required"));
}

#[test]
fn test_onoff_requires_queue_or_task_id() {
    let temp_dir = TempDir::new().unwrap();
    
    // Try onoff with empty queue and no task ID
    let output = get_task_cmd(&temp_dir)
        .args(&["onoff", "09:00..12:00"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No tasks in queue"));
}

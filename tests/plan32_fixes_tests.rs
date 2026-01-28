//! Tests for Plan 32 fixes - README/CLI alignment
//!
//! These tests lock in the designed behavior for:
//! - Time specification (past-only times)
//! - Display labels (kanban statuses, respawn label)
//! - Sessions modify command

mod acceptance_framework;
use acceptance_framework::{AcceptanceTestContext, GivenBuilder, WhenBuilder, ThenBuilder};

// =============================================================================
// Phase 1: Time Specification Tests
// =============================================================================

/// Test that `tatl on <time>` rejects future times
#[test]
fn test_on_rejects_future_time() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);

    // Create a task and enqueue it
    given.task_exists("Test task");
    given.stack_contains(&[1]);

    // Try to start timing with a future time (23:59 should be in the future for most test runs)
    let mut when = WhenBuilder::new(&ctx);
    when.execute(&["on", "23:59"]);

    let result = when.result().unwrap();
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);

    // Should fail or warn about future time
    // After fix: should reject future times
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        !result.status.success() || combined.contains("future") || combined.contains("past"),
        "on <time> should reject future times. stdout: {}, stderr: {}",
        stdout, stderr
    );
}

/// Test that `tatl on <time>` accepts past times without creating negative durations
#[test]
fn test_on_accepts_past_time_no_negative_duration() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);

    // Create a task and enqueue it
    given.task_exists("Test task");
    given.stack_contains(&[1]);

    // Start timing with a past time (00:01 should be in the past for most test runs)
    let mut when = WhenBuilder::new(&ctx);
    when.execute(&["on", "00:01"]);

    // Stop timing
    let mut when2 = WhenBuilder::new(&ctx);
    when2.execute(&["off"]);

    // Check sessions don't have negative durations
    let mut when3 = WhenBuilder::new(&ctx);
    when3.execute_success(&["sessions", "list"]);
    let result = when3.result().unwrap();
    let stdout = String::from_utf8_lossy(&result.stdout);

    // Should not contain negative durations (indicated by leading -)
    assert!(!stdout.contains("-0s") && !stdout.contains("--"),
        "Sessions should not have negative durations: {}", stdout);
}

/// Test that `tatl add --on=<time>` works with past times
#[test]
fn test_add_on_with_past_time() {
    let ctx = AcceptanceTestContext::new();

    // Add a task with --on=00:01 (past time)
    let mut when = WhenBuilder::new(&ctx);
    when.execute(&["add", "Test task", "-y", ":", "on", "00:01"]);

    let result = when.result().unwrap();
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);

    // Should create the task
    assert!(stdout.contains("Created task") || stdout.contains("task 1"),
        "Task should be created. stdout: {}, stderr: {}", stdout, stderr);

    // Should start timing (not fail)
    assert!(!stderr.contains("Failed to start timing"),
        "Should not fail to start timing. stderr: {}", stderr);

    // Stop timing to clean up
    let mut cleanup = WhenBuilder::new(&ctx);
    cleanup.execute(&["off"]);
}

// =============================================================================
// Phase 2: Display/Label Tests
// =============================================================================

/// Test that `projects report` shows correct kanban headers (Stalled, External, not Paused/NEXT/LIVE)
#[test]
fn test_projects_report_kanban_headers() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);

    // Create a project and task
    given.project_exists("testproj");
    given.task_exists_with_project("Test task", "testproj");

    // Get projects report
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["projects", "report"]);
    let result = when.result().unwrap();
    let stdout = String::from_utf8_lossy(&result.stdout);

    // Should have new kanban headers
    assert!(stdout.contains("Stalled"),
        "Report should show 'Stalled' column: {}", stdout);
    assert!(stdout.contains("External"),
        "Report should show 'External' column: {}", stdout);

    // Should NOT have old NEXT/LIVE columns
    assert!(!stdout.contains("NEXT"),
        "Report should not have NEXT column: {}", stdout);
    assert!(!stdout.contains("LIVE"),
        "Report should not have LIVE column: {}", stdout);
    assert!(!stdout.contains("Paused"),
        "Report should not have Paused column (should be Stalled): {}", stdout);
}

/// Test that `tatl show` displays "Respawn:" not "Recurrence:"
#[test]
fn test_show_respawn_label_none() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);

    // Create a task without respawn
    given.task_exists("Test task");

    // Show the task
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["show", "1"]);
    let result = when.result().unwrap();
    let stdout = String::from_utf8_lossy(&result.stdout);

    // Should NOT show "Recurrence:"
    assert!(!stdout.contains("Recurrence:"),
        "Show should not display 'Recurrence:' label: {}", stdout);
}

/// Test that task with respawn shows "Respawn:" label correctly
#[test]
fn test_show_respawn_label_with_respawn() {
    let ctx = AcceptanceTestContext::new();

    // Create a task with respawn via CLI
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["add", "Test task", "respawn=daily", "-y"]);

    // Show the task
    let mut when2 = WhenBuilder::new(&ctx);
    when2.execute_success(&["show", "1"]);
    let result = when2.result().unwrap();
    let stdout = String::from_utf8_lossy(&result.stdout);

    // Should show "Respawn:" with the pattern
    assert!(stdout.contains("Respawn:") && stdout.contains("daily"),
        "Show should display 'Respawn:' label with pattern: {}", stdout);
}

// =============================================================================
// Phase 3: Sessions Modify Tests
// =============================================================================

/// Test that `sessions modify` accepts -y flag at end of command
#[test]
fn test_sessions_modify_y_flag_at_end() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);

    // Create a task with a closed session
    given.task_exists("Test task");
    given.closed_session_exists(1, "00:01", "00:10");

    // Try to modify with -y at end (this was failing before)
    let mut when = WhenBuilder::new(&ctx);
    when.execute(&["sessions", "modify", "1", "00:02..00:09", "-y"]);

    let result = when.result().unwrap();
    let stderr = String::from_utf8_lossy(&result.stderr);

    // Should NOT fail with "Invalid argument: -y"
    assert!(!stderr.contains("Invalid argument: -y"),
        "sessions modify should accept -y flag at end. stderr: {}", stderr);
}

/// Test that `sessions modify` accepts --yes flag at end
#[test]
fn test_sessions_modify_yes_flag_at_end() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);

    // Create a task with a closed session
    given.task_exists("Test task");
    given.closed_session_exists(1, "00:01", "00:10");

    // Try to modify with --yes at end
    let mut when = WhenBuilder::new(&ctx);
    when.execute(&["sessions", "modify", "1", "00:02..00:09", "--yes"]);

    let result = when.result().unwrap();
    let stderr = String::from_utf8_lossy(&result.stderr);

    // Should NOT fail with "Invalid argument: --yes"
    assert!(!stderr.contains("Invalid argument: --yes"),
        "sessions modify should accept --yes flag at end. stderr: {}", stderr);
}

/// Test interval syntax for sessions modify (two-sided)
#[test]
fn test_sessions_modify_interval_syntax_both() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);

    // Create a task with a closed session
    given.task_exists("Test task");
    given.closed_session_exists(1, "00:01", "00:10");

    // Modify with interval syntax (two-sided)
    let mut when = WhenBuilder::new(&ctx);
    when.execute(&["sessions", "modify", "1", "00:02..00:09", "-y"]);

    let result = when.result().unwrap();

    // Should succeed
    assert!(result.status.success(),
        "sessions modify should accept two-sided interval syntax. status: {:?}",
        result.status);
}

/// Test one-sided interval syntax (start only)
#[test]
fn test_sessions_modify_start_only() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);

    // Create a task with a closed session
    given.task_exists("Test task");
    given.closed_session_exists(1, "00:01", "00:10");

    // Modify with start only
    let mut when = WhenBuilder::new(&ctx);
    when.execute(&["sessions", "modify", "1", "00:02..", "-y"]);

    let result = when.result().unwrap();

    // Should succeed
    assert!(result.status.success(),
        "sessions modify should accept start-only interval syntax. status: {:?}",
        result.status);
}

/// Test one-sided interval syntax (end only)
#[test]
fn test_sessions_modify_end_only() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);

    // Create a task with a closed session
    given.task_exists("Test task");
    given.closed_session_exists(1, "00:01", "00:10");

    // Modify with end only
    let mut when = WhenBuilder::new(&ctx);
    when.execute(&["sessions", "modify", "1", "..00:09", "-y"]);

    let result = when.result().unwrap();

    // Should succeed
    assert!(result.status.success(),
        "sessions modify should accept end-only interval syntax. status: {:?}",
        result.status);
}

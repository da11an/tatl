// Transaction and Atomicity Tests
// Verify that critical operations are atomic and rollback on failure

mod acceptance_framework;
use acceptance_framework::*;
use tatl::repo::{TaskRepo, StackRepo, SessionRepo};

// ============================================================================
// Atomic Operations Tests
// ============================================================================

#[test]
fn test_finish_next_atomic() {
    // Test that close --next is atomic:
    // 1. Close current session
    // 2. Complete task
    // 3. Remove from stack
    // 4. Start next task session
    // If any step fails, all should rollback
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task1 = given.task_exists("Task 1");
    let task2 = given.task_exists("Task 2");
    given.stack_contains(&[task1, task2]);
    given.clock_running_on_task_since(task1, "2026-01-10T09:00");
    
    // Complete task1 with --next
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["close", ":", "on"]);
    
    // Verify atomicity: all changes applied together
    let then = ThenBuilder::new(&ctx, None);
    then.task_status_is(task1, "closed")
        .stack_order_is(&[task2])
        .running_session_exists_for_task(task2);
    
    // Verify task1 session was closed
    let sessions = SessionRepo::get_by_task(ctx.db(), task1).unwrap();
    assert!(sessions.iter().any(|s| s.end_ts.is_some()), "Task1 session should be closed");
}

#[test]
fn test_on_task_atomic() {
    // Test that `on <task>` is atomic:
    // 1. Close existing session (if any)
    // 2. Push task to queue[0]
    // 3. Create new session
    // If any step fails, all should rollback
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task1 = given.task_exists("Task 1");
    let task2 = given.task_exists("Task 2");
    given.stack_contains(&[task1]);
    given.clock_running_on_task_since(task1, "2026-01-10T09:00");
    
    // Start timing task2 - should atomically:
    // 1. Close task1 session
    // 2. Push task2 to queue[0]
    // 3. Start task2 session
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["on", &task2.to_string()]);
    
    // Verify atomicity
    let then = ThenBuilder::new(&ctx, None);
    then.stack_order_is(&[task2, task1])
        .running_session_exists_for_task(task2);
    
    // Verify task1 session was closed
    let sessions = SessionRepo::get_by_task(ctx.db(), task1).unwrap();
    assert!(sessions.iter().any(|s| s.end_ts.is_some()), "Task1 session should be closed");
}

// ============================================================================
// Rollback on Failure Tests
// ============================================================================

#[test]
fn test_rollback_on_task_not_found() {
    // Test that if a task operation fails (e.g., task not found),
    // no partial changes are made
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task1 = given.task_exists("Task 1");
    given.stack_contains(&[task1]);
    given.clock_running_on_task_since(task1, "2026-01-10T09:00");
    
    // Try to start timing non-existent task - should fail and rollback
    let mut when = WhenBuilder::new(&ctx);
    when.execute_failure(&["on", "999"]);
    
    // Verify no changes were made
    let then = ThenBuilder::new(&ctx, None);
    then.stack_order_is(&[task1])
        .running_session_exists_for_task(task1);
}

// ============================================================================
// No Partial State Changes Tests
// ============================================================================

#[test]
fn test_no_partial_state_on_finish_failure() {
    // Test that if close operation fails partway through,
    // no partial state changes remain
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task1 = given.task_exists("Task 1");
    let task2 = given.task_exists("Task 2");
    given.stack_contains(&[task1, task2]);
    given.clock_running_on_task_since(task1, "2026-01-10T09:00");
    
    // Get initial state
    let initial_stack = StackRepo::get_or_create_default(ctx.db()).unwrap();
    let initial_items = StackRepo::get_items(ctx.db(), initial_stack.id.unwrap()).unwrap();
    let initial_session = SessionRepo::get_open(ctx.db()).unwrap();
    
    // Try to complete non-existent task
    let result = ctx.cmd().args(&["close", "999"]).output().unwrap();
    
    // Verify state is unchanged (regardless of exit code)
    let final_stack = StackRepo::get_or_create_default(ctx.db()).unwrap();
    let final_items = StackRepo::get_items(ctx.db(), final_stack.id.unwrap()).unwrap();
    let final_session = SessionRepo::get_open(ctx.db()).unwrap();
    
    assert_eq!(initial_items.len(), final_items.len(), "Stack should be unchanged");
    assert_eq!(initial_items[0].task_id, final_items[0].task_id, "Stack[0] should be unchanged");
    assert_eq!(initial_session.as_ref().map(|s| s.task_id), final_session.as_ref().map(|s| s.task_id),
               "Session should be unchanged");
    
    // Verify error message was printed
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("not found")
            || stderr.contains("No session")
            || stderr.contains("No matching tasks"),
            "Error message should indicate task not found or no session");
}

#[test]
fn test_no_partial_state_on_modify_failure() {
    // Test that if modify operation fails, no partial changes are made
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task1 = given.task_exists("Task 1");
    
    // Get initial state
    let initial_task = TaskRepo::get_by_id(ctx.db(), task1).unwrap().unwrap();
    let initial_description = initial_task.description.clone();
    
    // Try to modify with invalid project - should fail
    let mut when = WhenBuilder::new(&ctx);
    when.execute_failure(&["modify", &task1.to_string(), "project=work@home"]);
    
    // Verify task is unchanged
    let final_task = TaskRepo::get_by_id(ctx.db(), task1).unwrap().unwrap();
    assert_eq!(initial_description, final_task.description, "Task description should be unchanged");
}

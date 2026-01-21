// Test the acceptance test framework itself

mod acceptance_framework;
use acceptance_framework::*;
use tatl::repo::TaskRepo;

#[test]
fn test_framework_basic_setup() {
    // Test that the framework can create a context
    let ctx = AcceptanceTestContext::new();
    
    // Test that we can create a command
    let _cmd = ctx.cmd();
    
    // Test that database connection works
    let _conn = ctx.db();
}

#[test]
fn test_given_tasks_exist() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Create tasks
    let task1 = given.task_exists("Test task 1");
    let task2 = given.task_exists("Test task 2");
    
    // Verify they exist
    assert!(TaskRepo::get_by_id(ctx.db(), task1).unwrap().is_some());
    assert!(TaskRepo::get_by_id(ctx.db(), task2).unwrap().is_some());
}

#[test]
fn test_given_stack_contains() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Create tasks
    let task1 = given.task_exists("Task 1");
    let task2 = given.task_exists("Task 2");
    let task3 = given.task_exists("Task 3");
    
    // Set up stack
    given.stack_contains(&[task1, task2, task3]);
    
    // Verify stack order
    let then = ThenBuilder::new(&ctx, None);
    then.stack_order_is(&[task1, task2, task3]);
}

#[test]
fn test_when_execute_success() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Create a task
    let _task_id = given.task_exists("Test task");
    
    // Execute a command
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["list"]);
    
    // Verify we got a result
    assert!(when.result().is_some());
}

#[test]
fn test_then_exit_code() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Create a task
    let _task_id = given.task_exists("Test task");
    
    // Execute command
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["list"]);
    
    // Assert exit code
    let then = ThenBuilder::new(&ctx, when.result());
    then.exit_code_is(0);
}

#[test]
fn test_then_stack_order() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Create tasks and set up stack
    let task1 = given.task_exists("Task 1");
    let task2 = given.task_exists("Task 2");
    given.stack_contains(&[task1, task2]);
    
    // Assert stack order
    let then = ThenBuilder::new(&ctx, None);
    then.stack_order_is(&[task1, task2]);
}

#[test]
fn test_then_task_status() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Create a task
    let task_id = given.task_exists("Test task");
    
    // Assert status
    let then = ThenBuilder::new(&ctx, None);
    then.task_status_is(task_id, "pending");
}

#[test]
fn test_then_task_has_tag() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Create task with tag
    let task_id = given.task_exists_with_tags("Test task", &["urgent"]);
    
    // Assert tag
    let then = ThenBuilder::new(&ctx, None);
    then.task_has_tag(task_id, "urgent");
}

#[test]
fn test_then_project_exists() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Create project
    given.project_exists("work");
    
    // Assert project exists
    let then = ThenBuilder::new(&ctx, None);
    then.project_exists("work");
}

#[test]
fn test_then_project_does_not_exist() {
    let ctx = AcceptanceTestContext::new();
    let then = ThenBuilder::new(&ctx, None);
    
    // Assert project doesn't exist
    then.project_does_not_exist("nonexistent");
}

#[test]
fn test_then_no_running_session() {
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Ensure no running session
    given.no_running_session();
    
    // Assert no running session
    let then = ThenBuilder::new(&ctx, None);
    then.no_running_session_exists();
}

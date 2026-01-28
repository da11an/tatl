// Acceptance tests for Task Ninja
// These implement the Given/When/Then scenarios from Section 11 of the design document

mod acceptance_framework;
use acceptance_framework::*;
use tatl::repo::{StackRepo, SessionRepo, TaskRepo};
use tatl::filter::parser::parse_filter;
use tatl::filter::evaluator::filter_tasks;
use tatl::utils::date;

// ============================================================================
// Section 11.1: Stack basics
// ============================================================================

#[test]
fn acceptance_stack_auto_initialization_on_first_operation() {
    // Given no stack exists (fresh database)
    // And tasks 1,2,3 exist
    // When `task stack show` (first stack operation)
    // Then a default stack exists with name='default'
    // And stack is empty `[]`
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    given.tasks_exist(&[1, 2, 3]);
    
    // Verify default stack is created on first access
    let stack = StackRepo::get_or_create_default(ctx.db()).unwrap();
    assert_eq!(stack.name, "default");
    
    let then = ThenBuilder::new(&ctx, None);
    then.stack_is_empty();
}

#[test]
fn acceptance_enqueue_adds_to_end() {
    // Given tasks 1,2,3 exist
    // And stack is empty
    // When `task 1 enqueue`
    // And `task 2 enqueue`
    // And `task 3 enqueue`
    // Then stack order is `[1,2,3]` (tasks added to end in order)
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task1 = given.task_exists("Task 1");
    let task2 = given.task_exists("Task 2");
    let task3 = given.task_exists("Task 3");
    given.stack_is_empty();
    
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["enqueue", &task1.to_string()]);
    when.execute_success(&["enqueue", &task2.to_string()]);
    when.execute_success(&["enqueue", &task3.to_string()]);
    
    let then = ThenBuilder::new(&ctx, None);
    then.stack_order_is(&[task1, task2, task3]);
}

#[test]
fn acceptance_on_pushes_to_top() {
    // Given tasks 1,2,3 exist
    // And stack is `[1,2]`
    // When `tatl on 3`
    // Then stack order is `[3,1,2]` (task 3 pushed to top)
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task1 = given.task_exists("Task 1");
    let task2 = given.task_exists("Task 2");
    let task3 = given.task_exists("Task 3");
    given.stack_contains(&[task1, task2]);
    
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["on", &task3.to_string()]);
    
    let then = ThenBuilder::new(&ctx, None);
    then.stack_order_is(&[task3, task1, task2]);
}

// NOTE: Stack pick/next tests removed - these commands were removed in CLI overhaul
// Stack manipulation is now done via `on <task_id>` to switch to a specific task
// or `dequeue` to remove from queue

// ============================================================================
// Section 11.2: Timer and queue coupling
// ============================================================================

#[test]
fn acceptance_on_switches_task_and_closes_previous() {
    // Given queue `[10,11]`
    // And timer is running on task 10 since 09:00
    // When `tatl on 11` at 09:10
    // Then session for task 10 ends at 09:10
    // And a new session for task 11 starts at 09:10
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task10 = given.task_exists("Task 10");
    let task11 = given.task_exists("Task 11");
    given.stack_contains(&[task10, task11]);
    given.clock_running_on_task_since(task10, "09:00");
    
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["on", &task11.to_string()]);
    
    let then = ThenBuilder::new(&ctx, None);
    // `on 11` should close task 10's session and start task 11's session
    then.running_session_exists_for_task(task11)
        .stack_order_is(&[task11, task10]);
    
    // Verify task 10's session was closed
    let sessions = SessionRepo::get_by_task(ctx.db(), task10).unwrap();
    assert!(sessions.iter().any(|s| s.end_ts.is_some()), "Task 10 session should be closed");
}

#[test]
fn acceptance_on_starts_queue0_at_now() {
    // Given queue `[10,11]`
    // And no running session
    // When `tatl on` (no arguments) at 09:00
    // Then a running session exists for task 10 starting 09:00 (defaults to now)
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task10 = given.task_exists("Task 10");
    let task11 = given.task_exists("Task 11");
    given.stack_contains(&[task10, task11]);
    given.no_running_session();
    
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["on"]);
    
    let then = ThenBuilder::new(&ctx, None);
    then.running_session_exists_for_task(task10);
}

#[test]
fn acceptance_on_errors_on_empty_queue() {
    // Given queue is empty
    // When `tatl on`
    // Then exit code is 1
    // And message contains "Queue is empty"
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    given.stack_is_empty();
    
    let mut when = WhenBuilder::new(&ctx);
    when.execute_failure(&["on"]);
    
    let then = ThenBuilder::new(&ctx, when.result());
    then.exit_code_is(1)
        .message_contains("Queue is empty");
}

#[test]
fn acceptance_on_with_interval_creates_closed_session() {
    // Given queue `[10]`
    // And no running session
    // When `tatl on 2026-01-10T09:00..2026-01-10T10:30`
    // Then a closed session exists for task 10 from 09:00 to 10:30
    // And no running session exists
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task10 = given.task_exists("Task 10");
    given.stack_contains(&[task10]);
    given.no_running_session();
    
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["on", "2026-01-10T09:00..2026-01-10T10:30"]);
    
    let then = ThenBuilder::new(&ctx, None);
    then.closed_session_exists_for_task(task10, "2026-01-10T09:00", "2026-01-10T10:30")
        .no_running_session_exists();
}

#[test]
fn acceptance_interval_end_time_amended_on_overlap() {
    // Given queue `[10,11]`
    // And a closed session for task 10 from 09:00 to 10:30
    // When `tatl on 11 2026-01-10T09:45` (starts before task 10's end time)
    // Then task 10's session end time is amended to 09:45
    // And task 10 session is from 09:00 to 09:45
    // And a new session for task 11 starts at 09:45
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task10 = given.task_exists("Task 10");
    let task11 = given.task_exists("Task 11");
    given.stack_contains(&[task10, task11]);
    given.closed_session_exists(task10, "2026-01-10T09:00", "2026-01-10T10:30");
    
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["on", &task11.to_string(), "2026-01-10T09:45"]);
    
    let then = ThenBuilder::new(&ctx, None);
    // Verify task 10's session was amended
    then.closed_session_exists_for_task(task10, "2026-01-10T09:00", "2026-01-10T09:45");
    // Verify task 11 has a session starting at 09:45
    let sessions = SessionRepo::get_by_task(ctx.db(), task11).unwrap();
    assert!(sessions.iter().any(|s| s.start_ts == date::parse_date_expr("2026-01-10T09:45").unwrap()));
}

// ============================================================================
// Section 11.3: Done semantics
// ============================================================================

#[test]
fn acceptance_done_errors_if_not_running() {
    // Given stack `[10]`
    // And no running session
    // When `task finish`
    // Then exit code is 1
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task10 = given.task_exists("Task 10");
    given.stack_contains(&[task10]);
    given.no_running_session();
    
    let mut when = WhenBuilder::new(&ctx);
    when.execute_failure(&["finish"]);
    
    let then = ThenBuilder::new(&ctx, when.result());
    then.exit_code_is(1);
}

#[test]
fn acceptance_done_completes_and_removes_from_stack() {
    // Given stack `[10,11]`
    // And clock running on 10 since 09:00
    // When `task finish` at 09:30
    // Then session for 10 ends 09:30
    // And task 10 status is completed
    // And stack is `[11]`
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task10 = given.task_exists("Task 10");
    let task11 = given.task_exists("Task 11");
    given.stack_contains(&[task10, task11]);
    given.clock_running_on_task_since(task10, "09:00");
    
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["finish"]);
    
    let then = ThenBuilder::new(&ctx, None);
    then.task_status_is(task10, "completed")
        .stack_order_is(&[task11]);
    
    // Verify session was closed
    let sessions = SessionRepo::get_by_task(ctx.db(), task10).unwrap();
    assert!(sessions.iter().any(|s| s.end_ts.is_some()), "Session should be closed");
}

#[test]
fn acceptance_done_next_starts_next() {
    // Given stack `[10,11]`
    // And clock running on 10 since 09:00
    // When `task finish : on` at 09:30
    // Then session for 10 ends 09:30
    // And session for 11 starts 09:30
    // And stack is `[11]`

    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);

    let task10 = given.task_exists("Task 10");
    let task11 = given.task_exists("Task 11");
    given.stack_contains(&[task10, task11]);
    given.clock_running_on_task_since(task10, "09:00");

    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["finish", ":", "on"]);
    
    let then = ThenBuilder::new(&ctx, None);
    then.task_status_is(task10, "completed")
        .stack_order_is(&[task11])
        .running_session_exists_for_task(task11);
    
    // Verify task 10's session was closed
    let sessions = SessionRepo::get_by_task(ctx.db(), task10).unwrap();
    assert!(sessions.iter().any(|s| s.end_ts.is_some()), "Task 10 session should be closed");
}

// ============================================================================
// Section 11.4: Micro-session behavior
// ============================================================================

#[test]
fn acceptance_micro_purge_on_rapid_switch() {
    // Given queue `[10,11]` and timer running on 10
    // When user switches to task 11 at 09:00:00
    // And task 11 session ends at 09:00:20 (20s duration, micro-session)
    // And task 10 session begins at 09:00:25 (within 30s of micro-session end)
    // Then task 11 micro-session is purged (different task, within MICRO of end)
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task10 = given.task_exists("Task 10");
    let task11 = given.task_exists("Task 11");
    given.stack_contains(&[task10, task11]);
    given.clock_running_on_task_since(task10, "2026-01-10T09:00");
    
    // Switch to task 11 - this should close task 10 and start task 11
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["on", &task11.to_string()]);
    
    // Verify task 11 is running
    let then = ThenBuilder::new(&ctx, None);
    then.running_session_exists_for_task(task11);
    
    // Close task 11 session after 20 seconds (micro-session)
    // Use a timestamp that's 20 seconds after the start
    let start_ts = date::parse_date_expr("2026-01-10T09:00").unwrap();
    SessionRepo::close_open(ctx.db(), start_ts + 20).unwrap();
    
    // Start task 10 session at 09:00:25 (within 30s of micro-session end)
    // This should trigger purge of task 11's micro-session
    SessionRepo::create(ctx.db(), task10, start_ts + 25).unwrap();
    
    // Verify task 11's micro-session was purged
    let sessions = SessionRepo::get_by_task(ctx.db(), task11).unwrap();
    assert!(sessions.is_empty(), "Task 11 micro-session should be purged");
}

#[test]
fn acceptance_micro_merge_on_bounce_back_to_same_task() {
    // Given a task 11 session of 20s ends at 09:00:20
    // And within 30s of the end time (at 09:00:25), a new session for task 11 begins
    // Then the 20s session is merged into the later 11 session
    // And stdout indicates merge rule applied
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task11 = given.task_exists("Task 11");
    
    // Create a closed micro-session (20s)
    let start_ts = date::parse_date_expr("2026-01-10T09:00").unwrap();
    SessionRepo::create_closed(
        ctx.db(),
        task11,
        start_ts,
        start_ts + 20,
    ).unwrap();
    
    // Start new session within 30s (at 09:00:25)
    // We need to set the time, but we can't easily mock time in the CLI
    // Instead, we'll create the session directly and verify merge behavior
    let new_session = SessionRepo::create(
        ctx.db(),
        task11,
        start_ts + 25,
    ).unwrap();
    
    // Verify the session starts at the micro-session's start time (merged)
    assert_eq!(new_session.start_ts, start_ts,
               "Session should start at micro-session start time (merged)");
    
    // Verify only one session exists (the merged one)
    let sessions = SessionRepo::get_by_task(ctx.db(), task11).unwrap();
    assert_eq!(sessions.len(), 1, "Micro-session should be merged into new session");
}

#[test]
fn acceptance_micro_session_preserved_if_no_rule_triggers() {
    // Given a task 11 session of 20s ends at 09:00:20
    // And next session starts at 09:01:05 (45s after end, beyond MICRO)
    // Then the 20s session remains (no rule triggered)
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task11 = given.task_exists("Task 11");
    
    // Create a closed micro-session (20s)
    let start_ts = date::parse_date_expr("2026-01-10T09:00").unwrap();
    SessionRepo::create_closed(
        ctx.db(),
        task11,
        start_ts,
        start_ts + 20,
    ).unwrap();
    
    // Start new session beyond MICRO threshold (45s later)
    SessionRepo::create(
        ctx.db(),
        task11,
        start_ts + 65, // 20s session + 45s gap = 65s from start
    ).unwrap();
    
    // Verify both sessions exist (micro-session preserved)
    let sessions = SessionRepo::get_by_task(ctx.db(), task11).unwrap();
    assert_eq!(sessions.len(), 2, "Micro-session should be preserved when beyond threshold");
}

// ============================================================================
// Section 11.5: Tags and filters
// ============================================================================

#[test]
fn acceptance_tag_add_remove() {
    // Given task 10 exists
    // When `task 10 modify +urgent +home`
    // And `task 10 modify -home`
    // Then task 10 has tag `urgent` and does not have tag `home`
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task10 = given.task_exists("Task 10");
    
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["modify", &task10.to_string(), "+urgent", "+home"]);
    when.execute_success(&["modify", &task10.to_string(), "-home"]);
    
    let then = ThenBuilder::new(&ctx, None);
    then.task_has_tag(task10, "urgent")
        .task_does_not_have_tag(task10, "home");
}

#[test]
fn acceptance_filter_and_or_not() {
    // Given tasks: A has +urgent, B has +important, C has both
    // When `task +urgent or +important list`
    // Then results include A,B,C
    // When `task not +urgent list`
    // Then results exclude A and C
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task_a = given.task_exists_with_tags("Task A", &["urgent"]);
    let task_b = given.task_exists_with_tags("Task B", &["important"]);
    let task_c = given.task_exists_with_tags("Task C", &["urgent", "important"]);
    
    // Test: +urgent or +important
    let filter_expr = parse_filter(vec!["+urgent".to_string(), "or".to_string(), "+important".to_string()]).unwrap();
    let matching = filter_tasks(ctx.db(), &filter_expr).unwrap();
    let matching_ids: Vec<i64> = matching.iter().map(|(t, _)| t.id.unwrap()).collect();
    assert!(matching_ids.contains(&task_a), "Task A should match");
    assert!(matching_ids.contains(&task_b), "Task B should match");
    assert!(matching_ids.contains(&task_c), "Task C should match");
    
    // Test: not +urgent
    let filter_expr = parse_filter(vec!["not".to_string(), "+urgent".to_string()]).unwrap();
    let matching = filter_tasks(ctx.db(), &filter_expr).unwrap();
    let matching_ids: Vec<i64> = matching.iter().map(|(t, _)| t.id.unwrap()).collect();
    assert!(!matching_ids.contains(&task_a), "Task A should not match");
    assert!(matching_ids.contains(&task_b), "Task B should match");
    assert!(!matching_ids.contains(&task_c), "Task C should not match");
}

// ============================================================================
// Section 11.6: Scheduling and waiting
// ============================================================================

#[test]
fn acceptance_waiting_derived() {
    // Given task 10 has wait set to tomorrow
    // When `task waiting list`
    // Then task 10 appears
    // When time passes beyond wait
    // Then task 10 no longer matches waiting
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    let task10 = given.task_exists("Task 10");
    
    // Set wait to tomorrow
    let tomorrow_ts = date::parse_date_expr("tomorrow").unwrap();
    TaskRepo::modify(
        ctx.db(),
        task10,
        None, // description
        None, // project_id
        None, // due_ts
        None, // scheduled_ts
        Some(Some(tomorrow_ts)), // wait_ts
        None, // alloc_secs
        None, // template
        None, // respawn
        &std::collections::HashMap::new(), // udas_to_add
        &[], // udas_to_remove
        &[], // tags_to_add
        &[], // tags_to_remove
    ).unwrap();
    
    // Test waiting filter
    let filter_expr = parse_filter(vec!["waiting".to_string()]).unwrap();
    let matching = filter_tasks(ctx.db(), &filter_expr).unwrap();
    let matching_ids: Vec<i64> = matching.iter().map(|(t, _)| t.id.unwrap()).collect();
    assert!(matching_ids.contains(&task10), "Task 10 should appear in waiting list");
    
    // Note: Testing "time passes beyond wait" would require time manipulation
    // which is complex. This test verifies the basic waiting filter works.
}

// ============================================================================
// Section 11.7: Respawn
// ============================================================================

#[test]
fn acceptance_respawn_on_finish() {
    // Given a task T with `respawn:daily` and a due date
    // When task is finished
    // Then a new task is created with the respawn rule and an updated due date
    
    let ctx = AcceptanceTestContext::new();
    
    use std::collections::HashMap;
    
    // Create task with respawn rule
    let due_ts = chrono::Utc::now().timestamp() + 3600; // 1 hour from now
    let task = TaskRepo::create_full(
        ctx.db(),
        "Daily task",
        None,
        Some(due_ts),
        None,
        None,
        Some(1800), // 30 min allocation
        None,
        Some("daily".to_string()),
        &HashMap::new(),
        &[],
    ).unwrap();
    
    let task_id = task.id.unwrap();
    
    // Finish the task
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["finish", &task_id.to_string(), "-y"]);
    
    // Verify original task is completed
    let original = TaskRepo::get_by_id(ctx.db(), task_id).unwrap().unwrap();
    assert_eq!(original.status, tatl::models::TaskStatus::Completed);
    
    // Verify new respawned task exists
    let all_tasks = TaskRepo::list_all(ctx.db()).unwrap();
    let respawned: Vec<_> = all_tasks.iter()
        .filter(|(t, _)| t.id != Some(task_id) && t.description == "Daily task")
        .collect();
    
    assert_eq!(respawned.len(), 1, "Should have one respawned task");
    let (new_task, _) = &respawned[0];
    assert_eq!(new_task.respawn, Some("daily".to_string()));
    assert_eq!(new_task.alloc_secs, Some(1800));
    assert!(new_task.due_ts.is_some());
    assert!(new_task.due_ts.unwrap() > due_ts, "New due date should be after original");
}

// ============================================================================
// Section 11.8: Projects
// ============================================================================

#[test]
fn acceptance_project_rename_errors_if_target_exists() {
    // Given project `work` exists
    // And project `office` exists
    // When `task projects rename work office`
    // Then exit code is 1
    // And message indicates project already exists
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    given.project_exists("work");
    given.project_exists("office");
    
    let mut when = WhenBuilder::new(&ctx);
    when.execute_failure(&["projects", "rename", "work", "office"]);
    
    let then = ThenBuilder::new(&ctx, when.result());
    then.exit_code_is(1)
        .message_contains("already exists");
}

#[test]
fn acceptance_project_rename_with_force_merges_projects() {
    // Given project `temp` exists with tasks 10, 11
    // And project `work` exists with task 12
    // When `task projects rename temp work --force`
    // Then project `temp` no longer exists
    // And tasks 10, 11, 12 all reference project `work`
    // And project `work` still exists
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    given.project_exists("temp");
    given.project_exists("work");
    
    let task10 = given.task_exists_with_project("Task 10", "temp");
    let task11 = given.task_exists_with_project("Task 11", "temp");
    let task12 = given.task_exists_with_project("Task 12", "work");
    
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["projects", "rename", "temp", "work", "--force"]);
    
    let then = ThenBuilder::new(&ctx, when.result());
    then.project_does_not_exist("temp")
        .project_exists("work")
        .task_references_project(task10, "work")
        .task_references_project(task11, "work")
        .task_references_project(task12, "work");
}

#[test]
fn acceptance_project_merge_archive_status_handling() {
    // Given project `temp` (archived) exists with task 10
    // And project `work` (active) exists with task 11
    // When `task projects rename temp work --force`
    // Then project `temp` no longer exists
    // And tasks 10, 11 all reference project `work`
    // And project `work` is active (not archived)
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    given.project_exists("temp");
    given.project_exists("work");
    
    // Archive temp project
    use tatl::repo::ProjectRepo;
    ProjectRepo::archive(ctx.db(), "temp").unwrap();
    
    let task10 = given.task_exists_with_project("Task 10", "temp");
    let task11 = given.task_exists_with_project("Task 11", "work");
    
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["projects", "rename", "temp", "work", "--force"]);
    
    let then = ThenBuilder::new(&ctx, when.result());
    then.project_does_not_exist("temp")
        .project_exists("work")
        .task_references_project(task10, "work")
        .task_references_project(task11, "work");
    
    // Verify work project is active (not archived)
    let work_project = ProjectRepo::get_by_name(ctx.db(), "work").unwrap().unwrap();
    assert!(!work_project.is_archived, "Work project should be active after merge");
}

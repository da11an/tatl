// End-to-End Tests for Task Ninja
// These test complete workflows from start to finish

mod acceptance_framework;
use acceptance_framework::*;
use tatl::repo::{TaskRepo, ProjectRepo, StackRepo, SessionRepo, AnnotationRepo, TemplateRepo};
use tatl::filter::{parse_filter, filter_tasks};
use tatl::utils::date;
use std::collections::HashMap;
use serde_json::json;

// ============================================================================
// Complete Workflows
// ============================================================================

#[test]
fn e2e_complete_workflow_add_clock_annotate_finish() {
    // Complete workflow: add task → clock in → annotate → finish
    // This tests the most common user workflow
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Step 1: Add a task
    let task_id = given.task_exists("Complete project documentation");
    
    // Step 2: Add task to stack
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["enqueue", &task_id.to_string()]);
    
    // Step 3: Clock in
    when.execute_success(&["on"]);
    
    // Verify session is running
    let then = ThenBuilder::new(&ctx, None);
    then.running_session_exists_for_task(task_id);
    
    // Step 4: Add annotation (without explicit task ID - should use clocked-in task)
    when.execute_success(&["annotate", "Started working on documentation"]);
    
    // Verify annotation was created and linked to session
    let sessions = SessionRepo::get_by_task(ctx.db(), task_id).unwrap();
    let open_session = sessions.iter().find(|s| s.end_ts.is_none()).unwrap();
    let annotations = AnnotationRepo::get_by_session(ctx.db(), open_session.id.unwrap()).unwrap();
    assert_eq!(annotations.len(), 1);
    assert!(annotations[0].note.contains("Started working on documentation"));
    
    // Step 5: Finish the task
    when.execute_success(&["finish"]);
    
    // Verify task is completed and removed from stack
    then.task_status_is(task_id, "completed")
        .stack_is_empty();
    
    // Verify session was closed
    let sessions_after = SessionRepo::get_by_task(ctx.db(), task_id).unwrap();
    assert!(sessions_after.iter().all(|s| s.end_ts.is_some()), "All sessions should be closed");
}

#[test]
fn e2e_complete_workflow_with_project_and_tags() {
    // Complete workflow with project and tags: add → modify → clock in → finish
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Step 1: Create project
    given.project_exists("work");
    
    // Step 2: Add task with project and tags
    let task_id = given.task_exists("Review pull request");
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["modify", &task_id.to_string(), "project:work", "+urgent", "+code-review"]);
    
    // Verify task has project and tags
    let then = ThenBuilder::new(&ctx, None);
    then.task_references_project(task_id, "work")
        .task_has_tag(task_id, "urgent")
        .task_has_tag(task_id, "code-review");
    
    // Step 3: Add to stack and clock in
    when.execute_success(&["enqueue", &task_id.to_string()]);
    when.execute_success(&["on"]);
    
    // Step 4: Add annotation
    when.execute_success(&["annotate", &task_id.to_string(), "Found 3 issues to address"]);
    
    // Step 5: Finish task
    when.execute_success(&["finish"]);
    
    // Verify final state
    then.task_status_is(task_id, "completed");
    
    // Verify annotation is still linked
    let sessions = SessionRepo::get_by_task(ctx.db(), task_id).unwrap();
    let closed_session = sessions.iter().find(|s| s.end_ts.is_some()).unwrap();
    let annotations = AnnotationRepo::get_by_session(ctx.db(), closed_session.id.unwrap()).unwrap();
    assert_eq!(annotations.len(), 1);
}

#[test]
fn e2e_complete_workflow_with_finish_next() {
    // Complete workflow with --next flag: add multiple tasks → clock in → finish --next
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Step 1: Create multiple tasks
    let task1 = given.task_exists("Task 1");
    let task2 = given.task_exists("Task 2");
    let task3 = given.task_exists("Task 3");
    
    // Step 2: Add all to stack
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["enqueue", &task1.to_string()]);
    when.execute_success(&["enqueue", &task2.to_string()]);
    when.execute_success(&["enqueue", &task3.to_string()]);
    
    // Step 3: Clock in (starts task 1)
    when.execute_success(&["on"]);
    
    let then = ThenBuilder::new(&ctx, None);
    then.running_session_exists_for_task(task1);
    
    // Step 4: Finish task 1 with --next (should start task 2)
    when.execute_success(&["finish", "--next"]);
    
    // Verify task 1 is completed
    then.task_status_is(task1, "completed");
    
    // Verify task 2 is now running
    then.running_session_exists_for_task(task2);
    
    // Verify stack order
    then.stack_order_is(&[task2, task3]);
    
    // Step 5: Finish task 2 with --next (should start task 3)
    when.execute_success(&["finish", "--next"]);
    
    then.task_status_is(task2, "completed")
        .running_session_exists_for_task(task3)
        .stack_order_is(&[task3]);
}

// ============================================================================
// Complex Filter Scenarios
// ============================================================================

#[test]
fn e2e_complex_filter_scenarios() {
    // Test complex filter combinations with multiple tasks, projects, tags, and dates
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Create projects
    given.project_exists("work");
    given.project_exists("home");
    
    // Create tasks with various attributes
    let task1 = given.task_exists_with_project("Urgent work task", "work");
    let task2 = given.task_exists_with_project("Important work task", "work");
    let task3 = given.task_exists_with_project("Home task", "home");
    
    // Add tags
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["modify", &task1.to_string(), "+urgent", "+important"]);
    when.execute_success(&["modify", &task2.to_string(), "+important"]);
    when.execute_success(&["modify", &task3.to_string(), "+urgent"]);
    
    // Set due dates
    let tomorrow_ts = date::parse_date_expr("tomorrow").unwrap();
    TaskRepo::modify(
        ctx.db(),
        task1,
        None, // description
        None, // project_id
        Some(Some(tomorrow_ts)), // due_ts
        None, // scheduled_ts
        None, // wait_ts
        None, // alloc_secs
        None, // template
        None, // respawn
        &HashMap::new(), // udas_to_add
        &[], // udas_to_remove
        &[], // tags_to_add
        &[], // tags_to_remove
    ).unwrap();
    
    // Test 1: Filter by project
    let filter_expr = parse_filter(vec!["project:work".to_string()]).unwrap();
    let matching = filter_tasks(ctx.db(), &filter_expr).unwrap();
    let matching_ids: Vec<i64> = matching.iter().map(|(t, _)| t.id.unwrap()).collect();
    assert!(matching_ids.contains(&task1));
    assert!(matching_ids.contains(&task2));
    assert!(!matching_ids.contains(&task3));
    
    // Test 2: Filter by tag (OR)
    let filter_expr = parse_filter(vec!["+urgent".to_string(), "or".to_string(), "+important".to_string()]).unwrap();
    let matching = filter_tasks(ctx.db(), &filter_expr).unwrap();
    let matching_ids: Vec<i64> = matching.iter().map(|(t, _)| t.id.unwrap()).collect();
    assert!(matching_ids.contains(&task1)); // Has both
    assert!(matching_ids.contains(&task2)); // Has important
    assert!(matching_ids.contains(&task3)); // Has urgent
    
    // Test 3: Filter by project AND tag
    let filter_expr = parse_filter(vec!["project:work".to_string(), "+urgent".to_string()]).unwrap();
    let matching = filter_tasks(ctx.db(), &filter_expr).unwrap();
    let matching_ids: Vec<i64> = matching.iter().map(|(t, _)| t.id.unwrap()).collect();
    assert!(matching_ids.contains(&task1)); // work + urgent
    assert!(!matching_ids.contains(&task2)); // work but not urgent
    assert!(!matching_ids.contains(&task3)); // urgent but not work
    
    // Test 4: Filter by NOT
    let filter_expr = parse_filter(vec!["not".to_string(), "project:work".to_string()]).unwrap();
    let matching = filter_tasks(ctx.db(), &filter_expr).unwrap();
    let matching_ids: Vec<i64> = matching.iter().map(|(t, _)| t.id.unwrap()).collect();
    assert!(!matching_ids.contains(&task1));
    assert!(!matching_ids.contains(&task2));
    assert!(matching_ids.contains(&task3));
    
    // Test 5: Filter by due date
    let filter_expr = parse_filter(vec!["due:tomorrow".to_string()]).unwrap();
    let matching = filter_tasks(ctx.db(), &filter_expr).unwrap();
    let matching_ids: Vec<i64> = matching.iter().map(|(t, _)| t.id.unwrap()).collect();
    assert!(matching_ids.contains(&task1));
    assert!(!matching_ids.contains(&task2));
    assert!(!matching_ids.contains(&task3));
}

#[test]
fn e2e_complex_filter_with_nested_projects() {
    // Test filtering with nested projects (e.g., admin.email, sales.northamerica)
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Create nested projects
    given.project_exists("admin");
    given.project_exists("admin.email");
    given.project_exists("admin.email.inbox");
    given.project_exists("sales");
    given.project_exists("sales.northamerica");
    
    // Create tasks in nested projects
    let task1 = given.task_exists_with_project("Admin task", "admin");
    let task2 = given.task_exists_with_project("Email task", "admin.email");
    let task3 = given.task_exists_with_project("Inbox task", "admin.email.inbox");
    let task4 = given.task_exists_with_project("Sales task", "sales");
    let task5 = given.task_exists_with_project("NA sales task", "sales.northamerica");
    
    // Test: Filter by prefix "admin" should match admin, admin.email, admin.email.inbox
    let filter_expr = parse_filter(vec!["project:admin".to_string()]).unwrap();
    let matching = filter_tasks(ctx.db(), &filter_expr).unwrap();
    let matching_ids: Vec<i64> = matching.iter().map(|(t, _)| t.id.unwrap()).collect();
    assert!(matching_ids.contains(&task1));
    assert!(matching_ids.contains(&task2));
    assert!(matching_ids.contains(&task3));
    assert!(!matching_ids.contains(&task4));
    assert!(!matching_ids.contains(&task5));
    
    // Test: Filter by prefix "admin.email" should match admin.email and admin.email.inbox
    let filter_expr = parse_filter(vec!["project:admin.email".to_string()]).unwrap();
    let matching = filter_tasks(ctx.db(), &filter_expr).unwrap();
    let matching_ids: Vec<i64> = matching.iter().map(|(t, _)| t.id.unwrap()).collect();
    assert!(!matching_ids.contains(&task1));
    assert!(matching_ids.contains(&task2));
    assert!(matching_ids.contains(&task3));
}

// ============================================================================
// Respawn Workflows
// ============================================================================

#[test]
fn e2e_respawn_on_finish_workflow() {
    // Complete workflow: create task with respawn rule → finish → verify respawned instance
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Step 1: Create a project
    given.project_exists("daily");
    let project = ProjectRepo::get_by_name(ctx.db(), "daily").unwrap().unwrap();
    
    // Step 2: Create task with respawn rule and due date
    let due_ts = chrono::Utc::now().timestamp() + 3600; // 1 hour from now
    let task = TaskRepo::create_full(
        ctx.db(),
        "Daily standup",
        Some(project.id.unwrap()),
        Some(due_ts),
        None, // scheduled_ts
        None, // wait_ts
        Some(1800), // 30 min allocation
        None, // template
        Some("daily".to_string()), // respawn rule
        &HashMap::new(), // udas
        &["standup".to_string()], // tags
    ).unwrap();
    
    let task_id = task.id.unwrap();
    
    // Step 3: Finish the task
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["finish", &task_id.to_string(), "-y"]);
    
    // Step 4: Verify original task is completed
    let original_task = TaskRepo::get_by_id(ctx.db(), task_id).unwrap().unwrap();
    assert_eq!(original_task.status, tatl::models::TaskStatus::Completed);
    
    // Step 5: Verify a new task was respawned
    let all_tasks = TaskRepo::list_all(ctx.db()).unwrap();
    let respawned: Vec<_> = all_tasks.iter()
        .filter(|(t, _)| t.id != Some(task_id) && t.description == "Daily standup" && t.respawn.is_some())
        .collect();
    
    assert_eq!(respawned.len(), 1, "Should have respawned exactly one task");
    
    // Step 6: Verify respawned task attributes
    let (new_task, new_tags) = &respawned[0];
    assert_eq!(new_task.project_id, Some(project.id.unwrap()));
    assert_eq!(new_task.alloc_secs, Some(1800));
    assert_eq!(new_task.respawn, Some("daily".to_string()));
    assert!(new_tags.contains(&"standup".to_string()));
    // Due date should be next day
    assert!(new_task.due_ts.is_some());
    assert!(new_task.due_ts.unwrap() > due_ts);
}

#[test]
fn e2e_respawn_on_close_workflow() {
    // Test that respawn also happens on close (task abandoned but obligation persists)
    
    let ctx = AcceptanceTestContext::new();
    
    // Create task with respawn rule
    let task = TaskRepo::create_full(
        ctx.db(),
        "Weekly review",
        None,
        None, None, None,
        None,
        None,
        Some("weekly".to_string()), // respawn rule
        &HashMap::new(),
        &[],
    ).unwrap();
    
    let task_id = task.id.unwrap();
    
    // Close the task (abandon it)
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["close", &task_id.to_string(), "-y"]);
    
    // Verify original task is closed
    let original_task = TaskRepo::get_by_id(ctx.db(), task_id).unwrap().unwrap();
    assert_eq!(original_task.status, tatl::models::TaskStatus::Closed);
    
    // Verify a new task was respawned
    let all_tasks = TaskRepo::list_all(ctx.db()).unwrap();
    let respawned: Vec<_> = all_tasks.iter()
        .filter(|(t, _)| t.id != Some(task_id) && t.description == "Weekly review" && t.respawn.is_some())
        .collect();
    
    assert_eq!(respawned.len(), 1, "Should have respawned exactly one task");
}

// ============================================================================
// Project Management Workflows
// ============================================================================

#[test]
fn e2e_project_management_workflow() {
    // Complete workflow: create projects → assign tasks → rename/merge projects
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Step 1: Create projects
    given.project_exists("temp_project");
    given.project_exists("main_project");
    
    // Step 2: Create tasks in different projects
    let task1 = given.task_exists_with_project("Task in temp", "temp_project");
    let task2 = given.task_exists_with_project("Another task in temp", "temp_project");
    let task3 = given.task_exists_with_project("Task in main", "main_project");
    
    // Step 3: Verify initial state
    let then = ThenBuilder::new(&ctx, None);
    then.task_references_project(task1, "temp_project")
        .task_references_project(task2, "temp_project")
        .task_references_project(task3, "main_project");
    
    // Step 4: Merge temp_project into main_project
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["projects", "rename", "temp_project", "main_project", "--force"]);
    
    // Step 5: Verify merge
    then.project_does_not_exist("temp_project")
        .project_exists("main_project")
        .task_references_project(task1, "main_project")
        .task_references_project(task2, "main_project")
        .task_references_project(task3, "main_project");
    
    // Step 6: Archive project
    ProjectRepo::archive(ctx.db(), "main_project").unwrap();
    let archived_project = ProjectRepo::get_by_name(ctx.db(), "main_project").unwrap().unwrap();
    assert!(archived_project.is_archived);
}

#[test]
fn e2e_project_management_with_nested_projects() {
    // Test project management with nested project hierarchy
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Create nested project structure
    given.project_exists("company");
    given.project_exists("company.engineering");
    given.project_exists("company.engineering.backend");
    given.project_exists("company.engineering.frontend");
    given.project_exists("company.sales");
    
    // Create tasks in nested projects
    let task1 = given.task_exists_with_project("Backend task", "company.engineering.backend");
    let task2 = given.task_exists_with_project("Frontend task", "company.engineering.frontend");
    let task3 = given.task_exists_with_project("Sales task", "company.sales");
    
    // Rename a nested project
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["projects", "rename", "company.engineering.backend", "company.engineering.api", "--force"]);
    
    // Verify task moved to new project name
    let then = ThenBuilder::new(&ctx, None);
    then.task_references_project(task1, "company.engineering.api");
    
    // Verify other projects unchanged
    then.task_references_project(task2, "company.engineering.frontend")
        .task_references_project(task3, "company.sales");
}

#[test]
fn e2e_complete_project_lifecycle() {
    // Test complete project lifecycle: create → use → archive → unarchive
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Step 1: Create project
    given.project_exists("active_project");
    
    // Step 2: Create tasks in project
    let task1 = given.task_exists_with_project("Task 1", "active_project");
    let task2 = given.task_exists_with_project("Task 2", "active_project");
    
    // Step 3: Archive project
    ProjectRepo::archive(ctx.db(), "active_project").unwrap();
    let archived = ProjectRepo::get_by_name(ctx.db(), "active_project").unwrap().unwrap();
    assert!(archived.is_archived);
    
    // Step 4: Tasks should still reference archived project
    let then = ThenBuilder::new(&ctx, None);
    then.task_references_project(task1, "active_project")
        .task_references_project(task2, "active_project");
    
    // Step 5: Unarchive project
    ProjectRepo::unarchive(ctx.db(), "active_project").unwrap();
    let unarchived = ProjectRepo::get_by_name(ctx.db(), "active_project").unwrap().unwrap();
    assert!(!unarchived.is_archived);
}

// ============================================================================
// Multi-Task Operations Workflows
// ============================================================================

#[test]
fn e2e_multi_task_modify_workflow() {
    // Test modifying multiple tasks with filters
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Create tasks with tags
    let task1 = given.task_exists_with_tags("Task 1", &["urgent"]);
    let task2 = given.task_exists_with_tags("Task 2", &["urgent"]);
    let task3 = given.task_exists_with_tags("Task 3", &["normal"]);
    
    // Modify all urgent tasks at once
    let mut when = WhenBuilder::new(&ctx);
    // Use --yes to avoid interactive prompt in test
    // Note: description modification syntax is just the new description text
    when.execute_success(&["modify", "+urgent", "--yes", "Updated urgent task"]);
    
    // Verify tasks 1 and 2 were modified, task 3 was not
    let task1_updated = TaskRepo::get_by_id(ctx.db(), task1).unwrap().unwrap();
    let task2_updated = TaskRepo::get_by_id(ctx.db(), task2).unwrap().unwrap();
    let task3_updated = TaskRepo::get_by_id(ctx.db(), task3).unwrap().unwrap();
    
    assert_eq!(task1_updated.description, "Updated urgent task");
    assert_eq!(task2_updated.description, "Updated urgent task");
    assert_eq!(task3_updated.description, "Task 3"); // Unchanged
}

#[test]
fn e2e_multi_task_finish_workflow() {
    // Test completing multiple tasks with filters
    // Note: finish command requires tasks to be clocked in, so we'll clock them in first
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Create tasks
    let task1 = given.task_exists_with_tags("Complete me 1", &["done"]);
    let task2 = given.task_exists_with_tags("Complete me 2", &["done"]);
    let task3 = given.task_exists_with_tags("Keep me", &["active"]);
    
    // Add tasks to stack and clock them in one by one, then complete
    let mut when = WhenBuilder::new(&ctx);
    
    // Clock in task 1, complete it
    when.execute_success(&["on", &task1.to_string()]);
    when.execute_success(&["finish"]);
    
    // Clock in task 2, complete it
    when.execute_success(&["on", &task2.to_string()]);
    when.execute_success(&["finish"]);
    
    // Verify tasks 1 and 2 are completed, task 3 is not
    let then = ThenBuilder::new(&ctx, None);
    then.task_status_is(task1, "completed")
        .task_status_is(task2, "completed");
    
    let task3_updated = TaskRepo::get_by_id(ctx.db(), task3).unwrap().unwrap();
    assert_eq!(task3_updated.status.as_str(), "pending");
}

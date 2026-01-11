// End-to-End Tests for Task Ninja
// These test complete workflows from start to finish

mod acceptance_framework;
use acceptance_framework::*;
use task_ninja::repo::{TaskRepo, ProjectRepo, StackRepo, SessionRepo, AnnotationRepo, TemplateRepo};
use task_ninja::filter::{parse_filter, filter_tasks};
use task_ninja::utils::date;
use std::collections::HashMap;
use serde_json::json;

// ============================================================================
// Complete Workflows
// ============================================================================

#[test]
fn e2e_complete_workflow_add_clock_annotate_done() {
    // Complete workflow: add task → clock in → annotate → done
    // This tests the most common user workflow
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Step 1: Add a task
    let task_id = given.task_exists("Complete project documentation");
    
    // Step 2: Add task to stack
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&[&task_id.to_string(), "enqueue"]);
    
    // Step 3: Clock in
    when.execute_success(&["clock", "in"]);
    
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
    
    // Step 5: Complete the task
    when.execute_success(&["done"]);
    
    // Verify task is completed and removed from stack
    then.task_status_is(task_id, "completed")
        .stack_is_empty();
    
    // Verify session was closed
    let sessions_after = SessionRepo::get_by_task(ctx.db(), task_id).unwrap();
    assert!(sessions_after.iter().all(|s| s.end_ts.is_some()), "All sessions should be closed");
}

#[test]
fn e2e_complete_workflow_with_project_and_tags() {
    // Complete workflow with project and tags: add → modify → clock in → done
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Step 1: Create project
    given.project_exists("work");
    
    // Step 2: Add task with project and tags
    let task_id = given.task_exists("Review pull request");
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&[&task_id.to_string(), "modify", "project:work", "+urgent", "+code-review"]);
    
    // Verify task has project and tags
    let then = ThenBuilder::new(&ctx, None);
    then.task_references_project(task_id, "work")
        .task_has_tag(task_id, "urgent")
        .task_has_tag(task_id, "code-review");
    
    // Step 3: Add to stack and clock in
    when.execute_success(&[&task_id.to_string(), "enqueue"]);
    when.execute_success(&["clock", "in"]);
    
    // Step 4: Add annotation
    when.execute_success(&[&task_id.to_string(), "annotate", "Found 3 issues to address"]);
    
    // Step 5: Complete task
    when.execute_success(&["done"]);
    
    // Verify final state
    then.task_status_is(task_id, "completed");
    
    // Verify annotation is still linked
    let sessions = SessionRepo::get_by_task(ctx.db(), task_id).unwrap();
    let closed_session = sessions.iter().find(|s| s.end_ts.is_some()).unwrap();
    let annotations = AnnotationRepo::get_by_session(ctx.db(), closed_session.id.unwrap()).unwrap();
    assert_eq!(annotations.len(), 1);
}

#[test]
fn e2e_complete_workflow_with_done_next() {
    // Complete workflow with --next flag: add multiple tasks → clock in → done --next
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Step 1: Create multiple tasks
    let task1 = given.task_exists("Task 1");
    let task2 = given.task_exists("Task 2");
    let task3 = given.task_exists("Task 3");
    
    // Step 2: Add all to stack
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&[&task1.to_string(), "enqueue"]);
    when.execute_success(&[&task2.to_string(), "enqueue"]);
    when.execute_success(&[&task3.to_string(), "enqueue"]);
    
    // Step 3: Clock in (starts task 1)
    when.execute_success(&["clock", "in"]);
    
    let then = ThenBuilder::new(&ctx, None);
    then.running_session_exists_for_task(task1);
    
    // Step 4: Complete task 1 with --next (should start task 2)
    when.execute_success(&["done", "--next"]);
    
    // Verify task 1 is completed
    then.task_status_is(task1, "completed");
    
    // Verify task 2 is now running
    then.running_session_exists_for_task(task2);
    
    // Verify stack order
    then.stack_order_is(&[task2, task3]);
    
    // Step 5: Complete task 2 with --next (should start task 3)
    when.execute_success(&["done", "--next"]);
    
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
    when.execute_success(&[&task1.to_string(), "modify", "+urgent", "+important"]);
    when.execute_success(&[&task2.to_string(), "modify", "+important"]);
    when.execute_success(&[&task3.to_string(), "modify", "+urgent"]);
    
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
        None, // recur
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
// Recurrence Generation Workflows
// ============================================================================

#[test]
fn e2e_recurrence_generation_workflow() {
    // Complete workflow: create template → create seed task with recurrence → run recur → verify instances
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Step 1: Create a template
    given.project_exists("meetings");
    let project = ProjectRepo::get_by_name(ctx.db(), "meetings").unwrap().unwrap();
    
    let mut template_payload = HashMap::new();
    template_payload.insert("project_id".to_string(), json!(project.id.unwrap()));
    template_payload.insert("alloc_secs".to_string(), json!(3600)); // 1 hour
    template_payload.insert("tags".to_string(), json!(["meeting", "recurring"]));
    TemplateRepo::save(ctx.db(), "weekly_meeting", &template_payload).unwrap();
    
    // Step 2: Create seed task with recurrence
    let seed_task = TaskRepo::create_full(
        ctx.db(),
        "Weekly Team Standup",
        None, // project_id (will come from template)
        None, // due_ts
        None, // scheduled_ts
        None, // wait_ts
        None, // alloc_secs (will come from template)
        Some("weekly_meeting".to_string()), // template
        Some("weekly byweekday:mon".to_string()), // recur
        &HashMap::new(), // udas
        &[], // tags (will come from template)
    ).unwrap();
    
    let _seed_id = seed_task.id.unwrap();
    
    // Step 3: Run recurrence generation
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["recur", "run", "--until", "2026-02-01"]);
    
    // Step 4: Verify instances were created
    let all_tasks = TaskRepo::list_all(ctx.db()).unwrap();
    let instances: Vec<_> = all_tasks.iter()
        .filter(|(t, _)| t.recur.is_none() && t.description == "Weekly Team Standup")
        .collect();
    
    // Should have created instances for each Monday in the range
    assert!(instances.len() >= 1, "Should create at least one instance");
    
    // Step 5: Verify instance attributes (from template)
    let (instance, tags) = &instances[0];
    assert_eq!(instance.project_id, project.id);
    assert_eq!(instance.alloc_secs, Some(3600));
    assert!(tags.contains(&"meeting".to_string()));
    assert!(tags.contains(&"recurring".to_string()));
    
    // Step 6: Verify idempotency - run again, should not create duplicates
    let instances_before = instances.len();
    when.execute_success(&["recur", "run", "--until", "2026-02-01"]);
    
    let all_tasks_after = TaskRepo::list_all(ctx.db()).unwrap();
    let instances_after: Vec<_> = all_tasks_after.iter()
        .filter(|(t, _)| t.recur.is_none() && t.description == "Weekly Team Standup")
        .collect();
    
    assert_eq!(instances_before, instances_after.len(), "Should not create duplicate instances");
}

#[test]
fn e2e_recurrence_with_template_override() {
    // Test recurrence where seed task overrides template attributes
    
    let ctx = AcceptanceTestContext::new();
    
    // Create template
    let mut template_payload = HashMap::new();
    template_payload.insert("alloc_secs".to_string(), json!(3600)); // 1 hour
    template_payload.insert("tags".to_string(), json!(["template_tag"]));
    TemplateRepo::save(ctx.db(), "base_template", &template_payload).unwrap();
    
    // Create seed task that overrides template
    let _seed_task = TaskRepo::create_full(
        ctx.db(),
        "Daily Review",
        None,
        None, None, None,
        Some(1800), // Override: 30 minutes instead of 1 hour
        Some("base_template".to_string()),
        Some("daily".to_string()),
        &HashMap::new(),
        &["seed_tag".to_string()], // Add additional tag
    ).unwrap();
    
    // Run recurrence
    let mut when = WhenBuilder::new(&ctx);
    when.execute_success(&["recur", "run", "--until", "2026-01-15"]);
    
    // Verify instances have seed overrides
    let all_tasks = TaskRepo::list_all(ctx.db()).unwrap();
    let instances: Vec<_> = all_tasks.iter()
        .filter(|(t, _)| t.recur.is_none() && t.description == "Daily Review")
        .collect();
    
    assert!(instances.len() >= 1);
    let (instance, tags) = &instances[0];
    
    // Should have seed's alloc_secs (override), not template's
    assert_eq!(instance.alloc_secs, Some(1800));
    
    // Should have both template and seed tags
    assert!(tags.contains(&"template_tag".to_string()));
    assert!(tags.contains(&"seed_tag".to_string()));
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
    when.execute_success(&["+urgent", "modify", "Updated urgent task", "--yes"]);
    
    // Verify tasks 1 and 2 were modified, task 3 was not
    let task1_updated = TaskRepo::get_by_id(ctx.db(), task1).unwrap().unwrap();
    let task2_updated = TaskRepo::get_by_id(ctx.db(), task2).unwrap().unwrap();
    let task3_updated = TaskRepo::get_by_id(ctx.db(), task3).unwrap().unwrap();
    
    assert_eq!(task1_updated.description, "Updated urgent task");
    assert_eq!(task2_updated.description, "Updated urgent task");
    assert_eq!(task3_updated.description, "Task 3"); // Unchanged
}

#[test]
fn e2e_multi_task_done_workflow() {
    // Test completing multiple tasks with filters
    // Note: done command requires tasks to be clocked in, so we'll clock them in first
    
    let ctx = AcceptanceTestContext::new();
    let given = GivenBuilder::new(&ctx);
    
    // Create tasks
    let task1 = given.task_exists_with_tags("Complete me 1", &["done"]);
    let task2 = given.task_exists_with_tags("Complete me 2", &["done"]);
    let task3 = given.task_exists_with_tags("Keep me", &["active"]);
    
    // Add tasks to stack and clock them in one by one, then complete
    let mut when = WhenBuilder::new(&ctx);
    
    // Clock in task 1, complete it
    when.execute_success(&[&task1.to_string(), "clock", "in"]);
    when.execute_success(&["done"]);
    
    // Clock in task 2, complete it
    when.execute_success(&[&task2.to_string(), "clock", "in"]);
    when.execute_success(&["done"]);
    
    // Verify tasks 1 and 2 are completed, task 3 is not
    let then = ThenBuilder::new(&ctx, None);
    then.task_status_is(task1, "completed")
        .task_status_is(task2, "completed");
    
    let task3_updated = TaskRepo::get_by_id(ctx.db(), task3).unwrap().unwrap();
    assert_eq!(task3_updated.status.as_str(), "pending");
}

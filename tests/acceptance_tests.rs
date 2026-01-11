// Acceptance tests for Task Ninja
// These implement the Given/When/Then scenarios from Section 11 of the design document

mod acceptance_framework;
use acceptance_framework::*;
use assert_cmd::Command;
use predicates::prelude::*;

// Section 11.8: Projects

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

// More acceptance tests will be added as features are implemented

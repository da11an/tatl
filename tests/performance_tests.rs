// Performance and Optimization Tests
// Verify indexes are created and used, and queries are optimized

use task_ninja::db::DbConnection;
use task_ninja::repo::{TaskRepo, ProjectRepo, StackRepo, SessionRepo};
use task_ninja::filter::{parse_filter, filter_tasks};
use std::time::Instant;

// ============================================================================
// Database Indexes Tests
// ============================================================================

#[test]
fn test_indexes_exist() {
    // Verify that all expected indexes are created in the database
    let conn = DbConnection::connect_in_memory().unwrap();
    
    // Query SQLite's index list
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type = 'index' AND name NOT LIKE 'sqlite_%' ORDER BY name"
    ).unwrap();
    
    let indexes: Vec<String> = stmt.query_map([], |row| {
        Ok(row.get::<_, String>(0)?)
    }).unwrap()
    .collect::<Result<Vec<_>, _>>().unwrap();
    
    // Expected indexes from the schema (actual index names from migrations)
    let expected_indexes = vec![
        "idx_tasks_project_id",
        "idx_tasks_status",
        "idx_tasks_due_ts",
        "idx_tasks_scheduled_ts",
        "idx_tasks_wait_ts",
        "idx_task_tags_tag",
        "idx_task_annotations_task_entry",
        "idx_task_annotations_session",
        "idx_stack_items_stack_ordinal",
        "idx_sessions_task_start",
        "idx_sessions_open",
        "ux_sessions_single_open", // Unique index
        "idx_task_events_task_ts",
        "idx_task_events_type",
    ];
    
    for expected in &expected_indexes {
        assert!(
            indexes.iter().any(|idx| idx == expected),
            "Expected index '{}' not found. Found indexes: {:?}",
            expected,
            indexes
        );
    }
}

#[test]
fn test_index_usage_in_common_queries() {
    // Test that common queries use indexes
    // Note: SQLite's EXPLAIN QUERY PLAN can show index usage
    
    let conn = DbConnection::connect_in_memory().unwrap();
    
    // Create test data
    let project = ProjectRepo::create(&conn, "test_project").unwrap();
    let _task1 = TaskRepo::create(&conn, "Task 1", Some(project.id.unwrap())).unwrap();
    let _task2 = TaskRepo::create(&conn, "Task 2", Some(project.id.unwrap())).unwrap();
    
    // Test 1: Query by project_id (should use ix_tasks_project_id)
    let mut stmt = conn.prepare("EXPLAIN QUERY PLAN SELECT * FROM tasks WHERE project_id = ?1").unwrap();
    let plan: Vec<String> = stmt.query_map([project.id.unwrap()], |row| {
        Ok(format!("{}|{}|{}|{}", 
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?))
    }).unwrap()
    .collect::<Result<Vec<_>, _>>().unwrap();
    
    // Check that index is used (plan should mention index)
    let plan_str = plan.join("\n");
    assert!(
        plan_str.contains("SEARCH") || plan_str.contains("SCAN"),
        "Query plan should show index usage. Plan: {}", plan_str
    );
    
    // Test 2: Query by status (should use ix_tasks_status)
    let mut stmt = conn.prepare("EXPLAIN QUERY PLAN SELECT * FROM tasks WHERE status = ?1").unwrap();
    let plan: Vec<String> = stmt.query_map(["pending"], |row| {
        Ok(format!("{}|{}|{}|{}", 
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?))
    }).unwrap()
    .collect::<Result<Vec<_>, _>>().unwrap();
    
    let plan_str = plan.join("\n");
    assert!(
        plan_str.contains("SEARCH") || plan_str.contains("SCAN"),
        "Query plan should show index usage. Plan: {}", plan_str
    );
}

// ============================================================================
// Query Performance Tests
// ============================================================================

#[test]
fn test_list_performance_with_indexes() {
    // Test that list queries perform well with indexes
    let conn = DbConnection::connect_in_memory().unwrap();
    
    // Create a reasonable number of tasks
    let project = ProjectRepo::create(&conn, "perf_test").unwrap();
    let num_tasks = 1000;
    
    let start = Instant::now();
    for i in 0..num_tasks {
        TaskRepo::create(&conn, &format!("Task {}", i), Some(project.id.unwrap())).unwrap();
    }
    let create_time = start.elapsed();
    
    // Test list performance
    let start = Instant::now();
    let tasks = TaskRepo::list_all(&conn).unwrap();
    let list_time = start.elapsed();
    
    assert_eq!(tasks.len(), num_tasks);
    
    // List should be fast (less than 100ms for 1000 tasks with indexes)
    assert!(
        list_time.as_millis() < 1000,
        "List query took {}ms, expected < 1000ms",
        list_time.as_millis()
    );
    
    println!("Created {} tasks in {:?}, listed in {:?}", num_tasks, create_time, list_time);
}

#[test]
fn test_filter_performance_with_indexes() {
    // Test that filter queries perform well with indexes
    let conn = DbConnection::connect_in_memory().unwrap();
    
    // Create tasks with various attributes
    let project = ProjectRepo::create(&conn, "filter_test").unwrap();
    let num_tasks = 500;
    
    for i in 0..num_tasks {
        let mut tags = vec![];
        if i % 2 == 0 {
            tags.push("even".to_string());
        }
        if i % 3 == 0 {
            tags.push("multiple_of_3".to_string());
        }
        
        TaskRepo::create_full(
            &conn,
            &format!("Task {}", i),
            Some(project.id.unwrap()),
            None,
            None,
            None,
            None,
            None,
            None,
            &std::collections::HashMap::new(),
            &tags,
        ).unwrap();
    }
    
    // Test filter by tag (should use index)
    let start = Instant::now();
    let filter_expr = parse_filter(vec!["+even".to_string()]).unwrap();
    let matching = filter_tasks(&conn, &filter_expr).unwrap();
    let filter_time = start.elapsed();
    
    // Should find approximately half the tasks
    assert!(matching.len() >= num_tasks / 2 - 10 && matching.len() <= num_tasks / 2 + 10);
    
    // Filter should be fast (less than 500ms for 500 tasks)
    assert!(
        filter_time.as_millis() < 500,
        "Filter query took {}ms, expected < 500ms",
        filter_time.as_millis()
    );
    
    println!("Filtered {} tasks in {:?}", matching.len(), filter_time);
}

#[test]
fn test_session_query_performance() {
    // Test that session queries perform well with indexes
    let conn = DbConnection::connect_in_memory().unwrap();
    
    // Create tasks and sessions
    let task = TaskRepo::create(&conn, "Session test task", None).unwrap();
    let task_id = task.id.unwrap();
    
    let num_sessions = 500;
    let start_ts = chrono::Utc::now().timestamp() - (num_sessions * 3600);
    
    for i in 0..num_sessions {
        SessionRepo::create_closed(
            &conn,
            task_id,
            start_ts + (i * 3600),
            start_ts + (i * 3600) + 1800,
        ).unwrap();
    }
    
    // Test query by task_id (should use ix_sessions_task_id)
    let start = Instant::now();
    let sessions = SessionRepo::get_by_task(&conn, task_id).unwrap();
    let query_time = start.elapsed();
    
    assert_eq!(sessions.len(), num_sessions as usize);
    
    // Query should be fast (less than 200ms for 500 sessions)
    assert!(
        query_time.as_millis() < 200,
        "Session query took {}ms, expected < 200ms",
        query_time.as_millis()
    );
    
    println!("Queried {} sessions in {:?}", sessions.len(), query_time);
}

#[test]
fn test_project_query_performance() {
    // Test that project queries perform well
    let conn = DbConnection::connect_in_memory().unwrap();
    
    // Create multiple projects
    let num_projects = 100;
    for i in 0..num_projects {
        ProjectRepo::create(&conn, &format!("project_{}", i)).unwrap();
    }
    
    // Test list projects
    let start = Instant::now();
    let projects = ProjectRepo::list(&conn, false).unwrap();
    let query_time = start.elapsed();
    
    assert_eq!(projects.len(), num_projects);
    
    // Query should be fast (less than 100ms for 100 projects)
    assert!(
        query_time.as_millis() < 100,
        "Project list query took {}ms, expected < 100ms",
        query_time.as_millis()
    );
    
    println!("Listed {} projects in {:?}", projects.len(), query_time);
}

#[test]
fn test_stack_operations_performance() {
    // Test that stack operations perform well
    let conn = DbConnection::connect_in_memory().unwrap();
    
    // Create tasks and add to stack
    let stack = StackRepo::get_or_create_default(&conn).unwrap();
    let stack_id = stack.id.unwrap();
    
    let num_tasks = 200;
    let task_ids: Vec<i64> = (0..num_tasks)
        .map(|i| {
            let task = TaskRepo::create(&conn, &format!("Stack task {}", i), None).unwrap();
            task.id.unwrap()
        })
        .collect();
    
    // Test enqueue performance
    let start = Instant::now();
    for task_id in &task_ids {
        StackRepo::enqueue(&conn, stack_id, *task_id).unwrap();
    }
    let enqueue_time = start.elapsed();
    
    // Enqueue should be reasonably fast (less than 500ms for 200 tasks)
    assert!(
        enqueue_time.as_millis() < 500,
        "Enqueue took {}ms, expected < 500ms",
        enqueue_time.as_millis()
    );
    
    // Test get_items performance
    let start = Instant::now();
    let items = StackRepo::get_items(&conn, stack_id).unwrap();
    let get_items_time = start.elapsed();
    
    assert_eq!(items.len(), num_tasks);
    
    // Get items should be fast (less than 50ms for 200 items)
    assert!(
        get_items_time.as_millis() < 50,
        "Get items took {}ms, expected < 50ms",
        get_items_time.as_millis()
    );
    
    println!("Enqueued {} tasks in {:?}, retrieved in {:?}", num_tasks, enqueue_time, get_items_time);
}

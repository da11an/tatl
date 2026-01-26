//! Filter expression evaluator
//!
//! Evaluates filter expressions against tasks to determine which tasks match the filter criteria.
//!
//! # Evaluation Process
//!
//! 1. Load all tasks from the database
//! 2. For each task, evaluate the filter expression
//! 3. Return tasks that match
//!
//! # Filter Terms
//!
//! - `id` - Match by task ID
//! - `status:<status>` - Match by status (pending, completed, closed, deleted)
//! - `project:<name>` - Match by project (supports prefix matching for nested projects)
//! - `+tag` / `-tag` - Match by tag presence/absence
//! - `due:<expr>` - Match by due date
//! - `scheduled:<expr>` - Match by scheduled date
//! - `wait:<expr>` - Match by wait date
//! - `waiting` - Derived: matches tasks with wait_ts in the future
//! - `kanban:<status>` - Derived: matches tasks by kanban status (proposed, stalled, queued, done)

use crate::models::{Task, TaskStatus};
use crate::repo::{TaskRepo, SessionRepo, StackRepo, ExternalRepo};
use crate::filter::parser::FilterTerm;
use rusqlite::Connection;
use anyhow::Result;

#[derive(Debug, Clone)]
pub enum FilterExpr {
    All, // Match all
    Term(FilterTerm),
    And(Vec<FilterExpr>),
    Or(Vec<FilterExpr>),
    Not(Box<FilterExpr>),
}

impl FilterExpr {
    /// Evaluate filter against a task
    pub fn matches(&self, task: &Task, conn: &Connection) -> Result<bool> {
        match self {
            FilterExpr::All => Ok(true),
            FilterExpr::Term(term) => term.matches(task, conn),
            FilterExpr::And(exprs) => {
                for expr in exprs {
                    if !expr.matches(task, conn)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            FilterExpr::Or(exprs) => {
                for expr in exprs {
                    if expr.matches(task, conn)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            FilterExpr::Not(expr) => {
                Ok(!expr.matches(task, conn)?)
            }
        }
    }
}

impl FilterTerm {
    fn matches(&self, task: &Task, conn: &Connection) -> Result<bool> {
        match self {
            FilterTerm::Id(id) => {
                Ok(task.id == Some(*id))
            }
            FilterTerm::Status(statuses) => {
                // Multi-value status filter: status:pending,closed matches if task status is any of the values
                let task_status = task.status.as_str();
                Ok(statuses.iter().any(|s| task_status == s.as_str()))
            }
            FilterTerm::Project(project_names) => {
                // Multi-value project filter: project:pro1,pro2 matches if task's project matches ANY of the values (OR logic)
                // Special cases:
                // - project: or project:none matches tasks WITHOUT a project
                // Nested project prefix matching:
                // - project:admin matches admin, admin.email, admin.other, etc.
                // - project:admin.email matches only admin.email and nested projects like admin.email.inbox
                
                // Check if any of the filter values is empty or "none" (meaning: match tasks without project)
                let wants_no_project = project_names.iter().any(|n| n.is_empty() || n.eq_ignore_ascii_case("none"));
                
                if let Some(project_id) = task.project_id {
                    // Task HAS a project
                    if wants_no_project && project_names.len() == 1 {
                        // Only filtering for no-project, but task has one
                        return Ok(false);
                    }
                    
                    // Get project name from database by ID
                    let mut stmt = conn.prepare("SELECT name FROM projects WHERE id = ?1")?;
                    let project_name_opt: Option<String> = stmt.query_row([project_id], |row| row.get(0)).ok();
                    if let Some(pname) = project_name_opt {
                        // Check if task's project matches ANY of the provided project names
                        for project_name in project_names {
                            // Skip empty/none values (already handled above)
                            if project_name.is_empty() || project_name.eq_ignore_ascii_case("none") {
                                continue;
                            }
                            // Exact match
                            if pname == *project_name {
                                return Ok(true);
                            }
                            // Prefix match: pname starts with "project_name."
                            if pname.starts_with(&format!("{}.", project_name)) {
                                return Ok(true);
                            }
                        }
                        Ok(false)
                    } else {
                        Ok(false)
                    }
                } else {
                    // Task has NO project - match if filter wants no-project
                    Ok(wants_no_project)
                }
            }
            FilterTerm::Tag(tag, is_positive) => {
                let tags = TaskRepo::get_tags(conn, task.id.unwrap())?;
                let has_tag = tags.contains(tag);
                Ok(if *is_positive { has_tag } else { !has_tag })
            }
            FilterTerm::Due(expr) => {
                match expr.as_str() {
                    "any" => Ok(task.due_ts.is_some()),
                    "none" => Ok(task.due_ts.is_none()),
                    _ => {
                        // Parse date expression and compare
                        match crate::utils::parse_date_expr(expr) {
                            Ok(filter_ts) => {
                                // Match if task's due_ts is on the same day as filter_ts
                                if let Some(due_ts) = task.due_ts {
                                    // Compare dates (ignore time)
                                    let filter_date = chrono::DateTime::from_timestamp(filter_ts, 0)
                                        .map(|dt| dt.date_naive());
                                    let due_date = chrono::DateTime::from_timestamp(due_ts, 0)
                                        .map(|dt| dt.date_naive());
                                    Ok(filter_date == due_date)
                                } else {
                                    Ok(false)
                                }
                            }
                            Err(_) => {
                                // If date parsing fails, don't match
                                Ok(false)
                            }
                        }
                    }
                }
            }
            FilterTerm::Scheduled(expr) => {
                match expr.as_str() {
                    "any" => Ok(task.scheduled_ts.is_some()),
                    "none" => Ok(task.scheduled_ts.is_none()),
                    _ => {
                        // Parse date expression and compare
                        match crate::utils::parse_date_expr(expr) {
                            Ok(filter_ts) => {
                                // Match if task's scheduled_ts is on the same day as filter_ts
                                if let Some(scheduled_ts) = task.scheduled_ts {
                                    let filter_date = chrono::DateTime::from_timestamp(filter_ts, 0)
                                        .map(|dt| dt.date_naive());
                                    let scheduled_date = chrono::DateTime::from_timestamp(scheduled_ts, 0)
                                        .map(|dt| dt.date_naive());
                                    Ok(filter_date == scheduled_date)
                                } else {
                                    Ok(false)
                                }
                            }
                            Err(_) => {
                                Ok(false)
                            }
                        }
                    }
                }
            }
            FilterTerm::Wait(expr) => {
                match expr.as_str() {
                    "any" => Ok(task.wait_ts.is_some()),
                    "none" => Ok(task.wait_ts.is_none()),
                    _ => {
                        // Parse date expression and compare
                        match crate::utils::parse_date_expr(expr) {
                            Ok(filter_ts) => {
                                // Match if task's wait_ts is on the same day as filter_ts
                                if let Some(wait_ts) = task.wait_ts {
                                    let filter_date = chrono::DateTime::from_timestamp(filter_ts, 0)
                                        .map(|dt| dt.date_naive());
                                    let wait_date = chrono::DateTime::from_timestamp(wait_ts, 0)
                                        .map(|dt| dt.date_naive());
                                    Ok(filter_date == wait_date)
                                } else {
                                    Ok(false)
                                }
                            }
                            Err(_) => {
                                Ok(false)
                            }
                        }
                    }
                }
            }
            FilterTerm::Waiting => {
                Ok(task.is_waiting())
            }
            FilterTerm::Kanban(statuses) => {
                // Multi-value kanban filter: kanban:queued,stalled matches if task kanban is any of the values
                let task_kanban = calculate_task_kanban(task, conn)?;
                let task_kanban_lower = task_kanban.to_lowercase();
                Ok(statuses.iter().any(|s| task_kanban_lower == s.to_lowercase()))
            }
            FilterTerm::Desc(pattern) => {
                // Case-insensitive substring match on description
                let desc_lower = task.description.to_lowercase();
                let pattern_lower = pattern.to_lowercase();
                Ok(desc_lower.contains(&pattern_lower))
            }
            FilterTerm::External(recipient) => {
                // Check if task has active externals matching the recipient
                let task_id = task.id.unwrap_or(0);
                let externals = ExternalRepo::get_active_for_task(conn, task_id)?;
                Ok(externals.iter().any(|e| e.recipient == *recipient))
            }
        }
    }
}

/// Calculate the kanban status for a task
/// This is a helper function for filter evaluation
/// Matches the logic in calculate_kanban_status() in output.rs
fn calculate_task_kanban(task: &Task, conn: &Connection) -> Result<String> {
    // Completed/closed tasks are "done"
    if task.status == TaskStatus::Completed || task.status == TaskStatus::Closed {
        return Ok("done".to_string());
    }
    
    let task_id = task.id.unwrap_or(0);
    
    // Check for externals
    let has_externals = ExternalRepo::has_active_externals(conn, task_id)?;
    if has_externals {
        return Ok("external".to_string());
    }
    
    // Get stack position
    let stack = StackRepo::get_or_create_default(conn)?;
    let items = StackRepo::get_items(conn, stack.id.unwrap())?;
    let stack_position = items.iter().position(|item| item.task_id == task_id);
    
    // Check if task has sessions
    let all_sessions = SessionRepo::list_all(conn)?;
    let has_sessions = all_sessions.iter().any(|s| s.task_id == task_id);
    
    match stack_position {
        Some(_pos) => {
            // In stack (any position) = queued
            Ok("queued".to_string())
        }
        None => {
            // Not in stack
            if has_sessions {
                Ok("stalled".to_string())  // Has sessions but not in queue
            } else {
                Ok("proposed".to_string())  // New task, not started
            }
        }
    }
}

/// Get tasks matching a filter expression
pub fn filter_tasks(conn: &Connection, filter: &FilterExpr) -> Result<Vec<(Task, Vec<String>)>> {
    let all_tasks = TaskRepo::list_all(conn)?;
    let mut matching = Vec::new();
    
    for (task, tags) in all_tasks {
        if filter.matches(&task, conn)? {
            matching.push((task, tags));
        }
    }
    
    Ok(matching)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbConnection;
    use crate::repo::{ProjectRepo, TaskRepo};
    use crate::filter::parse_filter;

    #[test]
    fn test_filter_id() {
        let conn = DbConnection::connect_in_memory().unwrap();
        
        // Create tasks
        let task1 = TaskRepo::create(&conn, "Task 1", None).unwrap();
        TaskRepo::create(&conn, "Task 2", None).unwrap();
        
        // Filter by ID
        let filter = parse_filter(vec!["1".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, task1.id);
    }

    #[test]
    fn test_filter_status() {
        let conn = DbConnection::connect_in_memory().unwrap();
        
        // Create tasks (all pending by default)
        TaskRepo::create(&conn, "Task 1", None).unwrap();
        TaskRepo::create(&conn, "Task 2", None).unwrap();
        
        // Filter by status
        let filter = parse_filter(vec!["status:pending".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_filter_project() {
        let conn = DbConnection::connect_in_memory().unwrap();
        
        // Create projects
        let work = ProjectRepo::create(&conn, "work").unwrap();
        let home = ProjectRepo::create(&conn, "home").unwrap();
        
        // Create tasks
        TaskRepo::create(&conn, "Task 1", work.id).unwrap();
        TaskRepo::create(&conn, "Task 2", home.id).unwrap();
        TaskRepo::create(&conn, "Task 3", None).unwrap();
        
        // Filter by project
        let filter = parse_filter(vec!["project:work".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_filter_nested_project() {
        let conn = DbConnection::connect_in_memory().unwrap();
        
        // Create nested projects
        ProjectRepo::create(&conn, "admin").unwrap();
        ProjectRepo::create(&conn, "admin.email").unwrap();
        ProjectRepo::create(&conn, "admin.other").unwrap();
        
        let admin = ProjectRepo::get_by_name(&conn, "admin").unwrap().unwrap();
        let admin_email = ProjectRepo::get_by_name(&conn, "admin.email").unwrap().unwrap();
        let admin_other = ProjectRepo::get_by_name(&conn, "admin.other").unwrap().unwrap();
        
        // Create tasks
        TaskRepo::create(&conn, "Task 1", admin.id).unwrap();
        TaskRepo::create(&conn, "Task 2", admin_email.id).unwrap();
        TaskRepo::create(&conn, "Task 3", admin_other.id).unwrap();
        
        // Filter by parent project (should match all)
        let filter = parse_filter(vec!["project:admin".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 3);
        
        // Filter by specific nested project
        let filter = parse_filter(vec!["project:admin.email".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_filter_tags() {
        let conn = DbConnection::connect_in_memory().unwrap();
        
        // Create tasks with tags
        let task1 = TaskRepo::create_full(&conn, "Task 1", None, None, None, None, None, None, None, &std::collections::HashMap::new(), &["urgent".to_string()]).unwrap();
        TaskRepo::create_full(&conn, "Task 2", None, None, None, None, None, None, None, &std::collections::HashMap::new(), &["important".to_string()]).unwrap();
        
        // Filter by positive tag
        let filter = parse_filter(vec!["+urgent".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, task1.id);
        
        // Filter by negative tag
        let filter = parse_filter(vec!["-urgent".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_filter_due_any_none() {
        let conn = DbConnection::connect_in_memory().unwrap();
        
        // Create tasks with and without due dates
        let now = chrono::Utc::now().timestamp();
        let task1 = TaskRepo::create_full(&conn, "Task 1", None, Some(now), None, None, None, None, None, &std::collections::HashMap::new(), &[]).unwrap();
        TaskRepo::create_full(&conn, "Task 2", None, None, None, None, None, None, None, &std::collections::HashMap::new(), &[]).unwrap();
        
        // Filter by due:any
        let filter = parse_filter(vec!["due:any".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, task1.id);
        
        // Filter by due:none
        let filter = parse_filter(vec!["due:none".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_filter_waiting() {
        let conn = DbConnection::connect_in_memory().unwrap();
        
        // Create tasks
        let future = chrono::Utc::now().timestamp() + 3600; // 1 hour in future
        let past = chrono::Utc::now().timestamp() - 3600; // 1 hour in past
        
        TaskRepo::create_full(&conn, "Waiting task", None, None, None, Some(future), None, None, None, &std::collections::HashMap::new(), &[]).unwrap();
        TaskRepo::create_full(&conn, "Not waiting", None, None, None, Some(past), None, None, None, &std::collections::HashMap::new(), &[]).unwrap();
        TaskRepo::create_full(&conn, "No wait", None, None, None, None, None, None, None, &std::collections::HashMap::new(), &[]).unwrap();
        
        // Filter by waiting
        let filter = parse_filter(vec!["waiting".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_filter_combined() {
        let conn = DbConnection::connect_in_memory().unwrap();
        
        // Create project and tasks
        let work = ProjectRepo::create(&conn, "work").unwrap();
        let task1 = TaskRepo::create_full(&conn, "Task 1", work.id, None, None, None, None, None, None, &std::collections::HashMap::new(), &["urgent".to_string()]).unwrap();
        TaskRepo::create_full(&conn, "Task 2", work.id, None, None, None, None, None, None, &std::collections::HashMap::new(), &[]).unwrap();
        
        // Combined filter: project AND tag
        let filter = parse_filter(vec!["project:work".to_string(), "+urgent".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, task1.id);
    }
}

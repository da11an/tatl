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
//! - `status=<status>` - Match by status (open, closed, cancelled, deleted)
//! - `project=<name>` - Match by project (supports prefix matching for nested projects)
//! - `+tag` / `-tag` - Match by tag presence/absence
//! - `due=<expr>` - Match by due date (supports =, >, <, >=, <=, !=)
//! - `scheduled=<expr>` - Match by scheduled date
//! - `wait=<expr>` - Match by wait date
//! - `waiting` - Derived: matches tasks with wait_ts in the future
//! - `stage=<stage>` - Derived: matches tasks by stage (proposed, planned, in progress, suspended, active, external, completed, cancelled)

use crate::models::{Task, TaskStatus};
use crate::repo::{TaskRepo, SessionRepo, StackRepo, ExternalRepo};
use crate::filter::parser::{FilterTerm, ComparisonOp};
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

/// Helper to evaluate a date field with a comparison operator
fn match_date_field(
    task_ts: Option<i64>,
    op: &ComparisonOp,
    expr: &str,
) -> Result<bool> {
    match op {
        ComparisonOp::Eq => {
            match expr {
                "any" => Ok(task_ts.is_some()),
                "none" => Ok(task_ts.is_none()),
                _ => {
                    match crate::utils::parse_date_expr(expr) {
                        Ok(filter_ts) => {
                            if let Some(ts) = task_ts {
                                let filter_date = chrono::DateTime::from_timestamp(filter_ts, 0)
                                    .map(|dt| dt.date_naive());
                                let task_date = chrono::DateTime::from_timestamp(ts, 0)
                                    .map(|dt| dt.date_naive());
                                Ok(filter_date == task_date)
                            } else {
                                Ok(false)
                            }
                        }
                        Err(_) => Ok(false),
                    }
                }
            }
        }
        ComparisonOp::Neq => {
            match expr {
                "none" => Ok(task_ts.is_some()),
                "any" => Ok(task_ts.is_none()),
                _ => {
                    match crate::utils::parse_date_expr(expr) {
                        Ok(filter_ts) => {
                            if let Some(ts) = task_ts {
                                let filter_date = chrono::DateTime::from_timestamp(filter_ts, 0)
                                    .map(|dt| dt.date_naive());
                                let task_date = chrono::DateTime::from_timestamp(ts, 0)
                                    .map(|dt| dt.date_naive());
                                Ok(filter_date != task_date)
                            } else {
                                // Task has no date, so it's != any date
                                Ok(true)
                            }
                        }
                        Err(_) => Ok(false),
                    }
                }
            }
        }
        ComparisonOp::Gt => {
            match crate::utils::parse_date_expr(expr) {
                Ok(filter_ts) => {
                    if let Some(ts) = task_ts {
                        Ok(ts > filter_ts)
                    } else {
                        Ok(false)
                    }
                }
                Err(_) => Ok(false),
            }
        }
        ComparisonOp::Lt => {
            match crate::utils::parse_date_expr(expr) {
                Ok(filter_ts) => {
                    if let Some(ts) = task_ts {
                        Ok(ts < filter_ts)
                    } else {
                        Ok(false)
                    }
                }
                Err(_) => Ok(false),
            }
        }
        ComparisonOp::Gte => {
            match crate::utils::parse_date_expr(expr) {
                Ok(filter_ts) => {
                    if let Some(ts) = task_ts {
                        Ok(ts >= filter_ts)
                    } else {
                        Ok(false)
                    }
                }
                Err(_) => Ok(false),
            }
        }
        ComparisonOp::Lte => {
            match crate::utils::parse_date_expr(expr) {
                Ok(filter_ts) => {
                    if let Some(ts) = task_ts {
                        Ok(ts <= filter_ts)
                    } else {
                        Ok(false)
                    }
                }
                Err(_) => Ok(false),
            }
        }
    }
}

impl FilterTerm {
    fn matches(&self, task: &Task, conn: &Connection) -> Result<bool> {
        match self {
            FilterTerm::Id(op, id) => {
                let task_id = task.id.unwrap_or(0);
                match op {
                    ComparisonOp::Eq => Ok(task_id == *id),
                    ComparisonOp::Neq => Ok(task_id != *id),
                    ComparisonOp::Gt => Ok(task_id > *id),
                    ComparisonOp::Lt => Ok(task_id < *id),
                    ComparisonOp::Gte => Ok(task_id >= *id),
                    ComparisonOp::Lte => Ok(task_id <= *id),
                }
            }
            FilterTerm::Status(op, statuses) => {
                // Multi-value status filter: status=pending,closed matches if task status is any of the values
                let task_status = task.status.as_str();
                let matches_any = statuses.iter().any(|s| task_status == s.as_str());
                match op {
                    ComparisonOp::Eq => Ok(matches_any),
                    ComparisonOp::Neq => Ok(!matches_any),
                    _ => Err(anyhow::anyhow!("Status filter supports only '=' and '!='")),
                }
            }
            FilterTerm::Project(op, project_names) => {
                // Multi-value project filter: project=pro1,pro2 matches if task's project matches ANY of the values (OR logic)
                // Special cases:
                // - project= or project=none matches tasks WITHOUT a project
                // Nested project prefix matching:
                // - project=admin matches admin, admin.email, admin.other, etc.
                // - project=admin.email matches only admin.email and nested projects like admin.email.inbox

                // Check if any of the filter values is empty or "none" (meaning: match tasks without project)
                let wants_no_project = project_names.iter().any(|n| n.is_empty() || n.eq_ignore_ascii_case("none"));

                if let Some(project_id) = task.project_id {
                    // Task HAS a project
                    if wants_no_project && project_names.len() == 1 && *op == ComparisonOp::Eq {
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
                                return match op {
                                    ComparisonOp::Eq => Ok(true),
                                    ComparisonOp::Neq => Ok(false),
                                    _ => Err(anyhow::anyhow!("Project filter supports only '=' and '!='")),
                                };
                            }
                            // Prefix match: pname starts with "project_name."
                            if pname.starts_with(&format!("{}.", project_name)) {
                                return match op {
                                    ComparisonOp::Eq => Ok(true),
                                    ComparisonOp::Neq => Ok(false),
                                    _ => Err(anyhow::anyhow!("Project filter supports only '=' and '!='")),
                                };
                            }
                        }
                        match op {
                            ComparisonOp::Eq => Ok(false),
                            ComparisonOp::Neq => Ok(true),
                            _ => Err(anyhow::anyhow!("Project filter supports only '=' and '!='")),
                        }
                    } else {
                        match op {
                            ComparisonOp::Eq => Ok(false),
                            ComparisonOp::Neq => Ok(true),
                            _ => Err(anyhow::anyhow!("Project filter supports only '=' and '!='")),
                        }
                    }
                } else {
                    // Task has NO project - match if filter wants no-project
                    if wants_no_project {
                        return match op {
                            ComparisonOp::Eq => Ok(true),
                            ComparisonOp::Neq => Ok(false),
                            _ => Err(anyhow::anyhow!("Project filter supports only '=' and '!='")),
                        };
                    }
                    match op {
                        ComparisonOp::Eq => Ok(false),
                        ComparisonOp::Neq => Ok(true),
                        _ => Err(anyhow::anyhow!("Project filter supports only '=' and '!='")),
                    }
                }
            }
            FilterTerm::Tag(tag, is_positive) => {
                let tags = TaskRepo::get_tags(conn, task.id.unwrap())?;
                let has_tag = tags.contains(tag);
                Ok(if *is_positive { has_tag } else { !has_tag })
            }
            FilterTerm::Due(op, expr) => {
                match_date_field(task.due_ts, op, expr)
            }
            FilterTerm::Scheduled(op, expr) => {
                match_date_field(task.scheduled_ts, op, expr)
            }
            FilterTerm::Wait(op, expr) => {
                match_date_field(task.wait_ts, op, expr)
            }
            FilterTerm::Waiting => {
                Ok(task.is_waiting())
            }
            FilterTerm::Stage(op, statuses) => {
                // Multi-value stage filter: stage=planned,suspended matches if task stage is any of the values
                let task_stage = calculate_task_stage(task, conn)?;
                let task_stage_lower = task_stage.to_lowercase();
                let matches_any = statuses.iter().any(|s| task_stage_lower == s.to_lowercase());
                match op {
                    ComparisonOp::Eq => Ok(matches_any),
                    ComparisonOp::Neq => Ok(!matches_any),
                    _ => Err(anyhow::anyhow!("Stage filter supports only '=' and '!='")),
                }
            }
            FilterTerm::Desc(op, pattern) => {
                // Case-insensitive substring match on description
                let desc_lower = task.description.to_lowercase();
                let pattern_lower = pattern.to_lowercase();
                let contains = desc_lower.contains(&pattern_lower);
                match op {
                    ComparisonOp::Eq => Ok(contains),
                    ComparisonOp::Neq => Ok(!contains),
                    _ => Err(anyhow::anyhow!("Description filter supports only '=' and '!='")),
                }
            }
            FilterTerm::External(op, recipient) => {
                // Check if task has active externals matching the recipient
                let task_id = task.id.unwrap_or(0);
                let externals = ExternalRepo::get_active_for_task(conn, task_id)?;
                let has_match = externals.iter().any(|e| e.recipient == *recipient);
                match op {
                    ComparisonOp::Eq => Ok(has_match),
                    ComparisonOp::Neq => Ok(!has_match),
                    _ => Err(anyhow::anyhow!("External filter supports only '=' and '!='")),
                }
            }
        }
    }
}

/// Calculate the derived stage for a task (Plan 41 classification)
///
/// Precedence (highest wins):
/// 1. Closed (status=closed) → "completed"
/// 2. Cancelled (status=cancelled) → "cancelled"
/// 3. Active (timer on) → "active"
/// 4. External (waiting on external party) → "external"
/// 5. Internal open state mapping:
///    | Queue | Work History | Stage      |
///    | No    | No           | proposed   |
///    | Yes   | No           | planned    |
///    | Yes   | Yes          | in progress|
///    | No    | Yes          | suspended  |
pub fn calculate_task_stage(task: &Task, conn: &Connection) -> Result<String> {
    // 1. Closed → completed
    if task.status == TaskStatus::Closed {
        return Ok("completed".to_string());
    }

    // 2. Cancelled → cancelled
    if task.status == TaskStatus::Cancelled {
        return Ok("cancelled".to_string());
    }

    let task_id = task.id.unwrap_or(0);

    // 3. Active (timer on) → active
    let open_session = SessionRepo::get_open(conn)?;
    if let Some(ref session) = open_session {
        if session.task_id == task_id {
            return Ok("active".to_string());
        }
    }

    // 4. External waiting → external
    let has_externals = ExternalRepo::has_active_externals(conn, task_id)?;
    if has_externals {
        return Ok("external".to_string());
    }

    // 5. Internal open state mapping
    let stack = StackRepo::get_or_create_default(conn)?;
    let items = StackRepo::get_items(conn, stack.id.unwrap())?;
    let in_queue = items.iter().any(|item| item.task_id == task_id);

    let all_sessions = SessionRepo::list_all(conn)?;
    let has_sessions = all_sessions.iter().any(|s| s.task_id == task_id);

    Ok(match (in_queue, has_sessions) {
        (false, false) => "proposed".to_string(),
        (true, false) => "planned".to_string(),
        (true, true) => "in progress".to_string(),
        (false, true) => "suspended".to_string(),
    })
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

        // Create tasks (all open by default)
        TaskRepo::create(&conn, "Task 1", None).unwrap();
        TaskRepo::create(&conn, "Task 2", None).unwrap();

        // Filter by status
        let filter = parse_filter(vec!["status=open".to_string()]).unwrap();
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
        let filter = parse_filter(vec!["project=work".to_string()]).unwrap();
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
        let filter = parse_filter(vec!["project=admin".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 3);

        // Filter by specific nested project
        let filter = parse_filter(vec!["project=admin.email".to_string()]).unwrap();
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

        // Filter by due=any
        let filter = parse_filter(vec!["due=any".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, task1.id);

        // Filter by due=none
        let filter = parse_filter(vec!["due=none".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_filter_due_neq_none() {
        let conn = DbConnection::connect_in_memory().unwrap();

        // Create tasks with and without due dates
        let now = chrono::Utc::now().timestamp();
        let task1 = TaskRepo::create_full(&conn, "Task 1", None, Some(now), None, None, None, None, None, &std::collections::HashMap::new(), &[]).unwrap();
        TaskRepo::create_full(&conn, "Task 2", None, None, None, None, None, None, None, &std::collections::HashMap::new(), &[]).unwrap();

        // Filter by due!=none (should match tasks WITH a due date)
        let filter = parse_filter(vec!["due!=none".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, task1.id);
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
        let filter = parse_filter(vec!["project=work".to_string(), "+urgent".to_string()]).unwrap();
        let results = filter_tasks(&conn, &filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, task1.id);
    }
}

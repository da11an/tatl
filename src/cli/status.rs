// Status computation for commands without arguments

use crate::repo::{TaskRepo, ProjectRepo, StackRepo, SessionRepo, AnnotationRepo};
use crate::models::TaskStatus;
use anyhow::Result;
use chrono::{Local, TimeZone, Datelike};

/// Format duration for display (e.g., "2h30m", "45m", "15s")
fn format_duration_short(secs: i64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    
    if hours > 0 {
        format!("{}h{}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m", minutes)
    } else {
        format!("{}s", seconds)
    }
}

/// Format relative time (e.g., "2h ago", "30m ago")
fn format_relative_time(ts: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let diff = now - ts;
    
    if diff < 60 {
        format!("{}s ago", diff)
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

/// Compute status for root command (`task`)
pub fn compute_root_status(conn: &rusqlite::Connection) -> Result<String> {
    // Tasks in stack
    let stack = StackRepo::get_or_create_default(conn)?;
    let stack_items = StackRepo::get_items(conn, stack.id.unwrap())?;
    let tasks_in_stack = stack_items.len();
    
    // Backlogged tasks (pending status)
    let all_tasks = TaskRepo::list_all(conn)?;
    let backlogged = all_tasks.iter()
        .filter(|(task, _)| task.status == TaskStatus::Pending)
        .count();
    
    // Active projects
    let projects = ProjectRepo::list(conn, false)?;
    let active_projects = projects.len();
    
    // Clock state
    let open_session = SessionRepo::get_open(conn)?;
    let clock_status = if let Some(session) = open_session {
        let duration = chrono::Utc::now().timestamp() - session.start_ts;
        format!("in {}", format_duration_short(duration))
    } else {
        // Find most recent closed session
        let all_sessions = SessionRepo::list_all(conn)?;
        if let Some(last_session) = all_sessions.iter()
            .filter(|s| s.end_ts.is_some())
            .max_by_key(|s| s.end_ts.unwrap()) {
            let time_since = format_relative_time(last_session.end_ts.unwrap());
            format!("out ({})", time_since)
        } else {
            "out".to_string()
        }
    };
    
    Ok(format!(
        "Tasks: {} in progress, {} backlog; Projects: {} active; Clocked {}",
        tasks_in_stack, backlogged, active_projects, clock_status
    ))
}

/// Compute status for `task clock`
pub fn compute_clock_status(conn: &rusqlite::Connection) -> Result<String> {
    let stack = StackRepo::get_or_create_default(conn)?;
    let stack_items = StackRepo::get_items(conn, stack.id.unwrap())?;
    
    let (task_id, clock_state, duration_str) = if let Some(top_task) = stack_items.first() {
        let open_session = SessionRepo::get_open(conn)?;
        if let Some(session) = open_session {
            if session.task_id == top_task.task_id {
                let duration = chrono::Utc::now().timestamp() - session.start_ts;
                (top_task.task_id, "in", format_duration_short(duration))
            } else {
                (top_task.task_id, "out", "".to_string())
            }
        } else {
            (top_task.task_id, "out", "".to_string())
        }
    } else {
        return Ok("No task in stack".to_string());
    };
    
    // Compute logged time today
    let now = chrono::Utc::now();
    let today_start = Local.with_ymd_and_hms(
        now.year(), now.month(), now.day(), 0, 0, 0
    ).single()
    .map(|dt| dt.with_timezone(&chrono::Utc).timestamp())
    .unwrap_or(0);
    
    let all_sessions = SessionRepo::list_all(conn)?;
    let today_duration: i64 = all_sessions.iter()
        .filter_map(|s| {
            if s.start_ts >= today_start {
                if let Some(end_ts) = s.end_ts {
                    Some(end_ts - s.start_ts)
                } else {
                    // Open session
                    Some(now.timestamp() - s.start_ts)
                }
            } else {
                None
            }
        })
        .sum();
    
    // Compute logged time in last 7 days
    let week_start = today_start - (7 * 86400);
    let week_duration: i64 = all_sessions.iter()
        .filter_map(|s| {
            if s.start_ts >= week_start {
                if let Some(end_ts) = s.end_ts {
                    Some(end_ts - s.start_ts)
                } else {
                    // Open session
                    Some(now.timestamp() - s.start_ts)
                }
            } else {
                None
            }
        })
        .sum();
    
    let mut status = format!("Task {} clocked {}", task_id, clock_state);
    if !duration_str.is_empty() {
        status.push_str(&format!(" {}", duration_str));
    }
    status.push_str(&format!(". Logged {} today, {} in last 7 days", 
        format_duration_short(today_duration),
        format_duration_short(week_duration)));
    
    Ok(status)
}

/// Compute status for `task projects`
pub fn compute_projects_status(conn: &rusqlite::Connection) -> Result<String> {
    let all_projects = ProjectRepo::list(conn, true)?;
    let active = all_projects.iter().filter(|p| !p.is_archived).count();
    let archived = all_projects.iter().filter(|p| p.is_archived).count();
    
    Ok(format!("{} active projects, {} archived", active, archived))
}

/// Compute status for `task stack`
pub fn compute_stack_status(conn: &rusqlite::Connection) -> Result<String> {
    let stack = StackRepo::get_or_create_default(conn)?;
    let stack_items = StackRepo::get_items(conn, stack.id.unwrap())?;
    
    if stack_items.is_empty() {
        return Ok("Stack is empty".to_string());
    }
    
    let top_item = &stack_items[0];
    let task = TaskRepo::get_by_id(conn, top_item.task_id)?
        .ok_or_else(|| anyhow::anyhow!("Task {} not found", top_item.task_id))?;
    
    let description = if task.description.len() > 20 {
        format!("{}..", &task.description[..20])
    } else {
        task.description.clone()
    };
    
    Ok(format!(
        "Top: {} -- {}, {} tasks in stack",
        top_item.task_id, description, stack_items.len()
    ))
}

/// Compute status for `task recur`
pub fn compute_recur_status(conn: &rusqlite::Connection) -> Result<String> {
    let all_tasks = TaskRepo::list_all(conn)?;
    let recur_tasks: Vec<_> = all_tasks.iter()
        .filter(|(task, _)| task.recur.is_some())
        .collect();
    
    let count = recur_tasks.len();
    Ok(format!("{} recurring task{}", count, if count == 1 { "" } else { "s" }))
}

/// Compute status for `task sessions`
pub fn compute_sessions_status(conn: &rusqlite::Connection) -> Result<String> {
    let all_sessions = SessionRepo::list_all(conn)?;
    let open_count = all_sessions.iter().filter(|s| s.end_ts.is_none()).count();
    let closed_count = all_sessions.len() - open_count;
    
    Ok(format!("{} open session{}, {} closed", 
        open_count, 
        if open_count == 1 { "" } else { "s" },
        closed_count))
}

/// Compute status for `task annotate`
pub fn compute_annotate_status(conn: &rusqlite::Connection) -> Result<String> {
    let all_tasks = TaskRepo::list_all(conn)?;
    let mut total_annotations = 0;
    
    for (task, _) in &all_tasks {
        if let Some(task_id) = task.id {
            let annotations = AnnotationRepo::get_by_task(conn, task_id)?;
            total_annotations += annotations.len();
        }
    }
    
    Ok(format!("{} annotation{} across {} task{}", 
        total_annotations,
        if total_annotations == 1 { "" } else { "s" },
        all_tasks.len(),
        if all_tasks.len() == 1 { "" } else { "s" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbConnection;
    
    #[test]
    fn test_format_duration_short() {
        assert_eq!(format_duration_short(3661), "1h1m");
        assert_eq!(format_duration_short(1800), "30m");
        assert_eq!(format_duration_short(45), "45s");
    }
    
    #[test]
    fn test_compute_projects_status() {
        let conn = DbConnection::connect_in_memory().unwrap();
        let status = compute_projects_status(&conn).unwrap();
        assert!(status.contains("active projects"));
    }
}

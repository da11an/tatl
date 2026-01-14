// Sessions command handlers

use crate::db::DbConnection;
use crate::repo::{SessionRepo, TaskRepo, AnnotationRepo};
use crate::models::Session;
use crate::cli::error::{user_error, validate_task_id};
use crate::filter::{parse_filter, filter_tasks};
use crate::utils::parse_date_expr;
use anyhow::{Context, Result};
use chrono::{Local, TimeZone};
use rusqlite::Connection;
use serde_json;
use std::io::{self, Write};

/// Format timestamp for display
fn format_timestamp(ts: i64) -> String {
    let dt = Local.timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| Local.timestamp_opt(0, 0).single().unwrap());
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Format duration for display
fn format_duration(secs: i64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    
    if hours > 0 {
        format!("{}h{}m{}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m{}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Handle `task [<id>] sessions list [--json]`
pub fn handle_task_sessions_list(task_id_opt: Option<String>, json: bool) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    let sessions = if let Some(ref task_id_str) = task_id_opt {
        // List sessions for specific task
        let task_id = match validate_task_id(task_id_str) {
            Ok(id) => id,
            Err(e) => user_error(&e),
        };
        
        // Verify task exists
        if TaskRepo::get_by_id(&conn, task_id)?.is_none() {
            user_error(&format!("Task {} not found", task_id));
        }
        
        SessionRepo::get_by_task(&conn, task_id)?
    } else {
        // List all sessions
        SessionRepo::list_all(&conn)?
    };
    
    if json {
        // JSON output
        let mut json_sessions = Vec::new();
        for session in &sessions {
            let task = TaskRepo::get_by_id(&conn, session.task_id)?
                .ok_or_else(|| anyhow::anyhow!("Task {} not found", session.task_id))?;
            
            let json_session = serde_json::json!({
                "id": session.id,
                "task_id": session.task_id,
                "task_description": task.description,
                "start_ts": session.start_ts,
                "end_ts": session.end_ts,
                "duration_secs": session.duration_secs(),
                "is_open": session.is_open(),
            });
            
            json_sessions.push(json_session);
        }
        println!("{}", serde_json::to_string_pretty(&json_sessions)?);
    } else {
        // Human-readable output
        if sessions.is_empty() {
            println!("No sessions found.");
            return Ok(());
        }
        
        println!("{:<10} {:<6} {:<38} {:<20} {:<20} {:<12}", "Session ID", "Task", "Description", "Start", "End", "Duration");
        println!("{}", "-".repeat(106));
        
        for session in &sessions {
            let task = TaskRepo::get_by_id(&conn, session.task_id)?
                .ok_or_else(|| anyhow::anyhow!("Task {} not found", session.task_id))?;
            
            let session_id_str = session.id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "?".to_string());
            
            let description = if task.description.len() > 36 {
                format!("{}..", &task.description[..36])
            } else {
                task.description.clone()
            };
            
            let start_str = format_timestamp(session.start_ts);
            let end_str = if let Some(end_ts) = session.end_ts {
                format_timestamp(end_ts)
            } else {
                "(running)".to_string()
            };
            
            let duration_str = if let Some(duration) = session.duration_secs() {
                format_duration(duration)
            } else {
                format_duration(chrono::Utc::now().timestamp() - session.start_ts)
            };
            
            println!("{:<10} {:<6} {:<38} {:<20} {:<20} {:<12}", 
                session_id_str, session.task_id, description, start_str, end_str, duration_str);
        }
    }
    
    Ok(())
}

/// Handle `task [<id>] sessions show`
pub fn handle_task_sessions_show(task_id_opt: Option<String>) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    let session = if let Some(ref task_id_str) = task_id_opt {
        // Show most recent session for specific task
        let task_id = match validate_task_id(task_id_str) {
            Ok(id) => id,
            Err(e) => user_error(&e),
        };
        
        // Verify task exists
        if TaskRepo::get_by_id(&conn, task_id)?.is_none() {
            user_error(&format!("Task {} not found", task_id));
        }
        
        SessionRepo::get_most_recent_for_task(&conn, task_id)?
    } else {
        // Show current running session
        SessionRepo::get_open(&conn)?
    };
    
    if let Some(session) = session {
        let task = TaskRepo::get_by_id(&conn, session.task_id)?
            .ok_or_else(|| anyhow::anyhow!("Task {} not found", session.task_id))?;
        
        // Get linked annotations
        let annotations = if let Some(session_id) = session.id {
            AnnotationRepo::get_by_session(&conn, session_id)?
        } else {
            Vec::new()
        };
        
        println!("Session {} (Task {})", 
            session.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()),
            session.task_id);
        println!("Description: {}", task.description);
        println!("Start: {}", format_timestamp(session.start_ts));
        
        if let Some(end_ts) = session.end_ts {
            println!("End: {}", format_timestamp(end_ts));
            if let Some(duration) = session.duration_secs() {
                println!("Duration: {}", format_duration(duration));
            }
        } else {
            let current_duration = chrono::Utc::now().timestamp() - session.start_ts;
            println!("End: (running)");
            println!("Duration: {} (running)", format_duration(current_duration));
        }
        
        if !annotations.is_empty() {
            println!("\nLinked Annotations:");
            for annotation in &annotations {
                println!("  [{}] {}", 
                    annotation.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()),
                    annotation.note);
            }
        }
    } else {
        if task_id_opt.is_some() {
            println!("No sessions found for this task.");
        } else {
            println!("No session is currently running.");
        }
    }
    
    Ok(())
}

/// Handle `task sessions list [<filter>...] [--json]` with filter support
pub fn handle_task_sessions_list_with_filter(filter_args: Vec<String>, json: bool) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    let sessions = if filter_args.is_empty() {
        // List all sessions
        SessionRepo::list_all(&conn)?
    } else if filter_args.len() == 1 {
        // Single argument - try to parse as task ID first, otherwise treat as filter
        match validate_task_id(&filter_args[0]) {
            Ok(task_id) => {
                // Single task ID
                if TaskRepo::get_by_id(&conn, task_id)?.is_none() {
                    user_error(&format!("Task {} not found", task_id));
                }
                SessionRepo::get_by_task(&conn, task_id)?
            }
            Err(_) => {
                // Treat as filter - aggregate sessions across all matching tasks
                let filter_expr = match parse_filter(filter_args) {
                    Ok(expr) => expr,
                    Err(e) => user_error(&format!("Filter parse error: {}", e)),
                };
                let matching_tasks = filter_tasks(&conn, &filter_expr)
                    .context("Failed to filter tasks")?;
                
                if matching_tasks.is_empty() {
                    if !json {
                        println!("No sessions found.");
                    }
                    return Ok(()); // No tasks, no sessions
                }
                
                let task_ids: Vec<i64> = matching_tasks.iter()
                    .filter_map(|(task, _)| task.id)
                    .collect();
                
                // Aggregate sessions from all matching tasks
                let mut all_sessions = Vec::new();
                for task_id in task_ids {
                    let mut task_sessions = SessionRepo::get_by_task(&conn, task_id)?;
                    all_sessions.append(&mut task_sessions);
                }
                
                // Sort by start time (newest first)
                all_sessions.sort_by(|a, b| b.start_ts.cmp(&a.start_ts));
                all_sessions
            }
        }
    } else {
        // Multiple arguments - treat as filter
        let filter_expr = match parse_filter(filter_args) {
            Ok(expr) => expr,
            Err(e) => user_error(&format!("Filter parse error: {}", e)),
        };
        let matching_tasks = filter_tasks(&conn, &filter_expr)
            .context("Failed to filter tasks")?;
        
        if matching_tasks.is_empty() {
            if !json {
                println!("No sessions found.");
            }
            return Ok(()); // No tasks, no sessions
        }
        
        let task_ids: Vec<i64> = matching_tasks.iter()
            .filter_map(|(task, _)| task.id)
            .collect();
        
        // Aggregate sessions from all matching tasks
        let mut all_sessions = Vec::new();
        for task_id in task_ids {
            let mut task_sessions = SessionRepo::get_by_task(&conn, task_id)?;
            all_sessions.append(&mut task_sessions);
        }
        
        // Sort by start time (newest first)
        all_sessions.sort_by(|a, b| b.start_ts.cmp(&a.start_ts));
        all_sessions
    };
    
    if json {
        // JSON output
        let mut json_sessions = Vec::new();
        for session in &sessions {
            let task = TaskRepo::get_by_id(&conn, session.task_id)?
                .ok_or_else(|| anyhow::anyhow!("Task {} not found", session.task_id))?;
            
            let json_session = serde_json::json!({
                "id": session.id,
                "task_id": session.task_id,
                "task_description": task.description,
                "start_ts": session.start_ts,
                "end_ts": session.end_ts,
                "duration_secs": session.duration_secs(),
                "is_open": session.is_open(),
            });
            
            json_sessions.push(json_session);
        }
        println!("{}", serde_json::to_string_pretty(&json_sessions)?);
    } else {
        // Human-readable output
        if sessions.is_empty() {
            println!("No sessions found.");
            return Ok(());
        }
        
        println!("{:<10} {:<6} {:<38} {:<20} {:<20} {:<12}", "Session ID", "Task", "Description", "Start", "End", "Duration");
        println!("{}", "-".repeat(106));
        
        for session in &sessions {
            let task = TaskRepo::get_by_id(&conn, session.task_id)?
                .ok_or_else(|| anyhow::anyhow!("Task {} not found", session.task_id))?;
            
            let session_id_str = session.id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "?".to_string());
            
            let description = if task.description.len() > 36 {
                format!("{}..", &task.description[..36])
            } else {
                task.description.clone()
            };
            
            let start_str = format_timestamp(session.start_ts);
            let end_str = if let Some(end_ts) = session.end_ts {
                format_timestamp(end_ts)
            } else {
                "(running)".to_string()
            };
            
            let duration_str = if let Some(duration) = session.duration_secs() {
                format_duration(duration)
            } else {
                format_duration(chrono::Utc::now().timestamp() - session.start_ts)
            };
            
            println!("{:<10} {:<6} {:<38} {:<20} {:<20} {:<12}", 
                session_id_str, session.task_id, description, start_str, end_str, duration_str);
        }
    }
    
    Ok(())
}

/// Handle `task [<id|filter>] sessions show` with filter support
pub fn handle_task_sessions_show_with_filter(id_or_filter_opt: Option<String>) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    let session = if let Some(ref id_or_filter) = id_or_filter_opt {
        // Try to parse as task ID first, otherwise treat as filter
        match validate_task_id(id_or_filter) {
            Ok(task_id) => {
                // Single task ID - show most recent session for this task
                if TaskRepo::get_by_id(&conn, task_id)?.is_none() {
                    user_error(&format!("Task {} not found", task_id));
                }
                SessionRepo::get_most_recent_for_task(&conn, task_id)?
            }
            Err(_) => {
                // Treat as filter - show most recent session from all matching tasks
                let filter_expr = match parse_filter(vec![id_or_filter.clone()]) {
                    Ok(expr) => expr,
                    Err(e) => user_error(&format!("Filter parse error: {}", e)),
                };
                let matching_tasks = filter_tasks(&conn, &filter_expr)
                    .context("Failed to filter tasks")?;
                
                if matching_tasks.is_empty() {
                    println!("No tasks found matching filter.");
                    return Ok(());
                }
                
                let task_ids: Vec<i64> = matching_tasks.iter()
                    .filter_map(|(task, _)| task.id)
                    .collect();
                
                // Find most recent session across all matching tasks
                let mut all_sessions = Vec::new();
                for task_id in task_ids {
                    if let Some(session) = SessionRepo::get_most_recent_for_task(&conn, task_id)? {
                        all_sessions.push(session);
                    }
                }
                
                // Get the most recent session overall
                all_sessions.sort_by(|a, b| b.start_ts.cmp(&a.start_ts));
                all_sessions.first().cloned()
            }
        }
    } else {
        // Show current running session
        SessionRepo::get_open(&conn)?
    };
    
    if let Some(session) = session {
        let task = TaskRepo::get_by_id(&conn, session.task_id)?
            .ok_or_else(|| anyhow::anyhow!("Task {} not found", session.task_id))?;
        
        // Get linked annotations
        let annotations = if let Some(session_id) = session.id {
            AnnotationRepo::get_by_session(&conn, session_id)?
        } else {
            Vec::new()
        };
        
        println!("Session {} (Task {})", 
            session.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()),
            session.task_id);
        println!("Description: {}", task.description);
        println!("Start: {}", format_timestamp(session.start_ts));
        
        if let Some(end_ts) = session.end_ts {
            println!("End: {}", format_timestamp(end_ts));
            if let Some(duration) = session.duration_secs() {
                println!("Duration: {}", format_duration(duration));
            }
        } else {
            let current_duration = chrono::Utc::now().timestamp() - session.start_ts;
            println!("End: (running)");
            println!("Duration: {} (running)", format_duration(current_duration));
        }
        
        if !annotations.is_empty() {
            println!("\nLinked Annotations:");
            for annotation in &annotations {
                println!("  [{}] {}", 
                    annotation.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()),
                    annotation.note);
            }
        }
    } else {
        if id_or_filter_opt.is_some() {
            println!("No sessions found for this task/filter.");
        } else {
            println!("No session is currently running.");
        }
    }
    
    Ok(())
}

/// Parse session modification arguments
/// Returns (start_ts, end_ts) where:
/// - Some(Some(ts)) = set to timestamp
/// - Some(None) = clear (for end only)
/// - None = no change
fn parse_session_modify_args(args: Vec<String>) -> Result<(Option<Option<i64>>, Option<Option<i64>>)> {
    let mut start: Option<Option<i64>> = None;
    let mut end: Option<Option<i64>> = None;
    
    for arg in args {
        if arg.starts_with("start:") {
            let expr = &arg[6..];
            if expr == "none" {
                return Err(anyhow::anyhow!("Cannot clear start time. Start time is required."));
            }
            let ts = parse_date_expr(expr)
                .context(format!("Failed to parse start time: {}", expr))?;
            start = Some(Some(ts));
        } else if arg.starts_with("end:") {
            let expr = &arg[4..];
            if expr == "none" {
                end = Some(None); // Clear end time (make open)
            } else if expr == "now" {
                end = Some(Some(chrono::Utc::now().timestamp()));
            } else {
                let ts = parse_date_expr(expr)
                    .context(format!("Failed to parse end time: {}", expr))?;
                end = Some(Some(ts));
            }
        } else if arg == "--yes" || arg == "--force" {
            // Flags are handled separately
            continue;
        } else {
            return Err(anyhow::anyhow!("Invalid argument: {}. Expected start:<expr> or end:<expr>", arg));
        }
    }
    
    Ok((start, end))
}

/// Format conflict error message
fn format_conflict_error(session: &Session, conflicts: &[Session], conn: &Connection) -> Result<String> {
    let task = TaskRepo::get_by_id(conn, session.task_id)?
        .ok_or_else(|| anyhow::anyhow!("Task {} not found", session.task_id))?;
    
    let mut msg = format!(
        "Error: Session modification would create conflicts:\n\n  Session {} (Task {}): {} - {}\n  Conflicts with:\n",
        session.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()),
        session.task_id,
        format_timestamp(session.start_ts),
        session.end_ts.map(|ts| format_timestamp(ts)).unwrap_or_else(|| "(running)".to_string())
    );
    
    for conflict in conflicts {
        let _conflict_task = TaskRepo::get_by_id(conn, conflict.task_id)?
            .ok_or_else(|| anyhow::anyhow!("Task {} not found", conflict.task_id))?;
        msg.push_str(&format!(
            "    - Session {} (Task {}): {} - {}\n",
            conflict.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()),
            conflict.task_id,
            format_timestamp(conflict.start_ts),
            conflict.end_ts.map(|ts| format_timestamp(ts)).unwrap_or_else(|| "(running)".to_string())
        ));
    }
    
    msg.push_str("\nUse --force to override (may require resolving conflicts manually).");
    Ok(msg)
}

/// Check for overlapping sessions
fn check_session_overlaps(
    conn: &Connection,
    session: &Session,
    new_start_ts: Option<i64>,
    new_end_ts: Option<Option<i64>>,
) -> Result<Vec<Session>> {
    let start_ts = new_start_ts.unwrap_or(session.start_ts);
    let end_ts = if let Some(new_end) = new_end_ts {
        new_end
    } else {
        session.end_ts
    };
    
    SessionRepo::find_overlapping_sessions(
        conn,
        session.task_id,
        start_ts,
        end_ts,
        session.id,
    )
}

/// Handle `task sessions <session_id> modify [start:<expr>] [end:<expr>] [--yes] [--force]`
pub fn handle_sessions_modify(session_id: i64, args: Vec<String>, yes: bool, force: bool) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // Get session
    let session = SessionRepo::get_by_id(&conn, session_id)?
        .ok_or_else(|| anyhow::anyhow!("Session {} not found", session_id))?;
    
    // Parse modification arguments
    let (start_opt, end_opt) = parse_session_modify_args(args)?;
    
    // Determine new values
    let new_start_ts = start_opt.map(|s| s.unwrap());
    let new_end_ts = end_opt;
    
    // Check if running session and trying to clear end time
    if session.is_open() && new_end_ts == Some(None) {
        user_error("Cannot clear end time of a running session. It is already open.");
    }
    
    // Check for overlaps
    let conflicts = check_session_overlaps(&conn, &session, new_start_ts, new_end_ts)?;
    
    if !conflicts.is_empty() && !force {
        let error_msg = format_conflict_error(&session, &conflicts, &conn)?;
        user_error(&error_msg);
    }
    
    // Show what will be modified
    let mut changes = Vec::new();
    if let Some(new_start) = new_start_ts {
        if new_start != session.start_ts {
            changes.push(format!("Start: {} -> {}", 
                format_timestamp(session.start_ts),
                format_timestamp(new_start)));
        }
    }
    if let Some(new_end) = new_end_ts {
        match (session.end_ts, new_end) {
            (Some(old_end), Some(new_end_ts)) if old_end != new_end_ts => {
                changes.push(format!("End: {} -> {}", 
                    format_timestamp(old_end),
                    format_timestamp(new_end_ts)));
            }
            (Some(_), None) => {
                changes.push("End: (closed) -> (open)".to_string());
            }
            (None, Some(new_end_ts)) => {
                changes.push(format!("End: (running) -> {}", 
                    format_timestamp(new_end_ts)));
            }
            _ => {}
        }
    }
    
    if changes.is_empty() {
        println!("No changes specified.");
        return Ok(());
    }
    
    // Confirmation prompt
    if !yes {
        println!("Modify session {}?", session_id);
        for change in &changes {
            println!("  {}", change);
        }
        if !conflicts.is_empty() {
            println!("\nWarning: This will create conflicts with {} other session(s).", conflicts.len());
        }
        print!("Are you sure? (y/n): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() != "y" {
            println!("Cancelled.");
            return Ok(());
        }
    }
    
    // Apply modifications
    if let Some(new_start) = new_start_ts {
        SessionRepo::modify_start_time(&conn, session_id, new_start)?;
    }
    if let Some(new_end) = new_end_ts {
        SessionRepo::modify_end_time(&conn, session_id, new_end)?;
    }
    
    println!("Modified session {}.", session_id);
    Ok(())
}

/// Handle `task sessions <session_id> delete [--yes]`
pub fn handle_sessions_delete(session_id: i64, yes: bool) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // Get session
    let session = SessionRepo::get_by_id(&conn, session_id)?
        .ok_or_else(|| anyhow::anyhow!("Session {} not found", session_id))?;
    
    // Check if running session
    if session.is_open() {
        user_error("Cannot delete running session. Please clock out first.");
    }
    
    // Get task info
    let task = TaskRepo::get_by_id(&conn, session.task_id)?
        .ok_or_else(|| anyhow::anyhow!("Task {} not found", session.task_id))?;
    
    // Get linked annotations count
    let annotations = if let Some(sid) = session.id {
        AnnotationRepo::get_by_session(&conn, sid)?
    } else {
        Vec::new()
    };
    
    // Confirmation prompt
    if !yes {
        println!("Delete session {}?", session_id);
        println!("  Task: {} ({})", session.task_id, task.description);
        println!("  Start: {}", format_timestamp(session.start_ts));
        if let Some(end_ts) = session.end_ts {
            println!("  End: {}", format_timestamp(end_ts));
            if let Some(duration) = session.duration_secs() {
                println!("  Duration: {}", format_duration(duration));
            }
        }
        println!("  Linked annotations: {}", annotations.len());
        print!("\nAre you sure? (y/n): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() != "y" {
            println!("Cancelled.");
            return Ok(());
        }
    }
    
    // Delete session
    SessionRepo::delete(&conn, session_id)?;
    
    println!("Deleted session {}.", session_id);
    Ok(())
}

/// Parse session add arguments (supports both labeled and positional formats)
/// Labeled: task:<id> start:<time> end:<time> [note:<note>]
/// Positional: <id> <start> <end> [<note>]
fn parse_session_add_args(args: Vec<String>) -> Result<(i64, i64, i64, Option<String>)> {
    if args.is_empty() {
        return Err(anyhow::anyhow!("Missing required arguments. Use: task sessions add task:<id> start:<time> end:<time> [note:<note>] or task sessions add <id> <start> <end> [<note>]"));
    }
    
    // Check if first argument starts with "task:" (labeled format)
    let is_labeled = args[0].starts_with("task:");
    
    if is_labeled {
        // Labeled format: task:<id> start:<time> end:<time> [note:<note>]
        let mut task_id: Option<i64> = None;
        let mut start_ts: Option<i64> = None;
        let mut end_ts: Option<i64> = None;
        let mut note: Option<String> = None;
        
        for arg in args {
            if arg.starts_with("task:") {
                let task_str = &arg[5..];
                task_id = Some(task_str.parse()
                    .map_err(|_| anyhow::anyhow!("Invalid task ID: {}", task_str))?);
            } else if arg.starts_with("start:") {
                let expr = &arg[6..];
                start_ts = Some(parse_date_expr(expr)
                    .context(format!("Failed to parse start time: {}", expr))?);
            } else if arg.starts_with("end:") {
                let expr = &arg[4..];
                end_ts = Some(parse_date_expr(expr)
                    .context(format!("Failed to parse end time: {}", expr))?);
            } else if arg.starts_with("note:") {
                note = Some(arg[5..].to_string());
            } else {
                return Err(anyhow::anyhow!("Invalid argument: {}. Expected task:<id>, start:<time>, end:<time>, or note:<note>", arg));
            }
        }
        
        let task_id = task_id.ok_or_else(|| anyhow::anyhow!("Missing required argument: task:<id>"))?;
        let start_ts = start_ts.ok_or_else(|| anyhow::anyhow!("Missing required argument: start:<time>"))?;
        let end_ts = end_ts.ok_or_else(|| anyhow::anyhow!("Missing required argument: end:<time>"))?;
        
        Ok((task_id, start_ts, end_ts, note))
    } else {
        // Positional format: <id> <start> <end> [<note>]
        if args.len() < 3 {
            return Err(anyhow::anyhow!("Missing required arguments. Expected: task sessions add <id> <start> <end> [<note>]"));
        }
        
        let task_id = args[0].parse()
            .map_err(|_| anyhow::anyhow!("Invalid task ID: {}", args[0]))?;
        let start_ts = parse_date_expr(&args[1])
            .context(format!("Failed to parse start time: {}", args[1]))?;
        let end_ts = parse_date_expr(&args[2])
            .context(format!("Failed to parse end time: {}", args[2]))?;
        let note = if args.len() > 3 {
            Some(args[3..].join(" ")) // Join remaining args as note
        } else {
            None
        };
        
        Ok((task_id, start_ts, end_ts, note))
    }
}

/// Handle `task sessions add task:<id> start:<time> end:<time> [note:<note>]`
/// Or: `task sessions add <id> <start> <end> [<note>]`
pub fn handle_sessions_add(args: Vec<String>) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // Parse arguments
    let (task_id, start_ts, end_ts, note) = parse_session_add_args(args)?;
    
    // Validate task exists
    let task = TaskRepo::get_by_id(&conn, task_id)?
        .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;
    
    // Validate start < end
    if start_ts >= end_ts {
        return Err(anyhow::anyhow!("Start time must be before end time. Start: {}, End: {}", 
            format_timestamp(start_ts), format_timestamp(end_ts)));
    }
    
    // Create closed session
    let session = SessionRepo::create_closed(&conn, task_id, start_ts, end_ts)
        .context("Failed to create session")?;
    
    let session_id = session.id.unwrap();
    
    // Create annotation if note provided
    if let Some(note_text) = note {
        if !note_text.trim().is_empty() {
            AnnotationRepo::create(&conn, task_id, note_text, Some(session_id))
                .context("Failed to create annotation")?;
        }
    }
    
    let duration = end_ts - start_ts;
    println!("Added session {} for task {} ({}): {} - {} ({})", 
        session_id,
        task_id,
        task.description,
        format_timestamp(start_ts),
        format_timestamp(end_ts),
        format_duration(duration));
    
    Ok(())
}

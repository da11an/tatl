// Sessions command handlers

use crate::db::DbConnection;
use crate::repo::{SessionRepo, TaskRepo, AnnotationRepo, ViewRepo};
use crate::models::Session;
use crate::cli::error::{user_error, validate_task_id};
use crate::filter::{parse_filter, filter_tasks};
use crate::utils::parse_date_expr;
use anyhow::{Context, Result};
use chrono::{Local, TimeZone};
use rusqlite::Connection;
use serde_json;
use std::io::{self, Write};
use std::cmp::Ordering;

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
struct ListRequest {
    filter_tokens: Vec<String>,
    sort_columns: Vec<String>,
    group_columns: Vec<String>,
    hide_columns: Vec<String>,
    save_alias: Option<String>,
}

fn parse_list_request(tokens: Vec<String>) -> ListRequest {
    let mut filter_tokens = Vec::new();
    let mut sort_columns = Vec::new();
    let mut group_columns = Vec::new();
    let mut hide_columns = Vec::new();
    let mut save_alias: Option<String> = None;
    
    for token in tokens {
        if let Some(spec) = token.strip_prefix("sort:") {
            sort_columns.extend(spec.split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()));
        } else if let Some(spec) = token.strip_prefix("group:") {
            group_columns.extend(spec.split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()));
        } else if let Some(spec) = token.strip_prefix("hide:") {
            hide_columns.extend(spec.split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()));
        } else if let Some(name) = token.strip_prefix("alias:") {
            if save_alias.is_none() && !name.is_empty() {
                save_alias = Some(name.to_string());
            }
        } else {
            filter_tokens.push(token);
        }
    }
    
    ListRequest {
        filter_tokens,
        sort_columns,
        group_columns,
        hide_columns,
        save_alias,
    }
}

fn is_view_name_token(token: &str) -> bool {
    !token.contains(':') && !token.starts_with('+') && !token.starts_with('-') && token.parse::<i64>().is_err()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SessionListColumn {
    SessionId,
    TaskId,
    Description,
    Start,
    End,
    Duration,
}

#[derive(Debug, Clone)]
enum SortValue {
    Int(i64),
    Str(String),
}

#[derive(Debug, Clone)]
struct SessionRow {
    values: std::collections::HashMap<SessionListColumn, String>,
    sort_values: std::collections::HashMap<SessionListColumn, Option<SortValue>>,
}

fn parse_session_column(name: &str) -> Option<SessionListColumn> {
    match name.to_lowercase().as_str() {
        "session" | "session_id" | "id" => Some(SessionListColumn::SessionId),
        "task" | "task_id" => Some(SessionListColumn::TaskId),
        "description" | "desc" => Some(SessionListColumn::Description),
        "start" => Some(SessionListColumn::Start),
        "end" => Some(SessionListColumn::End),
        "duration" => Some(SessionListColumn::Duration),
        _ => None,
    }
}

fn session_column_label(column: SessionListColumn) -> &'static str {
    match column {
        SessionListColumn::SessionId => "Session ID",
        SessionListColumn::TaskId => "Task",
        SessionListColumn::Description => "Description",
        SessionListColumn::Start => "Start",
        SessionListColumn::End => "End",
        SessionListColumn::Duration => "Duration",
    }
}

fn compare_sort_values(a: &Option<SortValue>, b: &Option<SortValue>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => match (a, b) {
            (SortValue::Int(a), SortValue::Int(b)) => a.cmp(b),
            (SortValue::Str(a), SortValue::Str(b)) => a.cmp(b),
            _ => sort_value_as_string(a).cmp(&sort_value_as_string(b)),
        },
    }
}

fn sort_value_as_string(value: &SortValue) -> String {
    match value {
        SortValue::Int(v) => v.to_string(),
        SortValue::Str(v) => v.clone(),
    }
}

fn format_sessions_list_table(
    sessions: &[Session],
    tasks_by_id: &std::collections::HashMap<i64, String>,
    sort_columns: &[String],
    group_columns: &[String],
) -> String {
    if sessions.is_empty() {
        return "No sessions found.".to_string();
    }
    
    let mut rows: Vec<SessionRow> = Vec::new();
    for session in sessions {
        let desc = tasks_by_id.get(&session.task_id).cloned().unwrap_or_default();
        let description = if desc.len() > 36 {
            format!("{}..", &desc[..36])
        } else {
            desc.clone()
        };
        
        let start_str = format_timestamp(session.start_ts);
        let end_str = if let Some(end_ts) = session.end_ts {
            format_timestamp(end_ts)
        } else {
            "(running)".to_string()
        };
        let duration_secs = session.duration_secs().unwrap_or_else(|| chrono::Utc::now().timestamp() - session.start_ts);
        let duration_str = format_duration(duration_secs);
        
        let mut values = std::collections::HashMap::new();
        values.insert(SessionListColumn::SessionId, session.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()));
        values.insert(SessionListColumn::TaskId, session.task_id.to_string());
        values.insert(SessionListColumn::Description, description);
        values.insert(SessionListColumn::Start, start_str.clone());
        values.insert(SessionListColumn::End, end_str.clone());
        values.insert(SessionListColumn::Duration, duration_str.clone());
        
        let mut sort_values = std::collections::HashMap::new();
        sort_values.insert(SessionListColumn::SessionId, session.id.map(SortValue::Int));
        sort_values.insert(SessionListColumn::TaskId, Some(SortValue::Int(session.task_id)));
        sort_values.insert(SessionListColumn::Description, Some(SortValue::Str(desc)));
        sort_values.insert(SessionListColumn::Start, Some(SortValue::Int(session.start_ts)));
        sort_values.insert(SessionListColumn::End, Some(SortValue::Int(session.end_ts.unwrap_or(0))));
        sort_values.insert(SessionListColumn::Duration, Some(SortValue::Int(duration_secs)));
        
        rows.push(SessionRow { values, sort_values });
    }
    
    let mut effective_sort_columns: Vec<String> = group_columns.to_vec();
    for sort_col in sort_columns {
        if !effective_sort_columns.iter().any(|c| c.eq_ignore_ascii_case(sort_col)) {
            effective_sort_columns.push(sort_col.clone());
        }
    }
    if !effective_sort_columns.is_empty() {
        let group_columns_parsed: Vec<SessionListColumn> = group_columns.iter()
            .filter_map(|name| parse_session_column(name))
            .collect();
        rows.sort_by(|a, b| {
            if !group_columns_parsed.is_empty() {
                let a_key: Vec<String> = group_columns_parsed.iter()
                    .map(|column| a.values.get(column).cloned().unwrap_or_default().trim().to_string())
                    .collect();
                let b_key: Vec<String> = group_columns_parsed.iter()
                    .map(|column| b.values.get(column).cloned().unwrap_or_default().trim().to_string())
                    .collect();
                if a_key != b_key {
                    return a_key.cmp(&b_key);
                }
            }
            for col_name in &effective_sort_columns {
                if let Some(column) = parse_session_column(col_name) {
                    let ordering = compare_sort_values(
                        a.sort_values.get(&column).unwrap_or(&None),
                        b.sort_values.get(&column).unwrap_or(&None),
                    );
                    if ordering != Ordering::Equal {
                        return ordering;
                    }
                }
            }
            Ordering::Equal
        });
    }
    
    let mut columns: Vec<SessionListColumn> = Vec::new();
    for col in sort_columns {
        if let Some(column) = parse_session_column(col) {
            if !columns.contains(&column) {
                columns.push(column);
            }
        }
    }
    for column in [SessionListColumn::SessionId, SessionListColumn::TaskId, SessionListColumn::Description] {
        if !columns.contains(&column) {
            columns.push(column);
        }
    }
    for col in group_columns {
        if let Some(column) = parse_session_column(col) {
            if !columns.contains(&column) {
                columns.push(column);
            }
        }
    }
    let default_columns = [
        SessionListColumn::Start,
        SessionListColumn::End,
        SessionListColumn::Duration,
    ];
    for column in default_columns {
        if !columns.contains(&column) {
            columns.push(column);
        }
    }
    
    let mut column_widths: std::collections::HashMap<SessionListColumn, usize> = std::collections::HashMap::new();
    for column in &columns {
        column_widths.insert(*column, session_column_label(*column).len().max(6));
    }
    for row in &rows {
        for column in &columns {
            if let Some(value) = row.values.get(column) {
                let max_len = if *column == SessionListColumn::Description {
                    value.len().min(50)
                } else {
                    value.len()
                };
                let entry = column_widths.entry(*column).or_insert(6);
                *entry = (*entry).max(max_len);
            }
        }
    }
    
    let mut output = String::new();
    for (idx, column) in columns.iter().enumerate() {
        let width = *column_widths.get(column).unwrap_or(&6);
        if idx == columns.len() - 1 {
            output.push_str(&format!("{:<width$}\n", session_column_label(*column), width = width));
        } else {
            output.push_str(&format!("{:<width$} ", session_column_label(*column), width = width));
        }
    }
    
    let total_width: usize = columns.iter()
        .map(|col| column_widths.get(col).copied().unwrap_or(6))
        .sum::<usize>() + (columns.len().saturating_sub(1));
    output.push_str(&format!("{}\n", "-".repeat(total_width)));
    
    if group_columns.is_empty() {
        for row in &rows {
            for (idx, column) in columns.iter().enumerate() {
                let width = *column_widths.get(column).unwrap_or(&6);
                let raw_value = row.values.get(column).cloned().unwrap_or_default();
                let value = if raw_value.len() > width {
                    format!("{}..", &raw_value[..width.saturating_sub(2)])
                } else {
                    raw_value
                };
                if idx == columns.len() - 1 {
                    output.push_str(&format!("{:<width$}\n", value, width = width));
                } else {
                    output.push_str(&format!("{:<width$} ", value, width = width));
                }
            }
        }
    } else {
        let group_columns_parsed: Vec<SessionListColumn> = group_columns.iter()
            .filter_map(|name| parse_session_column(name))
            .collect();
        let mut groups: Vec<(Vec<String>, Vec<&SessionRow>)> = Vec::new();
        let mut group_index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for row in &rows {
            let group_values: Vec<String> = group_columns_parsed.iter()
                .map(|column| row.values.get(column).cloned().unwrap_or_default().trim().to_string())
                .collect();
            let group_key = group_values.join("\u{1f}");
            if let Some(existing_idx) = group_index.get(&group_key).copied() {
                groups[existing_idx].1.push(row);
            } else {
                groups.push((group_values, vec![row]));
                group_index.insert(group_key, groups.len() - 1);
            }
        }
        
        for (group_values, group_rows) in groups {
            // Build group label from group values (joined with ":")
            let group_label = group_values.iter()
                .filter(|v| !v.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(":");
            
            // Embed group label at the start of the divider line in square brackets
            let bracket_label = format!("[{}]", group_label);
            let dash_count = total_width.saturating_sub(bracket_label.len());
            output.push_str(&format!("{}{}\n", bracket_label, "-".repeat(dash_count)));
            
            for row in group_rows {
                for (idx, column) in columns.iter().enumerate() {
                    let width = *column_widths.get(column).unwrap_or(&6);
                    let raw_value = row.values.get(column).cloned().unwrap_or_default();
                    let value = if raw_value.len() > width {
                        format!("{}..", &raw_value[..width.saturating_sub(2)])
                    } else {
                        raw_value
                    };
                    if idx == columns.len() - 1 {
                        output.push_str(&format!("{:<width$}\n", value, width = width));
                    } else {
                        output.push_str(&format!("{:<width$} ", value, width = width));
                    }
                }
            }
        }
    }
    
    output
}

pub fn handle_task_sessions_list_with_filter(filter_args: Vec<String>, json: bool) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    let mut request = parse_list_request(filter_args);
    if request.sort_columns.is_empty()
        && request.group_columns.is_empty()
        && request.filter_tokens.len() == 1
        && is_view_name_token(&request.filter_tokens[0])
    {
        if let Some(view) = ViewRepo::get_by_name(&conn, "sessions", &request.filter_tokens[0])? {
            request.filter_tokens = view.filter_tokens;
            request.sort_columns = view.sort_columns;
            request.group_columns = view.group_columns;
            request.hide_columns = view.hide_columns;
        }
    }
    
    if let Some(alias) = request.save_alias.clone() {
        ViewRepo::upsert(
            &conn,
            &alias,
            "sessions",
            &request.filter_tokens,
            &request.sort_columns,
            &request.group_columns,
            &request.hide_columns,
        )?;
        println!("Saved view '{}'.", alias);
    }
    
    let sessions = if request.filter_tokens.is_empty() {
        // List all sessions
        SessionRepo::list_all(&conn)?
    } else if request.filter_tokens.len() == 1 {
        // Single argument - try to parse as task ID first, otherwise treat as filter
        match validate_task_id(&request.filter_tokens[0]) {
            Ok(task_id) => {
                // Single task ID
                if TaskRepo::get_by_id(&conn, task_id)?.is_none() {
                    user_error(&format!("Task {} not found", task_id));
                }
                SessionRepo::get_by_task(&conn, task_id)?
            }
            Err(_) => {
                // Treat as filter - aggregate sessions across all matching tasks
                let filter_expr = match parse_filter(request.filter_tokens) {
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
        let filter_expr = match parse_filter(request.filter_tokens) {
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
        let mut tasks_by_id = std::collections::HashMap::new();
        for session in &sessions {
            if !tasks_by_id.contains_key(&session.task_id) {
                if let Ok(Some(task)) = TaskRepo::get_by_id(&conn, session.task_id) {
                    tasks_by_id.insert(session.task_id, task.description);
                }
            }
        }
        
        let table = format_sessions_list_table(
            &sessions,
            &tasks_by_id,
            &request.sort_columns,
            &request.group_columns,
        );
        println!("{}", table);
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
    let _task = TaskRepo::get_by_id(conn, session.task_id)?
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
        let task = TaskRepo::get_by_id(&conn, session.task_id)?
            .ok_or_else(|| anyhow::anyhow!("Task {} not found", session.task_id))?;
        println!("Modify session {} (task {}: {})?", session_id, session.task_id, task.description);
        for change in &changes {
            println!("  {}", change);
        }
        if !conflicts.is_empty() {
            println!("\nWarning: This will create conflicts with {} other session(s).", conflicts.len());
        }
        print!("Are you sure? ([y]/n): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        if !input.is_empty() && input != "y" && input != "yes" {
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

// ============================================================================
// Time Report
// ============================================================================

use std::collections::BTreeMap;

/// Node in the project hierarchy tree for time reporting
#[derive(Debug, Clone)]
struct ProjectNode {
    name: String,           // Just this segment (e.g., "frontend")
    full_path: String,      // Full path (e.g., "client.projectA.frontend")
    direct_secs: i64,       // Time from sessions directly on this project
    total_secs: i64,        // Direct + all children (computed after tree is built)
    children: BTreeMap<String, ProjectNode>,
}

impl ProjectNode {
    fn new(name: &str, full_path: &str) -> Self {
        ProjectNode {
            name: name.to_string(),
            full_path: full_path.to_string(),
            direct_secs: 0,
            total_secs: 0,
            children: BTreeMap::new(),
        }
    }
    
    /// Recursively compute total_secs from direct_secs + children totals
    fn compute_totals(&mut self) {
        for child in self.children.values_mut() {
            child.compute_totals();
        }
        let children_sum: i64 = self.children.values().map(|c| c.total_secs).sum();
        self.total_secs = self.direct_secs + children_sum;
    }
}

/// Calculate session duration within a given period
fn session_duration_in_period(session: &Session, period_start: i64, period_end: i64) -> i64 {
    let session_start = session.start_ts.max(period_start);
    let now = chrono::Utc::now().timestamp();
    let session_end = session.end_ts.unwrap_or(now).min(period_end);
    
    if session_start >= session_end {
        0
    } else {
        session_end - session_start
    }
}

/// Format duration as "Xh Ym" for readability
fn format_duration_hm(secs: i64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    format!("{}h {:02}m", hours, minutes)
}

/// Format percentage with one decimal place
fn format_percentage(part_secs: i64, total_secs: i64) -> String {
    if total_secs == 0 {
        "0.0%".to_string()
    } else {
        let pct = (part_secs as f64 / total_secs as f64) * 100.0;
        format!("{:.1}%", pct)
    }
}

/// Build project hierarchy tree from session data
fn build_project_tree(
    conn: &Connection,
    sessions: &[Session],
    period_start: i64,
    period_end: i64,
) -> (BTreeMap<String, ProjectNode>, i64) {
    let mut roots: BTreeMap<String, ProjectNode> = BTreeMap::new();
    let mut no_project_secs: i64 = 0;
    
    // Group sessions by task, then get project for each task
    let mut task_projects: std::collections::HashMap<i64, Option<String>> = std::collections::HashMap::new();
    
    for session in sessions {
        let duration = session_duration_in_period(session, period_start, period_end);
        if duration == 0 {
            continue;
        }
        
        // Get project for this task (cache to avoid repeated lookups)
        let project_name = task_projects.entry(session.task_id).or_insert_with(|| {
            if let Ok(Some(task)) = TaskRepo::get_by_id(conn, session.task_id) {
                task.project_id.and_then(|pid| {
                    crate::repo::ProjectRepo::get_by_id(conn, pid)
                        .ok()
                        .flatten()
                        .map(|p| p.name)
                })
            } else {
                None
            }
        });
        
        match project_name {
            Some(proj_name) => {
                // Insert into hierarchy
                let parts: Vec<&str> = proj_name.split('.').collect();
                insert_into_tree(&mut roots, &parts, duration);
            }
            None => {
                no_project_secs += duration;
            }
        }
    }
    
    // Compute totals for all nodes
    for node in roots.values_mut() {
        node.compute_totals();
    }
    
    (roots, no_project_secs)
}

/// Insert time into the project hierarchy tree
fn insert_into_tree(roots: &mut BTreeMap<String, ProjectNode>, parts: &[&str], duration: i64) {
    if parts.is_empty() {
        return;
    }
    
    let first = parts[0];
    let full_path = parts.join(".");
    
    // Get or create root node
    let root = roots.entry(first.to_string()).or_insert_with(|| {
        ProjectNode::new(first, first)
    });
    
    if parts.len() == 1 {
        // This is the target node - add direct time
        root.direct_secs += duration;
    } else {
        // Recurse into children
        insert_into_children(root, &parts[1..], &full_path, duration);
    }
}

/// Recursively insert into child nodes
fn insert_into_children(parent: &mut ProjectNode, parts: &[&str], full_path: &str, duration: i64) {
    if parts.is_empty() {
        return;
    }
    
    let first = parts[0];
    // Build the full path for this node
    let child_path = if parent.full_path.is_empty() {
        first.to_string()
    } else {
        format!("{}.{}", parent.full_path, first)
    };
    
    let child = parent.children.entry(first.to_string()).or_insert_with(|| {
        ProjectNode::new(first, &child_path)
    });
    
    if parts.len() == 1 {
        // This is the target node
        child.direct_secs += duration;
    } else {
        // Keep going deeper
        insert_into_children(child, &parts[1..], full_path, duration);
    }
}

/// Print project tree recursively with indentation
fn print_project_tree(
    node: &ProjectNode,
    depth: usize,
    total_secs: i64,
    project_width: usize,
) {
    let indent = "  ".repeat(depth);
    let name_display = format!("{}{}", indent, node.name);
    let time_str = format_duration_hm(node.total_secs);
    let pct_str = format_percentage(node.total_secs, total_secs);
    
    println!("{:<width$} {:>12} {:>8}", name_display, time_str, pct_str, width = project_width);
    
    for child in node.children.values() {
        print_project_tree(child, depth + 1, total_secs, project_width);
    }
}

/// Handle the sessions report command
pub fn handle_sessions_report(start: Option<String>, end: Option<String>) -> Result<()> {
    let conn = DbConnection::connect()?;
    let now = chrono::Utc::now().timestamp();
    
    // Parse start date
    let period_start = if let Some(start_expr) = start {
        parse_date_expr(&start_expr)
            .context(format!("Invalid start date: {}", start_expr))?
    } else {
        // Find earliest session
        let sessions = SessionRepo::list_all(&conn)?;
        sessions.iter().map(|s| s.start_ts).min().unwrap_or(now)
    };
    
    // Parse end date
    let period_end = if let Some(end_expr) = end {
        // Parse the date and use end of day
        let ts = parse_date_expr(&end_expr)
            .context(format!("Invalid end date: {}", end_expr))?;
        // Add 24 hours minus 1 second to include the full day
        ts + 86400 - 1
    } else {
        now
    };
    
    // Get all sessions that overlap with the period
    let all_sessions = SessionRepo::list_all(&conn)?;
    let sessions: Vec<_> = all_sessions.into_iter()
        .filter(|s| {
            let session_end = s.end_ts.unwrap_or(now);
            // Session overlaps if it starts before period ends and ends after period starts
            s.start_ts < period_end && session_end > period_start
        })
        .collect();
    
    if sessions.is_empty() {
        println!("No sessions found for this period.");
        return Ok(());
    }
    
    // Build project hierarchy tree
    let (roots, no_project_secs) = build_project_tree(&conn, &sessions, period_start, period_end);
    
    // Calculate grand total
    let project_total: i64 = roots.values().map(|n| n.total_secs).sum();
    let grand_total = project_total + no_project_secs;
    
    // Format date range
    let start_date = Local.timestamp_opt(period_start, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default();
    let end_date = Local.timestamp_opt(period_end, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default();
    
    let period_days = ((period_end - period_start) / 86400).max(1);
    
    // Print report
    let width = 80;
    let project_width = 40;
    
    println!();
    println!("Time Report: {} to {}", start_date, end_date);
    println!("{}", "=".repeat(width));
    println!();
    println!("{:<width$} {:>12} {:>8}", "Project", "Time", "%", width = project_width);
    println!("{}", "-".repeat(width));
    
    // Print project hierarchy
    for node in roots.values() {
        print_project_tree(node, 0, grand_total, project_width);
    }
    
    // Print no-project time if any
    if no_project_secs > 0 {
        let time_str = format_duration_hm(no_project_secs);
        let pct_str = format_percentage(no_project_secs, grand_total);
        println!("{:<width$} {:>12} {:>8}", "(no project)", time_str, pct_str, width = project_width);
    }
    
    println!("{}", "-".repeat(width));
    
    // Print grand total
    let total_time_str = format_duration_hm(grand_total);
    println!("{:<width$} {:>12} {:>8}", "TOTAL", total_time_str, "100.0%", width = project_width);
    println!();
    println!("Sessions: {} | Period: {} days", sessions.len(), period_days);
    println!();
    
    Ok(())
}

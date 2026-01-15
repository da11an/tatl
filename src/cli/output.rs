// Output formatting utilities

use crate::models::{Task, TaskStatus};
use crate::repo::{ProjectRepo, AnnotationRepo, SessionRepo, StackRepo, TaskRepo};
use crate::cli::priority::calculate_priority;
use chrono::Local;
use rusqlite::Connection;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::cmp::Ordering;

/// Kanban status values (derived from task state)
/// 
/// | Kanban    | Status    | Clock stack      | Sessions list                  | Clock status |
/// | --------- | --------- | ---------------- | ------------------------------ | ------------ |
/// | proposed  | pending   | Not in stack     | Task id not in sessions list   | N/A          |
/// | paused    | pending   | Not in stack     | Task id in sessions list       | N/A          |
/// | queued    | pending   | Position > 0     | Task id not in sessions list   | N/A          |
/// | working   | pending   | Position > 0     | Task id in sessions list       | N/A          |
/// | NEXT      | pending   | Position = 0     | N/A                            | Out          |
/// | LIVE      | pending   | Position = 0     | (Task id in sessions list)     | In           |
/// | done      | completed | (ineligible)     | N/A                            | N/A          |
pub fn calculate_kanban_status(
    task: &Task,
    stack_position: Option<usize>,
    has_sessions: bool,
    open_session_task_id: Option<i64>,
    stack_top_task_id: Option<i64>,
) -> &'static str {
    // Completed/closed tasks are "done"
    if task.status == TaskStatus::Completed || task.status == TaskStatus::Closed {
        return "done";
    }
    
    // At this point, task is pending (or other non-terminal status)
    match stack_position {
        Some(0) => {
            // Position 0 = top of stack
            if open_session_task_id == task.id {
                "LIVE"
            } else {
                "NEXT"
            }
        }
        Some(1) => {
            // Position 1 = NEXT if position 0 is LIVE
            if open_session_task_id.is_some() && stack_top_task_id == open_session_task_id {
                "NEXT"
            } else if has_sessions {
                "working"
            } else {
                "queued"
            }
        }
        Some(_pos) => {
            // Position > 1 = in stack but not at top
            if has_sessions {
                "working"
            } else {
                "queued"
            }
        }
        None => {
            // Not in stack
            if has_sessions {
                "paused"
            } else {
                "proposed"
            }
        }
    }
}

/// Get stack positions for all task IDs as a map (task_id -> position)
fn get_stack_positions(conn: &Connection) -> Result<HashMap<i64, usize>> {
    let stack = StackRepo::get_or_create_default(conn)?;
    let items = StackRepo::get_items(conn, stack.id.unwrap())?;
    
    let mut positions = HashMap::new();
    for (idx, item) in items.iter().enumerate() {
        positions.insert(item.task_id, idx);
    }
    
    Ok(positions)
}

/// Get set of task IDs that have any sessions
fn get_tasks_with_sessions(conn: &Connection) -> Result<HashSet<i64>> {
    let all_sessions = SessionRepo::list_all(conn)?;
    let task_ids: HashSet<i64> = all_sessions.iter().map(|s| s.task_id).collect();
    Ok(task_ids)
}

/// Format timestamp for display
pub fn format_timestamp(ts: i64) -> String {
    use chrono::TimeZone;
    let dt = Local.timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| Local.timestamp_opt(0, 0).single().unwrap());
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Format date for display (date only, no time)
pub fn format_date(ts: i64) -> String {
    use chrono::TimeZone;
    let dt = Local.timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| Local.timestamp_opt(0, 0).single().unwrap());
    dt.format("%Y-%m-%d").to_string()
}

/// Format date as relative time (e.g., "2 days ago", "in 3 days", "today", "overdue")
pub fn format_relative_date(ts: i64) -> String {
    use chrono::{Local, TimeZone, Datelike};
    let now = Local::now();
    let due_dt = Local.timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| Local.timestamp_opt(0, 0).single().unwrap());
    
    let today = now.date_naive();
    let due_date = due_dt.date_naive();
    let days_diff = (due_date - today).num_days();
    
    if days_diff < 0 {
        // Past date
        if days_diff >= -30 {
            // Within last 30 days - show "X days ago"
            let days = (-days_diff) as u32;
            if days == 1 {
                "overdue".to_string()
            } else {
                format!("{} days ago", days)
            }
        } else {
            // More than 30 days ago - show "overdue"
            "overdue".to_string()
        }
    } else if days_diff == 0 {
        "today".to_string()
    } else if days_diff == 1 {
        "tomorrow".to_string()
    } else if days_diff <= 365 {
        // Within a year - show "in X days"
        format!("in {} days", days_diff)
    } else {
        // More than a year in future - show absolute date
        format_date(ts)
    }
}

/// Format duration for display
pub fn format_duration(secs: i64) -> String {
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

#[derive(Debug, Clone, Default)]
pub struct TaskListOptions {
    pub use_relative_time: bool,
    pub sort_columns: Vec<String>,
    pub group_columns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TaskListColumn {
    Id,
    Description,
    Kanban,
    Project,
    Tags,
    Due,
    Alloc,
    Priority,
    Clock,
    Status,
}

#[derive(Debug, Clone)]
enum SortValue {
    Int(i64),
    Float(f64),
    Str(String),
}

#[derive(Debug, Clone)]
struct TaskRow {
    task: Task,
    tags: Vec<String>,
    values: HashMap<TaskListColumn, String>,
    sort_values: HashMap<TaskListColumn, Option<SortValue>>,
}

fn parse_task_column(name: &str) -> Option<TaskListColumn> {
    match name.to_lowercase().as_str() {
        "id" => Some(TaskListColumn::Id),
        "description" | "desc" => Some(TaskListColumn::Description),
        "kanban" => Some(TaskListColumn::Kanban),
        "project" | "proj" => Some(TaskListColumn::Project),
        "tags" | "tag" => Some(TaskListColumn::Tags),
        "due" => Some(TaskListColumn::Due),
        "alloc" | "allocation" => Some(TaskListColumn::Alloc),
        "priority" | "prio" | "pri" => Some(TaskListColumn::Priority),
        "clock" => Some(TaskListColumn::Clock),
        "status" => Some(TaskListColumn::Status),
        _ => None,
    }
}

fn column_label(column: TaskListColumn) -> &'static str {
    match column {
        TaskListColumn::Id => "ID",
        TaskListColumn::Description => "Description",
        TaskListColumn::Kanban => "Kanban",
        TaskListColumn::Project => "Project",
        TaskListColumn::Tags => "Tags",
        TaskListColumn::Due => "Due",
        TaskListColumn::Alloc => "Alloc",
        TaskListColumn::Priority => "Priority",
        TaskListColumn::Clock => "Clock",
        TaskListColumn::Status => "Status",
    }
}

fn compare_sort_values(a: &Option<SortValue>, b: &Option<SortValue>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => match (a, b) {
            (SortValue::Int(a), SortValue::Int(b)) => a.cmp(b),
            (SortValue::Float(a), SortValue::Float(b)) => a
                .partial_cmp(b)
                .unwrap_or(Ordering::Equal),
            (SortValue::Str(a), SortValue::Str(b)) => a.cmp(b),
            _ => sort_value_as_string(a).cmp(&sort_value_as_string(b)),
        },
    }
}

fn sort_value_as_string(value: &SortValue) -> String {
    match value {
        SortValue::Int(v) => v.to_string(),
        SortValue::Float(v) => format!("{:.6}", v),
        SortValue::Str(v) => v.clone(),
    }
}

/// Format task list as a table
pub fn format_task_list_table(
    conn: &Connection,
    tasks: &[(Task, Vec<String>)],
    options: &TaskListOptions,
) -> Result<String> {
    if tasks.is_empty() {
        return Ok("No tasks found.".to_string());
    }
    
    // Pre-compute kanban-related data for all tasks (batch queries for performance)
    let stack_positions = get_stack_positions(conn)?;
    let stack = StackRepo::get_or_create_default(conn)?;
    let stack_items = StackRepo::get_items(conn, stack.id.unwrap())?;
    let stack_top_task_id = stack_items.first().map(|item| item.task_id);
    let tasks_with_sessions = get_tasks_with_sessions(conn)?;
    let open_session_task_id = SessionRepo::get_open(conn)?.map(|s| s.task_id);
    
    let mut rows: Vec<TaskRow> = Vec::new();
    for (task, tags) in tasks {
        let task_id = task.id.unwrap_or(0);
        let stack_pos = stack_positions.get(&task_id).copied();
        let has_sessions = tasks_with_sessions.contains(&task_id);
        let kanban = calculate_kanban_status(
            task,
            stack_pos,
            has_sessions,
            open_session_task_id,
            stack_top_task_id,
        );
        
        let project = if let Some(project_id) = task.project_id {
            if let Ok(Some(proj)) = ProjectRepo::get_by_id(conn, project_id) {
                proj.name
            } else {
                format!("[{}]", project_id)
            }
        } else {
            String::new()
        };
        
        let tag_str = if !tags.is_empty() {
            tags.iter().map(|t| format!("+{}", t)).collect::<Vec<_>>().join(" ")
        } else {
            String::new()
        };
        
        let due = if let Some(due_ts) = task.due_ts {
            if options.use_relative_time {
                format_relative_date(due_ts)
            } else {
                format_date(due_ts)
            }
        } else {
            String::new()
        };
        
        let alloc = if let Some(alloc_secs) = task.alloc_secs {
            format_duration(alloc_secs)
        } else {
            String::new()
        };
        
        let clock = if let Some(task_id) = task.id {
            if let Ok(total_logged) = TaskRepo::get_total_logged_time(conn, task_id) {
                if total_logged > 0 {
                    format_duration(total_logged)
                } else {
                    "0s".to_string()
                }
            } else {
                "0s".to_string()
            }
        } else {
            "0s".to_string()
        };
        
        let priority = if task.status == TaskStatus::Pending {
            if let Ok(prio) = calculate_priority(task, conn) {
                format!("{:.1}", prio)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        let mut values = HashMap::new();
        values.insert(TaskListColumn::Id, task.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()));
        values.insert(TaskListColumn::Description, task.description.clone());
        values.insert(TaskListColumn::Kanban, kanban.to_string());
        values.insert(TaskListColumn::Project, project.clone());
        values.insert(TaskListColumn::Tags, tag_str.clone());
        values.insert(TaskListColumn::Due, due.clone());
        values.insert(TaskListColumn::Alloc, alloc.clone());
        values.insert(TaskListColumn::Priority, priority.clone());
        values.insert(TaskListColumn::Clock, clock.clone());
        values.insert(TaskListColumn::Status, task.status.as_str().to_string());
        
        let mut sort_values = HashMap::new();
        sort_values.insert(TaskListColumn::Id, task.id.map(SortValue::Int));
        sort_values.insert(TaskListColumn::Description, Some(SortValue::Str(task.description.clone())));
        sort_values.insert(TaskListColumn::Kanban, Some(SortValue::Str(kanban.to_string())));
        sort_values.insert(TaskListColumn::Project, Some(SortValue::Str(project)));
        sort_values.insert(TaskListColumn::Tags, Some(SortValue::Str(tag_str)));
        sort_values.insert(TaskListColumn::Due, task.due_ts.map(SortValue::Int));
        sort_values.insert(TaskListColumn::Alloc, task.alloc_secs.map(SortValue::Int));
        sort_values.insert(TaskListColumn::Priority, if task.status == TaskStatus::Pending {
            calculate_priority(task, conn).ok().map(SortValue::Float)
        } else {
            None
        });
        sort_values.insert(TaskListColumn::Clock, if let Some(task_id) = task.id {
            TaskRepo::get_total_logged_time(conn, task_id).ok().map(SortValue::Int)
        } else {
            None
        });
        sort_values.insert(TaskListColumn::Status, Some(SortValue::Str(task.status.as_str().to_string())));
        
        rows.push(TaskRow {
            task: task.clone(),
            tags: tags.clone(),
            values,
            sort_values,
        });
    }
    
    // Build column order
    let mut columns: Vec<TaskListColumn> = Vec::new();
    for col in &options.sort_columns {
        let column = parse_task_column(col)
            .ok_or_else(|| anyhow::anyhow!("Unknown sort column: {}", col))?;
        if !columns.contains(&column) {
            columns.push(column);
        }
    }
    for column in [TaskListColumn::Id, TaskListColumn::Description] {
        if !columns.contains(&column) {
            columns.push(column);
        }
    }
    for col in &options.group_columns {
        let column = parse_task_column(col)
            .ok_or_else(|| anyhow::anyhow!("Unknown group column: {}", col))?;
        if !columns.contains(&column) {
            columns.push(column);
        }
    }
    
    let default_columns = [
        TaskListColumn::Kanban,
        TaskListColumn::Project,
        TaskListColumn::Tags,
        TaskListColumn::Due,
        TaskListColumn::Alloc,
        TaskListColumn::Priority,
        TaskListColumn::Clock,
    ];
    for column in default_columns {
        if !columns.contains(&column) {
            columns.push(column);
        }
    }
    
    // Calculate column widths
    let mut column_widths: HashMap<TaskListColumn, usize> = HashMap::new();
    for column in &columns {
        column_widths.insert(*column, column_label(*column).len().max(4));
    }
    
    for row in &rows {
        for column in &columns {
            if let Some(value) = row.values.get(column) {
                let max_len = if *column == TaskListColumn::Description {
                    value.len().min(50)
                } else {
                    value.len()
                };
                let entry = column_widths.entry(*column).or_insert(4);
                *entry = (*entry).max(max_len);
            }
        }
    }
    
    // Build header
    let mut output = String::new();
    for (idx, column) in columns.iter().enumerate() {
        let width = *column_widths.get(column).unwrap_or(&4);
        if idx == columns.len() - 1 {
            output.push_str(&format!("{:<width$}\n", column_label(*column), width = width));
        } else {
            output.push_str(&format!("{:<width$} ", column_label(*column), width = width));
        }
    }
    
    // Separator line
    let total_width: usize = columns.iter()
        .map(|col| column_widths.get(col).copied().unwrap_or(4))
        .sum::<usize>() + (columns.len().saturating_sub(1));
    output.push_str(&format!("{}\n", "-".repeat(total_width)));
    
    // Apply sorting (ensure grouped rows are contiguous by sorting on group columns first)
    let mut effective_sort_columns = options.group_columns.clone();
    for sort_col in &options.sort_columns {
        if !effective_sort_columns.iter().any(|c| c.eq_ignore_ascii_case(sort_col)) {
            effective_sort_columns.push(sort_col.clone());
        }
    }
    if !effective_sort_columns.is_empty() {
        let group_columns_parsed: Vec<TaskListColumn> = options.group_columns.iter()
            .filter_map(|name| parse_task_column(name))
            .collect();
        rows.sort_by(|a, b| {
            if !group_columns_parsed.is_empty() {
                let a_key: Vec<String> = group_columns_parsed.iter()
                    .map(|column| normalize_group_value(*column, a.values.get(column).map(String::as_str).unwrap_or_default()))
                    .collect();
                let b_key: Vec<String> = group_columns_parsed.iter()
                    .map(|column| normalize_group_value(*column, b.values.get(column).map(String::as_str).unwrap_or_default()))
                    .collect();
                if a_key != b_key {
                    return a_key.cmp(&b_key);
                }
            }
            for col_name in &effective_sort_columns {
                if let Some(column) = parse_task_column(col_name) {
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
    
    // Build rows with optional grouping
    if options.group_columns.is_empty() {
        for row in &rows {
            for (idx, column) in columns.iter().enumerate() {
                let width = *column_widths.get(column).unwrap_or(&4);
                let raw_value = row.values.get(column).cloned().unwrap_or_default();
                let value = if *column == TaskListColumn::Description && raw_value.len() > width {
                    format!("{}..", &raw_value[..width.saturating_sub(2)])
                } else if raw_value.len() > width {
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
        let group_columns_parsed: Vec<TaskListColumn> = options.group_columns.iter()
            .filter_map(|name| parse_task_column(name))
            .collect();
        let mut groups: Vec<(Vec<String>, Vec<&TaskRow>)> = Vec::new();
        let mut group_index: HashMap<String, usize> = HashMap::new();
        for row in &rows {
            let group_values: Vec<String> = group_columns_parsed.iter()
                .map(|column| {
                    let value = row.values.get(column).cloned().unwrap_or_default();
                    normalize_group_value(*column, &value)
                })
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
            
            // Embed group label at the end of the divider line
            let dash_count = total_width.saturating_sub(group_label.len());
            output.push_str(&format!("{}{}\n", "-".repeat(dash_count), group_label));
            
            for row in group_rows {
                for (idx, column) in columns.iter().enumerate() {
                    let width = *column_widths.get(column).unwrap_or(&4);
                    let raw_value = row.values.get(column).cloned().unwrap_or_default();
                    let value = if *column == TaskListColumn::Description && raw_value.len() > width {
                        format!("{}..", &raw_value[..width.saturating_sub(2)])
                    } else if raw_value.len() > width {
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
    
    Ok(output)
}

fn normalize_group_value(column: TaskListColumn, value: &str) -> String {
    let trimmed = value.trim();
    match column {
        TaskListColumn::Status | TaskListColumn::Kanban => trimmed.to_lowercase(),
        _ => trimmed.to_string(),
    }
}

/// Format stack display
pub fn format_stack_display(items: &[(i64, i32)]) -> String {
    if items.is_empty() {
        return "Stack is empty.".to_string();
    }
    
    let mut output = String::new();
    output.push_str("Stack:\n");
    
    for (idx, (task_id, _ordinal)) in items.iter().enumerate() {
        output.push_str(&format!("  [{}] Task {}\n", idx, task_id));
    }
    
    output
}

/// Format clock list as a table with position and full task details
pub fn format_clock_list_table(
    conn: &Connection,
    clock_tasks: &[(usize, Task, Vec<String>)],
) -> Result<String> {
    if clock_tasks.is_empty() {
        return Ok("Clock stack is empty.".to_string());
    }
    
    // Calculate column widths
    let mut pos_width = 6; // "Pos" header
    let mut id_width = 4;
    let mut desc_width = 20;
    let mut status_width = 10;
    let mut project_width = 15;
    let mut tags_width = 20;
    let mut due_width = 12;
    
    // First pass: calculate widths
    for (position, task, tags) in clock_tasks {
        pos_width = pos_width.max(position.to_string().len());
        id_width = id_width.max(task.id.map(|id| id.to_string().len()).unwrap_or(0));
        desc_width = desc_width.max(task.description.len().min(50));
        status_width = status_width.max(task.status.as_str().len());
        
        if let Some(project_id) = task.project_id {
            if let Ok(Some(project)) = ProjectRepo::get_by_id(conn, project_id) {
                project_width = project_width.max(project.name.len().min(15));
            }
        }
        
        if !tags.is_empty() {
            let tag_str = tags.iter().map(|t| format!("+{}", t)).collect::<Vec<_>>().join(" ");
            tags_width = tags_width.max(tag_str.len().min(30));
        }
        
        if task.due_ts.is_some() {
            due_width = due_width.max(12);
        }
    }
    
    // Build header
    let mut output = String::new();
    output.push_str(&format!(
        "{:<pos$} {:<id$} {:<desc$} {:<status$} {:<project$} {:<tags$} {:<due$}\n",
        "Pos", "ID", "Description", "Status", "Project", "Tags", "Due",
        pos = pos_width,
        id = id_width,
        desc = desc_width,
        status = status_width,
        project = project_width,
        tags = tags_width,
        due = due_width
    ));
    
    // Separator line
    let total_width = pos_width + id_width + desc_width + status_width + project_width + tags_width + due_width + 6;
    output.push_str(&format!("{}\n", "-".repeat(total_width)));
    
    // Build rows (already sorted by position)
    for (position, task, tags) in clock_tasks {
        let pos_str = position.to_string();
        let id = task.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string());
        
        let desc = if task.description.len() > desc_width {
            format!("{}..", &task.description[..desc_width.saturating_sub(2)])
        } else {
            task.description.clone()
        };
        
        let status = task.status.as_str();
        
        let project = if let Some(project_id) = task.project_id {
            if let Ok(Some(proj)) = ProjectRepo::get_by_id(conn, project_id) {
                if proj.name.len() > project_width {
                    format!("{}..", &proj.name[..project_width.saturating_sub(2)])
                } else {
                    proj.name
                }
            } else {
                format!("[{}]", project_id)
            }
        } else {
            String::new()
        };
        
        let tag_str = if !tags.is_empty() {
            let full = tags.iter().map(|t| format!("+{}", t)).collect::<Vec<_>>().join(" ");
            if full.len() > tags_width {
                format!("{}..", &full[..tags_width.saturating_sub(2)])
            } else {
                full
            }
        } else {
            String::new()
        };
        
        let due = if let Some(due_ts) = task.due_ts {
            format_date(due_ts)
        } else {
            String::new()
        };
        
        output.push_str(&format!(
            "{:<pos$} {:<id$} {:<desc$} {:<status$} {:<project$} {:<tags$} {:<due$}\n",
            pos_str, id, desc, status, project, tag_str, due,
            pos = pos_width,
            id = id_width,
            desc = desc_width,
            status = status_width,
            project = project_width,
            tags = tags_width,
            due = due_width
        ));
    }
    
    Ok(output)
}

/// Format clock transition message
pub fn format_clock_transition(
    action: &str,
    task_id: Option<i64>,
    task_description: Option<&str>,
) -> String {
    match (action, task_id, task_description) {
        ("started", Some(id), Some(desc)) => {
            format!("Started timing task {}: {}", id, desc)
        }
        ("started", Some(id), None) => {
            format!("Started timing task {}", id)
        }
        ("stopped", Some(id), Some(desc)) => {
            format!("Stopped timing task {}: {}", id, desc)
        }
        ("stopped", Some(id), None) => {
            format!("Stopped timing task {}", id)
        }
        ("switched", Some(old_id), _) => {
            format!("Switched from task {} to task {}", old_id, task_id.unwrap_or(0))
        }
        _ => format!("Clock {}", action)
    }
}

/// Format task summary report
pub fn format_task_summary(
    conn: &Connection,
    task: &crate::models::Task,
    tags: &[String],
    annotations: &[crate::models::Annotation],
    sessions: &[crate::models::Session],
    stack_position: Option<(i32, i32)>, // (position, total)
) -> Result<String> {
    let mut output = String::new();
    
    // Header
    let header = format!("Task {}: {}", 
        task.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()),
        task.description);
    output.push_str(&header);
    output.push_str("\n");
    output.push_str(&"=".repeat(header.len().max(60)));
    output.push_str("\n\n");
    
    // Description
    output.push_str("Description:\n");
    output.push_str(&format!("  {}\n\n", task.description));
    
    // Basic Info
    output.push_str("Status: ");
    output.push_str(task.status.as_str());
    output.push_str("\n");
    output.push_str(&format!("Created: {}\n", format_timestamp(task.created_ts)));
    output.push_str(&format!("Modified: {}\n\n", format_timestamp(task.modified_ts)));
    
    // Attributes
    output.push_str("Attributes:\n");
    
    // Project
    if let Some(project_id) = task.project_id {
        if let Ok(Some(project)) = ProjectRepo::get_by_id(conn, project_id) {
            output.push_str(&format!("  Project:     {}\n", project.name));
        } else {
            output.push_str(&format!("  Project:     [{}]\n", project_id));
        }
    } else {
        output.push_str("  Project:     (none)\n");
    }
    
    // Due
    if let Some(due_ts) = task.due_ts {
        output.push_str(&format!("  Due:         {}\n", format_date(due_ts)));
    } else {
        output.push_str("  Due:         (none)\n");
    }
    
    // Scheduled
    if let Some(scheduled_ts) = task.scheduled_ts {
        output.push_str(&format!("  Scheduled:   {}\n", format_date(scheduled_ts)));
    } else {
        output.push_str("  Scheduled:   (none)\n");
    }
    
    // Wait
    if let Some(wait_ts) = task.wait_ts {
        output.push_str(&format!("  Wait:        {}\n", format_date(wait_ts)));
    } else {
        output.push_str("  Wait:        (none)\n");
    }
    
    // Allocation
    if let Some(alloc_secs) = task.alloc_secs {
        output.push_str(&format!("  Allocation:  {}\n", format_duration(alloc_secs)));
    } else {
        output.push_str("  Allocation:  (none)\n");
    }
    
    // Tags
    if !tags.is_empty() {
        let tag_str = tags.iter().map(|t| format!("+{}", t)).collect::<Vec<_>>().join(" ");
        output.push_str(&format!("  Tags:        {}\n", tag_str));
    } else {
        output.push_str("  Tags:        (none)\n");
    }
    
    // Template
    if let Some(ref template) = task.template {
        output.push_str(&format!("  Template:    {}\n", template));
    } else {
        output.push_str("  Template:    (none)\n");
    }
    
    // Recurrence
    if let Some(ref recur) = task.recur {
        output.push_str(&format!("  Recurrence:  {}\n", recur));
    } else {
        output.push_str("  Recurrence:  none\n");
    }
    
    output.push_str("\n");
    
    // User-Defined Attributes
    if !task.udas.is_empty() {
        output.push_str("User-Defined Attributes:\n");
        let mut udas: Vec<_> = task.udas.iter().collect();
        udas.sort_by_key(|(k, _)| *k);
        for (key, value) in udas {
            output.push_str(&format!("  {}:    {}\n", key, value));
        }
        output.push_str("\n");
    }
    
    // Stack
    if let Some((position, total)) = stack_position {
        output.push_str("Stack:\n");
        output.push_str(&format!("  Position:    {} of {}\n\n", position + 1, total));
    }
    
    // Recurrence details (if recurring)
    if task.recur.is_some() {
        output.push_str("Recurrence:\n");
        output.push_str(&format!("  Type:        {}\n", task.recur.as_ref().unwrap()));
        // TODO: Add more recurrence details if needed (next occurrence, etc.)
        output.push_str("\n");
    }
    
    // Annotations
    output.push_str(&format!("Annotations ({}):\n", annotations.len()));
    if annotations.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for (idx, annotation) in annotations.iter().enumerate() {
            output.push_str(&format!("  {}. {}\n", idx + 1, format_timestamp(annotation.entry_ts)));
            // Format note with indentation for multi-line notes
            for line in annotation.note.lines() {
                output.push_str(&format!("     {}\n", line));
            }
            output.push_str("\n");
        }
    }
    output.push_str("\n");
    
    // Sessions
    output.push_str(&format!("Sessions ({}):\n", sessions.len()));
    if sessions.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for (idx, session) in sessions.iter().enumerate() {
            output.push_str(&format!("  {}. {} - ", idx + 1, format_timestamp(session.start_ts)));
            if let Some(end_ts) = session.end_ts {
                output.push_str(&format!("{}", format_timestamp(end_ts)));
                if let Some(duration) = session.duration_secs() {
                    output.push_str(&format!(" ({})", format_duration(duration)));
                }
            } else {
                output.push_str("(running)");
                let current_duration = chrono::Utc::now().timestamp() - session.start_ts;
                output.push_str(&format!(" ({})", format_duration(current_duration)));
            }
            output.push_str("\n");
        }
    }
    output.push_str("\n");
    
    // Total Time
    let total_secs: i64 = sessions.iter()
        .filter_map(|s| s.duration_secs())
        .sum();
    output.push_str(&format!("Total Time: {}\n", format_duration(total_secs)));
    
    Ok(output)
}

/// Format dashboard output
pub fn format_dashboard(
    conn: &Connection,
    clock_state: Option<(i64, i64)>,
    clock_stack_tasks: &[(usize, Task, Vec<String>)],
    priority_tasks: &[(Task, Vec<String>, f64)],
    today_session_count: usize,
    today_duration: i64,
    overdue_count: usize,
    next_overdue_ts: Option<i64>,
) -> Result<String> {
    let mut output = String::new();
    
    // Clock Status Section
    output.push_str("=== Clock Status ===\n");
    if let Some((task_id, duration)) = clock_state {
        let task_desc = TaskRepo::get_by_id(conn, task_id)
            .ok()
            .flatten()
            .map(|t| t.description)
            .unwrap_or_else(|| "".to_string());
        let desc_str = if task_desc.is_empty() {
            "".to_string()
        } else {
            format!(": {}", task_desc)
        };
        if duration > 0 {
            output.push_str(&format!(
                "Clocked IN on task {}{} ({})\n",
                task_id,
                desc_str,
                format_duration(duration)
            ));
        } else {
            output.push_str(&format!(
                "Clocked OUT (task {}{} in stack)\n",
                task_id,
                desc_str
            ));
        }
    } else {
        output.push_str("Clocked OUT (no task in stack)\n");
    }
    output.push_str("\n");
    
    // Clock Stack Section (top 3)
    output.push_str("=== Clock Stack (Top 3) ===\n");
    if clock_stack_tasks.is_empty() {
        output.push_str("Stack is empty.\n");
    } else {
        for (idx, task, tags) in clock_stack_tasks {
            let project_name = if let Some(project_id) = task.project_id {
                ProjectRepo::get_by_id(conn, project_id)
                    .ok()
                    .flatten()
                    .map(|p| p.name)
                    .unwrap_or_else(|| "?".to_string())
            } else {
                "".to_string()
            };
            
            let project_str = if project_name.is_empty() {
                "".to_string()
            } else {
                format!(" project:{}", project_name)
            };
            
            let tags_str = if tags.is_empty() {
                "".to_string()
            } else {
                format!(" {}", tags.iter().map(|t| format!("+{}", t)).collect::<Vec<_>>().join(" "))
            };
            
            let due_str = if let Some(due_ts) = task.due_ts {
                format!(" due:{}", format_date(due_ts))
            } else {
                "".to_string()
            };
            
            let alloc_str = if let Some(alloc) = task.alloc_secs {
                format!(" alloc:{}", format_duration(alloc))
            } else {
                "".to_string()
            };
            
            output.push_str(&format!(
                "[{}] {}: {}{}{}{}{}\n",
                idx, task.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()),
                task.description,
                project_str,
                tags_str,
                due_str,
                alloc_str,
            ));
        }
    }
    output.push_str("\n");
    
    // Priority Tasks Section (top 3 NOT in clock stack)
    output.push_str("=== Priority Tasks (Top 3) ===\n");
    if priority_tasks.is_empty() {
        output.push_str("No priority tasks (all tasks are in clock stack or completed).\n");
    } else {
        for (task, tags, priority) in priority_tasks {
            let project_name = if let Some(project_id) = task.project_id {
                ProjectRepo::get_by_id(conn, project_id)
                    .ok()
                    .flatten()
                    .map(|p| p.name)
                    .unwrap_or_else(|| "?".to_string())
            } else {
                "".to_string()
            };
            
            let project_str = if project_name.is_empty() {
                "".to_string()
            } else {
                format!(" project:{}", project_name)
            };
            
            let tags_str = if tags.is_empty() {
                "".to_string()
            } else {
                format!(" {}", tags.iter().map(|t| format!("+{}", t)).collect::<Vec<_>>().join(" "))
            };
            
            let due_str = if let Some(due_ts) = task.due_ts {
                format!(" due:{}", format_date(due_ts))
            } else {
                "".to_string()
            };
            
            let alloc_str = if let Some(alloc) = task.alloc_secs {
                format!(" alloc:{}", format_duration(alloc))
            } else {
                "".to_string()
            };
            
            output.push_str(&format!(
                "{}: {}{}{}{}{} (priority: {:.1})\n",
                task.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()),
                task.description,
                project_str,
                tags_str,
                due_str,
                alloc_str,
                priority,
            ));
        }
    }
    output.push_str("\n");
    
    // Today's Sessions Section
    output.push_str("=== Today's Sessions ===\n");
    output.push_str(&format!("{} session(s), {}\n", today_session_count, format_duration(today_duration)));
    output.push_str("\n");
    
    // Overdue Tasks Section
    output.push_str("=== Overdue Tasks ===\n");
    if overdue_count > 0 {
        output.push_str(&format!("{} task(s) overdue\n", overdue_count));
    } else if let Some(next_ts) = next_overdue_ts {
        output.push_str(&format!("No overdue tasks. Next due: {}\n", format_date(next_ts)));
    } else {
        output.push_str("No overdue tasks. No tasks with due dates.\n");
    }
    
    Ok(output)
}

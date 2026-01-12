// Output formatting utilities

use crate::models::Task;
use crate::repo::{ProjectRepo, AnnotationRepo, SessionRepo, StackRepo, TaskRepo};
use chrono::Local;
use rusqlite::Connection;
use anyhow::Result;

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

/// Format task list as a table
pub fn format_task_list_table(
    conn: &Connection,
    tasks: &[(Task, Vec<String>)],
) -> Result<String> {
    if tasks.is_empty() {
        return Ok("No tasks found.".to_string());
    }
    
    // Calculate column widths
    let mut id_width = 4;
    let mut desc_width = 20;
    let mut status_width = 10;
    let mut project_width = 15;
    let mut tags_width = 20;
    let mut due_width = 12;
    
    // First pass: calculate widths
    for (task, tags) in tasks {
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
        "{:<id$} {:<desc$} {:<status$} {:<project$} {:<tags$} {:<due$}\n",
        "ID", "Description", "Status", "Project", "Tags", "Due",
        id = id_width,
        desc = desc_width,
        status = status_width,
        project = project_width,
        tags = tags_width,
        due = due_width
    ));
    
    // Separator line
    let total_width = id_width + desc_width + status_width + project_width + tags_width + due_width + 5;
    output.push_str(&format!("{}\n", "-".repeat(total_width)));
    
    // Build rows
    for (task, tags) in tasks {
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
            "{:<id$} {:<desc$} {:<status$} {:<project$} {:<tags$} {:<due$}\n",
            id, desc, status, project, tag_str, due,
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

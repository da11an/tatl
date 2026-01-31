// Priority/Urgency calculation modeled on Taskwarrior

use crate::models::Task;
use crate::repo::TaskRepo;
use rusqlite::Connection;
use anyhow::Result;
use chrono::Utc;

/// Calculate priority/urgency score for a task (Taskwarrior-style)
/// 
/// Priority is calculated using a polynomial with configurable coefficients:
/// - Due date proximity (higher urgency for tasks due soon or overdue)
/// - Allocation (tasks with less allocation remaining get higher urgency)
/// - Age (older tasks get slightly higher urgency)
/// - Status (open tasks only)
///
/// Returns a floating-point urgency score (higher = more urgent)
pub fn calculate_priority(task: &Task, conn: &Connection) -> Result<f64> {
    let mut urgency = 0.0;
    let now = Utc::now().timestamp();

    // Base urgency for open tasks
    if task.status == crate::models::TaskStatus::Open {
        urgency += 1.0;
    }
    
    // Due date urgency (Taskwarrior-style)
    if let Some(due_ts) = task.due_ts {
        let days_until_due = (due_ts - now) as f64 / 86400.0;
        
        if days_until_due < 0.0 {
            // Overdue - high urgency that increases with lateness
            // Formula: 15.0 - (days overdue * 0.5), minimum 1.0
            urgency += (15.0 - (days_until_due.abs() * 0.5)).max(1.0);
        } else if days_until_due <= 7.0 {
            // Due within a week - urgency increases as deadline approaches
            // Formula: 12.0 - days_until_due
            urgency += (12.0 - days_until_due).max(1.0);
        } else if days_until_due <= 30.0 {
            // Due within a month - moderate urgency
            // Formula: 5.0 - (days_until_due / 10.0)
            urgency += (5.0 - (days_until_due / 10.0)).max(0.5);
        } else {
            // Due far in the future - low urgency
            // Formula: 2.0 / (1.0 + days_until_due / 30.0)
            urgency += 2.0 / (1.0 + days_until_due / 30.0);
        }
    }
    
    // Allocation urgency (tasks with less time remaining get higher urgency)
    if let Some(alloc_secs) = task.alloc_secs {
        if alloc_secs > 0 {
            // Get total time logged for this task
            if let Some(task_id) = task.id {
                if let Ok(total_logged) = TaskRepo::get_total_logged_time(conn, task_id) {
                    let remaining_secs = alloc_secs.saturating_sub(total_logged);
                    
                    if remaining_secs < alloc_secs / 4 {
                        // Less than 25% allocation remaining - high urgency
                        urgency += 3.0;
                    } else if remaining_secs < alloc_secs / 2 {
                        // Less than 50% allocation remaining - moderate urgency
                        urgency += 1.5;
                    } else {
                        // More than 50% remaining - low urgency
                        urgency += 0.5;
                    }
                }
            }
        }
    }
    
    // Age urgency (older tasks get slightly higher urgency)
    let age_days = (now - task.created_ts) as f64 / 86400.0;
    if age_days > 30.0 {
        // Tasks older than 30 days get a small boost
        urgency += (age_days / 30.0).min(2.0) * 0.1;
    }
    
    Ok(urgency)
}

/// Get top N priority tasks (not in clock stack)
pub fn get_top_priority_tasks(
    conn: &Connection,
    exclude_task_ids: &[i64],
    limit: usize,
) -> Result<Vec<(Task, Vec<String>, f64)>> {
    let all_tasks = TaskRepo::list_all(conn)?;
    
    // Filter out excluded tasks and calculate priority
    let mut tasks_with_priority: Vec<(Task, Vec<String>, f64)> = all_tasks
        .into_iter()
        .filter(|(task, _)| {
            !exclude_task_ids.contains(&task.id.unwrap_or(0)) &&
            task.status == crate::models::TaskStatus::Open
        })
        .filter_map(|(task, tags)| {
            if let Ok(priority) = calculate_priority(&task, conn) {
                Some((task, tags, priority))
            } else {
                None
            }
        })
        .collect();
    
    // Sort by priority (descending) and take top N
    tasks_with_priority.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    tasks_with_priority.truncate(limit);
    
    Ok(tasks_with_priority)
}

//! Respawn logic for creating new task instances on completion
//!
//! When a task with a respawn rule is finished or closed, this module
//! calculates the next due date and creates a new task instance.

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Timelike, Utc, Weekday};
use rusqlite::Connection;
use anyhow::Result;
use crate::respawn::parser::{RespawnRule, RespawnPattern};
use crate::repo::TaskRepo;
use crate::models::Task;

/// Calculate the next occurrence timestamp from a given completion time
///
/// # Arguments
/// * `rule` - The respawn rule
/// * `from_ts` - Unix timestamp (UTC) of when the task was completed
/// * `original_due_ts` - Original due timestamp (used to preserve time-of-day)
///
/// # Returns
/// The next occurrence timestamp, or None if invalid
pub fn next_occurrence(rule: &RespawnRule, from_ts: i64, original_due_ts: Option<i64>) -> Option<i64> {
    let from_dt = DateTime::<Utc>::from_timestamp(from_ts, 0)?;
    
    // Extract time-of-day from original due date, or use midnight
    let (hour, minute, second) = if let Some(due_ts) = original_due_ts {
        let due_dt = DateTime::<Utc>::from_timestamp(due_ts, 0)?;
        (due_dt.hour(), due_dt.minute(), due_dt.second())
    } else {
        (0, 0, 0)
    };
    
    let next_date = match &rule.pattern {
        RespawnPattern::Daily => {
            // Next day at the same time
            let next = from_dt.date_naive() + Duration::days(1);
            next
        }
        RespawnPattern::Weekly => {
            // Same weekday next week
            let next = from_dt.date_naive() + Duration::weeks(1);
            next
        }
        RespawnPattern::Monthly => {
            // Same day next month
            next_month_same_day(from_dt.date_naive())
        }
        RespawnPattern::Yearly => {
            // Same date next year
            next_year_same_date(from_dt.date_naive())
        }
        RespawnPattern::EveryDays(n) => {
            let next = from_dt.date_naive() + Duration::days(*n as i64);
            next
        }
        RespawnPattern::EveryWeeks(n) => {
            let next = from_dt.date_naive() + Duration::weeks(*n as i64);
            next
        }
        RespawnPattern::EveryMonths(n) => {
            add_months(from_dt.date_naive(), *n)
        }
        RespawnPattern::EveryYears(n) => {
            add_years(from_dt.date_naive(), *n)
        }
        RespawnPattern::Weekdays(weekdays) => {
            // Find the next matching weekday after from_ts
            next_matching_weekday(from_dt.date_naive(), weekdays)
        }
        RespawnPattern::Monthdays(days) => {
            // Find the next matching day of month after from_ts
            next_matching_monthday(from_dt.date_naive(), days)
        }
        RespawnPattern::NthWeekday { nth, weekday } => {
            // Find the next Nth weekday of a month
            next_nth_weekday(from_dt.date_naive(), *nth, *weekday)
        }
    };
    
    // Combine date with preserved time
    let next_dt = next_date.and_hms_opt(hour, minute, second)?;
    let next_utc = Utc.from_utc_datetime(&next_dt);
    
    Some(next_utc.timestamp())
}

/// Respawn a task after completion
///
/// Creates a new task instance with the respawn rule, updated due date,
/// and all other attributes carried forward.
///
/// # Arguments
/// * `conn` - Database connection
/// * `task` - The completed task
/// * `completion_ts` - When the task was completed
///
/// # Returns
/// The new task ID if respawned, or None if the task has no respawn rule
pub fn respawn_task(conn: &Connection, task: &Task, completion_ts: i64) -> Result<Option<i64>> {
    // Check if task has a respawn rule
    let respawn_str = match &task.respawn {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(None),
    };
    
    // Parse the respawn rule
    let rule = RespawnRule::parse(respawn_str)?;
    
    // Calculate next due date
    let next_due_ts = next_occurrence(&rule, completion_ts, task.due_ts);
    
    // Get task tags
    let task_id = task.id.ok_or_else(|| anyhow::anyhow!("Task has no ID"))?;
    let tags = TaskRepo::get_tags(conn, task_id)?;
    
    // Create new task instance with carried-forward attributes
    let new_task = TaskRepo::create_full(
        conn,
        &task.description,
        task.project_id,
        next_due_ts,
        task.scheduled_ts, // Keep scheduled_ts? Could also recalculate
        task.wait_ts,      // Keep wait_ts? Could also clear it
        task.alloc_secs,
        task.template.clone(),
        Some(respawn_str.clone()), // Carry respawn rule forward
        &task.udas,
        &tags,
    )?;
    
    Ok(new_task.id)
}

// Helper functions for date calculations

fn next_month_same_day(from: NaiveDate) -> NaiveDate {
    let (year, month) = if from.month() == 12 {
        (from.year() + 1, 1)
    } else {
        (from.year(), from.month() + 1)
    };
    
    // Handle days that don't exist in the next month (e.g., Jan 31 -> Feb 28)
    let day = from.day().min(days_in_month(year, month));
    
    NaiveDate::from_ymd_opt(year, month, day).unwrap_or(from)
}

fn next_year_same_date(from: NaiveDate) -> NaiveDate {
    let year = from.year() + 1;
    let month = from.month();
    let day = from.day().min(days_in_month(year, month));
    
    NaiveDate::from_ymd_opt(year, month, day).unwrap_or(from)
}

fn add_months(from: NaiveDate, months: i32) -> NaiveDate {
    let total_months = from.year() * 12 + from.month() as i32 - 1 + months;
    let year = total_months / 12;
    let month = (total_months % 12 + 1) as u32;
    let day = from.day().min(days_in_month(year, month));
    
    NaiveDate::from_ymd_opt(year, month, day).unwrap_or(from)
}

fn add_years(from: NaiveDate, years: i32) -> NaiveDate {
    let year = from.year() + years;
    let month = from.month();
    let day = from.day().min(days_in_month(year, month));
    
    NaiveDate::from_ymd_opt(year, month, day).unwrap_or(from)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) { 29 } else { 28 }
        }
        _ => 31,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn next_matching_weekday(from: NaiveDate, weekdays: &[u32]) -> NaiveDate {
    // Start from the day after completion
    let mut current = from + Duration::days(1);
    
    // Search up to 7 days to find a match
    for _ in 0..7 {
        let weekday_num = current.weekday().num_days_from_monday();
        if weekdays.contains(&weekday_num) {
            return current;
        }
        current = current + Duration::days(1);
    }
    
    // Should never reach here if weekdays is non-empty
    from + Duration::days(1)
}

fn next_matching_monthday(from: NaiveDate, days: &[u32]) -> NaiveDate {
    // Start from the day after completion
    let start = from + Duration::days(1);
    
    // Check remaining days in current month
    for day in days.iter() {
        if *day > start.day() && *day <= days_in_month(start.year(), start.month()) {
            if let Some(date) = NaiveDate::from_ymd_opt(start.year(), start.month(), *day) {
                return date;
            }
        }
    }
    
    // Check next month
    let next_month = next_month_same_day(from);
    let (year, month) = (next_month.year(), next_month.month());
    
    for day in days.iter() {
        if *day <= days_in_month(year, month) {
            if let Some(date) = NaiveDate::from_ymd_opt(year, month, *day) {
                return date;
            }
        }
    }
    
    // Fallback: first valid day in the month after next
    let month_after = add_months(from, 2);
    NaiveDate::from_ymd_opt(month_after.year(), month_after.month(), days[0].min(28))
        .unwrap_or(from + Duration::days(1))
}

fn next_nth_weekday(from: NaiveDate, nth: u32, weekday: u32) -> NaiveDate {
    // Convert weekday number (0=Mon) to chrono::Weekday
    let target_weekday = match weekday {
        0 => Weekday::Mon,
        1 => Weekday::Tue,
        2 => Weekday::Wed,
        3 => Weekday::Thu,
        4 => Weekday::Fri,
        5 => Weekday::Sat,
        _ => Weekday::Sun,
    };
    
    // Try current month first
    if let Some(date) = nth_weekday_of_month(from.year(), from.month(), nth, target_weekday) {
        if date > from {
            return date;
        }
    }
    
    // Try next month
    let (next_year, next_month) = if from.month() == 12 {
        (from.year() + 1, 1)
    } else {
        (from.year(), from.month() + 1)
    };
    
    if let Some(date) = nth_weekday_of_month(next_year, next_month, nth, target_weekday) {
        return date;
    }
    
    // Fallback (shouldn't happen for nth <= 5)
    from + Duration::days(28)
}

fn nth_weekday_of_month(year: i32, month: u32, nth: u32, weekday: Weekday) -> Option<NaiveDate> {
    let first_of_month = NaiveDate::from_ymd_opt(year, month, 1)?;
    let first_weekday = first_of_month.weekday();
    
    // Calculate days until the first occurrence of the target weekday
    let days_until = (weekday.num_days_from_monday() as i32 - first_weekday.num_days_from_monday() as i32 + 7) % 7;
    
    // Calculate the day of the nth occurrence
    let day = 1 + days_until as u32 + (nth - 1) * 7;
    
    // Check if it's valid for this month
    if day <= days_in_month(year, month) {
        NaiveDate::from_ymd_opt(year, month, day)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_occurrence_daily() {
        let rule = RespawnRule::parse("daily").unwrap();
        let from_ts = Utc.with_ymd_and_hms(2026, 1, 21, 10, 0, 0).unwrap().timestamp();
        let due_ts = Some(Utc.with_ymd_and_hms(2026, 1, 21, 9, 0, 0).unwrap().timestamp());
        
        let next = next_occurrence(&rule, from_ts, due_ts).unwrap();
        let next_dt = DateTime::<Utc>::from_timestamp(next, 0).unwrap();
        
        assert_eq!(next_dt.day(), 22);
        assert_eq!(next_dt.hour(), 9); // Preserves time from due date
    }

    #[test]
    fn test_next_occurrence_weekly() {
        let rule = RespawnRule::parse("weekly").unwrap();
        let from_ts = Utc.with_ymd_and_hms(2026, 1, 21, 10, 0, 0).unwrap().timestamp();
        
        let next = next_occurrence(&rule, from_ts, None).unwrap();
        let next_dt = DateTime::<Utc>::from_timestamp(next, 0).unwrap();
        
        assert_eq!(next_dt.day(), 28);
    }

    #[test]
    fn test_next_occurrence_monthdays() {
        let rule = RespawnRule::parse("monthdays:14,30").unwrap();
        
        // Completed on Jan 31, should respawn on Feb 14
        let from_ts = Utc.with_ymd_and_hms(2026, 1, 31, 10, 0, 0).unwrap().timestamp();
        let next = next_occurrence(&rule, from_ts, None).unwrap();
        let next_dt = DateTime::<Utc>::from_timestamp(next, 0).unwrap();
        
        assert_eq!(next_dt.month(), 2);
        assert_eq!(next_dt.day(), 14);
    }

    #[test]
    fn test_next_occurrence_weekdays() {
        let rule = RespawnRule::parse("weekdays:mon,wed,fri").unwrap();
        
        // Completed on Monday (Jan 20, 2026), next should be Wednesday (Jan 22)
        let from_ts = Utc.with_ymd_and_hms(2026, 1, 20, 10, 0, 0).unwrap().timestamp();
        let next = next_occurrence(&rule, from_ts, None).unwrap();
        let next_dt = DateTime::<Utc>::from_timestamp(next, 0).unwrap();
        
        // Jan 20 2026 is Tuesday, so next wed is Jan 21
        // Wait, let me check: Jan 1 2026 is Thursday
        // Jan 20 2026 = Thursday + 19 days = Thursday + 2 weeks + 5 days = Tuesday
        // Actually, let me calculate properly:
        // We complete on Tuesday (Jan 20), next Mon/Wed/Fri after that is Wed (Jan 21)
        assert_eq!(next_dt.weekday(), Weekday::Wed);
    }

    #[test]
    fn test_next_occurrence_nth_weekday() {
        let rule = RespawnRule::parse("nth:1:mon").unwrap();
        
        // Completed on Jan 10, 2026, first Monday of Feb is Feb 2
        let from_ts = Utc.with_ymd_and_hms(2026, 1, 10, 10, 0, 0).unwrap().timestamp();
        let next = next_occurrence(&rule, from_ts, None).unwrap();
        let next_dt = DateTime::<Utc>::from_timestamp(next, 0).unwrap();
        
        // First Monday of Feb 2026
        assert_eq!(next_dt.month(), 2);
        assert_eq!(next_dt.weekday(), Weekday::Mon);
    }

    #[test]
    fn test_month_boundary_handling() {
        // Jan 31 + 1 month should be Feb 28
        let jan31 = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        let next = next_month_same_day(jan31);
        assert_eq!(next.month(), 2);
        assert_eq!(next.day(), 28);
    }

    #[test]
    fn test_leap_year() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2025));
        assert!(!is_leap_year(2026));
        assert!(is_leap_year(2028));
        assert!(!is_leap_year(2100));
        assert!(is_leap_year(2000));
    }
}

//! Date expression parsing
//!
//! Comprehensive date expression parser supporting:
//! - Absolute dates: `2026-01-15`, `2026-01-15T09:00`
//! - Relative dates: `today`, `tomorrow`, `+2d`, `-1w`
//! - Time-only expressions: `09:00`, `17:30` (with 24-hour window rule)
//! - End-of-period: `eod`, `eow`, `eom`
//!
//! # Time-Only Expression Rule
//!
//! Time-only expressions (e.g., `09:00`) resolve to the nearest occurrence:
//! - Window: 8 hours in the past, 16 hours in the future
//! - If future is no more than twice as close as past: use future
//! - Otherwise: use nearest option
//!
//! # DST Handling
//!
//! - Dates are parsed in local timezone
//! - Stored as UTC timestamps
//! - Fall back hour (ambiguous): use first occurrence
//! - Spring forward hour (invalid): error

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Datelike, Timelike};
use anyhow::Result;

/// Parse a date expression and return Unix timestamp (UTC)
///
/// # Arguments
/// * `expr` - Date expression string
///
/// # Returns
/// Unix timestamp (UTC) as i64
///
/// # Examples
///
/// ```
/// use tatl::utils::parse_date_expr;
///
/// let ts = parse_date_expr("2026-01-15").unwrap();
/// let ts2 = parse_date_expr("tomorrow").unwrap();
/// let ts3 = parse_date_expr("09:00").unwrap();
/// ```
pub fn parse_date_expr(expr: &str) -> Result<i64> {
    let expr_lower = expr.to_lowercase();
    
    // Absolute dates: 2026-01-10, 2026-01-10T14:30
    if let Ok(date) = NaiveDate::parse_from_str(expr, "%Y-%m-%d") {
        let datetime = date.and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
        return parse_local_datetime(datetime);
    }
    
    if let Ok(datetime) = NaiveDateTime::parse_from_str(expr, "%Y-%m-%dT%H:%M") {
        return parse_local_datetime(datetime);
    }
    
    // Relative dates: today, tomorrow, eod, eow, eom
    let now = Local::now();
    match expr_lower.as_str() {
        "today" => {
            let today = now.date_naive().and_hms_opt(0, 0, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
            return parse_local_datetime(today);
        }
        "tomorrow" => {
            let tomorrow = (now.date_naive() + chrono::Duration::days(1))
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
            return parse_local_datetime(tomorrow);
        }
        "eod" => {
            // End of day: 23:59:59
            let today = now.date_naive();
            let eod = today.and_hms_opt(23, 59, 59)
                .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
            return parse_local_datetime(eod);
        }
        "eow" => {
            // End of week: Sunday 23:59:59
            let today = now.date_naive();
            let days_until_sunday = (7 - today.weekday().num_days_from_sunday()) % 7;
            let sunday = if days_until_sunday == 0 && now.time().hour() == 23 && now.time().minute() == 59 {
                today // Already at end of week
            } else {
                today + chrono::Duration::days(days_until_sunday as i64)
            };
            let eow = sunday.and_hms_opt(23, 59, 59)
                .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
            return parse_local_datetime(eow);
        }
        "eom" => {
            // End of month: last day of month 23:59:59
            let today = now.date_naive();
            let year = today.year();
            let month = today.month();
            // Get first day of next month, then subtract 1 day
            let next_month = if month == 12 {
                NaiveDate::from_ymd_opt(year + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(year, month + 1, 1)
            };
            let last_day = next_month
                .ok_or_else(|| anyhow::anyhow!("Invalid date"))?
                .checked_sub_signed(chrono::Duration::days(1))
                .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
            let eom = last_day.and_hms_opt(23, 59, 59)
                .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
            return parse_local_datetime(eom);
        }
        _ => {}
    }
    
    // Relative date offsets: +2d, +1w, -3d, etc.
    if expr_lower.starts_with('+') || expr_lower.starts_with('-') {
        if let Some(offset) = parse_relative_offset(&expr_lower)? {
            let base_date = now.date_naive();
            let target_date = base_date + chrono::Duration::days(offset);
            let target_datetime = target_date.and_hms_opt(0, 0, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
            return parse_local_datetime(target_datetime);
        }
    }

    // Relative date offsets without sign: 1w, 2weeks, 3 days, 1week
    if let Some(offset) = parse_relative_offset_without_sign(&expr_lower)? {
        let base_date = now.date_naive();
        let target_date = base_date + chrono::Duration::days(offset);
        let target_datetime = target_date.and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
        return parse_local_datetime(target_datetime);
    }

    // "in <n> <unit>" expressions
    if let Some(expr) = expr_lower.strip_prefix("in ") {
        if let Some(offset) = parse_relative_offset_without_sign(expr)? {
            let base_date = now.date_naive();
            let target_date = base_date + chrono::Duration::days(offset);
            let target_datetime = target_date.and_hms_opt(0, 0, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
            return parse_local_datetime(target_datetime);
        }
    }

    if expr_lower == "next week" {
        let base_date = now.date_naive();
        let target_date = base_date + chrono::Duration::days(7);
        let target_datetime = target_date.and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
        return parse_local_datetime(target_datetime);
    }
    
    // Time-only expressions: 9am, 14:30, noon, midnight
    if let Some(time) = parse_time_only(expr, &now)? {
        return Ok(time.timestamp());
    }
    
    anyhow::bail!("Unsupported date expression: {}", expr)
}

/// Parse a local datetime and convert to UTC timestamp
/// Handles DST transitions:
/// - Fall back hour: use first occurrence (earlier timestamp)
/// - Spring forward hour: error on invalid time
fn parse_local_datetime(dt: NaiveDateTime) -> Result<i64> {
    let local_dt = Local.from_local_datetime(&dt);
    
    match local_dt {
        chrono::LocalResult::Single(dt) => Ok(dt.timestamp()),
        chrono::LocalResult::Ambiguous(dt1, _dt2) => {
            // Fall back hour: use first occurrence (earlier timestamp)
            Ok(dt1.timestamp())
        }
        chrono::LocalResult::None => {
            // Spring forward hour: invalid time
            anyhow::bail!("Invalid time: {} falls in a DST transition gap", dt)
        }
    }
}

/// Parse relative date offset: +2d, +1w, -3d, etc.
fn parse_relative_offset(expr: &str) -> Result<Option<i64>> {
    let mut remaining = expr;
    
    // Handle sign
    let sign = if remaining.starts_with('+') {
        remaining = &remaining[1..];
        1
    } else if remaining.starts_with('-') {
        remaining = &remaining[1..];
        -1
    } else {
        return Ok(None);
    };
    
    // Parse number and unit
    let mut chars = remaining.chars().peekable();
    let mut num_str = String::new();
    
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() {
            num_str.push(ch);
            chars.next();
        } else {
            break;
        }
    }
    
    if num_str.is_empty() {
        return Ok(None);
    }
    
    let num = num_str.parse::<i64>()?;
    let unit: String = chars.collect();
    
    let total_days = match unit.to_lowercase().as_str() {
        "d" | "day" | "days" => num,
        "w" | "week" | "weeks" => num * 7,
        "m" | "month" | "months" => {
            // Approximate: use 30 days per month
            num * 30
        }
        "y" | "year" | "years" => {
            // Approximate: use 365 days per year
            num * 365
        }
        _ => return Ok(None),
    };
    
    Ok(Some(sign * total_days))
}

/// Parse relative date offset without sign: 2d, 1week, 3 days, 1w
fn parse_relative_offset_without_sign(expr: &str) -> Result<Option<i64>> {
    let normalized = expr.replace(' ', "");
    let mut chars = normalized.chars().peekable();
    let mut num_str = String::new();
    
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() {
            num_str.push(ch);
            chars.next();
        } else {
            break;
        }
    }
    
    if num_str.is_empty() {
        return Ok(None);
    }
    
    let num = num_str.parse::<i64>()?;
    let unit: String = chars.collect();
    
    let total_days = match unit.to_lowercase().as_str() {
        "d" | "day" | "days" => num,
        "w" | "week" | "weeks" => num * 7,
        "m" | "month" | "months" => num * 30,
        "y" | "year" | "years" => num * 365,
        _ => return Ok(None),
    };
    
    Ok(Some(total_days))
}

/// Parse time-only expression with 24-hour window rule
/// Window: 8 hours in the past, 16 hours in the future
/// If future is no more than twice as far as past, choose future; otherwise choose nearest
fn parse_time_only(expr: &str, now: &DateTime<Local>) -> Result<Option<DateTime<Local>>> {
    let expr_lower = expr.to_lowercase();
    
    // Special times: noon, midnight
    let time = match expr_lower.as_str() {
        "noon" => NaiveTime::from_hms_opt(12, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid time"))?,
        "midnight" => NaiveTime::from_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid time"))?,
        _ => {
            // Try 12-hour format: 9am, 2pm
            if let Some(time) = parse_12hour_format(&expr_lower) {
                time
            } else if let Ok(time) = NaiveTime::parse_from_str(expr, "%H:%M") {
                time
            } else {
                return Ok(None);
            }
        }
    };
    
    let today = now.date_naive();
    let yesterday = today.checked_sub_signed(chrono::Duration::days(1))
        .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
    let tomorrow = today.checked_add_signed(chrono::Duration::days(1))
        .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
    
    // Calculate all three possibilities
    let past = Local.from_local_datetime(&yesterday.and_time(time))
        .earliest()
        .ok_or_else(|| anyhow::anyhow!("Invalid past time"))?;
    let today_time = Local.from_local_datetime(&today.and_time(time))
        .earliest()
        .ok_or_else(|| anyhow::anyhow!("Invalid today time"))?;
    let future = Local.from_local_datetime(&tomorrow.and_time(time))
        .earliest()
        .ok_or_else(|| anyhow::anyhow!("Invalid future time"))?;
    
    // Determine which instance to use based on 24-hour window rule
    let now_ts = now.timestamp();
    let past_ts = past.timestamp();
    let today_ts = today_time.timestamp();
    let future_ts = future.timestamp();
    
    // Calculate distances
    let past_dist = now_ts - past_ts;
    let today_dist_past = if today_ts < now_ts { now_ts - today_ts } else { 0 };
    let today_dist_future = if today_ts > now_ts { today_ts - now_ts } else { 0 };
    let future_dist = future_ts - now_ts;
    
    // Window: 8 hours (28800s) in past, 16 hours (57600s) in future
    let window_past = 8 * 3600;
    let window_future = 16 * 3600;
    
    // Check if today's time is in the window
    if today_dist_past > 0 && today_dist_past <= window_past {
        // Today's time is in past window
        if today_dist_past <= past_dist {
            return Ok(Some(today_time));
        }
    }
    
    if today_dist_future > 0 && today_dist_future <= window_future {
        // Today's time is in future window
        if past_dist > 0 && past_dist <= window_past {
            // Both past and future in window - check "twice as close" rule
            if today_dist_future <= 2 * past_dist {
                return Ok(Some(today_time));
            } else {
                return Ok(Some(past));
            }
        } else {
            return Ok(Some(today_time));
        }
    }
    
    // Check yesterday's time
    if past_dist > 0 && past_dist <= window_past {
        if future_dist > 0 && future_dist <= window_future {
            // Both in window - check "twice as close" rule
            if future_dist <= 2 * past_dist {
                return Ok(Some(future));
            } else {
                return Ok(Some(past));
            }
        } else {
            return Ok(Some(past));
        }
    }
    
    // Check tomorrow's time
    if future_dist > 0 && future_dist <= window_future {
        return Ok(Some(future));
    }
    
    // Default: choose nearest
    let mut nearest = today_time;
    let mut nearest_dist = (today_ts - now_ts).abs();
    
    if (past_ts - now_ts).abs() < nearest_dist {
        nearest = past;
        nearest_dist = (past_ts - now_ts).abs();
    }
    
    if (future_ts - now_ts).abs() < nearest_dist {
        nearest = future;
    }
    
    Ok(Some(nearest))
}

/// Parse 12-hour format: 9am, 2pm, etc.
fn parse_12hour_format(expr: &str) -> Option<NaiveTime> {
    let expr = expr.trim();
    
    // Remove am/pm
    let (time_str, period) = if expr.ends_with("am") {
        (&expr[..expr.len() - 2], "am")
    } else if expr.ends_with("pm") {
        (&expr[..expr.len() - 2], "pm")
    } else {
        return None;
    };
    
    // Parse hour
    let hour: u32 = time_str.parse().ok()?;
    
    if hour < 1 || hour > 12 {
        return None;
    }
    
    let hour_24 = match period {
        "am" => if hour == 12 { 0 } else { hour },
        "pm" => if hour == 12 { 12 } else { hour + 12 },
        _ => return None,
    };
    
    NaiveTime::from_hms_opt(hour_24, 0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_absolute_date() {
        assert!(parse_date_expr("2026-01-10").is_ok());
        assert!(parse_date_expr("2026-01-10T14:30").is_ok());
    }

    #[test]
    fn test_relative_date() {
        assert!(parse_date_expr("today").is_ok());
        assert!(parse_date_expr("tomorrow").is_ok());
        assert!(parse_date_expr("+2d").is_ok());
        assert!(parse_date_expr("+1w").is_ok());
        assert!(parse_date_expr("-3d").is_ok());
        assert!(parse_date_expr("1week").is_ok());
        assert!(parse_date_expr("2weeks").is_ok());
        assert!(parse_date_expr("1w").is_ok());
        assert!(parse_date_expr("in 1 week").is_ok());
        assert!(parse_date_expr("next week").is_ok());
    }

    #[test]
    fn test_end_of_period() {
        assert!(parse_date_expr("eod").is_ok());
        assert!(parse_date_expr("eow").is_ok());
        assert!(parse_date_expr("eom").is_ok());
    }

    #[test]
    fn test_time_only() {
        assert!(parse_date_expr("9am").is_ok());
        assert!(parse_date_expr("2pm").is_ok());
        assert!(parse_date_expr("14:30").is_ok());
        assert!(parse_date_expr("noon").is_ok());
        assert!(parse_date_expr("midnight").is_ok());
    }
}

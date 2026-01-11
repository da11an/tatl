// Date expression parsing (simplified for MVP)
// Full implementation will come in Phase 9

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone};
use anyhow::{Context, Result};

/// Parse a date expression and return Unix timestamp (UTC)
/// For MVP, supports basic formats - full implementation in Phase 9
pub fn parse_date_expr(expr: &str) -> Result<i64> {
    // Absolute dates: 2026-01-10, 2026-01-10T14:30
    if let Ok(date) = NaiveDate::parse_from_str(expr, "%Y-%m-%d") {
        let datetime = date.and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
        let local_dt = Local.from_local_datetime(&datetime)
            .single()
            .ok_or_else(|| anyhow::anyhow!("Ambiguous date"))?;
        return Ok(local_dt.timestamp());
    }
    
    if let Ok(datetime) = NaiveDateTime::parse_from_str(expr, "%Y-%m-%dT%H:%M") {
        let local_dt = Local.from_local_datetime(&datetime)
            .single()
            .ok_or_else(|| anyhow::anyhow!("Ambiguous datetime"))?;
        return Ok(local_dt.timestamp());
    }
    
    // Relative dates: today, tomorrow
    let now = Local::now();
    match expr {
        "today" => {
            let today = now.date_naive().and_hms_opt(0, 0, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
            let local_dt = Local.from_local_datetime(&today)
                .single()
                .ok_or_else(|| anyhow::anyhow!("Ambiguous date"))?;
            Ok(local_dt.timestamp())
        }
        "tomorrow" => {
            let tomorrow = (now.date_naive() + chrono::Duration::days(1))
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
            let local_dt = Local.from_local_datetime(&tomorrow)
                .single()
                .ok_or_else(|| anyhow::anyhow!("Ambiguous date"))?;
            Ok(local_dt.timestamp())
        }
        _ => {
            // For MVP, return error for unsupported formats
            // Full implementation in Phase 9
            anyhow::bail!("Unsupported date expression: {}. Full date parsing will be implemented in Phase 9.", expr)
        }
    }
}

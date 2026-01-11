// Duration parsing (simplified for MVP)
// Full implementation will come in Phase 9

use anyhow::{Context, Result};

/// Parse a duration expression and return seconds
/// For MVP, supports basic formats - full implementation in Phase 9
pub fn parse_duration(expr: &str) -> Result<i64> {
    // Basic format: 30s, 10m, 2h, 1h30m
    // For MVP, support simple formats
    let mut total_secs = 0i64;
    let mut remaining = expr;
    
    // Parse units in order: d, h, m, s
    while !remaining.is_empty() {
        let mut found = false;
        
        // Try days
        if let Some(pos) = remaining.find('d') {
            if let Ok(days) = remaining[..pos].parse::<i64>() {
                total_secs += days * 86400;
                remaining = &remaining[pos + 1..];
                found = true;
            }
        }
        
        // Try hours
        if !found {
            if let Some(pos) = remaining.find('h') {
                if let Ok(hours) = remaining[..pos].parse::<i64>() {
                    total_secs += hours * 3600;
                    remaining = &remaining[pos + 1..];
                    found = true;
                }
            }
        }
        
        // Try minutes
        if !found {
            if let Some(pos) = remaining.find('m') {
                if let Ok(mins) = remaining[..pos].parse::<i64>() {
                    total_secs += mins * 60;
                    remaining = &remaining[pos + 1..];
                    found = true;
                }
            }
        }
        
        // Try seconds
        if !found {
            if let Some(pos) = remaining.find('s') {
                if let Ok(secs) = remaining[..pos].parse::<i64>() {
                    total_secs += secs;
                    remaining = &remaining[pos + 1..];
                    found = true;
                }
            }
        }
        
        if !found {
            anyhow::bail!("Invalid duration format: {}", expr);
        }
    }
    
    if total_secs == 0 {
        anyhow::bail!("Duration must be greater than 0");
    }
    
    Ok(total_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s").unwrap(), 30);
        assert_eq!(parse_duration("10m").unwrap(), 600);
        assert_eq!(parse_duration("2h").unwrap(), 7200);
        assert_eq!(parse_duration("1h30m").unwrap(), 5400);
    }
}

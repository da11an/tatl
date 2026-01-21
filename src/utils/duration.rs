//! Duration parsing
//!
//! Parses duration expressions into seconds.
//!
//! # Format
//!
//! - Units: `d` (days), `h` (hours), `m` (minutes), `s` (seconds)
//! - Ordering: Must appear largest to smallest (d, h, m, s)
//! - Each unit type may appear at most once
//!
//! # Examples
//!
//! ```text
//! 1h       // 1 hour
//! 2h30m    // 2 hours 30 minutes
//! 1d2h     // 1 day 2 hours
//! 30m      // 30 minutes
//! 1h15m30s // 1 hour 15 minutes 30 seconds
//! ```

use anyhow::Result;

/// Parse a duration expression and return seconds
///
/// # Arguments
/// * `expr` - Duration expression (e.g., "1h30m", "2d", "45s")
///
/// # Returns
/// Duration in seconds as i64
///
/// # Examples
///
/// ```
/// use tatl::utils::parse_duration;
///
/// assert_eq!(parse_duration("1h").unwrap(), 3600);
/// assert_eq!(parse_duration("2h30m").unwrap(), 9000);
/// assert_eq!(parse_duration("1d").unwrap(), 86400);
/// ```
pub fn parse_duration(expr: &str) -> Result<i64> {
    let expr = expr.trim();
    
    if expr.is_empty() {
        anyhow::bail!("Duration cannot be empty");
    }
    
    // Track which units we've seen and their order
    let mut seen_units = Vec::new();
    let mut total_secs = 0i64;
    let mut remaining = expr;
    
    // Unit order: d > h > m > s
    let unit_order = ['d', 'h', 'm', 's'];
    
    while !remaining.is_empty() {
        let mut found = false;
        
        // Try each unit in order
        for &unit in &unit_order {
            if let Some(pos) = remaining.find(unit) {
                // Check if we've already seen this unit
                if seen_units.contains(&unit) {
                    anyhow::bail!("Unit '{}' appears multiple times in duration: {}", unit, expr);
                }
                
                // Check ordering: must be after all previously seen units (largest to smallest)
                // unit_order: ['d', 'h', 'm', 's'] with indices [0, 1, 2, 3]
                // We want d > h > m > s, so current_idx should be > last_idx (going from larger to smaller)
                if let Some(&last_unit) = seen_units.last() {
                    let last_idx = unit_order.iter().position(|&u| u == last_unit).unwrap();
                    let current_idx = unit_order.iter().position(|&u| u == unit).unwrap();
                    if current_idx < last_idx {
                        anyhow::bail!("Units must be in order (d, h, m, s): {}", expr);
                    }
                }
                
                // Parse the number
                let num_str = &remaining[..pos];
                if num_str.is_empty() {
                    anyhow::bail!("Missing number before unit '{}' in duration: {}", unit, expr);
                }
                
                let num = num_str.parse::<i64>()
                    .map_err(|_| anyhow::anyhow!("Invalid number '{}' before unit '{}' in duration: {}", num_str, unit, expr))?;
                
                if num < 0 {
                    anyhow::bail!("Duration values cannot be negative: {}", expr);
                }
                
                // Add to total
                match unit {
                    'd' => total_secs += num * 86400,
                    'h' => total_secs += num * 3600,
                    'm' => total_secs += num * 60,
                    's' => total_secs += num,
                    _ => unreachable!(),
                }
                
                seen_units.push(unit);
                remaining = &remaining[pos + 1..];
                found = true;
                break;
            }
        }
        
        if !found {
            // Check for invalid characters
            if remaining.chars().next().unwrap().is_alphabetic() {
                anyhow::bail!("Unknown unit in duration: {}", expr);
            } else {
                anyhow::bail!("Invalid duration format: {}", expr);
            }
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

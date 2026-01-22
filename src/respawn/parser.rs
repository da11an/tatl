//! Respawn rule parser
//!
//! Parses respawn rules that define when the next instance of a task should be created
//! after the current one is completed.

use anyhow::{Context, Result};

/// Respawn frequency/pattern
#[derive(Debug, Clone, PartialEq)]
pub enum RespawnPattern {
    /// Every day at the same time
    Daily,
    /// Every week on the same weekday
    Weekly,
    /// Every month on the same day
    Monthly,
    /// Every year on the same date
    Yearly,
    /// Every N days
    EveryDays(i32),
    /// Every N weeks
    EveryWeeks(i32),
    /// Every N months
    EveryMonths(i32),
    /// Every N years
    EveryYears(i32),
    /// Specific weekdays (0=Monday, 6=Sunday)
    Weekdays(Vec<u32>),
    /// Specific days of month (1-31)
    Monthdays(Vec<u32>),
    /// Nth weekday of month (e.g., 2nd Tuesday)
    NthWeekday { nth: u32, weekday: u32 },
}

/// Respawn rule
#[derive(Debug, Clone)]
pub struct RespawnRule {
    pub pattern: RespawnPattern,
}

impl RespawnRule {
    /// Parse a respawn rule string
    /// 
    /// Supported formats:
    /// - `daily`, `weekly`, `monthly`, `yearly` - basic frequencies
    /// - `every:Nd`, `every:Nw`, `every:Nm`, `every:Ny` - interval frequencies
    /// - `weekdays:mon,wed,fri` - specific weekdays
    /// - `monthdays:1,15` - specific days of month
    /// - `nth:2:tue` - Nth weekday of month (e.g., 2nd Tuesday)
    pub fn parse(rule_str: &str) -> Result<Self> {
        let rule_lower = rule_str.to_lowercase().trim().to_string();
        
        if rule_lower.is_empty() {
            anyhow::bail!("Empty respawn rule");
        }
        
        let pattern = Self::parse_pattern(&rule_lower)?;
        
        Ok(RespawnRule { pattern })
    }
    
    fn parse_pattern(pattern_str: &str) -> Result<RespawnPattern> {
        // Check for simple frequencies
        match pattern_str {
            "daily" => return Ok(RespawnPattern::Daily),
            "weekly" => return Ok(RespawnPattern::Weekly),
            "monthly" => return Ok(RespawnPattern::Monthly),
            "yearly" => return Ok(RespawnPattern::Yearly),
            _ => {}
        }
        
        // Check for every:Nx pattern
        if pattern_str.starts_with("every:") {
            return Self::parse_every(&pattern_str[6..]);
        }
        
        // Check for weekdays: pattern
        if pattern_str.starts_with("weekdays:") {
            let weekdays = Self::parse_weekdays(&pattern_str[9..])?;
            return Ok(RespawnPattern::Weekdays(weekdays));
        }
        
        // Check for monthdays: pattern
        if pattern_str.starts_with("monthdays:") {
            let days = Self::parse_monthdays(&pattern_str[10..])?;
            return Ok(RespawnPattern::Monthdays(days));
        }
        
        // Check for nth:N:weekday pattern
        if pattern_str.starts_with("nth:") {
            return Self::parse_nth_weekday(&pattern_str[4..]);
        }
        
        anyhow::bail!("Unknown respawn pattern: {}", pattern_str);
    }
    
    fn parse_every(interval_str: &str) -> Result<RespawnPattern> {
        if interval_str.is_empty() {
            anyhow::bail!("Invalid interval pattern: empty");
        }
        
        // Parse number and unit
        let mut num_end = 0;
        for (i, ch) in interval_str.char_indices() {
            if !ch.is_ascii_digit() {
                num_end = i;
                break;
            }
        }
        
        if num_end == 0 {
            anyhow::bail!("Invalid interval pattern: no number found");
        }
        
        let num: i32 = interval_str[..num_end].parse()
            .context("Invalid number in interval")?;
        
        if num <= 0 {
            anyhow::bail!("Interval number must be greater than 0");
        }
        
        let unit = &interval_str[num_end..];
        match unit {
            "d" => Ok(RespawnPattern::EveryDays(num)),
            "w" => Ok(RespawnPattern::EveryWeeks(num)),
            "m" => Ok(RespawnPattern::EveryMonths(num)),
            "y" => Ok(RespawnPattern::EveryYears(num)),
            _ => anyhow::bail!("Invalid unit in interval: '{}' (expected d, w, m, or y)", unit),
        }
    }
    
    fn parse_weekdays(weekdays_str: &str) -> Result<Vec<u32>> {
        let mut weekdays = Vec::new();
        
        for weekday_str in weekdays_str.split(',') {
            let weekday_str = weekday_str.trim();
            if weekday_str.is_empty() {
                continue;
            }
            
            let weekday = Self::parse_weekday(weekday_str)?;
            
            if !weekdays.contains(&weekday) {
                weekdays.push(weekday);
            }
        }
        
        if weekdays.is_empty() {
            anyhow::bail!("No valid weekdays specified");
        }
        
        // Sort weekdays for consistent ordering
        weekdays.sort();
        
        Ok(weekdays)
    }
    
    fn parse_weekday(weekday_str: &str) -> Result<u32> {
        match weekday_str {
            "mon" | "monday" => Ok(0),
            "tue" | "tuesday" => Ok(1),
            "wed" | "wednesday" => Ok(2),
            "thu" | "thursday" => Ok(3),
            "fri" | "friday" => Ok(4),
            "sat" | "saturday" => Ok(5),
            "sun" | "sunday" => Ok(6),
            _ => anyhow::bail!("Invalid weekday: {}", weekday_str),
        }
    }
    
    fn parse_monthdays(days_str: &str) -> Result<Vec<u32>> {
        let mut days = Vec::new();
        
        for day_str in days_str.split(',') {
            let day_str = day_str.trim();
            if day_str.is_empty() {
                continue;
            }
            
            let day: u32 = day_str.parse()
                .context("Invalid day of month")?;
            
            if day < 1 || day > 31 {
                anyhow::bail!("Day of month must be between 1 and 31: {}", day);
            }
            
            if !days.contains(&day) {
                days.push(day);
            }
        }
        
        if days.is_empty() {
            anyhow::bail!("No valid days of month specified");
        }
        
        // Sort days for consistent ordering
        days.sort();
        
        Ok(days)
    }
    
    fn parse_nth_weekday(nth_str: &str) -> Result<RespawnPattern> {
        // Format: N:weekday (e.g., "2:tue" for 2nd Tuesday)
        let parts: Vec<&str> = nth_str.split(':').collect();
        
        if parts.len() != 2 {
            anyhow::bail!("Invalid nth weekday format: expected 'N:weekday' (e.g., 'nth:2:tue')");
        }
        
        let nth: u32 = parts[0].parse()
            .context("Invalid nth value")?;
        
        if nth < 1 || nth > 5 {
            anyhow::bail!("Nth value must be between 1 and 5: {}", nth);
        }
        
        let weekday = Self::parse_weekday(parts[1])?;
        
        Ok(RespawnPattern::NthWeekday { nth, weekday })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_frequencies() {
        assert!(RespawnRule::parse("daily").is_ok());
        assert!(RespawnRule::parse("weekly").is_ok());
        assert!(RespawnRule::parse("monthly").is_ok());
        assert!(RespawnRule::parse("yearly").is_ok());
    }

    #[test]
    fn test_parse_interval_frequencies() {
        let rule = RespawnRule::parse("every:2d").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::EveryDays(2));
        
        let rule = RespawnRule::parse("every:3w").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::EveryWeeks(3));
        
        let rule = RespawnRule::parse("every:2m").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::EveryMonths(2));
        
        let rule = RespawnRule::parse("every:1y").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::EveryYears(1));
    }

    #[test]
    fn test_parse_weekdays() {
        let rule = RespawnRule::parse("weekdays:mon,wed,fri").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::Weekdays(vec![0, 2, 4]));
    }

    #[test]
    fn test_parse_monthdays() {
        let rule = RespawnRule::parse("monthdays:1,15").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::Monthdays(vec![1, 15]));
    }

    #[test]
    fn test_parse_nth_weekday() {
        let rule = RespawnRule::parse("nth:2:tue").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::NthWeekday { nth: 2, weekday: 1 });
        
        let rule = RespawnRule::parse("nth:1:mon").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::NthWeekday { nth: 1, weekday: 0 });
    }

    #[test]
    fn test_invalid_patterns() {
        assert!(RespawnRule::parse("").is_err());
        assert!(RespawnRule::parse("invalid").is_err());
        assert!(RespawnRule::parse("every:0d").is_err());
        assert!(RespawnRule::parse("every:-1d").is_err());
        assert!(RespawnRule::parse("monthdays:0").is_err());
        assert!(RespawnRule::parse("monthdays:32").is_err());
        assert!(RespawnRule::parse("nth:0:mon").is_err());
        assert!(RespawnRule::parse("nth:6:mon").is_err());
    }
}

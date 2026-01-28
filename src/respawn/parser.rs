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
    /// - `Nd`, `Nw`, `Nm`, `Ny` - interval frequencies (e.g., `2d`, `3w`)
    /// - `mon,wed,fri` - specific weekdays
    /// - `1,15` - specific days of month
    /// - `2nd-tue` - Nth weekday of month (e.g., 2nd Tuesday)
    pub fn parse(rule_str: &str) -> Result<Self> {
        let rule_lower = rule_str.to_lowercase().trim().to_string();

        if rule_lower.is_empty() {
            anyhow::bail!("Empty respawn rule");
        }

        let pattern = Self::parse_pattern(&rule_lower)?;

        Ok(RespawnRule { pattern })
    }

    /// Return a human-readable description of the respawn pattern
    pub fn describe(&self) -> String {
        match &self.pattern {
            RespawnPattern::Daily => "When completed, a new task will be created for the next day".to_string(),
            RespawnPattern::Weekly => "When completed, a new task will be created for the next week".to_string(),
            RespawnPattern::Monthly => "When completed, a new task will be created for the next month".to_string(),
            RespawnPattern::Yearly => "When completed, a new task will be created for the next year".to_string(),
            RespawnPattern::EveryDays(n) => format!("When completed, a new task will be created for {} days later", n),
            RespawnPattern::EveryWeeks(n) => format!("When completed, a new task will be created for {} weeks later", n),
            RespawnPattern::EveryMonths(n) => format!("When completed, a new task will be created for {} months later", n),
            RespawnPattern::EveryYears(n) => format!("When completed, a new task will be created for {} years later", n),
            RespawnPattern::Weekdays(days) => {
                let day_names: Vec<&str> = days.iter().map(|d| match d {
                    0 => "Mon", 1 => "Tue", 2 => "Wed", 3 => "Thu",
                    4 => "Fri", 5 => "Sat", 6 => "Sun", _ => "?",
                }).collect();
                format!("When completed, a new task will be created for the next {}", day_names.join(", "))
            }
            RespawnPattern::Monthdays(days) => {
                let day_strs: Vec<String> = days.iter().map(|d| d.to_string()).collect();
                format!("When completed, a new task will be created for day {} of the next month", day_strs.join(" or "))
            }
            RespawnPattern::NthWeekday { nth, weekday } => {
                let weekday_name = match weekday {
                    0 => "Monday", 1 => "Tuesday", 2 => "Wednesday", 3 => "Thursday",
                    4 => "Friday", 5 => "Saturday", 6 => "Sunday", _ => "?",
                };
                let ordinal = match nth {
                    1 => "1st", 2 => "2nd", 3 => "3rd", 4 => "4th", 5 => "5th", _ => "?",
                };
                format!("When completed, a new task will be created for the {} {} of the next month", ordinal, weekday_name)
            }
        }
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

        // Check for Nth-weekday pattern (e.g., "2nd-tue", "1st-mon", "last-fri")
        if pattern_str.contains('-') {
            if let Ok(pattern) = Self::parse_nth_weekday(pattern_str) {
                return Ok(pattern);
            }
        }

        // Check for interval pattern (e.g., "2d", "3w", "2m", "1y")
        if let Ok(pattern) = Self::parse_every(pattern_str) {
            return Ok(pattern);
        }

        // Check for comma-separated weekday names (e.g., "mon,wed,fri")
        if pattern_str.contains(',') || Self::is_weekday_name(pattern_str) {
            if let Ok(weekdays) = Self::parse_weekdays(pattern_str) {
                return Ok(RespawnPattern::Weekdays(weekdays));
            }
        }

        // Check for comma-separated numbers (monthdays, e.g., "1,15")
        if pattern_str.chars().all(|c| c.is_ascii_digit() || c == ',') {
            if let Ok(days) = Self::parse_monthdays(pattern_str) {
                return Ok(RespawnPattern::Monthdays(days));
            }
        }

        // Legacy support: try old prefixed formats
        if pattern_str.starts_with("every:") {
            return Self::parse_every(&pattern_str[6..]);
        }
        if pattern_str.starts_with("weekdays:") {
            let weekdays = Self::parse_weekdays(&pattern_str[9..])?;
            return Ok(RespawnPattern::Weekdays(weekdays));
        }
        if pattern_str.starts_with("monthdays:") {
            let days = Self::parse_monthdays(&pattern_str[10..])?;
            return Ok(RespawnPattern::Monthdays(days));
        }
        if pattern_str.starts_with("nth:") {
            return Self::parse_nth_weekday_legacy(&pattern_str[4..]);
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

    fn is_weekday_name(s: &str) -> bool {
        matches!(s, "mon" | "monday" | "tue" | "tuesday" | "wed" | "wednesday"
            | "thu" | "thursday" | "fri" | "friday" | "sat" | "saturday"
            | "sun" | "sunday")
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

    /// Parse Nth-weekday format: "2nd-tue", "1st-mon", "last-fri"
    fn parse_nth_weekday(nth_str: &str) -> Result<RespawnPattern> {
        let parts: Vec<&str> = nth_str.split('-').collect();

        if parts.len() != 2 {
            anyhow::bail!("Invalid nth weekday format: expected 'Nth-weekday' (e.g., '2nd-tue')");
        }

        let nth: u32 = match parts[0] {
            "1st" | "first" => 1,
            "2nd" | "second" => 2,
            "3rd" | "third" => 3,
            "4th" | "fourth" => 4,
            "5th" | "fifth" => 5,
            "last" => 5,
            _ => anyhow::bail!("Invalid ordinal: '{}' (expected 1st, 2nd, 3rd, 4th, 5th, or last)", parts[0]),
        };

        let weekday = Self::parse_weekday(parts[1])?;

        Ok(RespawnPattern::NthWeekday { nth, weekday })
    }

    /// Legacy format: "N:weekday" (e.g., "2:tue")
    fn parse_nth_weekday_legacy(nth_str: &str) -> Result<RespawnPattern> {
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
        let rule = RespawnRule::parse("2d").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::EveryDays(2));

        let rule = RespawnRule::parse("3w").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::EveryWeeks(3));

        let rule = RespawnRule::parse("2m").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::EveryMonths(2));

        let rule = RespawnRule::parse("1y").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::EveryYears(1));
    }

    #[test]
    fn test_parse_weekdays() {
        let rule = RespawnRule::parse("mon,wed,fri").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::Weekdays(vec![0, 2, 4]));
    }

    #[test]
    fn test_parse_monthdays() {
        let rule = RespawnRule::parse("1,15").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::Monthdays(vec![1, 15]));
    }

    #[test]
    fn test_parse_nth_weekday() {
        let rule = RespawnRule::parse("2nd-tue").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::NthWeekday { nth: 2, weekday: 1 });

        let rule = RespawnRule::parse("1st-mon").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::NthWeekday { nth: 1, weekday: 0 });

        let rule = RespawnRule::parse("last-fri").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::NthWeekday { nth: 5, weekday: 4 });
    }

    #[test]
    fn test_legacy_formats_still_work() {
        // Legacy every: prefix
        let rule = RespawnRule::parse("every:2d").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::EveryDays(2));

        // Legacy weekdays: prefix
        let rule = RespawnRule::parse("weekdays:mon,wed,fri").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::Weekdays(vec![0, 2, 4]));

        // Legacy monthdays: prefix
        let rule = RespawnRule::parse("monthdays:1,15").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::Monthdays(vec![1, 15]));

        // Legacy nth: prefix
        let rule = RespawnRule::parse("nth:2:tue").unwrap();
        assert_eq!(rule.pattern, RespawnPattern::NthWeekday { nth: 2, weekday: 1 });
    }

    #[test]
    fn test_invalid_patterns() {
        assert!(RespawnRule::parse("").is_err());
        assert!(RespawnRule::parse("invalid").is_err());
        assert!(RespawnRule::parse("0d").is_err());
        assert!(RespawnRule::parse("0,32").is_err());
    }
}

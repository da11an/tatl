// Recurrence rule parser

use anyhow::{Context, Result};

/// Recurrence frequency
#[derive(Debug, Clone, PartialEq)]
pub enum RecurFrequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
    EveryDays(i32),
    EveryWeeks(i32),
    EveryMonths(i32),
    EveryYears(i32),
}

/// Recurrence rule
#[derive(Debug, Clone)]
pub struct RecurRule {
    pub frequency: RecurFrequency,
    pub byweekday: Option<Vec<u32>>, // 0=Monday, 6=Sunday
    pub bymonthday: Option<Vec<u32>>, // 1-31
}

impl RecurRule {
    /// Parse a recurrence rule string
    pub fn parse(rule_str: &str) -> Result<Self> {
        let rule_lower = rule_str.to_lowercase();
        let parts: Vec<&str> = rule_lower.split_whitespace().collect();
        
        if parts.is_empty() {
            anyhow::bail!("Empty recurrence rule");
        }
        
        // Parse frequency (first part)
        let frequency = Self::parse_frequency(parts[0])?;
        
        // Parse modifiers (remaining parts)
        let mut byweekday = None;
        let mut bymonthday = None;
        
        for part in parts.iter().skip(1) {
            if part.starts_with("byweekday:") {
                let weekdays_str = &part[10..]; // Skip "byweekday:"
                byweekday = Some(Self::parse_weekdays(weekdays_str)?);
            } else if part.starts_with("bymonthday:") {
                let days_str = &part[11..]; // Skip "bymonthday:"
                bymonthday = Some(Self::parse_monthdays(days_str)?);
            } else {
                anyhow::bail!("Unknown modifier: {}", part);
            }
        }
        
        // Validate modifier compatibility
        match &frequency {
            RecurFrequency::Daily | RecurFrequency::EveryDays(_) => {
                if byweekday.is_some() || bymonthday.is_some() {
                    anyhow::bail!("byweekday and bymonthday modifiers are not compatible with daily frequency");
                }
            }
            RecurFrequency::Weekly | RecurFrequency::EveryWeeks(_) => {
                if bymonthday.is_some() {
                    anyhow::bail!("bymonthday modifier is not compatible with weekly frequency");
                }
            }
            RecurFrequency::Monthly | RecurFrequency::EveryMonths(_) => {
                if byweekday.is_some() {
                    anyhow::bail!("byweekday modifier is not compatible with monthly frequency");
                }
            }
            RecurFrequency::Yearly | RecurFrequency::EveryYears(_) => {
                // Yearly can have both modifiers (e.g., "yearly bymonthday:1 byweekday:mon" for first Monday of January)
                // But for MVP, we'll keep it simple and not support this combination
                if byweekday.is_some() && bymonthday.is_some() {
                    anyhow::bail!("Combining byweekday and bymonthday is not supported in MVP");
                }
            }
        }
        
        Ok(RecurRule {
            frequency,
            byweekday,
            bymonthday,
        })
    }
    
    fn parse_frequency(freq_str: &str) -> Result<RecurFrequency> {
        match freq_str {
            "daily" => Ok(RecurFrequency::Daily),
            "weekly" => Ok(RecurFrequency::Weekly),
            "monthly" => Ok(RecurFrequency::Monthly),
            "yearly" => Ok(RecurFrequency::Yearly),
            _ => {
                // Try interval frequency: every:Nd, every:Nw, etc.
                if freq_str.starts_with("every:") {
                    let interval_str = &freq_str[6..]; // Skip "every:"
                    if interval_str.is_empty() {
                        anyhow::bail!("Invalid interval frequency: {}", freq_str);
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
                        anyhow::bail!("Invalid interval frequency: {}", freq_str);
                    }
                    
                    let num: i32 = interval_str[..num_end].parse()
                        .context("Invalid number in interval frequency")?;
                    
                    if num <= 0 {
                        anyhow::bail!("Interval number must be greater than 0");
                    }
                    
                    let unit = &interval_str[num_end..];
                    match unit {
                        "d" => Ok(RecurFrequency::EveryDays(num)),
                        "w" => Ok(RecurFrequency::EveryWeeks(num)),
                        "m" => Ok(RecurFrequency::EveryMonths(num)),
                        "y" => Ok(RecurFrequency::EveryYears(num)),
                        _ => anyhow::bail!("Invalid unit in interval frequency: {}", unit),
                    }
                } else {
                    anyhow::bail!("Unknown frequency: {}", freq_str);
                }
            }
        }
    }
    
    fn parse_weekdays(weekdays_str: &str) -> Result<Vec<u32>> {
        let mut weekdays = Vec::new();
        
        // Split by comma or space
        for weekday_str in weekdays_str.split(&[',', ' '][..]) {
            let weekday_str = weekday_str.trim();
            if weekday_str.is_empty() {
                continue;
            }
            
            let weekday = match weekday_str {
                "mon" | "monday" => 0,
                "tue" | "tuesday" => 1,
                "wed" | "wednesday" => 2,
                "thu" | "thursday" => 3,
                "fri" | "friday" => 4,
                "sat" | "saturday" => 5,
                "sun" | "sunday" => 6,
                _ => anyhow::bail!("Invalid weekday: {}", weekday_str),
            };
            
            if !weekdays.contains(&weekday) {
                weekdays.push(weekday);
            }
        }
        
        if weekdays.is_empty() {
            anyhow::bail!("No valid weekdays specified");
        }
        
        Ok(weekdays)
    }
    
    fn parse_monthdays(days_str: &str) -> Result<Vec<u32>> {
        let mut days = Vec::new();
        
        // Split by comma or space
        for day_str in days_str.split(&[',', ' '][..]) {
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
        
        Ok(days)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_frequencies() {
        assert!(RecurRule::parse("daily").is_ok());
        assert!(RecurRule::parse("weekly").is_ok());
        assert!(RecurRule::parse("monthly").is_ok());
        assert!(RecurRule::parse("yearly").is_ok());
    }

    #[test]
    fn test_parse_interval_frequencies() {
        assert!(RecurRule::parse("every:2d").is_ok());
        assert!(RecurRule::parse("every:3w").is_ok());
        assert!(RecurRule::parse("every:2m").is_ok());
        assert!(RecurRule::parse("every:1y").is_ok());
    }

    #[test]
    fn test_parse_weekday_modifier() {
        let rule = RecurRule::parse("weekly byweekday:mon,wed,fri").unwrap();
        assert_eq!(rule.frequency, RecurFrequency::Weekly);
        assert_eq!(rule.byweekday, Some(vec![0, 2, 4]));
    }

    #[test]
    fn test_parse_monthday_modifier() {
        let rule = RecurRule::parse("monthly bymonthday:1,15").unwrap();
        assert_eq!(rule.frequency, RecurFrequency::Monthly);
        assert_eq!(rule.bymonthday, Some(vec![1, 15]));
    }

    #[test]
    fn test_modifier_validation() {
        // Daily with weekday modifier should fail
        assert!(RecurRule::parse("daily byweekday:mon").is_err());
        
        // Weekly with monthday modifier should fail
        assert!(RecurRule::parse("weekly bymonthday:1").is_err());
        
        // Monthly with weekday modifier should fail
        assert!(RecurRule::parse("monthly byweekday:mon").is_err());
    }
}

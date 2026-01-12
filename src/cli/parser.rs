// CLI parsing utilities for task commands

use std::collections::HashMap;

/// Parsed task arguments from command line
#[derive(Debug, Default)]
pub struct ParsedTaskArgs {
    pub description: Vec<String>,
    pub project: Option<String>,
    pub due: Option<String>,
    pub scheduled: Option<String>,
    pub wait: Option<String>,
    pub alloc: Option<String>,
    pub template: Option<String>,
    pub recur: Option<String>,
    pub tags_add: Vec<String>,
    pub tags_remove: Vec<String>,
    pub udas: HashMap<String, String>,
}

/// Parse task add/modify arguments
/// Description is tokens that don't match field patterns, tag patterns, or flags
/// Field tokens, tag tokens, and flags can appear anywhere in the argument list
pub fn parse_task_args(args: Vec<String>) -> ParsedTaskArgs {
    let mut parsed = ParsedTaskArgs::default();
    let mut description_parts = Vec::new();
    
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        
        // Check for flags (--flag)
        if arg.starts_with("--") {
            // Skip flags for now (will handle --template, --yes, etc. in command handler)
            i += 1;
            continue;
        }
        
        // Check for field tokens (field:value)
        if let Some((field, value)) = parse_field_token(arg) {
            match field.as_str() {
                "project" | "pro" => parsed.project = Some(value),
                "due" => parsed.due = Some(value),
                "scheduled" => parsed.scheduled = Some(value),
                "wait" => parsed.wait = Some(value),
                "alloc" => parsed.alloc = Some(value),
                "template" => parsed.template = Some(value),
                "recur" => parsed.recur = Some(value),
                _ => {
                    // Check if it's a UDA (uda.<key>:<value>)
                    if field.starts_with("uda.") {
                        let key = field.strip_prefix("uda.").unwrap().to_string();
                        if value == "none" {
                            // UDA clearing - handled separately
                        } else {
                            parsed.udas.insert(key, value);
                        }
                    }
                }
            }
        }
        // Check for tag tokens (+tag or -tag)
        else if let Some(tag) = parse_tag_token(arg) {
            if tag.starts_with('+') {
                parsed.tags_add.push(tag.strip_prefix('+').unwrap().to_string());
            } else if tag.starts_with('-') {
                parsed.tags_remove.push(tag.strip_prefix('-').unwrap().to_string());
            }
        }
        // Regular description token
        else {
            description_parts.push(arg.clone());
        }
        
        i += 1;
    }
    
    parsed.description = description_parts;
    parsed
}

/// Parse a field token (field:value)
fn parse_field_token(token: &str) -> Option<(String, String)> {
    if let Some(colon_pos) = token.find(':') {
        let field = token[..colon_pos].to_string();
        let value = token[colon_pos + 1..].to_string();
        Some((field, value))
    } else {
        None
    }
}

/// Parse a tag token (+tag or -tag)
fn parse_tag_token(token: &str) -> Option<String> {
    if token.starts_with('+') || token.starts_with('-') {
        // Validate tag charset: [A-Za-z0-9_\-\.]+
        let tag_part = if token.starts_with('+') {
            &token[1..]
        } else {
            &token[1..]
        };
        
        if tag_part.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.') {
            Some(token.to_string())
        } else {
            None
        }
    } else {
        None
    }
}

/// Join description parts into a single string
pub fn join_description(parts: &[String]) -> String {
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_description() {
        let args = vec!["fix".to_string(), "the".to_string(), "bug".to_string()];
        let parsed = parse_task_args(args);
        assert_eq!(parsed.description, vec!["fix", "the", "bug"]);
    }

    #[test]
    fn test_parse_with_project() {
        let args = vec!["fix".to_string(), "bug".to_string(), "project:work".to_string()];
        let parsed = parse_task_args(args);
        assert_eq!(parsed.description, vec!["fix", "bug"]);
        assert_eq!(parsed.project, Some("work".to_string()));
    }
    
    #[test]
    fn test_parse_with_project_abbreviation() {
        let args = vec!["fix".to_string(), "bug".to_string(), "pro:work".to_string()];
        let parsed = parse_task_args(args);
        assert_eq!(parsed.description, vec!["fix", "bug"]);
        assert_eq!(parsed.project, Some("work".to_string()));
    }

    #[test]
    fn test_parse_with_tags() {
        let args = vec!["fix".to_string(), "bug".to_string(), "+urgent".to_string(), "+important".to_string()];
        let parsed = parse_task_args(args);
        assert_eq!(parsed.description, vec!["fix", "bug"]);
        assert_eq!(parsed.tags_add, vec!["urgent", "important"]);
    }

    #[test]
    fn test_parse_mixed_order() {
        let args = vec!["project:work".to_string(), "fix".to_string(), "bug".to_string(), "+urgent".to_string()];
        let parsed = parse_task_args(args);
        assert_eq!(parsed.description, vec!["fix", "bug"]);
        assert_eq!(parsed.project, Some("work".to_string()));
        assert_eq!(parsed.tags_add, vec!["urgent"]);
    }

    #[test]
    fn test_parse_udas() {
        let args = vec!["fix".to_string(), "bug".to_string(), "uda.priority:high".to_string(), "uda.estimate:2h".to_string()];
        let parsed = parse_task_args(args);
        assert_eq!(parsed.description, vec!["fix", "bug"]);
        assert_eq!(parsed.udas.get("priority"), Some(&"high".to_string()));
        assert_eq!(parsed.udas.get("estimate"), Some(&"2h".to_string()));
    }
}

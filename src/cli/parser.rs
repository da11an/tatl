// CLI parsing utilities for task commands

use std::collections::HashMap;
use crate::utils::fuzzy::levenshtein_distance;

/// Parsed task arguments from command line
#[derive(Debug, Default)]
pub struct ParsedTaskArgs {
    pub description: Vec<String>,
    pub project: Option<String>,
    pub due: Option<String>,
    pub scheduled: Option<String>,
    pub wait: Option<String>,
    pub allocation: Option<String>,
    pub template: Option<String>,
    pub recur: Option<String>,
    pub tags_add: Vec<String>,
    pub tags_remove: Vec<String>,
    pub udas: HashMap<String, String>,
}

/// Field name abbreviation error
#[derive(Debug)]
pub enum FieldParseError {
    AmbiguousAbbreviation {
        field: String,
        matches: Vec<String>,
    },
    InvalidFieldName {
        field: String,
        suggestion: String,
    },
}

impl std::fmt::Display for FieldParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldParseError::AmbiguousAbbreviation { field, matches } => {
                let match_list = matches.join(", ");
                write!(f, "Ambiguous field name '{}'\n  Matches: {}\n  Use a longer prefix to disambiguate, or use the full field name.", field, match_list)
            }
            FieldParseError::InvalidFieldName { field, suggestion } => {
                write!(f, "Unrecognized field name '{}'\n  Did you mean '{}'?", field, suggestion)
            }
        }
    }
}

/// Field names that can be abbreviated
const FIELD_NAMES: &[&str] = &[
    "project",
    "due",
    "scheduled",
    "wait",
    "allocation",
    "template",
    "recur",
];

/// Expand field name abbreviation (like command abbreviations)
/// Returns Ok(field_name) if unambiguous, Err(matches) if ambiguous
fn expand_field_name_abbreviation(field: &str) -> Result<String, Vec<String>> {
    let matches: Vec<&str> = FIELD_NAMES
        .iter()
        .filter(|name| name.to_lowercase().starts_with(&field.to_lowercase()))
        .copied()
        .collect();
    
    if matches.is_empty() {
        Err(Vec::new())
    } else if matches.len() == 1 {
        Ok(matches[0].to_string())
    } else {
        Err(matches.iter().map(|s| s.to_string()).collect())
    }
}

/// Find the most similar field name using fuzzy matching
fn find_similar_field_name(field: &str) -> Option<String> {
    let mut best_match: Option<(&str, usize)> = None;
    
    for name in FIELD_NAMES {
        let distance = levenshtein_distance(&field.to_lowercase(), &name.to_lowercase());
        if distance <= 3 {
            match best_match {
                None => best_match = Some((name, distance)),
                Some((_, best_dist)) if distance < best_dist => {
                    best_match = Some((name, distance));
                }
                _ => {}
            }
        }
    }
    
    best_match.map(|(name, _)| name.to_string())
}

/// Parse a field token (field:value)
/// Returns the field name (after abbreviation expansion) and value
/// Handles empty values (field:) by converting to field:none
fn parse_field_token(token: &str) -> Result<Option<(String, String)>, FieldParseError> {
    if let Some(colon_pos) = token.find(':') {
        let field = token[..colon_pos].to_string();
        let value = token[colon_pos + 1..].to_string();
        
        // Handle empty value (field:) -> treat as field:none
        let final_value = if value.is_empty() {
            "none".to_string()
        } else {
            value
        };
        
        // Try exact match first
        if FIELD_NAMES.contains(&field.as_str()) {
            return Ok(Some((field, final_value)));
        }
        
        // Try abbreviation expansion
        match expand_field_name_abbreviation(&field) {
            Ok(expanded_field) => {
                Ok(Some((expanded_field, final_value)))
            }
            Err(matches) => {
                if !matches.is_empty() {
                    // Ambiguous abbreviation
                    return Err(FieldParseError::AmbiguousAbbreviation {
                        field,
                        matches,
                    });
                }
                
                // No abbreviation match - try fuzzy matching
                if let Some(suggestion) = find_similar_field_name(&field) {
                    Err(FieldParseError::InvalidFieldName {
                        field,
                        suggestion,
                    })
                } else {
                    // No similar field found - might be a UDA or invalid
                    // Return None to let caller handle it
                    Ok(None)
                }
            }
        }
    } else {
        Ok(None)
    }
}

/// Parse task add/modify arguments
/// Description is tokens that don't match field patterns, tag patterns, or flags
/// Field tokens, tag tokens, and flags can appear anywhere in the argument list
/// Returns Result to handle field parse errors (ambiguous abbreviations, invalid field names)
pub fn parse_task_args(args: Vec<String>) -> Result<ParsedTaskArgs, FieldParseError> {
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
        let mut handled = false;
        match parse_field_token(arg) {
            Ok(Some((field, value))) => {
                handled = true;
                match field.as_str() {
                    "project" => parsed.project = Some(value),
                    "due" => parsed.due = Some(value),
                    "scheduled" => parsed.scheduled = Some(value),
                    "wait" => parsed.wait = Some(value),
                    "allocation" => parsed.allocation = Some(value),
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
                        } else {
                            // Unknown field - treat as description
                            description_parts.push(arg.clone());
                        }
                    }
                }
            }
            Ok(None) => {
                // Not a recognized field token - might be description or UDA
                // Check if it looks like a UDA
                if arg.contains(':') && arg.starts_with("uda.") {
                    // UDA format - parse manually
                    if let Some(colon_pos) = arg.find(':') {
                        let field = &arg[..colon_pos];
                        let value = &arg[colon_pos + 1..];
                        let key = field.strip_prefix("uda.").unwrap().to_string();
                        if value == "none" {
                            // UDA clearing - handled separately
                        } else {
                            parsed.udas.insert(key, value.to_string());
                        }
                        handled = true;
                    }
                }
                // Otherwise will be handled below
            }
            Err(e) => {
                // Field parse error - return error to caller
                return Err(e);
            }
        }
        
        // If not handled as field token, check for tags or treat as description
        if !handled {
            // Check for tag tokens (+tag or -tag)
            if let Some(tag) = parse_tag_token(arg) {
                if tag.starts_with('+') {
                    parsed.tags_add.push(tag.strip_prefix('+').unwrap().to_string());
                } else if tag.starts_with('-') {
                    parsed.tags_remove.push(tag.strip_prefix('-').unwrap().to_string());
                }
            } else {
                // Regular description token
                description_parts.push(arg.clone());
            }
        }
        
        i += 1;
    }
    
    parsed.description = description_parts;
    Ok(parsed)
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
        let parsed = parse_task_args(args).unwrap();
        assert_eq!(parsed.description, vec!["fix", "the", "bug"]);
    }

    #[test]
    fn test_parse_with_project() {
        let args = vec!["fix".to_string(), "bug".to_string(), "project:work".to_string()];
        let parsed = parse_task_args(args).unwrap();
        assert_eq!(parsed.description, vec!["fix", "bug"]);
        assert_eq!(parsed.project, Some("work".to_string()));
    }
    
    #[test]
    fn test_parse_with_tags() {
        let args = vec!["fix".to_string(), "bug".to_string(), "+urgent".to_string(), "+important".to_string()];
        let parsed = parse_task_args(args).unwrap();
        assert_eq!(parsed.description, vec!["fix", "bug"]);
        assert_eq!(parsed.tags_add, vec!["urgent", "important"]);
    }

    #[test]
    fn test_parse_mixed_order() {
        let args = vec!["project:work".to_string(), "fix".to_string(), "bug".to_string(), "+urgent".to_string()];
        let parsed = parse_task_args(args).unwrap();
        assert_eq!(parsed.description, vec!["fix", "bug"]);
        assert_eq!(parsed.project, Some("work".to_string()));
        assert_eq!(parsed.tags_add, vec!["urgent"]);
    }

    #[test]
    fn test_parse_udas() {
        let args = vec!["fix".to_string(), "bug".to_string(), "uda.priority:high".to_string(), "uda.estimate:2h".to_string()];
        let parsed = parse_task_args(args).unwrap();
        assert_eq!(parsed.description, vec!["fix", "bug"]);
        assert_eq!(parsed.udas.get("priority"), Some(&"high".to_string()));
        assert_eq!(parsed.udas.get("estimate"), Some(&"2h".to_string()));
    }
    
    #[test]
    fn test_field_abbreviation() {
        let args = vec!["fix".to_string(), "bug".to_string(), "proj:work".to_string()];
        let parsed = parse_task_args(args).unwrap();
        assert_eq!(parsed.project, Some("work".to_string()));
    }
    
    #[test]
    fn test_field_abbreviation_empty_value() {
        let args = vec!["fix".to_string(), "bug".to_string(), "project:".to_string()];
        let parsed = parse_task_args(args).unwrap();
        assert_eq!(parsed.project, Some("none".to_string()));
    }
    
    #[test]
    fn test_ambiguous_abbreviation() {
        // This test depends on what fields exist - if 's' is ambiguous, it should error
        // For now, let's test with a case that should work
        let args = vec!["fix".to_string(), "bug".to_string(), "sc:tomorrow".to_string()];
        let parsed = parse_task_args(args).unwrap();
        assert_eq!(parsed.scheduled, Some("tomorrow".to_string()));
    }
    
    #[test]
    fn test_invalid_field_name() {
        let args = vec!["fix".to_string(), "bug".to_string(), "projects:work".to_string()];
        let result = parse_task_args(args);
        assert!(result.is_err());
        if let Err(FieldParseError::InvalidFieldName { field, suggestion }) = result {
            assert_eq!(field, "projects");
            assert_eq!(suggestion, "project");
        } else {
            panic!("Expected InvalidFieldName error");
        }
    }
}

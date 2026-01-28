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
    pub respawn: Option<String>,
    pub tags_add: Vec<String>,
    pub tags_remove: Vec<String>,
    pub udas: HashMap<String, String>,
}

/// Field name abbreviation error
#[derive(Debug)]
pub enum FieldParseError {
    InvalidFieldName {
        field: String,
        suggestion: String,
    },
    ReadOnlyField {
        field: String,
        hint: String,
    },
    UnknownFieldToken {
        token: String,
    },
    InvalidTag {
        message: String,
    },
}

impl std::fmt::Display for FieldParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldParseError::InvalidFieldName { field, suggestion } => {
                write!(f, "Unrecognized field name '{}'\n  Did you mean '{}'?", field, suggestion)
            }
            FieldParseError::ReadOnlyField { field, hint } => {
                write!(f, "Field '{}' cannot be modified directly.\n  {}", field, hint)
            }
            FieldParseError::UnknownFieldToken { token } => {
                write!(f, "Unrecognized field token '{}'\n  If this is meant to be part of the description, remove the equals sign or quote the entire description.", token)
            }
            FieldParseError::InvalidTag { message } => {
                write!(f, "{}", message)
            }
        }
    }
}

/// Valid field names (exact match only, no abbreviations)
const FIELD_NAMES: &[&str] = &[
    "project",
    "due",
    "scheduled",
    "wait",
    "allocation",
    "template",
    "respawn",
];

/// Fields that are read-only (cannot be modified via modify command)
/// These exist to give helpful error messages when users try to modify them
const READ_ONLY_FIELDS: &[&str] = &[
    "status",      // Use finish/close commands instead
    "created",     // Immutable
    "modified",    // Automatically updated
    "id",          // Immutable
];

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

/// Get hint for read-only field
fn get_read_only_hint(field: &str) -> String {
    match field.to_lowercase().as_str() {
        "status" => "Use 'tatl finish' to complete, 'tatl close' to close, or 'tatl reopen' to reopen a task.".to_string(),
        "created" => "Created timestamp is set automatically and cannot be changed.".to_string(),
        "modified" => "Modified timestamp is updated automatically.".to_string(),
        "id" => "Task ID is assigned automatically and cannot be changed.".to_string(),
        _ => "This field is read-only.".to_string(),
    }
}

/// Parse a field token (field=value)
/// Returns the field name and value
/// Handles empty values (field=) by converting to field=none
fn parse_field_token(token: &str) -> Result<Option<(String, String)>, FieldParseError> {
    if let Some(eq_pos) = token.find('=') {
        let field = token[..eq_pos].to_string();
        let value = token[eq_pos + 1..].to_string();

        // Handle empty value (field=) -> treat as field=none
        let final_value = if value.is_empty() {
            "none".to_string()
        } else {
            value
        };

        // Check for read-only fields first
        if READ_ONLY_FIELDS.iter().any(|f| f.eq_ignore_ascii_case(&field)) {
            return Err(FieldParseError::ReadOnlyField {
                field: field.clone(),
                hint: get_read_only_hint(&field),
            });
        }

        // Exact match only
        if FIELD_NAMES.contains(&field.as_str()) {
            return Ok(Some((field, final_value)));
        }

        // No exact match - try fuzzy matching for typo suggestions
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
    } else {
        Ok(None)
    }
}

/// Parse task add/modify arguments
/// Description is tokens that don't match field patterns, tag patterns, or flags
/// Field tokens, tag tokens, and flags can appear anywhere in the argument list
/// Returns Result to handle field parse errors
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

        // Check for field tokens (field=value)
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
                    "respawn" => parsed.respawn = Some(value),
                    _ => {
                        // Check if it's a UDA (uda.<key>=<value>)
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
                if arg.contains('=') && arg.starts_with("uda.") {
                    // UDA format - parse manually
                    if let Some(eq_pos) = arg.find('=') {
                        let field = &arg[..eq_pos];
                        let value = &arg[eq_pos + 1..];
                        let key = field.strip_prefix("uda.").unwrap().to_string();
                        if value == "none" {
                            // UDA clearing - handled separately
                        } else {
                            parsed.udas.insert(key, value.to_string());
                        }
                        handled = true;
                    }
                } else if arg.contains('=') && !arg.starts_with('+') && !arg.starts_with('-') {
                    // Looks like a field token (contains =) but wasn't recognized
                    // This is likely a typo or unknown field
                    if let Some(eq_pos) = arg.find('=') {
                        let potential_field = &arg[..eq_pos];
                        // If the potential field name is alphabetic and looks like a field, it's probably a typo
                        if potential_field.chars().all(|c| c.is_ascii_alphabetic() || c == '_' || c == '.')
                           && potential_field.len() >= 2
                           && !potential_field.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                            return Err(FieldParseError::UnknownFieldToken {
                                token: arg.clone(),
                            });
                        }
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
            if arg.starts_with('+') || arg.starts_with('-') {
                // This looks like a tag token
                if let Some(tag) = parse_tag_token(arg) {
                    if tag.starts_with('+') {
                        parsed.tags_add.push(tag.strip_prefix('+').unwrap().to_string());
                    } else if tag.starts_with('-') {
                        parsed.tags_remove.push(tag.strip_prefix('-').unwrap().to_string());
                    }
                } else {
                    // Tag syntax used but invalid
                    let tag_part = &arg[1..];
                    if tag_part.is_empty() {
                        return Err(FieldParseError::InvalidTag {
                            message: "Tag name cannot be empty. Use '+tagname' to add a tag.".to_string(),
                        });
                    } else {
                        return Err(FieldParseError::InvalidTag {
                            message: format!("Invalid tag '{}'. Tags can only contain letters, numbers, underscores, hyphens, and dots.", tag_part),
                        });
                    }
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
/// Returns None if:
/// - Token doesn't start with + or -
/// - Tag name is empty (just "+" or "-")
/// - Tag name contains invalid characters
fn parse_tag_token(token: &str) -> Option<String> {
    if token.starts_with('+') || token.starts_with('-') {
        // Validate tag charset: [A-Za-z0-9_\-\.]+
        let tag_part = &token[1..];

        // Tag name cannot be empty
        if tag_part.is_empty() {
            return None;
        }

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
        let args = vec!["fix".to_string(), "bug".to_string(), "project=work".to_string()];
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
        let args = vec!["project=work".to_string(), "fix".to_string(), "bug".to_string(), "+urgent".to_string()];
        let parsed = parse_task_args(args).unwrap();
        assert_eq!(parsed.description, vec!["fix", "bug"]);
        assert_eq!(parsed.project, Some("work".to_string()));
        assert_eq!(parsed.tags_add, vec!["urgent"]);
    }

    #[test]
    fn test_parse_udas() {
        let args = vec!["fix".to_string(), "bug".to_string(), "uda.priority=high".to_string(), "uda.estimate=2h".to_string()];
        let parsed = parse_task_args(args).unwrap();
        assert_eq!(parsed.description, vec!["fix", "bug"]);
        assert_eq!(parsed.udas.get("priority"), Some(&"high".to_string()));
        assert_eq!(parsed.udas.get("estimate"), Some(&"2h".to_string()));
    }

    #[test]
    fn test_field_empty_value() {
        let args = vec!["fix".to_string(), "bug".to_string(), "project=".to_string()];
        let parsed = parse_task_args(args).unwrap();
        assert_eq!(parsed.project, Some("none".to_string()));
    }

    #[test]
    fn test_invalid_field_name() {
        let args = vec!["fix".to_string(), "bug".to_string(), "projects=work".to_string()];
        let result = parse_task_args(args);
        assert!(result.is_err());
        if let Err(FieldParseError::InvalidFieldName { field, suggestion }) = result {
            assert_eq!(field, "projects");
            assert_eq!(suggestion, "project");
        } else {
            panic!("Expected InvalidFieldName error");
        }
    }

    #[test]
    fn test_time_expressions_not_confused_with_fields() {
        // Time expressions like 09:00 contain : but no = so should not be parsed as fields
        let args = vec!["meeting".to_string(), "at".to_string(), "09:00".to_string()];
        let parsed = parse_task_args(args).unwrap();
        assert_eq!(parsed.description, vec!["meeting", "at", "09:00"]);
    }
}

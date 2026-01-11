// Error handling utilities for consistent error messages and exit codes

use std::process;

/// Exit with a user error (exit code 1)
/// User errors are for invalid input, missing resources, etc.
pub fn user_error(message: &str) -> ! {
    eprintln!("Error: {}", message);
    process::exit(1);
}

/// Exit with an internal error (exit code >1)
/// Internal errors are for unexpected system failures, database corruption, etc.
pub fn internal_error(message: &str) -> ! {
    eprintln!("Internal error: {}", message);
    process::exit(2);
}

/// Format a user error message with context
pub fn user_error_with_context(message: &str, context: &str) -> ! {
    eprintln!("Error: {} ({})", message, context);
    process::exit(1);
}

/// Format an internal error message with context
pub fn internal_error_with_context(message: &str, context: &str) -> ! {
    eprintln!("Internal error: {} ({})", message, context);
    process::exit(2);
}

/// Validate that a string is not empty
pub fn validate_non_empty(value: &str, field_name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{} cannot be empty", field_name))
    } else {
        Ok(())
    }
}

/// Validate that a task ID is valid (positive integer)
pub fn validate_task_id(id_str: &str) -> Result<i64, String> {
    id_str.parse::<i64>()
        .map_err(|_| format!("Invalid task ID: '{}'. Task ID must be a number.", id_str))
        .and_then(|id| {
            if id > 0 {
                Ok(id)
            } else {
                Err(format!("Invalid task ID: {}. Task ID must be positive.", id))
            }
        })
}

/// Validate that a stack index is valid (non-negative integer)
pub fn validate_stack_index(index_str: &str) -> Result<i32, String> {
    index_str.parse::<i32>()
        .map_err(|_| format!("Invalid stack index: '{}'. Index must be a number.", index_str))
        .and_then(|idx| {
            if idx >= 0 {
                Ok(idx)
            } else {
                Err(format!("Invalid stack index: {}. Index must be non-negative.", idx))
            }
        })
}

/// Validate project name format (alphanumeric, dots, underscores, hyphens)
pub fn validate_project_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Project name cannot be empty".to_string());
    }
    
    // Allow alphanumeric, dots, underscores, hyphens (for nested projects)
    if name.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-') {
        Ok(())
    } else {
        Err(format!("Invalid project name: '{}'. Project names can only contain letters, numbers, dots, underscores, and hyphens.", name))
    }
}

/// Validate tag format
pub fn validate_tag(tag: &str) -> Result<(), String> {
    if tag.trim().is_empty() {
        return Err("Tag cannot be empty".to_string());
    }
    
    // Tag charset: [A-Za-z0-9_\-\.]+
    if tag.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.') {
        Ok(())
    } else {
        Err(format!("Invalid tag: '{}'. Tags can only contain letters, numbers, underscores, hyphens, and dots.", tag))
    }
}

/// Validate UDA key format
pub fn validate_uda_key(key: &str) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err("UDA key cannot be empty".to_string());
    }
    
    // UDA key format: [A-Za-z0-9_\-\.]+ (same as tags)
    if key.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.') {
        Ok(())
    } else {
        Err(format!("Invalid UDA key: '{}'. UDA keys can only contain letters, numbers, underscores, hyphens, and dots.", key))
    }
}

/// Validate template name format
pub fn validate_template_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Template name cannot be empty".to_string());
    }
    
    // Template names: alphanumeric, dots, underscores, hyphens
    if name.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-') {
        Ok(())
    } else {
        Err(format!("Invalid template name: '{}'. Template names can only contain letters, numbers, dots, underscores, and hyphens.", name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_non_empty() {
        assert!(validate_non_empty("test", "field").is_ok());
        assert!(validate_non_empty("", "field").is_err());
        assert!(validate_non_empty("   ", "field").is_err());
    }

    #[test]
    fn test_validate_task_id() {
        assert_eq!(validate_task_id("1"), Ok(1));
        assert_eq!(validate_task_id("42"), Ok(42));
        assert!(validate_task_id("0").is_err());
        assert!(validate_task_id("-1").is_err());
        assert!(validate_task_id("abc").is_err());
        assert!(validate_task_id("").is_err());
    }

    #[test]
    fn test_validate_stack_index() {
        assert_eq!(validate_stack_index("0"), Ok(0));
        assert_eq!(validate_stack_index("5"), Ok(5));
        assert!(validate_stack_index("-1").is_err());
        assert!(validate_stack_index("abc").is_err());
    }

    #[test]
    fn test_validate_project_name() {
        assert!(validate_project_name("work").is_ok());
        assert!(validate_project_name("admin.email").is_ok());
        assert!(validate_project_name("sales_north").is_ok());
        assert!(validate_project_name("test-project").is_ok());
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("work@home").is_err());
        assert!(validate_project_name("work home").is_err());
    }

    #[test]
    fn test_validate_tag() {
        assert!(validate_tag("urgent").is_ok());
        assert!(validate_tag("work-home").is_ok());
        assert!(validate_tag("test_tag").is_ok());
        assert!(validate_tag("").is_err());
        assert!(validate_tag("work@home").is_err());
        assert!(validate_tag("work home").is_err());
    }
}

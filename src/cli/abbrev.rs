// Command abbreviation matching for Task Ninja CLI

/// Find all commands that start with the given prefix (case-insensitive)
pub fn find_matching_commands<'a>(prefix: &str, commands: &'a [&str]) -> Vec<&'a str> {
    let prefix_lower = prefix.to_lowercase();
    commands.iter()
        .filter(|cmd| cmd.to_lowercase().starts_with(&prefix_lower))
        .copied()
        .collect()
}

/// Find a unique command match for the given prefix
/// Returns Ok(command) if exactly one match, Err(matches) if ambiguous, Err(empty) if no match
pub fn find_unique_command<'a>(prefix: &str, commands: &'a [&str]) -> Result<&'a str, Vec<&'a str>> {
    let matches = find_matching_commands(prefix, commands);
    
    if matches.is_empty() {
        Err(Vec::new())
    } else if matches.len() == 1 {
        Ok(matches[0])
    } else {
        Err(matches)
    }
}

/// Top-level commands in Task Ninja
pub const TOP_LEVEL_COMMANDS: &[&str] = &[
    "projects", "add", "list", "modify", "stack", "clock", 
    "annotate", "done", "recur", "sessions"
];

/// Project subcommands
pub const PROJECT_COMMANDS: &[&str] = &[
    "add", "list", "rename", "archive", "unarchive"
];

/// Stack subcommands
pub const STACK_COMMANDS: &[&str] = &[
    "show", "pick", "roll", "drop", "clear"
];

/// Clock subcommands
pub const CLOCK_COMMANDS: &[&str] = &[
    "in", "out"
];

/// Recur subcommands
pub const RECUR_COMMANDS: &[&str] = &[
    "run"
];

/// Sessions subcommands
pub const SESSIONS_COMMANDS: &[&str] = &[
    "list", "show"
];

/// Get subcommands for a given top-level command
pub fn get_subcommands(command: &str) -> Option<&'static [&'static str]> {
    match command {
        "projects" => Some(PROJECT_COMMANDS),
        "stack" => Some(STACK_COMMANDS),
        "clock" => Some(CLOCK_COMMANDS),
        "recur" => Some(RECUR_COMMANDS),
        "sessions" => Some(SESSIONS_COMMANDS),
        _ => None,
    }
}

/// Expand command abbreviations in argument list
/// Returns expanded args or error message
pub fn expand_command_abbreviations(args: Vec<String>) -> Result<Vec<String>, String> {
    if args.is_empty() {
        return Ok(args);
    }
    
    let mut expanded = Vec::new();
    let mut i = 0;
    
    while i < args.len() {
        let arg = &args[i];
        
        // Check if this is a top-level command (not a flag, not a number, not already expanded)
        if i == 0 && !arg.starts_with('-') && arg.parse::<i64>().is_err() {
            // Try to expand top-level command
            match find_unique_command(arg, TOP_LEVEL_COMMANDS) {
                Ok(full_cmd) => {
                    expanded.push(full_cmd.to_string());
                    
                    // If this command has subcommands, check the next arg
                    if let Some(subcommands) = get_subcommands(full_cmd) {
                        if i + 1 < args.len() {
                            let next_arg = &args[i + 1];
                            // Check if next arg is a subcommand (not a flag, not a number)
                            if !next_arg.starts_with('-') && next_arg.parse::<i64>().is_err() {
                                match find_unique_command(next_arg, subcommands) {
                                    Ok(full_subcmd) => {
                                        expanded.push(full_subcmd.to_string());
                                        i += 2;
                                        continue;
                                    }
                                    Err(matches) => {
                                        if matches.is_empty() {
                                            // No match - might be a filter or other arg, pass through
                                            expanded.push(next_arg.clone());
                                            i += 2;
                                            continue;
                                        } else {
                                            // Ambiguous subcommand
                                            let match_list = matches.join(", ");
                                            return Err(format!(
                                                "Ambiguous subcommand '{}'. Did you mean one of: {}?",
                                                next_arg, match_list
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    i += 1;
                    continue;
                }
                Err(matches) => {
                    if matches.is_empty() {
                        // No match - might be a filter or task ID, pass through
                        expanded.push(arg.clone());
                        i += 1;
                        continue;
                    } else {
                        // Ambiguous command
                        let match_list = matches.join(", ");
                        return Err(format!(
                            "Ambiguous command '{}'. Did you mean one of: {}?",
                            arg, match_list
                        ));
                    }
                }
            }
        }
        
        // Not a command to expand, pass through
        expanded.push(arg.clone());
        i += 1;
    }
    
    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_find_matching_commands() {
        let commands = &["list", "list-all", "list-tasks"];
        assert_eq!(find_matching_commands("l", commands), vec!["list", "list-all", "list-tasks"]);
        assert_eq!(find_matching_commands("li", commands), vec!["list", "list-all", "list-tasks"]);
        assert_eq!(find_matching_commands("list", commands), vec!["list", "list-all", "list-tasks"]);
        assert_eq!(find_matching_commands("list-", commands), vec!["list-all", "list-tasks"]);
    }
    
    #[test]
    fn test_find_unique_command() {
        let commands = &["list", "list-all", "modify"];
        assert_eq!(find_unique_command("m", commands), Ok("modify"));
        assert_eq!(find_unique_command("mod", commands), Ok("modify"));
        assert_eq!(find_unique_command("modify", commands), Ok("modify"));
        
        let matches = find_unique_command("l", commands);
        assert!(matches.is_err());
        if let Err(matches) = matches {
            assert_eq!(matches.len(), 2);
        }
    }
    
    #[test]
    fn test_expand_command_abbreviations() {
        // Test top-level abbreviation
        assert_eq!(
            expand_command_abbreviations(vec!["l".to_string()]),
            Ok(vec!["list".to_string()])
        );
        
        // Test subcommand abbreviation (use "ad" which uniquely matches "add")
        assert_eq!(
            expand_command_abbreviations(vec!["proj".to_string(), "ad".to_string(), "test".to_string()]),
            Ok(vec!["projects".to_string(), "add".to_string(), "test".to_string()])
        );
        
        // Test ambiguous top-level command
        let result = expand_command_abbreviations(vec!["a".to_string()]);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("Ambiguous"));
        }
        
        // Test ambiguous subcommand
        let result = expand_command_abbreviations(vec!["proj".to_string(), "a".to_string()]);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("Ambiguous subcommand"));
        }
    }
}

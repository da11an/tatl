use clap::{Parser, Subcommand};
use rusqlite::Connection;
use crate::db::DbConnection;
use crate::models::Task;
use crate::repo::{ProjectRepo, TaskRepo, StackRepo, SessionRepo, AnnotationRepo, TemplateRepo, ViewRepo};
use crate::cli::parser::{parse_task_args, join_description};
use crate::cli::commands_sessions::{handle_task_sessions_list_with_filter, handle_task_sessions_show_with_filter, handle_sessions_modify, handle_sessions_delete, handle_sessions_add, handle_sessions_report};
use crate::cli::output::{format_task_list_table, format_task_summary, TaskListOptions};
use crate::cli::error::{user_error, validate_task_id, validate_project_name, parse_task_id_spec, parse_task_id_list};
use crate::utils::{parse_date_expr, parse_duration, fuzzy};
use crate::filter::{parse_filter, filter_tasks};
use crate::recur::RecurGenerator;
use crate::cli::abbrev;
use chrono::{Local, TimeZone, Datelike};
use std::collections::HashMap;
use anyhow::{Context, Result};

#[derive(Parser)]
#[command(name = "tatl")]
#[command(about = "Task and Time Ledger - A powerful command-line task and time tracking tool")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Project management commands
    Projects {
        #[command(subcommand)]
        subcommand: ProjectCommands,
    },
    /// Add a new task
    Add {
        /// Automatically start timing after creating task
        #[arg(long = "on", visible_alias = "clock-in")]
        start_timing: bool,
        /// Automatically enqueue task to clock stack after creating
        #[arg(long = "enqueue")]
        enqueue: bool,
        /// Auto-confirm prompts (e.g., create new projects)
        #[arg(short = 'y', long)]
        yes: bool,
        /// Task description and fields (e.g., "fix bug project:work +urgent")
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// List tasks
    List {
        /// Filter arguments (e.g., "project:work +urgent")
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        filter: Vec<String>,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show Due dates as relative time (e.g., "2 days ago", "in 3 days")
        #[arg(long)]
        relative: bool,
    },
    /// Show detailed summary of task(s)
    Show {
        /// Task ID, range, or filter
        target: String,
    },
    /// Modify tasks
    Modify {
        /// Task ID or filter
        target: String,
        /// Modification arguments (description, fields, tags)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        /// Apply to all matching tasks without confirmation
        #[arg(short = 'y', long)]
        yes: bool,
        /// Force one-by-one confirmation for each task
        #[arg(long)]
        interactive: bool,
        /// Start timing after modification
        #[arg(long = "on")]
        start_timing: bool,
    },
    /// Start timing a task
    On {
        /// Task ID (optional, defaults to queue[0])
        task_id: Option<String>,
        /// Time expression or interval (e.g., "09:00" or "09:00..11:00")
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        time_args: Vec<String>,
    },
    /// Stop timing current task
    Off {
        /// End time (optional, defaults to now)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        time_args: Vec<String>,
    },
    /// Remove task from queue without finishing
    Dequeue {
        /// Task ID (optional, defaults to queue[0])
        task_id: Option<String>,
    },
    /// Annotate a task
    Annotate {
        /// Task ID (optional when clocked in)
        target: Option<String>,
        /// Annotation note text
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        note: Vec<String>,
        /// Override task selection
        #[arg(long)]
        task: Option<String>,
        /// Apply to all matching tasks without confirmation
        #[arg(short = 'y', long)]
        yes: bool,
        /// Force one-by-one confirmation for each task
        #[arg(long)]
        interactive: bool,
        /// Delete annotation by ID
        #[arg(long)]
        delete: Option<String>,
    },
    /// Mark task(s) as finished
    Finish {
        /// Task ID or filter (optional, defaults to queue[0])
        target: Option<String>,
        /// End time expression (optional, defaults to now)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        time_args: Vec<String>,
        /// Start next task in queue after completion
        #[arg(long)]
        next: bool,
        /// Complete all matching tasks without confirmation
        #[arg(short = 'y', long)]
        yes: bool,
        /// Force one-by-one confirmation for each task
        #[arg(long)]
        interactive: bool,
    },
    /// Mark task(s) as closed
    Close {
        /// Task ID or filter (optional, defaults to queue[0])
        target: Option<String>,
        /// Close all matching tasks without confirmation
        #[arg(short = 'y', long)]
        yes: bool,
        /// Force one-by-one confirmation for each task
        #[arg(long)]
        interactive: bool,
    },
    /// Permanently delete task(s)
    Delete {
        /// Task ID or filter
        target: String,
        /// Delete all matching tasks without confirmation
        #[arg(short = 'y', long)]
        yes: bool,
        /// Confirm each task one by one
        #[arg(long)]
        interactive: bool,
    },
    /// Add task to end of clock stack
    Enqueue {
        /// Task ID(s) to enqueue (comma-separated list)
        task_id: String,
    },
    /// Recurrence management commands
    Recur {
        #[command(subcommand)]
        subcommand: RecurCommands,
    },
    /// Sessions management commands
    Sessions {
        #[command(subcommand)]
        subcommand: SessionsCommands,
        /// Task ID or filter (optional)
        #[arg(long)]
        task: Option<String>,
    },
    /// Show dashboard with system status
    Status {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Create a new project
    Add {
        /// Project name (supports nested projects with dot notation, e.g., admin.email)
        name: String,
    },
    /// List projects
    List {
        /// Include archived projects
        #[arg(long)]
        archived: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Rename a project
    Rename {
        /// Current project name
        old_name: String,
        /// New project name
        new_name: String,
        /// Force merge if new name already exists
        #[arg(long)]
        force: bool,
    },
    /// Archive a project
    Archive {
        /// Project name to archive
        name: String,
    },
    /// Unarchive a project
    Unarchive {
        /// Project name to unarchive
        name: String,
    },
}


#[derive(Subcommand)]
pub enum RecurCommands {
    /// Generate recurring task instances
    Run {
        /// Generate occurrences until this date (default: now + 14 days)
        #[arg(long)]
        until: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum SessionsCommands {
    /// List session history
    List {
        /// Filter arguments (e.g., "project:work +urgent")
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        filter: Vec<String>,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Show detailed session information
    Show,
    /// Modify session start/end times
    Modify {
        /// Session ID
        session_id: i64,
        /// Modification arguments (start:<expr>, end:<expr>)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        /// Apply modification without confirmation
        #[arg(short = 'y', long)]
        yes: bool,
        /// Allow modification even with conflicts
        #[arg(long)]
        force: bool,
    },
    /// Delete a session
    Delete {
        /// Session ID
        session_id: i64,
        /// Delete without confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Add a manual session (for sessions not recorded via clock)
    /// Syntax: task sessions add task:<id> start:<time> end:<time> [note:<note>]
    /// Or: task sessions add <id> <start> <end> [<note>]
    Add {
        /// Arguments: task:<id> start:<time> end:<time> [note:<note>] or positional <id> <start> <end> [<note>]
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Generate a time report summarizing hours by project
    Report {
        /// Start date/time for report period (e.g., "2024-01-01", "-7d")
        #[arg(value_name = "START", allow_hyphen_values = true)]
        start: Option<String>,
        /// End date/time for report period (defaults to now)
        #[arg(value_name = "END", allow_hyphen_values = true)]
        end: Option<String>,
    },
}


pub fn run() -> Result<()> {
    // Get raw args
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    
    // Check for version flag early (before any processing)
    if args.iter().any(|a| a == "--version" || a == "-V") {
        // Use clap to handle version display properly
        let cli = Cli::try_parse_from(std::env::args());
        match cli {
            Ok(_) => return Ok(()), // Version was printed by clap
            Err(_e) => {
                // If parsing fails, just print version manually
                println!("tatl {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
        }
    }
    
    // Expand command abbreviations before processing
    args = match abbrev::expand_command_abbreviations(args) {
        Ok(expanded) => expanded,
        Err(e) => {
            user_error(&e);
        }
    };
    
    // Normalize "task <id> clock in" to "task clock in <id>"
    if args.len() >= 3
        && args[0].parse::<i64>().is_ok()
        && args[1] == "clock"
        && args[2] == "in"
    {
        let task_id = args.remove(0);
        args.insert(2, task_id);
    }
    
    // Optional: Handle implicit defaults (task 1 → task show 1)
    // This is an optional extension - can be removed if not desired
    if args.len() == 1 {
        let first_arg = &args[0];
        // Check if it's a numeric ID or ID spec (not a global subcommand)
        let is_global_subcommand = matches!(first_arg.as_str(), 
            "projects" | "clock" | "recur" | "sessions" | "add" | "list" | "modify" | "annotate" | "finish" | "close" | "delete" | "show" | "status");
        
        if !is_global_subcommand {
            // Try to parse as task ID spec
            if parse_task_id_spec(first_arg).is_ok() || validate_task_id(first_arg).is_ok() {
                // It's a valid task ID or ID spec - prepend "show"
                args.insert(0, "show".to_string());
            }
        }
    }
    
    // Check for help requests or empty args (before clap parsing)
    let is_help_request = args.is_empty() || 
        args.iter().any(|a| a == "--help" || a == "-h" || a == "help");
    
    // Note: Status lines have been removed from individual commands.
    // Use `task status` command for a consolidated dashboard view.
    // If help would be shown, just show help normally
    if is_help_request {
        // Let clap handle the help (will exit after printing)
        match Cli::try_parse() {
            Ok(_) => return Ok(()),
            Err(e) => {
                e.print()?;
                return Ok(());
            }
        }
    }
    
    // Use clap parsing with expanded args
    // Build args vector with program name for clap
    let clap_args = std::iter::once("tatl".to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    let cli = match Cli::try_parse_from(clap_args) {
        Ok(cli) => cli,
        Err(e) => {
            e.print()?;
            return Ok(());
        }
    };
    
    handle_command(cli)
}

fn handle_command(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Projects { subcommand } => handle_projects(subcommand),
        Commands::Add { args, start_timing, enqueue, yes } => handle_task_add(args, start_timing, enqueue, yes),
        Commands::List { filter, json, relative } => {
            handle_task_list(filter, json, relative)
        },
        Commands::Show { target } => handle_task_summary(target),
        Commands::Modify { target, args, yes, interactive, start_timing } => {
            handle_task_modify_with_on(target, args, yes, interactive, start_timing)
        }
        Commands::On { task_id, time_args } => handle_on(task_id, time_args),
        Commands::Off { time_args } => handle_off(time_args),
        Commands::Dequeue { task_id } => handle_dequeue(task_id),
        Commands::Annotate { target, note, task, yes, interactive, delete } => {
            if let Some(annotation_id) = delete {
                let target = target.or(task)
                    .unwrap_or_else(|| user_error("Task ID is required to delete an annotation."));
                handle_annotation_delete(target, annotation_id)
            } else {
                if target.is_none() && task.is_none() && note.is_empty() {
                    let help_args = vec!["tatl".to_string(), "annotate".to_string(), "--help".to_string()];
                    let _ = Cli::try_parse_from(help_args);
                    return Ok(());
                }
                let mut note_args = note;
                if let Some(target_token) = target {
                    if task.is_some() {
                        note_args.insert(0, target_token);
                        handle_annotation_add(task, note_args)
                    } else if let Ok(task_id) = validate_task_id(&target_token) {
                        let conn = DbConnection::connect()
                            .context("Failed to connect to database")?;
                        if TaskRepo::get_by_id(&conn, task_id)?.is_some() {
                            handle_annotation_add(Some(target_token), note_args)
                        } else {
                            let open_session = SessionRepo::get_open(&conn)?;
                            if open_session.is_some() {
                                note_args.insert(0, target_token);
                                handle_annotation_add(None, note_args)
                            } else {
                                user_error(&format!("Task {} not found", task_id));
                            }
                        }
                    } else if looks_like_filter(&target_token) {
                        handle_annotation_add_with_filter(target_token, note_args, yes, interactive)
                    } else {
                        note_args.insert(0, target_token);
                        handle_annotation_add(None, note_args)
                    }
                } else {
                    handle_annotation_add(task, note_args)
                }
            }
        }
        Commands::Finish { target, time_args, next, yes, interactive } => {
            // Convert time_args to optional end time
            let end_time = if time_args.is_empty() { None } else { Some(time_args.join(" ")) };
            handle_task_finish(target, end_time, next, yes, interactive)
        }
        Commands::Close { target, yes, interactive } => {
            handle_task_close_optional(target, yes, interactive)
        }
        Commands::Delete { target, yes, interactive } => {
            handle_task_delete(target, yes, interactive)
        }
        Commands::Enqueue { task_id } => {
            handle_task_enqueue(task_id)
        }
        Commands::Recur { subcommand } => {
            handle_recur(subcommand)
        }
        Commands::Sessions { subcommand, task } => {
            match subcommand {
                SessionsCommands::List { filter, json } => {
                    // If filter arguments provided, use them; otherwise fall back to --task flag for backward compatibility
                    if !filter.is_empty() {
                        handle_task_sessions_list_with_filter(filter, json)
                    } else if let Some(task_str) = task {
                        // Backward compatibility: support --task flag
                        handle_task_sessions_list_with_filter(vec![task_str], json)
                    } else {
                        handle_task_sessions_list_with_filter(vec![], json)
                    }
                }
                SessionsCommands::Show => {
                    handle_task_sessions_show_with_filter(task)
                }
                SessionsCommands::Modify { session_id, args, yes, force } => {
                    handle_sessions_modify(session_id, args, yes, force)
                }
                SessionsCommands::Delete { session_id, yes } => {
                    handle_sessions_delete(session_id, yes)
                }
                SessionsCommands::Add { args } => {
                    handle_sessions_add(args)
                }
                SessionsCommands::Report { start, end } => {
                    handle_sessions_report(start, end)
                }
            }
        }
        Commands::Status { json } => {
            handle_status(json)
        }
    }
}

/// Prompt user to create a new project
/// Returns: Some(true) if project should be created, Some(false) if skipped, None if cancelled
fn prompt_create_project(project_name: &str) -> Result<Option<bool>> {
    eprint!("This is a new project '{}'. Add new project? [y/n/c] (default: y): ", project_name);
    std::io::Write::flush(&mut std::io::stderr())
        .map_err(|e| anyhow::anyhow!("Failed to flush stderr: {}", e))?;
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)
        .map_err(|e| anyhow::anyhow!("Failed to read input: {}", e))?;
    
    let input = input.trim().to_lowercase();
    match input.as_str() {
        "y" | "yes" | "" => Ok(Some(true)),  // Empty input defaults to yes
        "n" | "no" => Ok(Some(false)),
        "c" | "cancel" => Ok(None),
        _ => {
            println!("Invalid response. Cancelled.");
            Ok(None)
        }
    }
}

/// Generate enhanced error message for project not found
fn project_not_found_error(conn: &Connection, project_name: &str) -> ! {
    // Get all projects (active first, then archived)
    let active_projects = ProjectRepo::list(conn, false)
        .unwrap_or_else(|_| Vec::new());
    let archived_projects = ProjectRepo::list(conn, true)
        .unwrap_or_else(|_| Vec::new());
    
    // Prepare project list: active first, then archived
    let mut all_projects: Vec<(String, bool)> = active_projects.iter()
        .map(|p| (p.name.clone(), p.is_archived))
        .collect();
    let mut archived_list: Vec<(String, bool)> = archived_projects.iter()
        .filter(|p| p.is_archived)
        .map(|p| (p.name.clone(), p.is_archived))
        .collect();
    all_projects.append(&mut archived_list);
    
    // Find near matches (max distance 3)
    let matches = fuzzy::find_near_project_matches(project_name, &all_projects, 3);
    
    if matches.is_empty() {
        // No near match found
        user_error(&format!("Project '{}' not found. To add: task projects add {}", project_name, project_name));
    } else {
        // Near matches found
        let match_names: Vec<String> = matches.iter().map(|(name, _)| format!("'{}'", name)).collect();
        let match_str = match_names.join(", ");
        user_error(&format!("Project '{}' not found. Did you mean {}?", project_name, match_str));
    }
}

fn handle_projects(cmd: ProjectCommands) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    match cmd {
        ProjectCommands::Add { name } => {
            // Validate project name
            if let Err(e) = validate_project_name(&name) {
                user_error(&e);
            }
            
            // Check if project already exists
            if let Some(_) = ProjectRepo::get_by_name(&conn, &name)? {
                user_error(&format!("Project '{}' already exists", name));
            }
            
            let project = ProjectRepo::create(&conn, &name)
                .map_err(|e| anyhow::anyhow!("Failed to create project: {}", e))?;
            
            println!("Created project '{}' (id: {})", project.name, project.id.unwrap());
            Ok(())
        }
        ProjectCommands::List { archived, json } => {
            let projects = ProjectRepo::list(&conn, archived)
                .context("Failed to list projects")?;
            
            if json {
                // JSON output - enhanced schema
                let json_projects: Vec<serde_json::Value> = projects.iter().map(|project| {
                    serde_json::json!({
                        "id": project.id,
                        "name": project.name,
                        "is_archived": project.is_archived,
                        "created_ts": project.created_ts,
                        "modified_ts": project.modified_ts,
                    })
                }).collect();
                println!("{}", serde_json::to_string_pretty(&json_projects)?);
            } else {
                // Human-readable table output
                if projects.is_empty() {
                    println!("No projects found.");
                } else {
                    println!("{:<6} {:<40} {:<10}", "ID", "Name", "Status");
                    println!("{}", "-".repeat(56));
                    for project in projects {
                        let status = if project.is_archived { "[archived]" } else { "[active]" };
                        println!("{:<6} {:<40} {:<10}", 
                            project.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()),
                            project.name,
                            status);
                    }
                }
            }
            Ok(())
        }
        ProjectCommands::Rename { old_name, new_name, force } => {
            // Validate project names
            if let Err(e) = validate_project_name(&old_name) {
                user_error(&e);
            }
            if let Err(e) = validate_project_name(&new_name) {
                user_error(&e);
            }
            
            // Check if old project exists
            if ProjectRepo::get_by_name(&conn, &old_name)?.is_none() {
                project_not_found_error(&conn, &old_name);
            }
            
            // Check if new name already exists
            if let Some(_) = ProjectRepo::get_by_name(&conn, &new_name)? {
                if force {
                    // Merge projects
                    ProjectRepo::merge(&conn, &old_name, &new_name)
                        .context("Failed to merge projects")?;
                    println!("Merged project '{}' into '{}'", old_name, new_name);
                } else {
                    user_error(&format!("Project '{}' already exists. Use --force to merge.", new_name));
                }
            } else {
                // Simple rename
                ProjectRepo::rename(&conn, &old_name, &new_name)
                    .context("Failed to rename project")?;
                println!("Renamed project '{}' to '{}'", old_name, new_name);
            }
            Ok(())
        }
        ProjectCommands::Archive { name } => {
            ProjectRepo::archive(&conn, &name)
                .context("Failed to archive project")?;
            println!("Archived project '{}'", name);
            Ok(())
        }
        ProjectCommands::Unarchive { name } => {
            ProjectRepo::unarchive(&conn, &name)
                .context("Failed to unarchive project")?;
            println!("Unarchived project '{}'", name);
            Ok(())
        }
    }
}

fn handle_task_add(mut args: Vec<String>, mut start_timing: bool, mut enqueue: bool, auto_yes: bool) -> Result<()> {
    // Extract --on and --enqueue flags from args if they appear after the description
    // (CLAP limitation: with trailing_var_arg, flags after args are treated as part of args)
    let mut filtered_args = Vec::new();
    for arg in args.iter() {
        if arg == "--on" || arg == "--clock-in" {
            start_timing = true;
            // Don't include it in the args passed to parse_task_args
        } else if arg == "--enqueue" {
            enqueue = true;
            // Don't include it in the args passed to parse_task_args
        } else {
            filtered_args.push(arg.clone());
        }
    }
    args = filtered_args;
    
    if args.is_empty() {
        user_error("Task description is required");
    }
    
    let parsed = match parse_task_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    
    // Validate description
    if parsed.description.is_empty() {
        user_error("Task description is required");
    }
    
    let description = join_description(&parsed.description);
    
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // Resolve project
    let project_id = if let Some(project_name) = parsed.project {
        let project = ProjectRepo::get_by_name(&conn, &project_name)?;
        if let Some(p) = project {
            Some(p.id.unwrap())
        } else {
            // Project doesn't exist - prompt user or auto-create
            if auto_yes {
                // Auto-create project (-y flag)
                if let Err(e) = validate_project_name(&project_name) {
                    user_error(&e);
                }
                let project = ProjectRepo::create(&conn, &project_name)
                    .map_err(|e| anyhow::anyhow!("Failed to create project: {}", e))?;
                println!("Created project '{}' (id: {})", project.name, project.id.unwrap());
                Some(project.id.unwrap())
            } else {
                // Interactive prompt
                match prompt_create_project(&project_name)? {
                    Some(true) => {
                        // User said yes - create project
                        if let Err(e) = validate_project_name(&project_name) {
                            user_error(&e);
                        }
                        let project = ProjectRepo::create(&conn, &project_name)
                            .map_err(|e| anyhow::anyhow!("Failed to create project: {}", e))?;
                        println!("Created project '{}' (id: {})", project.name, project.id.unwrap());
                        Some(project.id.unwrap())
                    }
                    Some(false) => {
                        // User said no - skip project, create task without it
                        None
                    }
                    None => {
                        // User cancelled
                        println!("Cancelled.");
                        return Ok(());
                    }
                }
            }
        }
    } else {
        None
    };
    
    // Parse dates (simplified for MVP)
    let due_ts = if let Some(due) = parsed.due {
        Some(parse_date_expr(&due).context("Failed to parse due date")?)
    } else {
        None
    };
    
    let scheduled_ts = if let Some(scheduled) = parsed.scheduled {
        Some(parse_date_expr(&scheduled).context("Failed to parse scheduled date")?)
    } else {
        None
    };
    
    let wait_ts = if let Some(wait) = parsed.wait {
        Some(parse_date_expr(&wait).context("Failed to parse wait date")?)
    } else {
        None
    };
    
    // Parse duration
    let alloc_secs = if let Some(allocation) = parsed.allocation {
        Some(parse_duration(&allocation).context("Failed to parse allocation duration")?)
    } else {
        None
    };
    
    // Load template if specified and merge attributes
    let (final_project_id, final_due_ts, final_scheduled_ts, final_wait_ts, final_alloc_secs, final_udas, final_tags) = 
        if let Some(template_name) = &parsed.template {
            // Load template
            let template = TemplateRepo::get_by_name(&conn, template_name)?;
            if let Some(tmpl) = template {
                // Merge template with task attributes (task overrides template)
                let (proj_id, due, scheduled, wait, alloc, udas, tags) = 
                    TemplateRepo::merge_attributes(
                        &tmpl,
                        project_id,
                        due_ts,
                        scheduled_ts,
                        wait_ts,
                        alloc_secs,
                        &parsed.udas,
                        &parsed.tags_add,
                    );
                (proj_id, due, scheduled, wait, alloc, udas, tags)
            } else {
                // Template not found - create it from current task attributes
                TemplateRepo::create_from_task(
                    &conn,
                    template_name,
                    project_id,
                    due_ts,
                    scheduled_ts,
                    wait_ts,
                    alloc_secs,
                    &parsed.udas,
                    &parsed.tags_add,
                )?;
                // Use task attributes as-is
                (project_id, due_ts, scheduled_ts, wait_ts, alloc_secs, parsed.udas, parsed.tags_add)
            }
        } else {
            // No template - use task attributes as-is
            (project_id, due_ts, scheduled_ts, wait_ts, alloc_secs, parsed.udas, parsed.tags_add)
        };
    
    // Create task
    let task = TaskRepo::create_full(
        &conn,
        &description,
        final_project_id,
        final_due_ts,
        final_scheduled_ts,
        final_wait_ts,
        final_alloc_secs,
        parsed.template,
        parsed.recur,
        &final_udas,
        &final_tags,
    )
    .context("Failed to create task")?;
    
    let task_id = task.id.unwrap();
    println!("Created task {}: {}", task_id, description);
    
    // If --on flag is set, start timing the newly created task (takes precedence over --enqueue)
    if start_timing {
        // handle_task_on will push to stack and start timing atomically
        handle_task_on(task_id.to_string(), Vec::new())
            .context("Failed to start timing task")?;
    } else if enqueue {
        // Enqueue to queue (adds to end, does not start timing)
        let stack = StackRepo::get_or_create_default(&conn)?;
        StackRepo::enqueue(&conn, stack.id.unwrap(), task_id)
            .context("Failed to enqueue task")?;
        println!("Enqueued task {}", task_id);
    }
    
    Ok(())
}

struct ListRequest {
    filter_tokens: Vec<String>,
    sort_columns: Vec<String>,
    group_columns: Vec<String>,
    hide_columns: Vec<String>,
    save_alias: Option<String>,
}

fn parse_list_request(tokens: Vec<String>) -> ListRequest {
    let mut filter_tokens = Vec::new();
    let mut sort_columns = Vec::new();
    let mut group_columns = Vec::new();
    let mut hide_columns = Vec::new();
    let mut save_alias: Option<String> = None;
    
    for token in tokens {
        if let Some(spec) = token.strip_prefix("sort:") {
            sort_columns.extend(spec.split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()));
        } else if let Some(spec) = token.strip_prefix("group:") {
            group_columns.extend(spec.split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()));
        } else if let Some(spec) = token.strip_prefix("hide:") {
            hide_columns.extend(spec.split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()));
        } else if let Some(name) = token.strip_prefix("alias:") {
            if save_alias.is_none() && !name.is_empty() {
                save_alias = Some(name.to_string());
            }
        } else {
            filter_tokens.push(token);
        }
    }
    
    ListRequest {
        filter_tokens,
        sort_columns,
        group_columns,
        hide_columns,
        save_alias,
    }
}

fn is_view_name_token(token: &str) -> bool {
    !token.contains(':') && !token.starts_with('+') && !token.starts_with('-') && token.parse::<i64>().is_err()
}

fn looks_like_filter(token: &str) -> bool {
    token.contains(':') || token.starts_with('+') || token.starts_with('-') || token == "waiting"
}

fn handle_task_list(filter_args: Vec<String>, json: bool, relative: bool) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    let mut request = parse_list_request(filter_args);
    
    if request.sort_columns.is_empty()
        && request.group_columns.is_empty()
        && request.filter_tokens.len() == 1
        && is_view_name_token(&request.filter_tokens[0])
    {
        if let Some(view) = ViewRepo::get_by_name(&conn, "tasks", &request.filter_tokens[0])? {
            request.filter_tokens = view.filter_tokens;
            request.sort_columns = view.sort_columns;
            request.group_columns = view.group_columns;
            request.hide_columns = view.hide_columns;
        }
    }
    
    if let Some(alias) = request.save_alias.clone() {
        ViewRepo::upsert(
            &conn,
            &alias,
            "tasks",
            &request.filter_tokens,
            &request.sort_columns,
            &request.group_columns,
            &request.hide_columns,
        )?;
        println!("Saved view '{}'.", alias);
    }
    
    // Parse filter if provided
    let tasks = if request.filter_tokens.is_empty() {
        TaskRepo::list_all(&conn)
            .context("Failed to list tasks")?
    } else if request.filter_tokens.len() == 1 {
        // Single argument - try to parse as ID spec (range/list) first
        match parse_task_id_spec(&request.filter_tokens[0]) {
            Ok(ids) => {
                // Valid ID spec - fetch tasks by IDs
                let mut tasks_by_id = Vec::new();
                for id in ids {
                    if let Some(task) = TaskRepo::get_by_id(&conn, id)? {
                        tasks_by_id.push((task, Vec::new())); // No tags for now
                    }
                }
                tasks_by_id
            }
            Err(_) => {
                // Not an ID spec - try as filter
                let filter_expr = parse_filter(request.filter_tokens)
                    .map_err(|e| anyhow::anyhow!("Filter parse error: {}", e))?;
                filter_tasks(&conn, &filter_expr)
                    .context("Failed to filter tasks")?
            }
        }
    } else {
        // Multiple arguments - treat as filter
        let filter_expr = parse_filter(request.filter_tokens)
            .map_err(|e| anyhow::anyhow!("Filter parse error: {}", e))?;
        filter_tasks(&conn, &filter_expr)
            .context("Failed to filter tasks")?
    };
    
    if tasks.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }
    
    if json {
        // JSON output
        let json_tasks: Vec<serde_json::Value> = tasks.iter().map(|(task, tags)| {
            serde_json::json!({
                "id": task.id,
                "description": task.description,
                "status": task.status.as_str(),
                "project_id": task.project_id,
                "due_ts": task.due_ts,
                "scheduled_ts": task.scheduled_ts,
                "wait_ts": task.wait_ts,
                "tags": tags,
                "udas": task.udas,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&json_tasks)?);
    } else {
        // Human-readable table output
        let options = TaskListOptions {
            use_relative_time: relative,
            sort_columns: request.sort_columns,
            group_columns: request.group_columns,
            hide_columns: request.hide_columns,
        };
        let table = format_task_list_table(&conn, &tasks, &options)?;
        print!("{}", table);
    }
    
    Ok(())
}

/// Handle task modify with optional --on flag
fn handle_task_modify_with_on(id_or_filter: String, args: Vec<String>, yes: bool, interactive: bool, start_timing: bool) -> Result<()> {
    // First, do the modification
    handle_task_modify(id_or_filter.clone(), args, yes, interactive)?;
    
    // If --on flag is set, start timing the task
    if start_timing {
        // Only works for single task modification
        if let Ok(task_id) = validate_task_id(&id_or_filter) {
            handle_task_on(task_id.to_string(), Vec::new())
                .context("Failed to start timing task")?;
        } else {
            eprintln!("Warning: --on flag only works with single task ID, not filters");
        }
    }
    
    Ok(())
}

fn handle_task_modify(id_or_filter: String, args: Vec<String>, yes: bool, interactive: bool) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // Try to parse as task ID spec (single ID, range, or list) first
    let task_ids: Vec<i64> = match parse_task_id_spec(&id_or_filter) {
        Ok(ids) => {
            // Valid ID spec (single, range, or list)
            ids
        }
        Err(_) => {
            // Not an ID spec - try single ID for backward compatibility
            match validate_task_id(&id_or_filter) {
                Ok(id) => {
                    if TaskRepo::get_by_id(&conn, id)?.is_none() {
                        user_error(&format!("Task {} not found", id));
                    }
                    vec![id]
                }
                Err(_) => {
                    // Treat as filter
                    let filter_expr = match parse_filter(vec![id_or_filter]) {
                        Ok(expr) => expr,
                        Err(e) => user_error(&format!("Filter parse error: {}", e)),
                    };
                    let matching_tasks = filter_tasks(&conn, &filter_expr)
                        .context("Failed to filter tasks")?;
                    
                    if matching_tasks.is_empty() {
                        user_error("No matching tasks found");
                    }
                    
                    matching_tasks.iter()
                        .filter_map(|(task, _)| task.id)
                        .collect()
                }
            }
        }
    };
    
    // Handle multiple tasks with confirmation
    if task_ids.len() > 1 {
        if !yes && !interactive {
            // Prompt for confirmation
            eprintln!("This will modify {} tasks. Continue? (yes/no/interactive): ", task_ids.len());
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)
                .map_err(|e| anyhow::anyhow!("Failed to read input: {}", e))?;
            let input = input.trim().to_lowercase();
            
            match input.as_str() {
                "y" | "yes" => {
                    // Continue with all
                }
                "n" | "no" => {
                    println!("Cancelled.");
                    return Ok(());
                }
                "i" | "interactive" => {
                    // Process one by one
                    for task_id in task_ids {
                        eprint!("Modify task {}? (y/n): ", task_id);
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)
                            .map_err(|e| anyhow::anyhow!("Failed to read input: {}", e))?;
                        if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
                            modify_single_task(&conn, task_id, &args, yes)?;
                        }
                    }
                    return Ok(());
                }
                _ => {
                    println!("Invalid response. Cancelled.");
                    return Ok(());
                }
            }
        } else if interactive {
            // Process one by one
            for task_id in task_ids {
                eprint!("Modify task {}? (y/n): ", task_id);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)
                    .map_err(|e| anyhow::anyhow!("Failed to read input: {}", e))?;
                if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
                    modify_single_task(&conn, task_id, &args, yes)?;
                }
            }
            return Ok(());
        }
        // else: yes flag - continue with all
    }
    
    // Apply modifications to all selected tasks
    for task_id in task_ids {
        modify_single_task(&conn, task_id, &args, yes)?;
    }
    
    Ok(())
}

fn modify_single_task(conn: &Connection, task_id: i64, args: &[String], auto_create_project: bool) -> Result<()> {
    // Parse modification arguments
    let parsed = match parse_task_args(args.to_vec()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    
    // Parse description (optional)
    let description = if parsed.description.is_empty() {
        None
    } else {
        Some(join_description(&parsed.description))
    };
    
    // Resolve project (handle clearing with project:none)
    let project_id = if let Some(project_name) = &parsed.project {
        if project_name == "none" {
            Some(None) // Clear project
        } else {
            let project = ProjectRepo::get_by_name(&conn, project_name)?;
            if let Some(p) = project {
                Some(Some(p.id.unwrap()))
            } else if auto_create_project {
                if let Err(e) = validate_project_name(&project_name) {
                    user_error(&e);
                }
                let project = ProjectRepo::create(&conn, project_name)
                    .map_err(|e| anyhow::anyhow!("Failed to create project: {}", e))?;
                println!("Created project '{}' (id: {})", project.name, project.id.unwrap());
                Some(Some(project.id.unwrap()))
            } else {
                match prompt_create_project(project_name)? {
                    Some(true) => {
                        if let Err(e) = validate_project_name(&project_name) {
                            user_error(&e);
                        }
                        let project = ProjectRepo::create(&conn, project_name)
                            .map_err(|e| anyhow::anyhow!("Failed to create project: {}", e))?;
                        println!("Created project '{}' (id: {})", project.name, project.id.unwrap());
                        Some(Some(project.id.unwrap()))
                    }
                    Some(false) => {
                        // Skip project update
                        None
                    }
                    None => {
                        println!("Cancelled.");
                        return Ok(());
                    }
                }
            }
        }
    } else {
        None // Don't change
    };
    
    // Parse dates (handle clearing with field:none)
    let due_ts = if let Some(due) = &parsed.due {
        if due == "none" {
            Some(None)
        } else {
            Some(Some(parse_date_expr(due).context("Failed to parse due date")?))
        }
    } else {
        None
    };
    
    let scheduled_ts = if let Some(scheduled) = &parsed.scheduled {
        if scheduled == "none" {
            Some(None)
        } else {
            Some(Some(parse_date_expr(scheduled).context("Failed to parse scheduled date")?))
        }
    } else {
        None
    };
    
    let wait_ts = if let Some(wait) = &parsed.wait {
        if wait == "none" {
            Some(None)
        } else {
            Some(Some(parse_date_expr(wait).context("Failed to parse wait date")?))
        }
    } else {
        None
    };
    
    // Parse duration (handle clearing)
    let alloc_secs = if let Some(allocation) = &parsed.allocation {
        if allocation == "none" {
            Some(None)
        } else {
            Some(Some(parse_duration(allocation).context("Failed to parse allocation duration")?))
        }
    } else {
        None
    };
    
    // Handle template and recur clearing
    let template = if let Some(tmpl) = &parsed.template {
        if tmpl == "none" {
            Some(None)
        } else {
            Some(Some(tmpl.clone()))
        }
    } else {
        None
    };
    
    let recur = if let Some(rec) = &parsed.recur {
        if rec == "none" {
            Some(None)
        } else {
            Some(Some(rec.clone()))
        }
    } else {
        None
    };
    
    // Separate UDAs to add and remove
    let mut udas_to_add = HashMap::new();
    let mut udas_to_remove = Vec::new();
    
    for (key, value) in &parsed.udas {
        if value == "none" {
            udas_to_remove.push(key.clone());
        } else {
            udas_to_add.insert(key.clone(), value.clone());
        }
    }
    
    // Apply modifications
    TaskRepo::modify(
        &conn,
        task_id,
        description,
        project_id,
        due_ts,
        scheduled_ts,
        wait_ts,
        alloc_secs,
        template,
        recur,
        &udas_to_add,
        &udas_to_remove,
        &parsed.tags_add,
        &parsed.tags_remove,
    )
    .context("Failed to modify task")?;
    
    println!("Modified task {}", task_id);
    Ok(())
}


fn handle_task_enqueue(task_id_str: String) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // Parse comma-separated list of IDs (preserves order)
    let task_ids = match parse_task_id_list(&task_id_str) {
        Ok(ids) => ids,
        Err(e) => user_error(&e),
    };
    
    // Validate all tasks exist before enqueueing any
    let mut valid_ids = Vec::new();
    let mut missing_ids = Vec::new();
    
    for task_id in &task_ids {
        if TaskRepo::get_by_id(&conn, *task_id)?.is_some() {
            valid_ids.push(*task_id);
        } else {
            missing_ids.push(*task_id);
        }
    }
    
    if !missing_ids.is_empty() {
        user_error(&format!("Task(s) not found: {}", 
            missing_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", ")));
    }
    
    if valid_ids.is_empty() {
        user_error("No valid tasks to enqueue");
    }
    
    // Enqueue all tasks in order
    let stack = StackRepo::get_or_create_default(&conn)?;
    let stack_id = stack.id.unwrap();
    
    for task_id in valid_ids {
        StackRepo::enqueue(&conn, stack_id, task_id)
            .context(format!("Failed to enqueue task {}", task_id))?;
        println!("Enqueued task {}", task_id);
    }
    
    Ok(())
}

/// Handle `tatl on [<task_id>] [<time>]` - Start timing
fn handle_on(task_id_opt: Option<String>, mut time_args: Vec<String>) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    if let Some(task_id_str) = task_id_opt {
        // Check if it's a valid task ID (numeric) or if it's actually a time expression
        if let Ok(_task_id) = task_id_str.parse::<i64>() {
            // Valid task ID - use it
            handle_task_on(task_id_str, time_args)
        } else {
            // Not a valid task ID - treat as time expression, use queue[0]
            time_args.insert(0, task_id_str);
            handle_on_queue_top(&conn, time_args)
        }
    } else {
        // Use queue[0]
        handle_on_queue_top(&conn, time_args)
    }
}

/// Handle `tatl off [<time>]` - Stop timing
fn handle_off(time_args: Vec<String>) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // Check if session is running
    let session_opt = SessionRepo::get_open(&conn)?;
    
    if session_opt.is_none() {
        user_error("No session is currently running.");
    }
    
    // Parse end time (defaults to "now")
    let end_ts = if time_args.is_empty() {
        chrono::Utc::now().timestamp()
    } else {
        let end_expr = time_args.join(" ");
        parse_date_expr(&end_expr)
            .context("Invalid end time expression")?
    };
    
    // Close session
    let closed = SessionRepo::close_open(&conn, end_ts)
        .context("Failed to close session")?;
    
    if let Some(session) = closed {
        // Get task description for better message
        let task = TaskRepo::get_by_id(&conn, session.task_id)?;
        let desc = task.as_ref().map(|t| t.description.as_str()).unwrap_or("");
        println!("Stopped timing task {}: {}", session.task_id, desc);
    }
    
    Ok(())
}

/// Handle `tatl dequeue [<task_id>]` - Remove from queue without finishing
fn handle_dequeue(task_id_opt: Option<String>) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    let stack = StackRepo::get_or_create_default(&conn)?;
    let stack_id = stack.id.unwrap();
    let items = StackRepo::get_items(&conn, stack_id)?;
    
    if items.is_empty() {
        user_error("Queue is empty.");
    }
    
    let task_id = if let Some(task_id_str) = task_id_opt {
        // Specific task ID provided
        match validate_task_id(&task_id_str) {
            Ok(id) => id,
            Err(e) => user_error(&e),
        }
    } else {
        // Default to queue[0]
        items[0].task_id
    };
    
    // Check if task is in the queue
    if !items.iter().any(|item| item.task_id == task_id) {
        user_error(&format!("Task {} is not in the queue", task_id));
    }
    
    // Remove from queue
    StackRepo::remove_task(&conn, stack_id, task_id)
        .context("Failed to remove task from queue")?;
    
    // Get task description for better message
    let task = TaskRepo::get_by_id(&conn, task_id)?;
    let desc = task.as_ref().map(|t| t.description.as_str()).unwrap_or("");
    println!("Removed task {} from queue: {}", task_id, desc);
    
    Ok(())
}

/// Start timing queue[0]
fn handle_on_queue_top(conn: &Connection, args: Vec<String>) -> Result<()> {
    // Get stack and check if it's empty
    let stack = StackRepo::get_or_create_default(conn)?;
    let stack_id = stack.id.unwrap();
    let items = StackRepo::get_items(conn, stack_id)?;
    
    if items.is_empty() {
        user_error("Queue is empty. Add a task to the queue first.");
    }
    
    // Get queue[0] task
    let task_id = items[0].task_id;
    
    // Parse arguments - check for interval syntax (start..end)
    let arg_str = args.join(" ");
    if let Some(sep_pos) = arg_str.find("..") {
        // Interval syntax: start..end (creates closed session)
        let start_expr = arg_str[..sep_pos].trim();
        let end_expr = arg_str[sep_pos + 2..].trim();
        
        let start_ts = if start_expr.is_empty() {
            chrono::Utc::now().timestamp()
        } else {
            parse_date_expr(start_expr)
                .context("Invalid start time expression")?
        };
        
        let end_ts = parse_date_expr(end_expr)
            .context("Invalid end time expression")?;
        
        // Check for overlap prevention
        check_and_amend_overlaps(conn, start_ts)?;
        
        // Closed sessions don't conflict with open session constraint
        // Create closed session
        SessionRepo::create_closed(conn, task_id, start_ts, end_ts)
            .context("Failed to create closed session")?;
        
        println!("Recorded session for task {} ({} to {})", task_id, start_ts, end_ts);
    } else {
        // Single start time or "now" (creates open session)
        // Check if session is already running (only for open sessions)
        if let Some(_) = SessionRepo::get_open(conn)? {
            user_error("A session is already running. Please use 'tatl off' first.");
        }
        
        let start_ts = if args.is_empty() {
            chrono::Utc::now().timestamp()
        } else {
            parse_date_expr(&arg_str)
                .context("Invalid start time expression")?
        };
        
        // Check for overlap prevention
        check_and_amend_overlaps(conn, start_ts)?;
        
        // Create open session
        SessionRepo::create(conn, task_id, start_ts)
            .context("Failed to start session")?;
        
        // Get task description for better message
        let task = TaskRepo::get_by_id(conn, task_id)?;
        let desc = task.as_ref().map(|t| t.description.as_str()).unwrap_or("");
        println!("Started timing task {}: {}", task_id, desc);
    }
    
    Ok(())
}

/// Start timing a specific task (pushes to queue[0] and starts timing)
fn handle_task_on(task_id_str: String, args: Vec<String>) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // Parse task ID
    let task_id = match validate_task_id(&task_id_str) {
        Ok(id) => id,
        Err(e) => user_error(&e),
    };
    
    // Check if task exists
    if TaskRepo::get_by_id(&conn, task_id)?.is_none() {
        user_error(&format!("Task {} not found", task_id));
    }
    
    // Parse arguments - check for interval syntax (start..end)
    let arg_str = args.join(" ");
    let (start_ts, end_ts_opt) = if let Some(sep_pos) = arg_str.find("..") {
        // Interval syntax: start..end
        let start_expr = arg_str[..sep_pos].trim();
        let end_expr = arg_str[sep_pos + 2..].trim();
        
        let start_ts = if start_expr.is_empty() {
            chrono::Utc::now().timestamp()
        } else {
            parse_date_expr(start_expr)
                .context("Invalid start time expression")?
        };
        
        let end_ts = parse_date_expr(end_expr)
            .context("Invalid end time expression")?;
        
        (start_ts, Some(end_ts))
    } else {
        // Single start time or "now"
        let start_ts = if args.is_empty() {
            chrono::Utc::now().timestamp()
        } else {
            parse_date_expr(&arg_str)
                .context("Invalid start time expression")?
        };
        (start_ts, None)
    };
    
    // Wrap entire operation in a transaction for atomicity
    // This ensures: close existing session + push to stack + create new session all succeed or fail together
    let tx = conn.unchecked_transaction()?;
    
    // Check if session is already running
    let existing_session = SessionRepo::get_open(&tx)?;
    
    // If session is running, close it at the effective start time
    if existing_session.is_some() {
        SessionRepo::close_open(&tx, start_ts)
            .context("Failed to close existing session")?;
    }
    
    // Check for overlap prevention (before creating new session)
    // Note: This might need to be done outside transaction if it queries other sessions
    // For now, we'll do it within the transaction
    check_and_amend_overlaps_transactional(&tx, start_ts)?;
    
    // Push task to stack[0]
    let stack = StackRepo::get_or_create_default(&tx)?;
    StackRepo::push_to_top(&tx, stack.id.unwrap(), task_id)
        .context("Failed to push task to stack")?;
    
    // Create session (closed if interval, open otherwise)
    if let Some(end_ts) = end_ts_opt {
        SessionRepo::create_closed(&tx, task_id, start_ts, end_ts)
            .context("Failed to create closed session")?;
        tx.commit()?;
        println!("Recorded session for task {} ({} to {})", task_id, start_ts, end_ts);
    } else {
        SessionRepo::create(&tx, task_id, start_ts)
            .context("Failed to start session")?;
        tx.commit()?;
        // Get task description for better message
        let task = TaskRepo::get_by_id(&conn, task_id)?;
        let desc = task.as_ref().map(|t| t.description.as_str()).unwrap_or("");
        println!("Started timing task {}: {}", task_id, desc);
    }
    
    Ok(())
}

/// Check for closed sessions that end after the given start time and amend them
/// to prevent overlap (non-transactional version)
fn check_and_amend_overlaps(conn: &Connection, new_start_ts: i64) -> Result<()> {
    check_and_amend_overlaps_transactional(conn, new_start_ts)
}

/// Check for closed sessions that end after the given start time and amend them
/// to prevent overlap (transactional version)
fn check_and_amend_overlaps_transactional(conn: &Connection, new_start_ts: i64) -> Result<()> {
    // Find closed sessions that end at or after the new start time
    let recent_sessions = SessionRepo::get_recent_closed_after(conn, new_start_ts)?;
    
    for session in recent_sessions {
        if let Some(end_ts) = session.end_ts {
            // If the session ends after the new start time, amend it
            if end_ts >= new_start_ts {
                SessionRepo::amend_end_time(conn, session.id.unwrap(), new_start_ts)
                    .context("Failed to amend session end time")?;
            }
        }
    }
    
    Ok(())
}

fn handle_annotation_add(task_id_opt: Option<String>, note_args: Vec<String>) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    if note_args.is_empty() {
        user_error("Annotation note cannot be empty");
    }
    
    let note = note_args.join(" ");
    
    // Determine task ID
    let task_id = if let Some(tid_str) = task_id_opt {
        // Task ID provided
        match validate_task_id(&tid_str) {
            Ok(id) => id,
            Err(e) => user_error(&e),
        }
    } else {
        // No task ID - check if clocked in
        let open_session = SessionRepo::get_open(&conn)?;
        if let Some(session) = open_session {
            session.task_id
        } else {
            user_error("No task ID provided and no session is running. Please specify a task ID or clock in first.");
        }
    };
    
    // Check if task exists
    if TaskRepo::get_by_id(&conn, task_id)?.is_none() {
        user_error(&format!("Task {} not found", task_id));
    }
    
    // Get current session if running (for session linking)
    let open_session = SessionRepo::get_open(&conn)?;
    let session_id = if let Some(session) = open_session {
        // Only link if the session is for the same task
        if session.task_id == task_id {
            session.id
        } else {
            None
        }
    } else {
        None
    };
    
    // Create annotation
    let annotation = AnnotationRepo::create(&conn, task_id, note, session_id)
        .context("Failed to create annotation")?;
    
    println!("Added annotation {} to task {}", annotation.id.unwrap(), task_id);
    Ok(())
}

/// Handle annotation with filter support (multi-task annotation)
fn handle_annotation_add_with_filter(id_or_filter: String, note_args: Vec<String>, yes: bool, interactive: bool) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    if note_args.is_empty() {
        user_error("Annotation note cannot be empty");
    }
    
    let note = note_args.join(" ");
    
    // Try to parse as task ID first, otherwise treat as filter
    let task_ids: Vec<i64> = match validate_task_id(&id_or_filter) {
        Ok(id) => {
            // Single task ID
            if TaskRepo::get_by_id(&conn, id)?.is_none() {
                user_error(&format!("Task {} not found", id));
            }
            vec![id]
        }
        Err(_) => {
            // Treat as filter
            let filter_expr = match parse_filter(vec![id_or_filter]) {
                Ok(expr) => expr,
                Err(e) => user_error(&format!("Filter parse error: {}", e)),
            };
            let matching_tasks = filter_tasks(&conn, &filter_expr)
                .context("Failed to filter tasks")?;
            
            if matching_tasks.is_empty() {
                user_error("No matching tasks found");
            }
            
            matching_tasks.iter()
                .filter_map(|(task, _)| task.id)
                .collect()
        }
    };
    
    // Get current session if running (for session linking)
    let open_session = SessionRepo::get_open(&conn)?;
    let _session_id = open_session.as_ref().and_then(|s| s.id);
    
    // Handle multiple tasks with confirmation
    if task_ids.len() > 1 {
        if !yes && !interactive {
            // Prompt for confirmation
            eprintln!("This will add annotation to {} tasks. Continue? (yes/no/interactive): ", task_ids.len());
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)
                .map_err(|e| anyhow::anyhow!("Failed to read input: {}", e))?;
            let input = input.trim().to_lowercase();
            
            match input.as_str() {
                "y" | "yes" => {
                    // Continue with all
                }
                "n" | "no" => {
                    println!("Cancelled.");
                    return Ok(());
                }
                "i" | "interactive" => {
                    // Process one by one
                    for task_id in task_ids {
                        eprint!("Add annotation to task {}? (y/n): ", task_id);
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)
                            .map_err(|e| anyhow::anyhow!("Failed to read input: {}", e))?;
                        if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
                            let link_session_id = if let Some(ref session) = open_session {
                                if session.task_id == task_id { session.id } else { None }
                            } else {
                                None
                            };
                            let annotation = AnnotationRepo::create(&conn, task_id, note.clone(), link_session_id)
                                .context("Failed to create annotation")?;
                            println!("Added annotation {} to task {}", annotation.id.unwrap(), task_id);
                        }
                    }
                    return Ok(());
                }
                _ => {
                    println!("Invalid response. Cancelled.");
                    return Ok(());
                }
            }
        } else if interactive {
            // Process one by one
            for task_id in task_ids {
                eprint!("Add annotation to task {}? (y/n): ", task_id);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)
                    .map_err(|e| anyhow::anyhow!("Failed to read input: {}", e))?;
                if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
                    let link_session_id = if let Some(ref session) = open_session {
                        if session.task_id == task_id { session.id } else { None }
                    } else {
                        None
                    };
                    let annotation = AnnotationRepo::create(&conn, task_id, note.clone(), link_session_id)
                        .context("Failed to create annotation")?;
                    println!("Added annotation {} to task {}", annotation.id.unwrap(), task_id);
                }
            }
            return Ok(());
        }
        // else: yes flag - continue with all
    }
    
    // Apply annotation to all selected tasks
    for task_id in task_ids {
        let link_session_id = if let Some(ref session) = open_session {
            if session.task_id == task_id { session.id } else { None }
        } else {
            None
        };
        let annotation = AnnotationRepo::create(&conn, task_id, note.clone(), link_session_id)
            .context("Failed to create annotation")?;
        println!("Added annotation {} to task {}", annotation.id.unwrap(), task_id);
    }
    
    Ok(())
}

fn handle_task_summary(id_or_filter: String) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // Parse task ID spec (single ID, range, or list)
    let task_ids: Vec<i64> = match parse_task_id_spec(&id_or_filter) {
        Ok(ids) => {
            // Valid ID spec
            ids
        }
        Err(_) => {
            // Try single ID for backward compatibility
            match validate_task_id(&id_or_filter) {
                Ok(id) => vec![id],
                Err(_) => {
                    // Not an ID - treat as filter
                    let filter_expr = match parse_filter(vec![id_or_filter.clone()]) {
                        Ok(expr) => expr,
                        Err(e) => user_error(&format!("Filter parse error: {}", e)),
                    };
                    let matching_tasks = filter_tasks(&conn, &filter_expr)
                        .context("Failed to filter tasks")?;
                    
                    if matching_tasks.is_empty() {
                        user_error("No matching tasks found");
                    }
                    
                    matching_tasks.iter()
                        .filter_map(|(task, _)| task.id)
                        .collect()
                }
            }
        }
    };
    
    // Get default stack to check positions
    let stack = StackRepo::get_or_create_default(&conn)?;
    let stack_id = stack.id.unwrap();
    let stack_items = StackRepo::get_items(&conn, stack_id)?;
    let stack_map: std::collections::HashMap<i64, i32> = stack_items.iter()
        .enumerate()
        .map(|(idx, item)| (item.task_id, idx as i32))
        .collect();
    let stack_total = stack_items.len() as i32;
    
    // Process each task
    let mut found_any = false;
    let last_id = *task_ids.last().unwrap_or(&0);
    for task_id in task_ids {
        // Get task
        let task = match TaskRepo::get_by_id(&conn, task_id)? {
            Some(t) => t,
            None => {
                eprintln!("Task {} not found", task_id);
                continue;
            }
        };
        
        found_any = true;
        
        // Get tags
        let tags = TaskRepo::get_tags(&conn, task_id)?;
        
        // Get annotations
        let annotations = AnnotationRepo::get_by_task(&conn, task_id)?;
        
        // Get sessions
        let sessions = SessionRepo::get_by_task(&conn, task_id)?;
        
        // Get stack position
        let stack_position = stack_map.get(&task_id)
            .map(|&pos| (pos, stack_total));
        
        // Format and print summary
        let summary = format_task_summary(&conn, &task, &tags, &annotations, &sessions, stack_position)?;
        print!("{}", summary);
        
        // Add separator between multiple tasks
        let is_last = task_id == last_id;
        if !is_last {
            println!();
        }
    }
    
    if !found_any {
        user_error("No tasks found");
    }
    
    Ok(())
}

fn handle_annotation_delete(task_id_str: String, annotation_id_str: String) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    let task_id = match validate_task_id(&task_id_str) {
        Ok(id) => id,
        Err(e) => user_error(&e),
    };
    
    let annotation_id: i64 = match annotation_id_str.parse() {
        Ok(id) => id,
        Err(_) => user_error(&format!("Invalid annotation ID: '{}'. Annotation ID must be a number.", annotation_id_str)),
    };
    
    // Check if task exists
    if TaskRepo::get_by_id(&conn, task_id)?.is_none() {
        user_error(&format!("Task {} not found", task_id));
    }
    
    // Delete annotation (verifies it belongs to the task)
    AnnotationRepo::delete_for_task(&conn, task_id, annotation_id)
        .context("Failed to delete annotation")?;
    
    println!("Deleted annotation {} from task {}", annotation_id, task_id);
    Ok(())
}

fn handle_task_finish(
    id_or_filter_opt: Option<String>,
    at_opt: Option<String>,
    next: bool,
    yes: bool,
    interactive: bool,
) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // Determine end time for session
    let end_ts = if let Some(at_expr) = at_opt {
        parse_date_expr(&at_expr).context("Invalid end time expression")?
    } else {
        chrono::Utc::now().timestamp()
    };
    
    // Get open session to check which tasks have running sessions
    let open_session = SessionRepo::get_open(&conn)?;
    let _running_task_id = open_session.as_ref().map(|s| s.task_id);
    
    // Determine which tasks to complete
    let task_ids = if let Some(id_or_filter) = id_or_filter_opt {
        // Task ID or filter provided
        // Try to parse as task ID spec (single ID, range, or list) first
        match parse_task_id_spec(&id_or_filter) {
            Ok(ids) => {
                // Valid ID spec (single, range, or list)
                // Verify all tasks exist
                let mut valid_ids = Vec::new();
                for task_id in ids {
                    if TaskRepo::get_by_id(&conn, task_id)?.is_some() {
                        valid_ids.push(task_id);
                    }
                }
                
                if valid_ids.is_empty() {
                    user_error("No matching tasks found.");
                }
                
                valid_ids
            }
            Err(_) => {
                // Not an ID spec - try single ID for backward compatibility
                if let Ok(task_id) = id_or_filter.parse::<i64>() {
                    // Single task ID - verify it exists
                    if TaskRepo::get_by_id(&conn, task_id)?.is_none() {
                        user_error(&format!("Task {} not found", task_id));
                    }
                    vec![task_id]
                } else {
                    // Filter expression
                    let filter_expr = parse_filter(vec![id_or_filter])
                        .map_err(|e| anyhow::anyhow!("Filter parse error: {}", e))?;
                    let matching_tasks = filter_tasks(&conn, &filter_expr)
                        .context("Failed to filter tasks")?;
                    
                    // Extract task IDs from matching tasks
                    let task_ids: Vec<i64> = matching_tasks
                        .iter()
                        .filter_map(|(task, _)| task.id)
                        .collect();
                    
                    if task_ids.is_empty() {
                        user_error("No matching tasks found.");
                    }
                    
                    task_ids
                }
            }
        }
    } else {
        // No ID provided - use stack[0]
        let stack = StackRepo::get_or_create_default(&conn)?;
        let stack_id = stack.id.unwrap();
        let items = StackRepo::get_items(&conn, stack_id)?;
        
        if items.is_empty() {
            user_error("Stack is empty. Cannot complete task.");
        }
        
        // Check if session is running
        if open_session.is_none() {
            user_error("No session is running. Cannot complete task.");
        }
        
        // Verify the running session is for stack[0]
        let stack_task_id = items[0].task_id;
        if let Some(session) = &open_session {
            if session.task_id != stack_task_id {
                user_error(&format!("Running session is for task {}, but stack[0] is task {}. Cannot complete.", session.task_id, stack_task_id));
            }
        }
        
        vec![stack_task_id]
    };
    
    // Handle multiple tasks with confirmation
    if task_ids.len() > 1 {
        if !yes && !interactive {
            // Prompt for confirmation
            println!("This will finish {} task(s).", task_ids.len());
            print!("Finish all tasks? (y/n/i): ");
            use std::io::{self, Write};
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();
            
            match input.as_str() {
                "y" | "yes" => {
                    // Complete all
                }
                "n" | "no" => {
                    println!("Cancelled.");
                    return Ok(());
                }
                "i" | "interactive" => {
                    // Interactive mode - confirm one by one
                    return handle_finish_interactive(&conn, &task_ids, end_ts, next);
                }
                _ => {
                    println!("Invalid input. Cancelled.");
                    return Ok(());
                }
            }
        } else if interactive {
            // Force interactive mode
            return handle_finish_interactive(&conn, &task_ids, end_ts, next);
        }
    }
    
    // Complete all tasks
    let mut completed_stack_top = false;
    for task_id in &task_ids {
        // Verify task exists
        if TaskRepo::get_by_id(&conn, *task_id)?.is_none() {
            eprintln!("Error: Task {} not found", task_id);
            continue; // Continue processing other tasks
        }
        
        // Check if session is running for this task - close it if it exists
        if let Some(session) = &open_session {
            if session.task_id == *task_id {
                // Close the session
                SessionRepo::close_open(&conn, end_ts)
                    .context("Failed to close session")?;
                completed_stack_top = true;
            }
        }
        // Note: We allow completing tasks even if no session is running
        
        // Mark task as completed
        TaskRepo::complete(&conn, *task_id)
            .context("Failed to finish task")?;
        
        // Remove from stack
        let stack = StackRepo::get_or_create_default(&conn)?;
        let stack_id = stack.id.unwrap();
        let items = StackRepo::get_items(&conn, stack_id)?;
        
        // Find the task in the stack and remove it
        if let Some(item) = items.iter().find(|item| item.task_id == *task_id) {
            // Drop the task at this position using its ordinal
            StackRepo::drop(&conn, stack_id, item.ordinal as i32)?;
        }
        
        println!("Finished task {}", task_id);
    }
    
    // If --next flag and we completed stack[0], start session for new stack[0]
    if next && completed_stack_top {
        let stack = StackRepo::get_or_create_default(&conn)?;
        let stack_id = stack.id.unwrap();
        let items = StackRepo::get_items(&conn, stack_id)?;
        if !items.is_empty() {
            let next_task_id = items[0].task_id;
            SessionRepo::create(&conn, next_task_id, end_ts)
                .context("Failed to start session for next task")?;
            // Get task description for better message
            let task = TaskRepo::get_by_id(&conn, next_task_id)?;
            let desc = task.as_ref().map(|t| t.description.as_str()).unwrap_or("");
            println!("Started timing task {}: {}", next_task_id, desc);
        }
    }
    
    Ok(())
}

fn handle_finish_interactive(conn: &Connection, task_ids: &[i64], end_ts: i64, next: bool) -> Result<()> {
    use std::io::{self, Write};
    
    let open_session = SessionRepo::get_open(conn)?;
    let mut completed_stack_top = false;
    
    for task_id in task_ids {
        // Get task description for display
        let task = TaskRepo::get_by_id(conn, *task_id)?;
        if task.is_none() {
            eprintln!("Error: Task {} not found", task_id);
            continue; // Continue processing other tasks
        }
        let task = task.unwrap();
        
        // Prompt for confirmation
        print!("Finish task {} ({})? (y/n): ", task_id, task.description);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        
        if input != "y" && input != "yes" {
            println!("Skipped task {}.", task_id);
            continue;
        }
        
        // Close the session if this is the running task
        if let Some(session) = &open_session {
            if session.task_id == *task_id {
                SessionRepo::close_open(conn, end_ts)
                    .context("Failed to close session")?;
                completed_stack_top = true;
            }
        }
        
        // Mark task as completed
        TaskRepo::complete(conn, *task_id)
            .context("Failed to finish task")?;
        
        // Remove from stack
        let stack = StackRepo::get_or_create_default(conn)?;
        let stack_id = stack.id.unwrap();
        let items = StackRepo::get_items(conn, stack_id)?;
        
        // Find the task in the stack and remove it
        if let Some(item) = items.iter().find(|item| item.task_id == *task_id) {
            // Drop the task at this position using its ordinal
            StackRepo::drop(conn, stack_id, item.ordinal as i32)?;
        }
        
        println!("Finished task {}", task_id);
    }
    
    // If --next flag and we completed stack[0], start session for new stack[0]
    if next && completed_stack_top {
        let stack = StackRepo::get_or_create_default(conn)?;
        let stack_id = stack.id.unwrap();
        let items = StackRepo::get_items(conn, stack_id)?;
        if !items.is_empty() {
            let next_task_id = items[0].task_id;
            SessionRepo::create(conn, next_task_id, end_ts)
                .context("Failed to start session for next task")?;
            println!("Started timing task {}", next_task_id);
        }
    }
    
    Ok(())
}

/// Handle task close with optional target (defaults to queue[0])
fn handle_task_close_optional(target: Option<String>, yes: bool, interactive: bool) -> Result<()> {
    let id_or_filter = if let Some(t) = target {
        t
    } else {
        // Default to queue[0]
        let conn = DbConnection::connect()
            .context("Failed to connect to database")?;
        let stack = StackRepo::get_or_create_default(&conn)?;
        let items = StackRepo::get_items(&conn, stack.id.unwrap())?;
        
        if items.is_empty() {
            user_error("No target specified and queue is empty.");
        }
        
        items[0].task_id.to_string()
    };
    
    handle_task_close(id_or_filter, yes, interactive)
}

fn handle_task_close(id_or_filter: String, yes: bool, interactive: bool) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // Resolve task IDs
    let task_ids: Vec<i64> = match parse_task_id_spec(&id_or_filter) {
        Ok(ids) => ids,
        Err(_) => {
            match validate_task_id(&id_or_filter) {
                Ok(id) => {
                    if TaskRepo::get_by_id(&conn, id)?.is_none() {
                        user_error(&format!("Task {} not found", id));
                    }
                    vec![id]
                }
                Err(_) => {
                    let filter_expr = match parse_filter(vec![id_or_filter]) {
                        Ok(expr) => expr,
                        Err(e) => user_error(&format!("Filter parse error: {}", e)),
                    };
                    let matching_tasks = filter_tasks(&conn, &filter_expr)
                        .context("Failed to filter tasks")?;
                    
                    if matching_tasks.is_empty() {
                        user_error("No matching tasks found");
                    }
                    
                    matching_tasks.iter()
                        .filter_map(|(task, _)| task.id)
                        .collect()
                }
            }
        }
    };
    
    if task_ids.len() > 1 {
        if !yes && !interactive {
            println!("This will close {} task(s).", task_ids.len());
            print!("Close all tasks? (y/n/i): ");
            use std::io::{self, Write};
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();
            
            match input.as_str() {
                "y" | "yes" => {}
                "n" | "no" => {
                    println!("Cancelled.");
                    return Ok(());
                }
                "i" | "interactive" => {
                    return handle_close_interactive(&conn, &task_ids);
                }
                _ => {
                    println!("Invalid input. Cancelled.");
                    return Ok(());
                }
            }
        } else if interactive {
            return handle_close_interactive(&conn, &task_ids);
        }
    }
    
    let end_ts = chrono::Utc::now().timestamp();
    let open_session = SessionRepo::get_open(&conn)?;
    let mut closed_open_session = false;
    
    for task_id in &task_ids {
        if TaskRepo::get_by_id(&conn, *task_id)?.is_none() {
            eprintln!("Error: Task {} not found", task_id);
            continue;
        }
        
        if let Some(session) = &open_session {
            if !closed_open_session && session.task_id == *task_id {
                SessionRepo::close_open(&conn, end_ts)
                    .context("Failed to close session")?;
                closed_open_session = true;
            }
        }
        
        TaskRepo::close(&conn, *task_id)
            .context("Failed to close task")?;
        
        let stack = StackRepo::get_or_create_default(&conn)?;
        let stack_id = stack.id.unwrap();
        let items = StackRepo::get_items(&conn, stack_id)?;
        if let Some(item) = items.iter().find(|item| item.task_id == *task_id) {
            StackRepo::drop(&conn, stack_id, item.ordinal as i32)?;
        }
        
        println!("Closed task {}", task_id);
    }
    
    Ok(())
}

fn handle_close_interactive(conn: &Connection, task_ids: &[i64]) -> Result<()> {
    use std::io::{self, Write};
    
    let end_ts = chrono::Utc::now().timestamp();
    let open_session = SessionRepo::get_open(conn)?;
    let mut closed_open_session = false;
    
    for task_id in task_ids {
        let task = match TaskRepo::get_by_id(conn, *task_id) {
            Ok(Some(task)) => task,
            Ok(None) => {
                eprintln!("Error: Task {} not found", task_id);
                continue;
            }
            Err(e) => {
                eprintln!("Error: Failed to get task {}: {}", task_id, e);
                continue;
            }
        };
        
        print!("Close task {} ({})? (y/n): ", task_id, task.description);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        
        if input != "y" && input != "yes" {
            println!("Skipped task {}.", task_id);
            continue;
        }
        
        if let Some(session) = &open_session {
            if !closed_open_session && session.task_id == *task_id {
                SessionRepo::close_open(conn, end_ts)
                    .context("Failed to close session")?;
                closed_open_session = true;
            }
        }
        
        TaskRepo::close(conn, *task_id)
            .context("Failed to close task")?;
        
        let stack = StackRepo::get_or_create_default(conn)?;
        let stack_id = stack.id.unwrap();
        let items = StackRepo::get_items(conn, stack_id)?;
        if let Some(item) = items.iter().find(|item| item.task_id == *task_id) {
            StackRepo::drop(conn, stack_id, item.ordinal as i32)?;
        }
        
        println!("Closed task {}", task_id);
    }
    
    Ok(())
}

/// Handle task deletion
fn handle_task_delete(id_or_filter: String, yes: bool, interactive: bool) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // Try to parse as task ID spec (single ID, range, or list) first
    let task_ids = match parse_task_id_spec(&id_or_filter) {
        Ok(ids) => {
            // Valid ID spec (single, range, or list)
            ids
        }
        Err(_) => {
            // Not an ID spec - try single ID for backward compatibility
            match validate_task_id(&id_or_filter) {
                Ok(task_id) => {
                    vec![task_id]
                }
                Err(_) => {
                    // Treat as filter - get all matching tasks
                    let filter_expr = parse_filter(vec![id_or_filter.clone()])
                        .map_err(|e| anyhow::anyhow!("Filter parse error: {}", e))?;
                    let matching_tasks = filter_tasks(&conn, &filter_expr)
                        .context("Failed to filter tasks")?;
                    
                    if matching_tasks.is_empty() {
                        user_error("No matching tasks found");
                    }
                    
                    matching_tasks.iter()
                        .filter_map(|(task, _)| task.id)
                        .collect()
                }
            }
        }
    };
    
    if interactive {
        handle_delete_interactive(&conn, &task_ids)
    } else if yes {
        handle_delete_yes(&conn, &task_ids)
    } else {
        handle_delete_confirm(&conn, &task_ids)
    }
}

/// Delete tasks with confirmation prompt
fn handle_delete_confirm(conn: &Connection, task_ids: &[i64]) -> Result<()> {
    use std::io::{self, Write};
    
    if task_ids.len() == 1 {
        // Single task - show description
        let task = TaskRepo::get_by_id(conn, task_ids[0])?
            .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_ids[0]))?;
        print!("Delete task {} ({})? (y/n): ", task_ids[0], task.description);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        
        if input != "y" && input != "yes" {
            println!("Cancelled.");
            return Ok(());
        }
        
        TaskRepo::delete(conn, task_ids[0])
            .context("Failed to delete task")?;
        println!("Deleted task {}: {}", task_ids[0], task.description);
    } else {
        // Multiple tasks - show count
        print!("Delete {} tasks? (y/n): ", task_ids.len());
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        
        if input != "y" && input != "yes" {
            println!("Cancelled.");
            return Ok(());
        }
        
        return handle_delete_yes(conn, task_ids);
    }
    
    Ok(())
}

/// Delete tasks without confirmation
fn handle_delete_yes(conn: &Connection, task_ids: &[i64]) -> Result<()> {
    let mut deleted_count = 0;
    
    for task_id in task_ids {
        match TaskRepo::get_by_id(conn, *task_id) {
            Ok(Some(task)) => {
                TaskRepo::delete(conn, *task_id)
                    .context(format!("Failed to delete task {}", task_id))?;
                println!("Deleted task {}: {}", task_id, task.description);
                deleted_count += 1;
            }
            Ok(None) => {
                eprintln!("Warning: Task {} not found, skipping", task_id);
            }
            Err(e) => {
                eprintln!("Error: Failed to get task {}: {}", task_id, e);
            }
        }
    }
    
    if deleted_count > 0 {
        println!("Deleted {} task(s)", deleted_count);
    }
    
    Ok(())
}

/// Delete tasks with interactive confirmation
fn handle_delete_interactive(conn: &Connection, task_ids: &[i64]) -> Result<()> {
    use std::io::{self, Write};
    
    let mut deleted_count = 0;
    
    for task_id in task_ids {
        let task = match TaskRepo::get_by_id(conn, *task_id) {
            Ok(Some(task)) => task,
            Ok(None) => {
                eprintln!("Warning: Task {} not found, skipping", task_id);
                continue;
            }
            Err(e) => {
                eprintln!("Error: Failed to get task {}: {}", task_id, e);
                continue;
            }
        };
        
        print!("Delete task {} ({})? (y/n): ", task_id, task.description);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        
        if input != "y" && input != "yes" {
            println!("Skipped task {}.", task_id);
            continue;
        }
        
        TaskRepo::delete(conn, *task_id)
            .context(format!("Failed to delete task {}", task_id))?;
        println!("Deleted task {}: {}", task_id, task.description);
        deleted_count += 1;
    }
    
    if deleted_count > 0 {
        println!("Deleted {} task(s)", deleted_count);
    }
    
    Ok(())
}

fn handle_recur(subcommand: RecurCommands) -> Result<()> {
    match subcommand {
        RecurCommands::Run { until } => {
            let conn = DbConnection::connect()?;
            
            // Parse until date (default: now + 14 days)
            let until_ts = if let Some(until_str) = until {
                parse_date_expr(&until_str)?
            } else {
                let now = chrono::Utc::now();
                (now + chrono::Duration::days(14)).timestamp()
            };
            
            let count = RecurGenerator::run(&conn, until_ts)?;
            println!("Generated {} recurring task instance(s)", count);
            Ok(())
        }
    }
}

fn handle_status(json: bool) -> Result<()> {
    use crate::cli::output::format_dashboard;
    use crate::models::TaskStatus;
    
    let conn = DbConnection::connect()?;
    
    // Query clock state and current task
    let open_session = SessionRepo::get_open(&conn)?;
    let stack = StackRepo::get_or_create_default(&conn)?;
    let stack_items = StackRepo::get_items(&conn, stack.id.unwrap())?;
    
    let clock_state = if let Some(session) = &open_session {
        let duration = chrono::Utc::now().timestamp() - session.start_ts;
        if let Some(top_item) = stack_items.first() {
            if session.task_id == top_item.task_id {
                Some((session.task_id, duration))
            } else {
                Some((top_item.task_id, 0)) // Clocked out but has task in stack
            }
        } else {
            Some((session.task_id, duration)) // Clocked in but no stack
        }
    } else if let Some(top_item) = stack_items.first() {
        Some((top_item.task_id, 0)) // Clocked out, task in stack
    } else {
        None // No clock, no stack
    };
    
    // Query top 3 clock stack tasks with details
    let clock_stack_tasks: Vec<(usize, Task, Vec<String>)> = stack_items
        .iter()
        .take(3)
        .enumerate()
        .filter_map(|(idx, item)| {
            if let Ok(Some(task)) = TaskRepo::get_by_id(&conn, item.task_id) {
                if let Ok(tags) = TaskRepo::get_tags(&conn, item.task_id) {
                    return Some((idx, task, tags));
                }
            }
            None
        })
        .collect();
    
    // Query today's session summary
    let now = chrono::Utc::now();
    let today_start = Local.with_ymd_and_hms(
        now.year(), now.month(), now.day(), 0, 0, 0
    ).single()
    .map(|dt| dt.with_timezone(&chrono::Utc).timestamp())
    .unwrap_or(0);
    
    let all_sessions = SessionRepo::list_all(&conn)?;
    let today_sessions: Vec<_> = all_sessions.iter()
        .filter(|s| s.start_ts >= today_start)
        .collect();
    
    let today_duration: i64 = today_sessions.iter()
        .filter_map(|s| {
            if let Some(end_ts) = s.end_ts {
                Some(end_ts - s.start_ts)
            } else {
                Some(now.timestamp() - s.start_ts)
            }
        })
        .sum();
    
    // Query overdue tasks (due_ts < now && status = pending)
    let all_tasks = TaskRepo::list_all(&conn)?;
    let now_ts = chrono::Utc::now().timestamp();
    
    let overdue_tasks: Vec<_> = all_tasks.iter()
        .filter(|(task, _)| {
            task.status == TaskStatus::Pending &&
            task.due_ts.is_some() &&
            task.due_ts.unwrap() < now_ts
        })
        .collect();
    
    // Calculate next overdue date if none overdue
    let next_overdue = if overdue_tasks.is_empty() {
        all_tasks.iter()
            .filter(|(task, _)| {
                task.status == TaskStatus::Pending &&
                task.due_ts.is_some() &&
                task.due_ts.unwrap() >= now_ts
            })
            .map(|(task, _)| task.due_ts.unwrap())
            .min()
    } else {
        None
    };
    
    // Query top 3 priority tasks NOT in clock stack
    use crate::cli::priority::get_top_priority_tasks;
    let stack_task_ids: Vec<i64> = stack_items.iter().map(|item| item.task_id).collect();
    let priority_tasks = get_top_priority_tasks(&conn, &stack_task_ids, 3)?;
    
    // Resolve clock task description (for status JSON)
    let clock_task_description = if let Some((task_id, _)) = clock_state {
        TaskRepo::get_by_id(&conn, task_id)?
            .map(|task| task.description)
    } else {
        None
    };
    
    // Format and display dashboard
    if json {
        let dashboard_json = serde_json::json!({
            "clock": {
                "state": if clock_state.is_some() { "in" } else { "out" },
                "task_id": clock_state.map(|(id, _)| id),
                "task_description": clock_task_description,
                "duration_secs": clock_state.map(|(_, d)| d),
            },
            "clock_stack": clock_stack_tasks.iter().map(|(idx, task, tags)| {
                serde_json::json!({
                    "position": idx,
                    "id": task.id,
                    "description": task.description,
                    "status": task.status.as_str(),
                    "project_id": task.project_id,
                    "tags": tags,
                    "due_ts": task.due_ts,
                    "allocation_secs": task.alloc_secs,
                })
            }).collect::<Vec<_>>(),
            "today_sessions": {
                "count": today_sessions.len(),
                "total_duration_secs": today_duration,
            },
            "overdue": {
                "count": overdue_tasks.len(),
                "next_overdue_ts": next_overdue,
            },
            "priority_tasks": priority_tasks.iter().map(|(task, tags, priority)| {
                serde_json::json!({
                    "id": task.id,
                    "description": task.description,
                    "status": task.status.as_str(),
                    "project_id": task.project_id,
                    "tags": tags,
                    "due_ts": task.due_ts,
                    "allocation_secs": task.alloc_secs,
                    "priority": priority,
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&dashboard_json)?);
    } else {
        let dashboard = format_dashboard(
            &conn,
            clock_state,
            &clock_stack_tasks,
            &priority_tasks,
            today_sessions.len(),
            today_duration,
            overdue_tasks.len(),
            next_overdue,
        )?;
        print!("{}", dashboard);
    }
    
    Ok(())
}

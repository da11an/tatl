use clap::{Parser, Subcommand};
use rusqlite::Connection;
use crate::db::DbConnection;
use crate::repo::{ProjectRepo, TaskRepo, StackRepo, SessionRepo, AnnotationRepo};
use crate::cli::parser::{parse_task_args, join_description};
use crate::utils::{parse_date_expr, parse_duration};
use crate::filter::{parse_filter, filter_tasks};
use std::collections::HashMap;
use anyhow::{Context, Result};

#[derive(Parser)]
#[command(name = "task")]
#[command(about = "Task Ninja - A powerful command-line task management tool")]
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
    },
    /// Modify tasks
    Modify {
        /// Task ID or filter (for now, only ID supported)
        id_or_filter: String,
        /// Modification arguments (description, fields, tags)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        /// Apply to all matching tasks without confirmation
        #[arg(long)]
        yes: bool,
        /// Force one-by-one confirmation for each task
        #[arg(long)]
        interactive: bool,
    },
    /// Stack management commands
    Stack {
        #[command(subcommand)]
        subcommand: StackCommands,
    },
    /// Clock management commands
    Clock {
        #[command(subcommand)]
        subcommand: ClockCommands,
    },
    /// Annotate a task
    Annotate {
        /// Annotation note text
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        note: Vec<String>,
        /// Delete annotation by ID
        #[arg(long)]
        delete: Option<String>,
    },
    /// Mark task(s) as done
    Done {
        /// Task ID or filter (optional, defaults to stack[0])
        id_or_filter: Option<String>,
        /// End time for session (date expression, defaults to now)
        #[arg(long)]
        at: Option<String>,
        /// Start next task in stack after completion
        #[arg(long)]
        next: bool,
        /// Complete all matching tasks without confirmation
        #[arg(long)]
        yes: bool,
        /// Force one-by-one confirmation for each task
        #[arg(long)]
        interactive: bool,
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
pub enum StackCommands {
    /// Show current stack
    Show {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Move task at position to top
    Pick {
        /// Stack position/index (0 = top, -1 = end)
        index: i32,
        /// Ensure clock is running after operation
        #[arg(long)]
        clock_in: bool,
        /// Ensure clock is stopped after operation
        #[arg(long)]
        clock_out: bool,
    },
    /// Rotate stack
    Roll {
        /// Number of positions to rotate (default: 1)
        #[arg(default_value = "1")]
        n: i32,
        /// Ensure clock is running after operation
        #[arg(long)]
        clock_in: bool,
        /// Ensure clock is stopped after operation
        #[arg(long)]
        clock_out: bool,
    },
    /// Remove task at position
    Drop {
        /// Stack position/index (0 = top, -1 = end)
        index: i32,
        /// Ensure clock is running after operation
        #[arg(long)]
        clock_in: bool,
        /// Ensure clock is stopped after operation
        #[arg(long)]
        clock_out: bool,
    },
    /// Clear all tasks from stack
    Clear {
        /// Ensure clock is stopped after operation
        #[arg(long)]
        clock_out: bool,
    },
}

#[derive(Subcommand)]
pub enum ClockCommands {
    /// Start timing the current task (stack[0])
    In {
        /// Start time (date expression, defaults to "now")
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Stop timing the current task
    Out {
        /// End time (date expression, defaults to "now")
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

pub fn run() -> Result<()> {
    // Get raw args to handle special syntax patterns
    let args: Vec<String> = std::env::args().skip(1).collect();
    
    // Check if this is task <id|filter> done pattern
    if args.len() >= 2 {
        if let Some(done_pos) = args.iter().position(|a| a == "done") {
            if done_pos > 0 {
                // We have task <id|filter> done
                let id_or_filter = args[0].clone();
                let done_args = args[done_pos + 1..].to_vec();
                
                // Parse flags
                let at = done_args.iter().position(|a| a == "--at")
                    .and_then(|pos| done_args.get(pos + 1).cloned());
                let next = done_args.contains(&"--next".to_string());
                let yes = done_args.contains(&"--yes".to_string());
                let interactive = done_args.contains(&"--interactive".to_string());
                
                return handle_task_done(Some(id_or_filter), at, next, yes, interactive);
            }
        }
    }
    
    // Check if this is task <id> annotate pattern
    if args.len() >= 2 {
        if let Some(annotate_pos) = args.iter().position(|a| a == "annotate") {
            if annotate_pos > 0 {
                // We have task <id> annotate
                let task_id = args[0].clone();
                let note_args = args[annotate_pos + 1..].to_vec();
                // Check for --delete flag
                if let Some(delete_pos) = note_args.iter().position(|a| a == "--delete") {
                    if delete_pos + 1 < note_args.len() {
                        let annotation_id = note_args[delete_pos + 1].clone();
                        return handle_annotation_delete(task_id, annotation_id);
                    }
                } else {
                    return handle_annotation_add(Some(task_id), note_args);
                }
            }
        }
    }
    
    // Check if this is task <id> clock in pattern
    if args.len() >= 3 {
        if let Some(clock_pos) = args.iter().position(|a| a == "clock") {
            if clock_pos > 0 && clock_pos + 1 < args.len() {
                if args[clock_pos + 1] == "in" {
                    // We have task <id> clock in
                    let task_id = args[0].clone();
                    let clock_args = args[clock_pos + 2..].to_vec();
                    return handle_task_clock_in(task_id, clock_args);
                }
            }
        }
    }
    
    // Check if this is task <id> enqueue pattern
    if args.len() >= 2 {
        if let Some(enqueue_pos) = args.iter().position(|a| a == "enqueue") {
            if enqueue_pos > 0 {
                // We have task <id> enqueue
                let task_id = args[0].clone();
                return handle_task_enqueue(task_id);
            }
        }
    }
    
    // Check if this is task <id|filter> modify pattern
    if args.len() >= 2 {
        // Look for "modify" subcommand
        if let Some(modify_pos) = args.iter().position(|a| a == "modify") {
            if modify_pos > 0 {
                // We have task <id|filter> modify
                let id_or_filter = args[0].clone();
                let modify_args = args[modify_pos + 1..].to_vec();
                
                // Parse flags
                let yes = modify_args.contains(&"--yes".to_string());
                let interactive = modify_args.contains(&"--interactive".to_string());
                
                return handle_task_modify(id_or_filter, modify_args, yes, interactive);
            }
        }
    }
    
    // Check if this is task stack <index> pick/drop pattern
    if args.len() >= 3 && args[0] == "stack" {
        if let Ok(index) = args[1].parse::<i32>() {
            if args[2] == "pick" {
                return handle_stack_pick(index);
            } else if args[2] == "drop" {
                return handle_stack_drop(index);
            }
        }
    }
    
    // Otherwise use clap parsing
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Projects { subcommand } => handle_projects(subcommand),
        Commands::Add { args } => handle_task_add(args),
        Commands::List { filter, json } => handle_task_list(filter, json),
        Commands::Modify { id_or_filter, args, yes, interactive } => {
            handle_task_modify(id_or_filter, args, yes, interactive)
        }
        Commands::Stack { subcommand } => handle_stack(subcommand),
        Commands::Clock { subcommand } => handle_clock(subcommand),
        Commands::Annotate { note, delete } => {
            if let Some(annotation_id) = delete {
                eprintln!("Error: Task ID required when deleting annotation. Use: task <id> annotate --delete <annotation_id>");
                std::process::exit(1);
            } else {
                handle_annotation_add(None, note)
            }
        }
        Commands::Done { id_or_filter, at, next, yes, interactive } => {
            handle_task_done(id_or_filter, at, next, yes, interactive)
        }
    }
}

fn handle_projects(cmd: ProjectCommands) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    match cmd {
        ProjectCommands::Add { name } => {
            // Check if project already exists
            if let Some(_) = ProjectRepo::get_by_name(&conn, &name)? {
                eprintln!("Error: Project '{}' already exists", name);
                std::process::exit(1);
            }
            
            let project = ProjectRepo::create(&conn, &name)
                .context("Failed to create project")?;
            
            println!("Created project '{}' (id: {})", project.name, project.id.unwrap());
            Ok(())
        }
        ProjectCommands::List { archived, json } => {
            let projects = ProjectRepo::list(&conn, archived)
                .context("Failed to list projects")?;
            
            if json {
                let json_output = serde_json::to_string_pretty(&projects)
                    .context("Failed to serialize projects to JSON")?;
                println!("{}", json_output);
            } else {
                if projects.is_empty() {
                    println!("No projects found.");
                } else {
                    for project in projects {
                        let status = if project.is_archived { "[archived]" } else { "" };
                        println!("{} {}", project.name, status);
                    }
                }
            }
            Ok(())
        }
        ProjectCommands::Rename { old_name, new_name, force } => {
            // Check if old project exists
            if ProjectRepo::get_by_name(&conn, &old_name)?.is_none() {
                eprintln!("Error: Project '{}' not found", old_name);
                std::process::exit(1);
            }
            
            // Check if new name already exists
            if let Some(_) = ProjectRepo::get_by_name(&conn, &new_name)? {
                if force {
                    // Merge projects
                    ProjectRepo::merge(&conn, &old_name, &new_name)
                        .context("Failed to merge projects")?;
                    println!("Merged project '{}' into '{}'", old_name, new_name);
                } else {
                    eprintln!("Error: Project '{}' already exists. Use --force to merge.", new_name);
                    std::process::exit(1);
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

fn handle_task_add(args: Vec<String>) -> Result<()> {
    if args.is_empty() {
        eprintln!("Error: Task description is required");
        std::process::exit(1);
    }
    
    let parsed = parse_task_args(args);
    
    // Validate description
    if parsed.description.is_empty() {
        eprintln!("Error: Task description is required");
        std::process::exit(1);
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
            eprintln!("Error: Project '{}' not found", project_name);
            std::process::exit(1);
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
    let alloc_secs = if let Some(alloc) = parsed.alloc {
        Some(parse_duration(&alloc).context("Failed to parse allocation duration")?)
    } else {
        None
    };
    
    // Create task
    let task = TaskRepo::create_full(
        &conn,
        &description,
        project_id,
        due_ts,
        scheduled_ts,
        wait_ts,
        alloc_secs,
        parsed.template,
        parsed.recur,
        &parsed.udas,
        &parsed.tags_add,
    )
    .context("Failed to create task")?;
    
    println!("Created task {}: {}", task.id.unwrap(), description);
    Ok(())
}

fn handle_task_list(filter_args: Vec<String>, json: bool) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // Parse filter if provided
    let tasks = if filter_args.is_empty() {
        TaskRepo::list_all(&conn)
            .context("Failed to list tasks")?
    } else {
        let filter_expr = parse_filter(filter_args)
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
        // Human-readable output
        for (task, tags) in tasks {
            let id = task.id.unwrap();
            let mut parts = vec![format!("{}", id), task.description];
            
            if let Some(project_id) = task.project_id {
                // TODO: Get project name (for now just show ID)
                parts.push(format!("[project:{}]", project_id));
            }
            
            if !tags.is_empty() {
                let tag_str = tags.iter().map(|t| format!("+{}", t)).collect::<Vec<_>>().join(" ");
                parts.push(tag_str);
            }
            
            println!("{}", parts.join(" "));
        }
    }
    
    Ok(())
}

fn handle_task_modify(id_or_filter: String, args: Vec<String>, _yes: bool, _interactive: bool) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // For now, only support numeric ID (full filter support in Phase 3)
    let task_id: i64 = id_or_filter.parse()
        .map_err(|_| anyhow::anyhow!("Invalid task ID: {}. Filter support will be added in Phase 3.", id_or_filter))?;
    
    // Check if task exists
    if TaskRepo::get_by_id(&conn, task_id)?.is_none() {
        eprintln!("Error: Task {} not found", task_id);
        std::process::exit(1);
    }
    
    // Parse modification arguments
    let parsed = parse_task_args(args);
    
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
            } else {
                eprintln!("Error: Project '{}' not found", project_name);
                std::process::exit(1);
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
    let alloc_secs = if let Some(alloc) = &parsed.alloc {
        if alloc == "none" {
            Some(None)
        } else {
            Some(Some(parse_duration(alloc).context("Failed to parse allocation duration")?))
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

fn handle_stack(cmd: StackCommands) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    match cmd {
        StackCommands::Show { json } => {
            let stack = StackRepo::get_or_create_default(&conn)?;
            let stack_id = stack.id.unwrap();
            let items = StackRepo::get_items(&conn, stack_id)?;
            
            if json {
                let json_items: Vec<serde_json::Value> = items.iter().enumerate().map(|(idx, item)| {
                    serde_json::json!({
                        "index": idx,
                        "task_id": item.task_id,
                        "ordinal": item.ordinal,
                    })
                }).collect();
                println!("{}", serde_json::to_string_pretty(&json_items)?);
            } else {
                if items.is_empty() {
                    println!("[]");
                } else {
                    let task_ids: Vec<String> = items.iter().map(|item| item.task_id.to_string()).collect();
                    println!("[{}]", task_ids.join(","));
                }
            }
            Ok(())
        }
        StackCommands::Pick { index, clock_in, clock_out } => {
            handle_stack_pick_with_clock(&conn, index, clock_in, clock_out)
        }
        StackCommands::Roll { n, clock_in, clock_out } => {
            handle_stack_roll_with_clock(&conn, n, clock_in, clock_out)
        }
        StackCommands::Drop { index, clock_in, clock_out } => {
            handle_stack_drop_with_clock(&conn, index, clock_in, clock_out)
        }
        StackCommands::Clear { clock_out } => {
            handle_stack_clear_with_clock(&conn, clock_out)
        }
    }
}

/// Handle stack pick with clock state management
fn handle_stack_pick_with_clock(conn: &Connection, index: i32, clock_in: bool, clock_out: bool) -> Result<()> {
    let stack = StackRepo::get_or_create_default(conn)?;
    let stack_id = stack.id.unwrap();
    
    // Get current stack state
    let items_before = StackRepo::get_items(conn, stack_id)?;
    let old_top_task = items_before.get(0).map(|item| item.task_id);
    
    // Perform the pick operation
    StackRepo::pick(conn, stack_id, index)
        .context("Failed to pick task")?;
    
    // Get new stack state
    let items_after = StackRepo::get_items(conn, stack_id)?;
    let new_top_task = items_after.get(0).map(|item| item.task_id);
    
    // Handle clock state
    handle_stack_clock_state(conn, old_top_task, new_top_task, clock_in, clock_out)?;
    
    println!("Moved task at position {} to top", index);
    Ok(())
}

/// Handle stack roll with clock state management
fn handle_stack_roll_with_clock(conn: &Connection, n: i32, clock_in: bool, clock_out: bool) -> Result<()> {
    let stack = StackRepo::get_or_create_default(conn)?;
    let stack_id = stack.id.unwrap();
    
    // Get current stack state
    let items_before = StackRepo::get_items(conn, stack_id)?;
    let old_top_task = items_before.get(0).map(|item| item.task_id);
    
    // Perform the roll operation
    StackRepo::roll(conn, stack_id, n)
        .context("Failed to roll stack")?;
    
    // Get new stack state
    let items_after = StackRepo::get_items(conn, stack_id)?;
    let new_top_task = items_after.get(0).map(|item| item.task_id);
    
    // Handle clock state
    handle_stack_clock_state(conn, old_top_task, new_top_task, clock_in, clock_out)?;
    
    println!("Rotated stack by {} position(s)", n);
    Ok(())
}

/// Handle stack drop with clock state management
fn handle_stack_drop_with_clock(conn: &Connection, index: i32, clock_in: bool, clock_out: bool) -> Result<()> {
    let stack = StackRepo::get_or_create_default(conn)?;
    let stack_id = stack.id.unwrap();
    
    // Get current stack state
    let items_before = StackRepo::get_items(conn, stack_id)?;
    let old_top_task = items_before.get(0).map(|item| item.task_id);
    
    // Perform the drop operation
    StackRepo::drop(conn, stack_id, index)
        .context("Failed to drop task")?;
    
    // Get new stack state
    let items_after = StackRepo::get_items(conn, stack_id)?;
    let new_top_task = items_after.get(0).map(|item| item.task_id);
    
    // Handle clock state
    handle_stack_clock_state(conn, old_top_task, new_top_task, clock_in, clock_out)?;
    
    println!("Removed task at position {}", index);
    Ok(())
}

/// Handle stack clear with clock state management
fn handle_stack_clear_with_clock(conn: &Connection, clock_out: bool) -> Result<()> {
    let stack = StackRepo::get_or_create_default(conn)?;
    let stack_id = stack.id.unwrap();
    
    // Get current stack state
    let items_before = StackRepo::get_items(conn, stack_id)?;
    let old_top_task = items_before.get(0).map(|item| item.task_id);
    
    // Perform the clear operation
    StackRepo::clear(conn, stack_id)
        .context("Failed to clear stack")?;
    
    // Handle clock state (new top is None since stack is empty)
    handle_stack_clock_state(conn, old_top_task, None, false, clock_out)?;
    
    println!("Cleared stack");
    Ok(())
}

/// Handle clock state changes for stack operations
/// Default behavior: if clock is running and stack[0] changes, close current session and start new one
/// Flags: --clock_in ensures clock is running, --clock_out ensures clock is stopped
fn handle_stack_clock_state(
    conn: &Connection,
    old_top_task: Option<i64>,
    new_top_task: Option<i64>,
    clock_in: bool,
    clock_out: bool,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let open_session = SessionRepo::get_open(conn)?;
    
    // Handle --clock_out flag first (takes precedence)
    if clock_out {
        if let Some(_) = open_session {
            SessionRepo::close_open(conn, now)
                .context("Failed to close session")?;
        }
        return Ok(());
    }
    
    // Handle --clock_in flag
    if clock_in {
        if let Some(task_id) = new_top_task {
            // Close existing session if any
            if let Some(_) = open_session {
                SessionRepo::close_open(conn, now)
                    .context("Failed to close existing session")?;
            }
            // Start new session for stack[0]
            SessionRepo::create(conn, task_id, now)
                .context("Failed to start session")?;
        }
        return Ok(());
    }
    
    // Default behavior: if clock is running and stack[0] changed, switch sessions
    if open_session.is_some() {
        if old_top_task != new_top_task {
            // Close current session
            SessionRepo::close_open(conn, now)
                .context("Failed to close session")?;
            
            // Start new session for new stack[0] if stack is not empty
            if let Some(task_id) = new_top_task {
                SessionRepo::create(conn, task_id, now)
                    .context("Failed to start new session")?;
            }
        }
    }
    // If clock is not running, do nothing (stack operations don't create sessions)
    
    Ok(())
}

fn handle_stack_pick(index: i32) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    handle_stack_pick_with_clock(&conn, index, false, false)
}

fn handle_stack_drop(index: i32) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    handle_stack_drop_with_clock(&conn, index, false, false)
}

fn handle_task_enqueue(task_id_str: String) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    let task_id: i64 = task_id_str.parse()
        .map_err(|_| anyhow::anyhow!("Invalid task ID: {}", task_id_str))?;
    
    // Check if task exists
    if TaskRepo::get_by_id(&conn, task_id)?.is_none() {
        eprintln!("Error: Task {} not found", task_id);
        std::process::exit(1);
    }
    
    let stack = StackRepo::get_or_create_default(&conn)?;
    StackRepo::enqueue(&conn, stack.id.unwrap(), task_id)
        .context("Failed to enqueue task")?;
    
    println!("Enqueued task {}", task_id);
    Ok(())
}

fn handle_clock(cmd: ClockCommands) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    match cmd {
        ClockCommands::In { args } => {
            handle_clock_in(&conn, args)
        }
        ClockCommands::Out { args } => {
            handle_clock_out(&conn, args)
        }
    }
}

fn handle_clock_in(conn: &Connection, args: Vec<String>) -> Result<()> {
    // Get stack and check if it's empty
    let stack = StackRepo::get_or_create_default(conn)?;
    let stack_id = stack.id.unwrap();
    let items = StackRepo::get_items(conn, stack_id)?;
    
    if items.is_empty() {
        eprintln!("Error: Stack is empty. Add a task to the stack first.");
        std::process::exit(1);
    }
    
    // Get stack[0] task
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
            eprintln!("Error: A session is already running. Please clock out first.");
            std::process::exit(1);
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
        
        println!("Started timing task {}", task_id);
    }
    
    Ok(())
}

fn handle_clock_out(conn: &Connection, args: Vec<String>) -> Result<()> {
    // Check if session is running
    let session_opt = SessionRepo::get_open(conn)?;
    
    if session_opt.is_none() {
        eprintln!("Error: No session is currently running.");
        std::process::exit(1);
    }
    
    // Parse end time (defaults to "now")
    let end_ts = if args.is_empty() {
        chrono::Utc::now().timestamp()
    } else {
        let end_expr = args.join(" ");
        parse_date_expr(&end_expr)
            .context("Invalid end time expression")?
    };
    
    // Close session
    let closed = SessionRepo::close_open(conn, end_ts)
        .context("Failed to close session")?;
    
    if let Some(session) = closed {
        println!("Stopped timing task {}", session.task_id);
    }
    
    Ok(())
}

fn handle_task_clock_in(task_id_str: String, args: Vec<String>) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // Parse task ID
    let task_id: i64 = task_id_str.parse()
        .map_err(|_| anyhow::anyhow!("Invalid task ID: {}", task_id_str))?;
    
    // Check if task exists
    if TaskRepo::get_by_id(&conn, task_id)?.is_none() {
        eprintln!("Error: Task {} not found", task_id);
        std::process::exit(1);
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
    
    // Check if session is already running
    let existing_session = SessionRepo::get_open(&conn)?;
    
    // If session is running, close it at the effective start time
    if existing_session.is_some() {
        SessionRepo::close_open(&conn, start_ts)
            .context("Failed to close existing session")?;
    }
    
    // Check for overlap prevention (before creating new session)
    check_and_amend_overlaps(&conn, start_ts)?;
    
    // Push task to stack[0]
    let stack = StackRepo::get_or_create_default(&conn)?;
    StackRepo::push_to_top(&conn, stack.id.unwrap(), task_id)
        .context("Failed to push task to stack")?;
    
    // Create session (closed if interval, open otherwise)
    if let Some(end_ts) = end_ts_opt {
        SessionRepo::create_closed(&conn, task_id, start_ts, end_ts)
            .context("Failed to create closed session")?;
        println!("Recorded session for task {} ({} to {})", task_id, start_ts, end_ts);
    } else {
        SessionRepo::create(&conn, task_id, start_ts)
            .context("Failed to start session")?;
        println!("Started timing task {}", task_id);
    }
    
    Ok(())
}

/// Check for closed sessions that end after the given start time and amend them
/// to prevent overlap
fn check_and_amend_overlaps(conn: &Connection, new_start_ts: i64) -> Result<()> {
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
        eprintln!("Error: Annotation note cannot be empty");
        std::process::exit(1);
    }
    
    let note = note_args.join(" ");
    
    // Determine task ID
    let task_id = if let Some(tid_str) = task_id_opt {
        // Task ID provided
        tid_str.parse::<i64>()
            .map_err(|_| anyhow::anyhow!("Invalid task ID: {}", tid_str))?
    } else {
        // No task ID - check if clocked in
        let open_session = SessionRepo::get_open(&conn)?;
        if let Some(session) = open_session {
            session.task_id
        } else {
            eprintln!("Error: No task ID provided and no session is running. Please specify a task ID or clock in first.");
            std::process::exit(1);
        }
    };
    
    // Check if task exists
    if TaskRepo::get_by_id(&conn, task_id)?.is_none() {
        eprintln!("Error: Task {} not found", task_id);
        std::process::exit(1);
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

fn handle_annotation_delete(task_id_str: String, annotation_id_str: String) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    let task_id: i64 = task_id_str.parse()
        .map_err(|_| anyhow::anyhow!("Invalid task ID: {}", task_id_str))?;
    
    let annotation_id: i64 = annotation_id_str.parse()
        .map_err(|_| anyhow::anyhow!("Invalid annotation ID: {}", annotation_id_str))?;
    
    // Check if task exists
    if TaskRepo::get_by_id(&conn, task_id)?.is_none() {
        eprintln!("Error: Task {} not found", task_id);
        std::process::exit(1);
    }
    
    // Delete annotation (verifies it belongs to the task)
    AnnotationRepo::delete_for_task(&conn, task_id, annotation_id)
        .context("Failed to delete annotation")?;
    
    println!("Deleted annotation {} from task {}", annotation_id, task_id);
    Ok(())
}

fn handle_task_done(
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
    let running_task_id = open_session.as_ref().map(|s| s.task_id);
    
    // Determine which tasks to complete
    let task_ids = if let Some(id_or_filter) = id_or_filter_opt {
        // Task ID or filter provided
        // Try to parse as ID first
        if let Ok(task_id) = id_or_filter.parse::<i64>() {
            // Single task ID
            vec![task_id]
        } else {
            // Filter expression
            let filter_expr = parse_filter(vec![id_or_filter])
                .map_err(|e| anyhow::anyhow!("Filter parse error: {}", e))?;
            let matching_tasks = filter_tasks(&conn, &filter_expr)
                .context("Failed to filter tasks")?;
            
            // Filter to only tasks with running sessions
            let tasks_with_sessions: Vec<i64> = matching_tasks
                .iter()
                .filter_map(|(task, _)| {
                    if let Some(task_id) = task.id {
                        if running_task_id == Some(task_id) {
                            Some(task_id)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
            
            if tasks_with_sessions.is_empty() {
                eprintln!("Error: No matching tasks with running sessions found.");
                std::process::exit(1);
            }
            
            tasks_with_sessions
        }
    } else {
        // No ID provided - use stack[0]
        let stack = StackRepo::get_or_create_default(&conn)?;
        let stack_id = stack.id.unwrap();
        let items = StackRepo::get_items(&conn, stack_id)?;
        
        if items.is_empty() {
            eprintln!("Error: Stack is empty. Cannot complete task.");
            std::process::exit(1);
        }
        
        // Check if session is running
        if open_session.is_none() {
            eprintln!("Error: No session is running. Cannot complete task.");
            std::process::exit(1);
        }
        
        // Verify the running session is for stack[0]
        let stack_task_id = items[0].task_id;
        if let Some(session) = &open_session {
            if session.task_id != stack_task_id {
                eprintln!("Error: Running session is for task {}, but stack[0] is task {}. Cannot complete.", session.task_id, stack_task_id);
                std::process::exit(1);
            }
        }
        
        vec![stack_task_id]
    };
    
    // Handle multiple tasks with confirmation
    if task_ids.len() > 1 {
        if !yes && !interactive {
            // Prompt for confirmation
            println!("This will complete {} task(s).", task_ids.len());
            print!("Complete all tasks? (y/n/i): ");
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
                    return handle_done_interactive(&conn, &task_ids, end_ts, next);
                }
                _ => {
                    println!("Invalid input. Cancelled.");
                    return Ok(());
                }
            }
        } else if interactive {
            // Force interactive mode
            return handle_done_interactive(&conn, &task_ids, end_ts, next);
        }
    }
    
    // Complete all tasks
    let mut completed_stack_top = false;
    for task_id in &task_ids {
        // Verify task exists
        if TaskRepo::get_by_id(&conn, *task_id)?.is_none() {
            eprintln!("Error: Task {} not found", task_id);
            continue;
        }
        
        // Check if session is running for this task
        if let Some(session) = &open_session {
            if session.task_id == *task_id {
                // Close the session
                SessionRepo::close_open(&conn, end_ts)
                    .context("Failed to close session")?;
                completed_stack_top = true;
            }
        } else {
            eprintln!("Warning: No session is running for task {}. Skipping.", task_id);
            continue;
        }
        
        // Mark task as completed
        TaskRepo::complete(&conn, *task_id)
            .context("Failed to complete task")?;
        
        // Remove from stack
        let stack = StackRepo::get_or_create_default(&conn)?;
        let stack_id = stack.id.unwrap();
        let items = StackRepo::get_items(&conn, stack_id)?;
        
        // Find the task in the stack and remove it
        if let Some(item) = items.iter().find(|item| item.task_id == *task_id) {
            // Drop the task at this position using its ordinal
            StackRepo::drop(&conn, stack_id, item.ordinal as i32)?;
        }
        
        println!("Completed task {}", task_id);
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
            println!("Started timing task {}", next_task_id);
        }
    }
    
    Ok(())
}

fn handle_done_interactive(conn: &Connection, task_ids: &[i64], end_ts: i64, next: bool) -> Result<()> {
    use std::io::{self, Write};
    
    let open_session = SessionRepo::get_open(conn)?;
    let mut completed_stack_top = false;
    
    for task_id in task_ids {
        // Get task description for display
        let task = TaskRepo::get_by_id(conn, *task_id)?;
        if task.is_none() {
            eprintln!("Error: Task {} not found", task_id);
            continue;
        }
        let task = task.unwrap();
        
        // Check if session is running for this task
        if let Some(session) = &open_session {
            if session.task_id != *task_id {
                println!("Task {}: No running session. Skipping.", task_id);
                continue;
            }
        } else {
            println!("Task {}: No running session. Skipping.", task_id);
            continue;
        }
        
        // Prompt for confirmation
        print!("Complete task {} ({})? (y/n): ", task_id, task.description);
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
            .context("Failed to complete task")?;
        
        // Remove from stack
        let stack = StackRepo::get_or_create_default(conn)?;
        let stack_id = stack.id.unwrap();
        let items = StackRepo::get_items(conn, stack_id)?;
        
        // Find the task in the stack and remove it
        if let Some(item) = items.iter().find(|item| item.task_id == *task_id) {
            // Drop the task at this position using its ordinal
            StackRepo::drop(conn, stack_id, item.ordinal as i32)?;
        }
        
        println!("Completed task {}", task_id);
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

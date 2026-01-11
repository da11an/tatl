use clap::{Parser, Subcommand};
use rusqlite::Connection;
use crate::db::DbConnection;
use crate::repo::{ProjectRepo, TaskRepo, StackRepo, SessionRepo};
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
    },
    /// Rotate stack
    Roll {
        /// Number of positions to rotate (default: 1)
        #[arg(default_value = "1")]
        n: i32,
    },
    /// Remove task at position
    Drop {
        /// Stack position/index (0 = top, -1 = end)
        index: i32,
    },
    /// Clear all tasks from stack
    Clear,
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
        StackCommands::Pick { index } => {
            let stack = StackRepo::get_or_create_default(&conn)?;
            StackRepo::pick(&conn, stack.id.unwrap(), index)
                .context("Failed to pick task")?;
            println!("Moved task at position {} to top", index);
            Ok(())
        }
        StackCommands::Roll { n } => {
            let stack = StackRepo::get_or_create_default(&conn)?;
            StackRepo::roll(&conn, stack.id.unwrap(), n)
                .context("Failed to roll stack")?;
            println!("Rotated stack by {} position(s)", n);
            Ok(())
        }
        StackCommands::Drop { index } => {
            let stack = StackRepo::get_or_create_default(&conn)?;
            StackRepo::drop(&conn, stack.id.unwrap(), index)
                .context("Failed to drop task")?;
            println!("Removed task at position {}", index);
            Ok(())
        }
        StackCommands::Clear => {
            let stack = StackRepo::get_or_create_default(&conn)?;
            StackRepo::clear(&conn, stack.id.unwrap())
                .context("Failed to clear stack")?;
            println!("Cleared stack");
            Ok(())
        }
    }
}

fn handle_stack_pick(index: i32) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    let stack = StackRepo::get_or_create_default(&conn)?;
    StackRepo::pick(&conn, stack.id.unwrap(), index)
        .context("Failed to pick task")?;
    println!("Moved task at position {} to top", index);
    Ok(())
}

fn handle_stack_drop(index: i32) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    let stack = StackRepo::get_or_create_default(&conn)?;
    StackRepo::drop(&conn, stack.id.unwrap(), index)
        .context("Failed to drop task")?;
    println!("Removed task at position {}", index);
    Ok(())
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
    
    // Check if session is already running
    if let Some(_) = SessionRepo::get_open(conn)? {
        eprintln!("Error: A session is already running. Please clock out first.");
        std::process::exit(1);
    }
    
    // Get stack[0] task
    let task_id = items[0].task_id;
    
    // Parse start time (defaults to "now")
    let start_ts = if args.is_empty() {
        chrono::Utc::now().timestamp()
    } else {
        let start_expr = args.join(" ");
        parse_date_expr(&start_expr)
            .context("Invalid start time expression")?
    };
    
    // Create session
    SessionRepo::create(conn, task_id, start_ts)
        .context("Failed to start session")?;
    
    println!("Started timing task {}", task_id);
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

fn handle_task_clock_in(_task_id_str: String, _args: Vec<String>) -> Result<()> {
    // This will be implemented in Phase 5.3
    // For now, just show an error
    eprintln!("Error: task <id> clock in not yet implemented (Phase 5.3)");
    std::process::exit(1);
}

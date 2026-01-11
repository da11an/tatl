use clap::{Parser, Subcommand};
use crate::db::DbConnection;
use crate::repo::{ProjectRepo, TaskRepo};
use crate::cli::parser::{parse_task_args, join_description};
use crate::utils::{parse_date_expr, parse_duration};
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

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Projects { subcommand } => handle_projects(subcommand),
        Commands::Add { args } => handle_task_add(args),
        Commands::List { json } => handle_task_list(json),
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

fn handle_task_list(json: bool) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    let tasks = TaskRepo::list_all(&conn)
        .context("Failed to list tasks")?;
    
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

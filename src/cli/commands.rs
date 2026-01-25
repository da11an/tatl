use clap::{Parser, Subcommand};
use rusqlite::Connection;
use crate::db::DbConnection;
use crate::repo::{ProjectRepo, TaskRepo, StackRepo, SessionRepo, AnnotationRepo, TemplateRepo, ViewRepo, ExternalRepo};
use crate::cli::parser::{parse_task_args, join_description};
use crate::cli::commands_sessions::{handle_task_sessions_list_with_filter, handle_task_sessions_show_with_filter, handle_sessions_modify, handle_sessions_delete, handle_sessions_report};
use crate::cli::output::{format_task_list_table, format_task_summary, TaskListOptions};
use crate::cli::error::{user_error, validate_task_id, validate_project_name, parse_task_id_spec, parse_task_id_list};
use crate::utils::{parse_date_expr, parse_duration, fuzzy};
use crate::filter::{parse_filter, filter_tasks};
use crate::respawn::respawn_task;
use crate::cli::abbrev;
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
    #[command(long_about = "Create a new task with optional attributes, timing, and queue placement.

The task description is all text that doesn't match field patterns. Field syntax includes:
  project:<name>     - Assign to project (creates if new with -y)
  due:<expr>         - Set due date (see DATE EXPRESSIONS below)
  scheduled:<expr>   - Set scheduled date
  wait:<expr>        - Set wait date
  allocation:<dur>   - Set time allocation (e.g., \"2h\", \"30m\", \"1d\")
  template:<name>    - Use template
  respawn:<pattern>  - Set respawn rule (see RESPAWN PATTERNS below)
  +<tag>             - Add tag
  -<tag>             - Remove tag
  uda.<key>:<value>  - Set user-defined attribute

DATE EXPRESSIONS:
  Relative: tomorrow, +3d, -1w, +2m, +1y
  Absolute: 2024-01-15, 2024-01-15 14:30
  Time-only: 09:00, 14:30

RESPAWN PATTERNS:
  Simple: daily, weekly, monthly, yearly
  Advanced: weekdays:mon,wed,fri, monthdays:1,15, nth:1:day, every:2w

If --onoff is specified, it takes precedence over --on and --enqueue.

EXAMPLES:
  tatl add \"Fix bug\" project:work +urgent
  tatl add \"Review PR\" due:tomorrow allocation:1h
  tatl add \"Daily standup\" respawn:daily due:09:00
  tatl add --on \"Start working on feature\"
  tatl add \"Forgot to track meeting\" --onoff 14:00..15:00 project:meetings")]
    Add {
        /// Start timing immediately after creation. If TIME is provided (e.g., --on=14:00), the session starts at that time instead of now. Pushes task to queue[0].
        #[arg(long = "on", visible_alias = "clock-in", num_args = 0..=1, require_equals = true, default_missing_value = "")]
        start_timing: Option<String>,
        /// Add historical session for the task. Interval format: \"start..end\" (e.g., \"09:00..12:00\"). Takes precedence over --on and --enqueue.
        #[arg(long = "onoff")]
        onoff_interval: Option<String>,
        /// Add task to end of queue without starting timing
        #[arg(long = "enqueue")]
        enqueue: bool,
        /// Auto-confirm prompts (create new projects, modify overlapping sessions)
        #[arg(short = 'y', long)]
        yes: bool,
        /// Task description and fields. The description is all text not matching field patterns. Examples: \"fix bug project:work +urgent\", \"Review PR due:tomorrow allocation:1h\"
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// List tasks
    #[command(long_about = "List tasks matching optional filter criteria.

FILTER SYNTAX:
  Field filters:
    id:<n>              - Match by task ID
    status:<status>      - Match by status (pending, completed, closed, deleted)
    project:<name>       - Match by project (supports prefix matching for nested projects)
    due:<expr>           - Match by due date (see DATE EXPRESSIONS)
    scheduled:<expr>     - Match by scheduled date
    wait:<expr>          - Match by wait date
    kanban:<status>      - Match by kanban status (proposed, stalled, queued, external, done)
    desc:<pattern>       - Match description containing pattern (case-insensitive)
    description:<pattern> - Alias for desc:
  
  Tag filters:
    +<tag>               - Tasks with tag
    -<tag>               - Tasks without tag
  
  Derived filters:
    waiting              - Tasks with wait_ts in the future
  
  Operators:
    (implicit AND)       - Adjacent terms are ANDed together
    or                   - OR operator (lowest precedence)
    not                  - NOT operator (highest precedence)
  
  Examples:
    project:work +urgent
    +urgent or +important
    not +waiting
    project:work +urgent or project:home +important
    desc:bug status:pending
    due:tomorrow kanban:queued

DATE EXPRESSIONS (for due:, scheduled:, wait:):
  Relative: tomorrow, +3d, -1w, +2m, +1y
  Absolute: 2024-01-15, 2024-01-15 14:30
  Time-only: 09:00, 14:30
  Intervals: -7d..now, 2024-01-01..2024-01-31

EXAMPLES:
  tatl list
  tatl list project:work +urgent
  tatl list +urgent or +important
  tatl list desc:bug status:pending
  tatl list due:tomorrow kanban:queued --relative")]
    List {
        /// Filter arguments. Multiple filters are ANDed together. Use 'or' for OR, 'not' for NOT. Examples: \"project:work +urgent\", \"+urgent or +important\", \"desc:bug status:pending\"
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        filter: Vec<String>,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show due dates as relative time (e.g., \"2 days ago\", \"in 3 days\")
        #[arg(long)]
        relative: bool,
    },
    /// Show detailed summary of task(s)
    #[command(long_about = "Show detailed information about one or more tasks.

TARGET SYNTAX:
  Single ID:       10
  ID range:        1-5 (tasks 1, 2, 3, 4, 5)
  ID list:         1,3,5 (tasks 1, 3, and 5)
  Filter:          project:work +urgent (same filter syntax as 'tatl list')

The output includes task details, annotations, sessions, and related information.

EXAMPLES:
  tatl show 10
  tatl show 1-5
  tatl show project:work +urgent")]
    Show {
        /// Task ID, ID range (e.g., \"1-5\"), ID list (e.g., \"1,3,5\"), or filter expression. Examples: \"10\", \"1-5\", \"1,3,5\", \"project:work +urgent\"
        target: String,
    },
    /// Modify tasks
    #[command(long_about = "Modify one or more tasks. Target can be a task ID, ID range (e.g., \"1-5\"), ID list (e.g., \"1,3,5\"), or filter expression.

MODIFICATION SYNTAX:
  Field modifications:
    project:<name>       - Assign to project (use \"project:none\" to clear)
    due:<expr>           - Set due date (use \"due:none\" to clear, see DATE EXPRESSIONS)
    scheduled:<expr>      - Set scheduled date (use \"scheduled:none\" to clear)
    wait:<expr>           - Set wait date (use \"wait:none\" to clear)
    allocation:<dur>      - Set time allocation (e.g., \"2h\", \"30m\", use \"allocation:none\" to clear)
    template:<name>       - Set template (use \"template:none\" to clear)
    respawn:<pattern>     - Set respawn rule (use \"respawn:none\" to clear, see RESPAWN PATTERNS)
    uda.<key>:<value>     - Set user-defined attribute (use \"uda.<key>:none\" to clear)
  
  Tag modifications:
    +<tag>                - Add tag
    -<tag>                - Remove tag
  
  Description:
    Any text not matching field patterns becomes the new description.

RESPAWN PATTERNS:
  Simple: daily, weekly, monthly, yearly
  Advanced: weekdays:mon,wed,fri, monthdays:1,15, nth:1:day, every:2w
  
  Respawn rules are validated on modification. A preview message shows what will happen when the task is completed.

DATE EXPRESSIONS:
  Relative: tomorrow, +3d, -1w, +2m, +1y
  Absolute: 2024-01-15, 2024-01-15 14:30
  Time-only: 09:00, 14:30

FILTER SYNTAX (for target selection):
  Same as 'tatl list' filter syntax. See 'tatl list --help' for details.

EXAMPLES:
  tatl modify 10 +urgent due:+2d
  tatl modify project:work description:Updated description
  tatl modify +urgent due:+1d --yes
  tatl modify 5 respawn:daily due:09:00
  tatl modify 1-5 project:work --yes")]
    Modify {
        /// Task ID, ID range (e.g., \"1-5\"), ID list (e.g., \"1,3,5\"), or filter expression. Examples: \"10\", \"1-5\", \"1,3,5\", \"project:work +urgent\"
        target: String,
        /// Modification arguments. Field syntax: project:<name>, due:<expr>, +tag, -tag, etc. Any text not matching field patterns becomes the new description.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        /// Apply to all matching tasks without confirmation (also auto-creates new projects if needed)
        #[arg(short = 'y', long)]
        yes: bool,
        /// Force one-by-one confirmation for each task
        #[arg(long)]
        interactive: bool,
        /// Start timing after modification (pushes to queue[0] and starts timing)
        #[arg(long = "on")]
        start_timing: bool,
    },
    /// Start timing a task
    #[command(long_about = "Start timing a task. If task_id is provided, pushes that task to queue[0] and starts timing. If omitted, starts timing queue[0].

TIME EXPRESSIONS:
  Time-only:       09:00, 14:30 (starts session at that time today)
  Date + time:     2024-01-15 09:00 (starts session at specific date/time)
  Interval:        09:00..11:00 (creates session from 09:00 to 11:00 today)

If an interval is provided, creates a historical session instead of starting a new one.")]
    On {
        /// Task ID (optional, defaults to queue[0]). If provided, pushes task to queue[0] and starts timing.
        task_id: Option<String>,
        /// Time expression or interval. Time-only (e.g., \"09:00\") starts session at that time today. Interval (e.g., \"09:00..11:00\") creates historical session.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        time_args: Vec<String>,
    },
    /// Stop timing current task
    #[command(long_about = "Stop timing the current task (queue[0]). If end time is provided, sets session end to that time instead of now.

TIME EXPRESSIONS:
  Time-only:       14:30 (ends session at that time today)
  Date + time:     2024-01-15 14:30 (ends session at specific date/time)")]
    Off {
        /// End time (optional, defaults to now). Time-only (e.g., \"14:30\") ends session at that time today.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        time_args: Vec<String>,
    },
    /// Stop current session and resume (capture break)
    #[command(long_about = "Capture a break in your work session. Stops the current session and immediately starts a new one for the same task.

This is useful when you're interrupted or take a break. The current session ends at the specified time (or now), and a new session starts immediately after.

TIME EXPRESSIONS:
  Time-only:       14:30 (ends current session at 14:30, starts new one immediately)
  Interval:        14:30..15:00 (ends current session at 14:30, starts new one at 15:00)")]
    Offon {
        /// Time expression or interval. Time-only (e.g., \"14:30\") ends current session at that time and starts new one immediately. Interval (e.g., \"14:30..15:00\") ends current session at start time and starts new one at end time.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        time_args: Vec<String>,
        /// Skip confirmation for history modifications
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Add historical session (or insert into existing time)
    #[command(long_about = "Add a historical session for a task. Useful for logging time you forgot to track or correcting session times.

INTERVAL SYNTAX:
  Time interval:    09:00..12:00 (creates 3-hour session today)
  Date + interval:  2024-01-15 09:00..12:00 (creates session on specific date)
  Task + interval:  <task_id> 09:00..12:00 (adds session to specific task)

If the interval overlaps with existing sessions, you'll be prompted to modify them (use -y to auto-confirm).")]
    Onoff {
        /// Time interval or task ID + interval. Format: \"start..end\" (e.g., \"09:00..12:00\") or \"<task_id> start..end\". If task_id is omitted, uses queue[0] or prompts.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        /// Skip confirmation for overlapping session modifications
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Remove task from queue without finishing
    #[command(long_about = "Remove a task from the queue without completing it. The task remains in pending status.")]
    Dequeue {
        /// Task ID (optional, defaults to queue[0])
        task_id: Option<String>,
    },
    /// Annotate a task
    #[command(long_about = "Add a note or annotation to a task. If you're currently timing a task (queue[0]), the annotation is automatically linked to that task and the current session.

TARGET SYNTAX:
  Omit (when clocked in):  Uses queue[0] and current session
  Task ID:                 10
  ID range:                1-5
  ID list:                 1,3,5
  Filter:                  project:work +urgent

Use --delete <annotation_id> to remove an annotation.")]
    Annotate {
        /// Task ID, ID range, ID list, or filter (optional when clocked in, defaults to queue[0]). Examples: \"10\", \"1-5\", \"1,3,5\", \"project:work +urgent\"
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
    #[command(long_about = "Mark one or more tasks as completed. If task has a respawn rule, a new instance will be created when completed.

TARGET SYNTAX:
  Omit:              Uses queue[0] (current task)
  Task ID:           10
  ID range:          1-5
  ID list:           1,3,5
  Filter:            project:work +urgent

TIME EXPRESSIONS:
  Omit:              Ends session at now
  Time-only:         14:30 (ends session at that time today)
  Date + time:       2024-01-15 14:30

If --next is specified, automatically starts timing the next task in queue after completion.")]
    Finish {
        /// Task ID, ID range, ID list, or filter (optional, defaults to queue[0]). Examples: \"10\", \"1-5\", \"1,3,5\", \"project:work +urgent\"
        target: Option<String>,
        /// End time expression (optional, defaults to now). Time-only (e.g., \"14:30\") ends session at that time today.
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
    #[command(long_about = "Mark one or more tasks as closed (cancelled, won't do, etc.). If task has a respawn rule, a new instance will be created when closed.

TARGET SYNTAX:
  Omit:              Uses queue[0] (current task)
  Task ID:           10
  ID range:          1-5
  ID list:           1,3,5
  Filter:            project:work +urgent")]
    Close {
        /// Task ID, ID range, ID list, or filter (optional, defaults to queue[0]). Examples: \"10\", \"1-5\", \"1,3,5\", \"project:work +urgent\"
        target: Option<String>,
        /// Close all matching tasks without confirmation
        #[arg(short = 'y', long)]
        yes: bool,
        /// Force one-by-one confirmation for each task
        #[arg(long)]
        interactive: bool,
    },
    /// Reopen completed or closed task(s)
    #[command(long_about = "Reopen one or more completed or closed tasks, setting their status back to pending.

TARGET SYNTAX:
  Task ID:           10
  ID range:          1-5
  ID list:           1,3,5
  Filter:            project:work status:completed")]
    Reopen {
        /// Task ID, ID range, ID list, or filter. Examples: \"10\", \"1-5\", \"1,3,5\", \"project:work status:completed\"
        target: String,
        /// Reopen all matching tasks without confirmation
        #[arg(short = 'y', long)]
        yes: bool,
        /// Force one-by-one confirmation for each task
        #[arg(long)]
        interactive: bool,
    },
    /// Permanently delete task(s)
    #[command(long_about = "Permanently delete one or more tasks. This action cannot be undone. All associated sessions, annotations, and events are also deleted.

TARGET SYNTAX:
  Task ID:           10
  ID range:          1-5
  ID list:           1,3,5
  Filter:            project:work status:completed")]
    Delete {
        /// Task ID, ID range, ID list, or filter. Examples: \"10\", \"1-5\", \"1,3,5\", \"project:work status:completed\"
        target: String,
        /// Delete all matching tasks without confirmation
        #[arg(short = 'y', long)]
        yes: bool,
        /// Confirm each task one by one
        #[arg(long)]
        interactive: bool,
    },
    /// Add task to end of clock stack
    #[command(long_about = "Add one or more tasks to the end of the queue. Tasks are added in the order specified. Does not start timing.")]
    Enqueue {
        /// Task ID(s) to enqueue. Can be a single ID or comma-separated list (e.g., \"5\" or \"1,3,5\")
        task_id: String,
    },
    /// Send task to external party for review/approval
    #[command(long_about = "Send a task to an external party (colleague, supervisor, release window, etc.). The task will be removed from the queue and marked as 'external' in kanban view.

The task remains visible but is no longer in your active queue. When the external party returns it, use 'collect' to bring it back under your control.

EXAMPLES:
  tatl send 10 colleague \"Please review this PR\"
  tatl send 5 Release_5.2
  tatl send 3 supervisor \"Needs approval\"")]
    Send {
        /// Task ID
        task_id: String,
        /// Recipient name (e.g., \"colleague\", \"supervisor\", \"Release_5.2\", \"Customer\")
        recipient: String,
        /// Optional request/note about what was requested
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        request: Vec<String>,
    },
    /// Collect task back from external party
    #[command(long_about = "Collect a task that was sent to an external party. Marks all externals for the task as returned. The task returns to normal kanban flow (proposed or stalled, depending on whether it has sessions).

After collecting, you can:
  - Re-queue it: tatl enqueue <task_id>
  - Finish it: tatl finish <task_id>
  - Close it: tatl close <task_id>

EXAMPLES:
  tatl collect 10")]
    Collect {
        /// Task ID
        task_id: String,
    },
    /// List external tasks
    #[command(long_about = "List all tasks that are currently with external parties. Shows task ID, description, recipient, and when it was sent.

FILTERING:
  Filter by recipient: tatl externals colleague
  Filter by task: tatl externals 10

EXAMPLES:
  tatl externals
  tatl externals colleague
  tatl externals Release_5.2")]
    Externals {
        /// Optional filter: recipient name or task ID
        filter: Option<String>,
    },
    /// Sessions management commands
    #[command(long_about = "Manage work sessions. Sessions track time spent on tasks.")]
    Sessions {
        #[command(subcommand)]
        subcommand: SessionsCommands,
        /// Task ID or filter (optional). Filters sessions to specific tasks.
        #[arg(long)]
        task: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Create a new project
    #[command(long_about = "Create a new project. Projects support hierarchical organization using dot notation (e.g., 'work', 'work.email', 'work.email.inbox').")]
    Add {
        /// Project name. Supports nested projects with dot notation (e.g., \"work\", \"work.email\", \"work.email.inbox\")
        name: String,
    },
    /// List projects
    #[command(long_about = "List all projects. Shows project hierarchy and task counts.")]
    List {
        /// Include archived projects in the list
        #[arg(long)]
        archived: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Rename a project
    #[command(long_about = "Rename a project. All tasks assigned to the old project name will be moved to the new name. Use --force to merge with an existing project.")]
    Rename {
        /// Current project name
        old_name: String,
        /// New project name
        new_name: String,
        /// Force merge if new name already exists (moves all tasks from old to new)
        #[arg(long)]
        force: bool,
    },
    /// Archive a project
    #[command(long_about = "Archive a project. Archived projects are hidden from normal listings but can be viewed with --archived flag.")]
    Archive {
        /// Project name to archive
        name: String,
    },
    /// Unarchive a project
    #[command(long_about = "Unarchive a project, making it visible in normal listings again.")]
    Unarchive {
        /// Project name to unarchive
        name: String,
    },
    /// Show task counts by kanban status per project
    #[command(long_about = "Generate a report showing task counts grouped by project and kanban status (proposed, stalled, queued, external, done).")]
    Report,
}


#[derive(Subcommand)]
pub enum SessionsCommands {
    /// List session history
    #[command(long_about = "List work sessions. Can filter by date range, project, tags, or task.

FILTER SYNTAX:
  Date filters:
    -7d              - Last 7 days (relative date)
    -7d..now         - Date interval (last 7 days to now)
    2024-01-01..now  - Date interval (absolute start to now)
  
  Task filters:
    project:<name>   - Sessions for tasks in project
    +<tag>           - Sessions for tasks with tag
    task:<id>        - Sessions for specific task
  
  Examples:
    tatl sessions list -7d
    tatl sessions list -7d..now
    tatl sessions list project:work
    tatl sessions list -7d project:work")]
    List {
        /// Filter arguments. Date filters: -7d, -7d..now, <start>..<end>. Task filters: project:<name>, +tag, task:<id>. Examples: \"-7d\", \"-7d..now\", \"project:work\"
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        filter: Vec<String>,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Show details of the current active session
    #[command(long_about = "Show detailed information about the current active session. If no session is active, displays a message indicating so.")]
    Show,
    /// Modify session start/end times
    #[command(long_about = "Modify the start and/or end time of a session.

INTERVAL SYNTAX:
  <start>..<end>   - Modify both start and end times
  <start>..        - Modify start time only (keep current end)
  ..<end>          - Modify end time only (keep current start)

  Examples:
    09:00..17:00              - Set session to 09:00-17:00 today
    09:00..                   - Change start to 09:00
    ..17:00                   - Change end to 17:00
    2024-01-15 09:00..12:00   - Set specific date and times

If the modification creates overlapping sessions, you'll be prompted to resolve conflicts (use --force to allow overlaps).")]
    Modify {
        /// Session ID to modify
        session_id: i64,
        /// Time interval: \"<start>..<end>\", \"<start>..\", or \"..<end>\". Examples: \"09:00..17:00\", \"09:00..\", \"..17:00\"
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        /// Apply modification without confirmation
        #[arg(short = 'y', long)]
        yes: bool,
        /// Allow modification even with conflicts (creates overlapping sessions)
        #[arg(long)]
        force: bool,
    },
    /// Delete a session
    #[command(long_about = "Permanently delete a session. This action cannot be undone.")]
    Delete {
        /// Session ID to delete
        session_id: i64,
        /// Delete without confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Generate a time report summarizing hours by project
    #[command(long_about = "Generate a time report showing total hours worked by project, optionally filtered by date range and task criteria.

REPORT SYNTAX:
  Date interval:     -7d, -7d..now, 2024-01-01..2024-01-31
  Task filters:      project:<name>, +tag, task:<id>
  
  Examples:
    tatl sessions report
    tatl sessions report -7d
    tatl sessions report -7d..now project:work
    tatl sessions report 2024-01-01..2024-01-31 +urgent")]
    Report {
        /// Report arguments. Date interval: -7d, -7d..now, <start>..<end>. Task filters: project:<name>, +tag, task:<id>. Examples: \"-7d\", \"-7d..now\", \"-7d project:work\"
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
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
            "projects" | "sessions" | "add" | "list" | "modify" | "annotate" | "finish" | "close" | "delete" | "show" | "status");
        
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
        Commands::Add { args, start_timing, onoff_interval, enqueue, yes } => handle_task_add(args, start_timing, onoff_interval, enqueue, yes),
        Commands::List { filter, json, relative } => {
            handle_task_list(filter, json, relative)
        },
        Commands::Show { target } => handle_task_summary(target),
        Commands::Modify { target, args, yes, interactive, start_timing } => {
            handle_task_modify_with_on(target, args, yes, interactive, start_timing)
        }
        Commands::On { task_id, time_args } => handle_on(task_id, time_args),
        Commands::Off { time_args } => handle_off(time_args),
        Commands::Offon { time_args, yes } => handle_offon(time_args, yes),
        Commands::Onoff { args, yes } => handle_onoff(args, yes),
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
        Commands::Reopen { target, yes, interactive } => {
            handle_task_reopen(target, yes, interactive)
        }
        Commands::Delete { target, yes, interactive } => {
            handle_task_delete(target, yes, interactive)
        }
        Commands::Enqueue { task_id } => {
            handle_task_enqueue(task_id)
        }
        Commands::Send { task_id, recipient, request } => {
            handle_send(task_id, recipient, request)
        },
        Commands::Collect { task_id } => {
            handle_collect(task_id)
        },
        Commands::Externals { filter } => {
            handle_externals(filter)
        },
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
                SessionsCommands::Report { args } => {
                    handle_sessions_report(args)
                }
            }
        }
    }
}

/// Prompt user to create a new project
/// Returns: Some(true) if project should be created, Some(false) if skipped, None if cancelled
fn prompt_create_project(project_name: &str, conn: &Connection) -> Result<Option<bool>> {
    // Check for similar existing projects
    let all_projects = ProjectRepo::list(conn, false)?; // active projects only
    let project_tuples: Vec<(String, bool)> = all_projects.iter()
        .map(|p| (p.name.clone(), p.is_archived))
        .collect();
    let near_matches = fuzzy::find_near_project_matches(project_name, &project_tuples, 2);
    
    if !near_matches.is_empty() {
        let match_names: Vec<&str> = near_matches.iter().map(|(name, _)| name.as_str()).collect();
        eprintln!("Note: Similar existing projects: {}", match_names.join(", "));
    }
    
    eprint!("'{}' is a new project. Create it? [y/n/c] (default: y): ", project_name);
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
                    println!("{} {} {}", "─".repeat(6), "─".repeat(40), "─".repeat(10));
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
        ProjectCommands::Report => {
            handle_projects_report(&conn)
        }
    }
}

fn handle_projects_report(conn: &Connection) -> Result<()> {
    use crate::models::TaskStatus;
    use std::collections::BTreeMap;
    
    // Get all tasks
    let all_tasks = TaskRepo::list_all(conn)?;
    
    // Get stack items for kanban status calculation
    let stack = StackRepo::get_or_create_default(conn)?;
    let stack_items = StackRepo::get_items(conn, stack.id.unwrap())?;
    let stack_task_ids: std::collections::HashSet<i64> = stack_items.iter().map(|i| i.task_id).collect();
    
    // Get open session for LIVE status
    let open_session = SessionRepo::get_open(conn)?;
    let live_task_id = open_session.as_ref().map(|s| s.task_id);
    
    // Build project hierarchy with counts
    #[derive(Default)]
    struct ProjectStats {
        proposed: i64,
        queued: i64,
        paused: i64,
        next: i64,
        live: i64,
        done: i64,
    }
    
    impl ProjectStats {
        fn total(&self) -> i64 {
            self.proposed + self.queued + self.paused + self.next + self.live + self.done
        }
    }
    
    let mut project_stats: BTreeMap<String, ProjectStats> = BTreeMap::new();
    let mut no_project_stats = ProjectStats::default();
    
    // Calculate kanban status for each task
    for (task, _tags) in &all_tasks {
        let task_id = task.id.unwrap();
        
        // Calculate kanban status
        let kanban = if task.status == TaskStatus::Completed || task.status == TaskStatus::Closed {
            "done"
        } else if Some(task_id) == live_task_id {
            "live"
        } else if stack_task_ids.contains(&task_id) {
            if stack_items.first().map(|i| i.task_id) == Some(task_id) {
                "next"
            } else {
                "queued"
            }
        } else {
            // Not in queue - check if has sessions
            let sessions = SessionRepo::get_by_task(conn, task_id)?;
            if !sessions.is_empty() {
                "paused"
            } else {
                "proposed"
            }
        };
        
        // Get project name
        let project_name = if let Some(pid) = task.project_id {
            let mut stmt = conn.prepare("SELECT name FROM projects WHERE id = ?1")?;
            stmt.query_row([pid], |row| row.get::<_, String>(0)).ok()
        } else {
            None
        };
        
        // Update stats
        let stats = if let Some(name) = project_name {
            project_stats.entry(name).or_default()
        } else {
            &mut no_project_stats
        };
        
        match kanban {
            "proposed" => stats.proposed += 1,
            "queued" => stats.queued += 1,
            "paused" => stats.paused += 1,
            "next" => stats.next += 1,
            "live" => stats.live += 1,
            "done" => stats.done += 1,
            _ => {}
        }
    }
    
    // Calculate totals
    let mut total_stats = ProjectStats::default();
    for stats in project_stats.values() {
        total_stats.proposed += stats.proposed;
        total_stats.queued += stats.queued;
        total_stats.paused += stats.paused;
        total_stats.next += stats.next;
        total_stats.live += stats.live;
        total_stats.done += stats.done;
    }
    total_stats.proposed += no_project_stats.proposed;
    total_stats.queued += no_project_stats.queued;
    total_stats.paused += no_project_stats.paused;
    total_stats.next += no_project_stats.next;
    total_stats.live += no_project_stats.live;
    total_stats.done += no_project_stats.done;
    
    // Print report
    let pw = 25; // project width
    println!("{:<pw$} {:>8} {:>8} {:>8} {:>6} {:>6} {:>6} {:>6}", 
        "Project", "Proposed", "Queued", "Paused", "NEXT", "LIVE", "Done", "Total", pw = pw);
    println!("{} {} {} {} {} {} {} {}", 
        "─".repeat(pw), "─".repeat(8), "─".repeat(8), "─".repeat(8), 
        "─".repeat(6), "─".repeat(6), "─".repeat(6), "─".repeat(6));
    
    for (name, stats) in &project_stats {
        println!("{:<pw$} {:>8} {:>8} {:>8} {:>6} {:>6} {:>6} {:>6}",
            truncate_str(name, pw),
            stats.proposed, stats.queued, stats.paused, 
            stats.next, stats.live, stats.done, stats.total(),
            pw = pw);
    }
    
    if no_project_stats.total() > 0 {
        println!("{:<pw$} {:>8} {:>8} {:>8} {:>6} {:>6} {:>6} {:>6}",
            "(no project)",
            no_project_stats.proposed, no_project_stats.queued, no_project_stats.paused,
            no_project_stats.next, no_project_stats.live, no_project_stats.done, no_project_stats.total(),
            pw = pw);
    }
    
    println!("{} {} {} {} {} {} {} {}", 
        "─".repeat(pw), "─".repeat(8), "─".repeat(8), "─".repeat(8), 
        "─".repeat(6), "─".repeat(6), "─".repeat(6), "─".repeat(6));
    println!("{:<pw$} {:>8} {:>8} {:>8} {:>6} {:>6} {:>6} {:>6}",
        "TOTAL",
        total_stats.proposed, total_stats.queued, total_stats.paused,
        total_stats.next, total_stats.live, total_stats.done, total_stats.total(),
        pw = pw);
    
    Ok(())
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

fn handle_send(task_id_str: String, recipient: String, request: Vec<String>) -> Result<()> {
    let conn = DbConnection::connect()?;
    let task_id = validate_task_id(&task_id_str)
        .map_err(|e| anyhow::anyhow!("Invalid task ID: {}", e))?;
    
    // Verify task exists
    let task = TaskRepo::get_by_id(&conn, task_id)?
        .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;
    
    // Check if task is already sent to this recipient
    let existing_externals = ExternalRepo::get_active_for_task(&conn, task_id)?;
    if existing_externals.iter().any(|e| e.recipient == recipient) {
        return Err(anyhow::anyhow!("Task {} is already sent to {}", task_id, recipient));
    }
    
    // Remove from queue if present
    let stack = StackRepo::get_or_create_default(&conn)?;
    if let Some(stack_id) = stack.id {
        let items = StackRepo::get_items(&conn, stack_id)?;
        if items.iter().any(|item| item.task_id == task_id) {
            StackRepo::remove_task(&conn, stack_id, task_id)?;
        }
    }
    
    // Create external record
    let request_str = if request.is_empty() {
        None
    } else {
        Some(request.join(" "))
    };
    
    ExternalRepo::create(&conn, task_id, recipient.clone(), request_str)?;
    
    println!("Sent task {} to {}: {}", task_id, recipient, task.description);
    Ok(())
}

fn handle_collect(task_id_str: String) -> Result<()> {
    let conn = DbConnection::connect()?;
    let task_id = validate_task_id(&task_id_str)
        .map_err(|e| anyhow::anyhow!("Invalid task ID: {}", e))?;
    
    // Verify task exists
    let task = TaskRepo::get_by_id(&conn, task_id)?
        .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;
    
    // Get active externals
    let externals = ExternalRepo::get_active_for_task(&conn, task_id)?;
    if externals.is_empty() {
        return Err(anyhow::anyhow!("Task {} has no active externals", task_id));
    }
    
    // Mark all as returned
    ExternalRepo::mark_all_returned_for_task(&conn, task_id)?;
    
    println!("Collected task {}: {}", task_id, task.description);
    println!("  Returned from: {}", externals.iter().map(|e| e.recipient.as_str()).collect::<Vec<_>>().join(", "));
    Ok(())
}

fn handle_externals(filter: Option<String>) -> Result<()> {
    let conn = DbConnection::connect()?;
    
    let externals = if let Some(filter_str) = filter {
        // Try parsing as task ID first
        if let Ok(task_id) = validate_task_id(&filter_str) {
            ExternalRepo::get_active_for_task(&conn, task_id)?
        } else {
            // Treat as recipient name
            ExternalRepo::get_by_recipient(&conn, &filter_str)?
        }
    } else {
        ExternalRepo::get_all_active(&conn)?
    };
    
    if externals.is_empty() {
        println!("No external tasks found.");
        return Ok(());
    }
    
    // Group by task_id and fetch task details
    use std::collections::HashMap;
    let mut task_externals: HashMap<i64, Vec<&crate::models::External>> = HashMap::new();
    for external in &externals {
        task_externals.entry(external.task_id).or_insert_with(Vec::new).push(external);
    }
    
    println!("{:<6} {:<40} {:<20} {:<30}", "ID", "Description", "Recipient", "Sent");
    println!("{} {} {} {}", "─".repeat(6), "─".repeat(40), "─".repeat(20), "─".repeat(30));
    
    for (task_id, externals_list) in task_externals {
        if let Some(task) = TaskRepo::get_by_id(&conn, task_id)? {
            let desc = if task.description.len() > 40 {
                format!("{}…", &task.description[..39])
            } else {
                task.description.clone()
            };
            
            for (idx, external) in externals_list.iter().enumerate() {
                let sent_date = crate::cli::output::format_date(external.sent_ts);
                if idx == 0 {
                    println!("{:<6} {:<40} {:<20} {:<30}", task_id, desc, external.recipient, sent_date);
                } else {
                    println!("{:<6} {:<40} {:<20} {:<30}", "", "", external.recipient, sent_date);
                }
            }
        }
    }
    
    Ok(())
}

fn handle_task_add(mut args: Vec<String>, mut start_timing: Option<String>, mut onoff_interval: Option<String>, mut enqueue: bool, auto_yes: bool) -> Result<()> {
    // Extract --on, --onoff, and --enqueue flags from args if they appear after the description
    // (CLAP limitation: with trailing_var_arg, flags after args are treated as part of args)
    let mut filtered_args = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--on" || args[i] == "--clock-in" {
            // Bare --on flag (no time specified, starts at now)
            start_timing = Some(String::new());
            // Don't include it in the args passed to parse_task_args
        } else if args[i].starts_with("--on=") || args[i].starts_with("--clock-in=") {
            // Handle --on=time format (start at specified time)
            let eq_pos = args[i].find('=').unwrap();
            start_timing = Some(args[i][eq_pos + 1..].to_string());
        } else if args[i] == "--enqueue" {
            enqueue = true;
            // Don't include it in the args passed to parse_task_args
        } else if args[i] == "--onoff" {
            // Take the next arg as the interval
            if i + 1 < args.len() {
                onoff_interval = Some(args[i + 1].clone());
                i += 1; // Skip the interval value
            }
        } else if args[i].starts_with("--onoff=") {
            // Handle --onoff=value format
            onoff_interval = Some(args[i][8..].to_string());
        } else {
            filtered_args.push(args[i].clone());
        }
        i += 1;
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
    
    // Resolve project (handle clearing with project:none or project:)
    let project_id = if let Some(project_name) = parsed.project {
        if project_name == "none" {
            // project:none or project: (empty) means no project
            None
        } else {
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
                    match prompt_create_project(&project_name, &conn)? {
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
        parsed.respawn,
        &final_udas,
        &final_tags,
    )
    .context("Failed to create task")?;
    
    let task_id = task.id.unwrap();
    println!("Created task {}: {}", task_id, description);
    
    // If --onoff is set, add historical session (takes precedence over --on and --enqueue)
    if let Some(interval) = onoff_interval {
        // Parse interval
        if !interval.contains("..") {
            user_error("--onoff requires interval format (e.g., '09:00..12:00')");
        }
        
        let sep_pos = interval.find("..").unwrap();
        let start_expr = interval[..sep_pos].trim();
        let end_expr = interval[sep_pos + 2..].trim();
        
        let start_ts = parse_date_expr(start_expr)
            .context("Invalid start time in --onoff interval")?;
        let end_ts = parse_date_expr(end_expr)
            .context("Invalid end time in --onoff interval")?;
        
        if start_ts >= end_ts {
            user_error(&format!(
                "Start time must be before end time. Got: {} >= {}",
                format_time(start_ts),
                format_time(end_ts)
            ));
        }
        
        // Check for overlapping sessions
        let overlapping = find_overlapping_sessions(&conn, start_ts, end_ts)?;
        let duration = end_ts - start_ts;
        
        if !overlapping.is_empty() {
            // Show what will be modified and ask for confirmation
            println!("\nInserting session {} ({}) for new task {} will modify {} existing session(s):\n",
                format_interval(start_ts, end_ts),
                format_duration_human(duration),
                task_id,
                overlapping.len());
            for session in &overlapping {
                let s_task = TaskRepo::get_by_id(&conn, session.task_id)?;
                let s_desc = s_task.as_ref().map(|t| t.description.as_str()).unwrap_or("");
                let s_duration = session.end_ts.unwrap_or(chrono::Utc::now().timestamp()) - session.start_ts;
                let modification = describe_session_modification(session, start_ts, end_ts);
                println!("  Session {} (task {}): {}", session.id.unwrap_or(0), session.task_id, s_desc);
                println!("    {} - {} ({})",
                    format_datetime(session.start_ts),
                    session.end_ts.map(format_datetime).unwrap_or_else(|| "running".to_string()),
                    format_duration_human(s_duration));
                println!("    → {}\n", modification);
            }
            
            if !auto_yes {
                print!("Continue? [y/N] ");
                std::io::Write::flush(&mut std::io::stdout())?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled.");
                    return Ok(());
                }
            }
            
            // Modify overlapping sessions and insert new one
            let tx = conn.unchecked_transaction()?;
            
            for session in overlapping {
                modify_session_for_removal(&tx, &session, start_ts, end_ts)?;
            }
            
            // Create the new session
            SessionRepo::create_closed(&tx, task_id, start_ts, end_ts)
                .context("Failed to create session")?;
            
            tx.commit()?;
            
            println!("Added session for task {} ({} - {}, {})", task_id, format_time(start_ts), format_time(end_ts), format_duration_human(duration));
        } else {
            // No overlaps - just add the session
            SessionRepo::create_closed(&conn, task_id, start_ts, end_ts)
                .context("Failed to create session")?;
            
            println!("Added session for task {} ({} - {}, {})", task_id, format_time(start_ts), format_time(end_ts), format_duration_human(duration));
        }
    } else if let Some(start_time) = start_timing {
        // If --on flag is set, start timing the newly created task (takes precedence over --enqueue)
        // handle_task_on will push to stack and start timing atomically
        let time_args = if start_time.is_empty() {
            Vec::new()
        } else {
            vec![start_time]
        };
        handle_task_on(task_id.to_string(), time_args)
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
                match prompt_create_project(project_name, conn)? {
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
    
    // Handle template and respawn clearing
    let template = if let Some(tmpl) = &parsed.template {
        if tmpl == "none" {
            Some(None)
        } else {
            Some(Some(tmpl.clone()))
        }
    } else {
        None
    };
    
    let respawn = if let Some(resp) = &parsed.respawn {
        if resp == "none" {
            Some(None)
        } else {
            // Validate respawn rule before accepting
            use crate::respawn::parser::RespawnRule;
            RespawnRule::parse(resp).map_err(|e| {
                anyhow::anyhow!("Invalid respawn rule '{}': {}", resp, e)
            })?;
            Some(Some(resp.clone()))
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
        respawn,
        &udas_to_add,
        &udas_to_remove,
        &parsed.tags_add,
        &parsed.tags_remove,
    )
    .with_context(|| format!("Failed to modify task {}", task_id))?;
    
    println!("Modified task {}", task_id);
    
    // If respawn was set (and not clearing), show description of what will happen
    if let Some(resp_str) = &parsed.respawn {
        if resp_str != "none" {
            use crate::respawn::parser::RespawnRule;
            if let Ok(rule) = RespawnRule::parse(resp_str) {
                println!("↻ {}", rule.describe());
            }
        }
    }
    
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

/// Handle `tatl offon <stop>[..<start>] [<task_id>]` - Stop current session and resume
/// 
/// When a session is running: Stops it at <stop> and starts a new one (at <start> or now)
/// When no session is running: Operates on history (finds and modifies overlapping sessions)
fn handle_offon(time_args: Vec<String>, mut yes: bool) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    // Extract -y or --yes from args (CLAP can miss them with trailing_var_arg)
    let filtered_args: Vec<String> = time_args.iter()
        .filter(|a| {
            if *a == "-y" || *a == "--yes" {
                yes = true;
                false
            } else {
                true
            }
        })
        .cloned()
        .collect();
    
    // Check if session is currently running
    let current_session = SessionRepo::get_open(&conn)?;
    
    if current_session.is_some() {
        // Current session mode: stop and resume
        handle_offon_current_session(&conn, filtered_args)
    } else {
        // History mode: find and modify overlapping sessions
        handle_offon_history(&conn, filtered_args, yes)
    }
}

/// Handle offon when a session is currently running
fn handle_offon_current_session(conn: &Connection, time_args: Vec<String>) -> Result<()> {
    if time_args.is_empty() {
        user_error("Time expression required. Usage: tatl offon <stop> or tatl offon <stop>..<start>");
    }
    
    let arg_str = time_args.join(" ");
    
    // Parse time arguments - check for interval syntax
    let (stop_ts, start_ts_opt) = parse_offon_time_args(&arg_str)?;
    
    // Get current session
    let current_session = SessionRepo::get_open(conn)?
        .expect("Session should exist - checked in caller");
    
    // Use a transaction for atomicity
    let tx = conn.unchecked_transaction()?;
    
    // Close current session at stop_ts
    SessionRepo::close_open(&tx, stop_ts)
        .context("Failed to close session")?;
    
    let current_task_id = current_session.task_id;
    let task = TaskRepo::get_by_id(&tx, current_task_id)?;
    let desc = task.as_ref().map(|t| t.description.as_str()).unwrap_or("");
    
    // Determine resume time
    let resume_ts = start_ts_opt.unwrap_or_else(|| chrono::Utc::now().timestamp());
    
    // Get queue[0] for resume task (defaults to same task)
    let stack = StackRepo::get_or_create_default(&tx)?;
    let items = StackRepo::get_items(&tx, stack.id.unwrap())?;
    
    let resume_task_id = if items.is_empty() {
        current_task_id // Resume same task if queue is empty
    } else {
        items[0].task_id
    };
    
    // Start new session
    SessionRepo::create(&tx, resume_task_id, resume_ts)
        .context("Failed to start new session")?;
    
    tx.commit()?;
    
    // Format output
    if let Some(start_ts) = start_ts_opt {
        let break_duration = start_ts - stop_ts;
        println!("Stopped timing task {} at {} (break: {}s)", current_task_id, format_time(stop_ts), break_duration);
    } else {
        println!("Stopped timing task {}: {} at {}", current_task_id, desc, format_time(stop_ts));
    }
    
    let resume_task = TaskRepo::get_by_id(conn, resume_task_id)?;
    let resume_desc = resume_task.as_ref().map(|t| t.description.as_str()).unwrap_or("");
    println!("Started timing task {}: {}", resume_task_id, resume_desc);
    
    Ok(())
}

/// Handle offon in history mode (no current session)
fn handle_offon_history(conn: &Connection, time_args: Vec<String>, yes: bool) -> Result<()> {
    if time_args.is_empty() {
        user_error("Time expression required. Usage: tatl offon <time> or tatl offon <stop>..<start>");
    }
    
    let arg_str = time_args.join(" ");
    
    // Parse as single time or interval
    let (remove_start, remove_end) = if arg_str.contains("..") {
        parse_offon_time_args(&arg_str)?
            .pipe(|(start, end_opt)| (start, end_opt.unwrap_or(start)))
    } else {
        // Single time point - split at that point
        let time = parse_date_expr(&arg_str)
            .context("Invalid time expression")?;
        (time, time)
    };
    
    // Find all overlapping sessions
    let overlapping = find_overlapping_sessions(conn, remove_start, remove_end)?;
    
    if overlapping.is_empty() {
        user_error(&format!(
            "No sessions found overlapping with the specified time/interval ({}).",
            format_interval(remove_start, remove_end)
        ));
    }
    
    // Show what will be modified and ask for confirmation
    println!("\nRemoving interval {} will modify {} session(s):\n", 
        format_interval(remove_start, remove_end), overlapping.len());
    for session in &overlapping {
        let task = TaskRepo::get_by_id(conn, session.task_id)?;
        let desc = task.as_ref().map(|t| t.description.as_str()).unwrap_or("");
        let duration = session.end_ts.unwrap_or(chrono::Utc::now().timestamp()) - session.start_ts;
        let modification = describe_session_modification(session, remove_start, remove_end);
        println!("  Session {} (task {}): {}", session.id.unwrap_or(0), session.task_id, desc);
        println!("    {} - {} ({})", 
            format_datetime(session.start_ts),
            session.end_ts.map(format_datetime).unwrap_or_else(|| "running".to_string()),
            format_duration_human(duration));
        println!("    → {}\n", modification);
    }
    
    if !yes {
        print!("Continue? [y/N] ");
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }
    
    // Modify sessions
    let tx = conn.unchecked_transaction()?;
    
    for session in overlapping {
        modify_session_for_removal(&tx, &session, remove_start, remove_end)?;
    }
    
    tx.commit()?;
    
    println!("Sessions modified.");
    
    Ok(())
}

/// Handle `tatl onoff <start>..<end> [<task_id>] [note:<text>]` - Add historical session
fn handle_onoff(args: Vec<String>, mut yes: bool) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    
    if args.is_empty() {
        user_error("Interval required. Usage: tatl onoff <start>..<end> [<task_id>]");
    }
    
    // Extract -y or --yes from args (CLAP can miss them with trailing_var_arg)
    let filtered_args: Vec<String> = args.iter()
        .filter(|a| {
            if *a == "-y" || *a == "--yes" {
                yes = true;
                false
            } else {
                true
            }
        })
        .cloned()
        .collect();
    
    // Parse arguments
    let (start_ts, end_ts, task_id_opt, note_opt) = parse_onoff_args(&filtered_args)?;
    
    // Determine task (task_id or queue[0])
    let task_id = if let Some(id) = task_id_opt {
        id
    } else {
        // Get queue[0]
        let stack = StackRepo::get_or_create_default(&conn)?;
        let items = StackRepo::get_items(&conn, stack.id.unwrap())?;
        
        if items.is_empty() {
            user_error("No tasks in queue. Specify a task ID or enqueue a task first.");
        }
        
        items[0].task_id
    };
    
    // Validate task exists
    let task = TaskRepo::get_by_id(&conn, task_id)?
        .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;
    
    // Validate start < end
    if start_ts >= end_ts {
        user_error(&format!(
            "Start time must be before end time. Got: {} >= {}",
            format_time(start_ts),
            format_time(end_ts)
        ));
    }
    
    // Check for overlapping sessions
    let overlapping = find_overlapping_sessions(&conn, start_ts, end_ts)?;
    
    if !overlapping.is_empty() {
        // Insertion mode: clear overlapping time and insert new session
        let duration = end_ts - start_ts;
        println!("\nInserting session {} ({}) for task {} will modify {} existing session(s):\n",
            format_interval(start_ts, end_ts),
            format_duration_human(duration),
            task_id,
            overlapping.len());
        for session in &overlapping {
            let s_task = TaskRepo::get_by_id(&conn, session.task_id)?;
            let s_desc = s_task.as_ref().map(|t| t.description.as_str()).unwrap_or("");
            let s_duration = session.end_ts.unwrap_or(chrono::Utc::now().timestamp()) - session.start_ts;
            let modification = describe_session_modification(session, start_ts, end_ts);
            println!("  Session {} (task {}): {}", session.id.unwrap_or(0), session.task_id, s_desc);
            println!("    {} - {} ({})",
                format_datetime(session.start_ts),
                session.end_ts.map(format_datetime).unwrap_or_else(|| "running".to_string()),
                format_duration_human(s_duration));
            println!("    → {}\n", modification);
        }
        
        if !yes {
            print!("Continue? [y/N] ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Cancelled.");
                return Ok(());
            }
        }
        
        // Modify overlapping sessions and insert new one
        let tx = conn.unchecked_transaction()?;
        
        for session in overlapping {
            modify_session_for_removal(&tx, &session, start_ts, end_ts)?;
        }
        
        // Create the new session
        let session = SessionRepo::create_closed(&tx, task_id, start_ts, end_ts)
            .context("Failed to create session")?;
        
        // Add annotation if note provided
        if let Some(note_text) = note_opt {
            if !note_text.trim().is_empty() {
                AnnotationRepo::create(&tx, task_id, note_text, session.id)
                    .context("Failed to create annotation")?;
            }
        }
        
        tx.commit()?;
        
        println!("Inserted session for task {}: {} ({} - {}, {})", 
            task_id, task.description, format_time(start_ts), format_time(end_ts), format_duration_human(duration));
    } else {
        // Simple mode: just add the session
        let session = SessionRepo::create_closed(&conn, task_id, start_ts, end_ts)
            .context("Failed to create session")?;
        
        // Add annotation if note provided
        if let Some(note_text) = note_opt {
            if !note_text.trim().is_empty() {
                AnnotationRepo::create(&conn, task_id, note_text, session.id)
                    .context("Failed to create annotation")?;
            }
        }
        
        let duration = end_ts - start_ts;
        println!("Added session for task {}: {} ({} - {}, {})", 
            task_id, task.description, format_time(start_ts), format_time(end_ts), format_duration_human(duration));
    }
    
    Ok(())
}

/// Parse offon time arguments: <stop> or <stop>..<start>
fn parse_offon_time_args(arg_str: &str) -> Result<(i64, Option<i64>)> {
    if let Some(sep_pos) = arg_str.find("..") {
        let stop_expr = arg_str[..sep_pos].trim();
        let start_expr = arg_str[sep_pos + 2..].trim();
        
        let stop_ts = parse_date_expr(stop_expr)
            .context("Invalid stop time expression")?;
        
        let start_ts = if start_expr.is_empty() {
            chrono::Utc::now().timestamp()
        } else {
            parse_date_expr(start_expr)
                .context("Invalid start time expression")?
        };
        
        Ok((stop_ts, Some(start_ts)))
    } else {
        let stop_ts = parse_date_expr(arg_str)
            .context("Invalid time expression")?;
        Ok((stop_ts, None))
    }
}

/// Parse onoff arguments: <start>..<end> [<task_id>] [note:<text>]
fn parse_onoff_args(args: &[String]) -> Result<(i64, i64, Option<i64>, Option<String>)> {
    let mut start_ts: Option<i64> = None;
    let mut end_ts: Option<i64> = None;
    let mut task_id: Option<i64> = None;
    let mut note: Option<String> = None;
    
    for arg in args {
        if arg.starts_with("note:") {
            note = Some(arg[5..].to_string());
        } else if arg.contains("..") {
            // Interval
            let sep_pos = arg.find("..").unwrap();
            let start_expr = arg[..sep_pos].trim();
            let end_expr = arg[sep_pos + 2..].trim();
            
            start_ts = Some(parse_date_expr(start_expr)
                .context("Invalid start time expression")?);
            end_ts = Some(parse_date_expr(end_expr)
                .context("Invalid end time expression")?);
        } else if let Ok(id) = arg.parse::<i64>() {
            task_id = Some(id);
        } else {
            // Try to parse as time expression (might be part of interval that got split)
            // For now, just ignore unknown args
        }
    }
    
    let start = start_ts.ok_or_else(|| anyhow::anyhow!("Interval required (use <start>..<end>)"))?;
    let end = end_ts.ok_or_else(|| anyhow::anyhow!("Interval required (use <start>..<end>)"))?;
    
    Ok((start, end, task_id, note))
}

/// Find all sessions overlapping with the given interval
fn find_overlapping_sessions(conn: &Connection, start: i64, end: i64) -> Result<Vec<crate::models::Session>> {
    let all_sessions = SessionRepo::list_all(conn)?;
    
    let overlapping: Vec<_> = all_sessions.into_iter()
        .filter(|s| {
            let s_start = s.start_ts;
            let s_end = s.end_ts.unwrap_or(i64::MAX);
            
            // Overlap condition: session.start < end && session.end > start
            s_start < end && s_end > start
        })
        .collect();
    
    Ok(overlapping)
}

/// Modify a session to remove the specified interval
fn modify_session_for_removal(conn: &Connection, session: &crate::models::Session, remove_start: i64, remove_end: i64) -> Result<()> {
    let s_start = session.start_ts;
    let is_open = session.end_ts.is_none();
    let s_end = session.end_ts.unwrap_or(i64::MAX);
    let session_id = session.id.unwrap();
    
    if remove_start <= s_start && remove_end >= s_end {
        // Entirely includes: remove session completely
        SessionRepo::delete(conn, session_id)?;
        
    } else if remove_start > s_start && remove_end < s_end {
        // Falls within: split into two sessions
        // First part: s_start to remove_start (always closed)
        SessionRepo::update_times(conn, session_id, s_start, Some(remove_start))?;
        // Second part: remove_end to s_end
        if is_open {
            // Original was open - second part should remain open
            SessionRepo::create(conn, session.task_id, remove_end)?;
        } else {
            SessionRepo::create_closed(conn, session.task_id, remove_end, s_end)?;
        }
        
    } else if remove_start <= s_start && remove_end < s_end {
        // Overlaps start: truncate at remove_end
        let new_end = if is_open { None } else { Some(s_end) };
        SessionRepo::update_times(conn, session_id, remove_end, new_end)?;
        
    } else if remove_start > s_start && remove_end >= s_end {
        // Overlaps end: truncate at remove_start (always closes the session)
        SessionRepo::update_times(conn, session_id, s_start, Some(remove_start))?;
        
    } else if remove_start == remove_end {
        // Single time point: split at that point
        SessionRepo::update_times(conn, session_id, s_start, Some(remove_start))?;
        if is_open {
            // Original was open - second part should remain open
            SessionRepo::create(conn, session.task_id, remove_start)?;
        } else {
            SessionRepo::create_closed(conn, session.task_id, remove_start, s_end)?;
        }
    }
    
    Ok(())
}

/// Format a timestamp for display
fn format_time(ts: i64) -> String {
    use chrono::TimeZone;
    chrono::Local.timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%H:%M").to_string())
        .unwrap_or_else(|| ts.to_string())
}

/// Format an interval for display
fn format_interval(start: i64, end: i64) -> String {
    if start == end {
        format_time(start)
    } else {
        format!("{}..{}", format_time(start), format_time(end))
    }
}

/// Format a timestamp with date for display
fn format_datetime(ts: i64) -> String {
    use chrono::TimeZone;
    chrono::Local.timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| ts.to_string())
}

/// Format duration in human-readable form
fn format_duration_human(seconds: i64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else {
        let hours = seconds / 3600;
        let mins = (seconds % 3600) / 60;
        if mins == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h {}m", hours, mins)
        }
    }
}

/// Describe what will happen to a session when removing an interval
fn describe_session_modification(session: &crate::models::Session, remove_start: i64, remove_end: i64) -> String {
    let s_start = session.start_ts;
    let s_end = session.end_ts.unwrap_or(i64::MAX);
    
    if remove_start <= s_start && remove_end >= s_end {
        "REMOVE (entirely included)".to_string()
    } else if remove_start > s_start && remove_end < s_end {
        format!("SPLIT into {} and {}", 
            format_interval(s_start, remove_start),
            format_interval(remove_end, s_end))
    } else if remove_start <= s_start && remove_end < s_end {
        format!("TRUNCATE start → {}", format_interval(remove_end, s_end))
    } else if remove_start > s_start && remove_end >= s_end {
        format!("TRUNCATE end → {}", format_interval(s_start, remove_start))
    } else if remove_start == remove_end {
        format!("SPLIT at {} → {} and {}",
            format_time(remove_start),
            format_interval(s_start, remove_start),
            format_interval(remove_start, s_end))
    } else {
        "MODIFY".to_string()
    }
}

/// Trait extension for piping values
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R where F: FnOnce(Self) -> R {
        f(self)
    }
}

impl<T> Pipe for T {}

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
        
        // Get task before completing (to check respawn rule)
        let task = TaskRepo::get_by_id(&conn, *task_id)?
            .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;
        
        // Mark task as completed
        TaskRepo::complete(&conn, *task_id)
            .context("Failed to finish task")?;
        
        // Handle respawn if task has respawn rule
        if let Some(new_task_id) = respawn_task(&conn, &task, end_ts)? {
            // Get new task for display
            if let Some(new_task) = TaskRepo::get_by_id(&conn, new_task_id)? {
                let due_str = if let Some(due_ts) = new_task.due_ts {
                    format!(", due: {}", format_datetime(due_ts))
                } else {
                    String::new()
                };
                println!("↻ Respawned as task {}{}", new_task_id, due_str);
            }
        }
        
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
        
        // Handle respawn if task has respawn rule
        if let Some(new_task_id) = respawn_task(conn, &task, end_ts)? {
            // Get new task for display
            if let Some(new_task) = TaskRepo::get_by_id(conn, new_task_id)? {
                let due_str = if let Some(due_ts) = new_task.due_ts {
                    format!(", due: {}", format_datetime(due_ts))
                } else {
                    String::new()
                };
                println!("↻ Respawned as task {}{}", new_task_id, due_str);
            }
        }
        
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
        
        // Get task before closing (to check respawn rule)
        let task = TaskRepo::get_by_id(&conn, *task_id)?
            .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;
        
        TaskRepo::close(&conn, *task_id)
            .context("Failed to close task")?;
        
        // Handle respawn if task has respawn rule
        if let Some(new_task_id) = respawn_task(&conn, &task, end_ts)? {
            // Get new task for display
            if let Some(new_task) = TaskRepo::get_by_id(&conn, new_task_id)? {
                let due_str = if let Some(due_ts) = new_task.due_ts {
                    format!(", due: {}", format_datetime(due_ts))
                } else {
                    String::new()
                };
                println!("↻ Respawned as task {}{}", new_task_id, due_str);
            }
        }
        
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
        
        // Handle respawn if task has respawn rule
        if let Some(new_task_id) = respawn_task(conn, &task, end_ts)? {
            // Get new task for display
            if let Some(new_task) = TaskRepo::get_by_id(conn, new_task_id)? {
                let due_str = if let Some(due_ts) = new_task.due_ts {
                    format!(", due: {}", format_datetime(due_ts))
                } else {
                    String::new()
                };
                println!("↻ Respawned as task {}{}", new_task_id, due_str);
            }
        }
        
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

/// Handle task reopen (set status back to pending)
fn handle_task_reopen(id_or_filter: String, yes: bool, interactive: bool) -> Result<()> {
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
            println!("This will reopen {} task(s).", task_ids.len());
            print!("Reopen all tasks? (y/n/i): ");
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
                    return handle_reopen_interactive(&conn, &task_ids);
                }
                _ => {
                    println!("Invalid input. Cancelled.");
                    return Ok(());
                }
            }
        } else if interactive {
            return handle_reopen_interactive(&conn, &task_ids);
        }
    }
    
    for task_id in &task_ids {
        let task = match TaskRepo::get_by_id(&conn, *task_id)? {
            Some(task) => task,
            None => {
                eprintln!("Error: Task {} not found", task_id);
                continue;
            }
        };
        
        if task.status == crate::models::TaskStatus::Pending {
            println!("Task {} is already pending", task_id);
            continue;
        }
        
        TaskRepo::reopen(&conn, *task_id)
            .context("Failed to reopen task")?;
        
        println!("Reopened task {}: {}", task_id, task.description);
    }
    
    Ok(())
}

fn handle_reopen_interactive(conn: &Connection, task_ids: &[i64]) -> Result<()> {
    use std::io::{self, Write};
    
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
        
        if task.status == crate::models::TaskStatus::Pending {
            println!("Task {} is already pending, skipping.", task_id);
            continue;
        }
        
        print!("Reopen task {} ({})? (y/n): ", task_id, task.description);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        
        if input != "y" && input != "yes" {
            println!("Skipped task {}.", task_id);
            continue;
        }
        
        TaskRepo::reopen(conn, *task_id)
            .context("Failed to reopen task")?;
        
        println!("Reopened task {}: {}", task_id, task.description);
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

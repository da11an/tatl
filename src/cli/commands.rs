use clap::{Parser, Subcommand};
use rusqlite::Connection;
use chrono::{Local, TimeZone};
use crate::db::DbConnection;
use crate::repo::{ProjectRepo, TaskRepo, StackRepo, SessionRepo, AnnotationRepo, TemplateRepo, ViewRepo, ExternalRepo, StageRepo};
use crate::models::TaskStatus;
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
#[command(about = "Task and Time Ledger - A powerful command-line task and time tracking tool

PIPE OPERATOR ' : ' (space-colon-space):
  Many commands can be chained with the pipe operator.
  The pipe passes the created or selected task ID into the next command so you can compose flows.
  See long help (--help) for examples and supported commands.")]
#[command(long_about = "Task and Time Ledger - A powerful command-line task and time tracking tool

PIPE OPERATOR ' : ' (space-colon-space):
  Many commands can be chained with the pipe operator.
  The pipe passes the created or selected task ID into the next command so you can compose flows.

  Examples:
    tatl add \"Task\" : on                 # Create and start timing
    tatl add \"Task\" : enqueue            # Create and enqueue
    tatl add \"Task\" : onoff 09:00..10:00 : close  # Create task, backfill 9-10 am session, close

  Supported piped commands include: add, modify, close, enqueue, cancel, reopen,
  annotate, send, collect, on, dequeue, onoff, offon, off, clone.")]
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
    #[command(long_about = "Create a new task with optional attributes.

The task description is all text that doesn't match field patterns. Field syntax includes:
  project=<name>     - Assign to project (creates if new with -y)
  due=<expr>         - Set due date (see DATE EXPRESSIONS below)
  scheduled=<expr>   - Set scheduled date
  wait=<expr>        - Set wait date
  allocation=<dur>   - Set time allocation (e.g., \"2h\", \"30m\", \"1d\")
  template=<name>    - Use template
  respawn=<pattern>  - Set respawn rule (see RESPAWN PATTERNS below)
  parent=<id>        - Set parent task (creates nesting)
  +<tag>             - Add tag
  -<tag>             - Remove tag
  uda.<key>=<value>  - Set user-defined attribute

DATE EXPRESSIONS:
  Relative: tomorrow, +3d, -1w, +2m, +1y
  Absolute: 2024-01-15, 2024-01-15 14:30
  Time-only: 09:00, 14:30

RESPAWN PATTERNS:
  Simple: daily, weekly, monthly, yearly
  Interval: 2d, 3w, 2m, 1y
  Weekdays: mon,wed,fri
  Monthdays: 1,15
  Nth weekday: 2nd-tue, 1st-mon, last-fri

PIPE OPERATOR ( : ):
  Chain commands using the pipe operator (space-colon-space).
  The pipe passes the created task ID to the next command.

  tatl add \"Task\" project=work : on           # Create and start timing
  tatl add \"Task\" : onoff 09:00..10:00         # Create with historical session
  tatl add \"Task\" : enqueue                    # Create and enqueue
  tatl add \"Task\" : close                      # Create as closed
  tatl add \"Task\" : onoff 09:00..10:00 : close  # Historical session + close
  tatl add \"Task\" : cancel                     # Create as cancelled
  tatl add \"Task\" : annotate \"note\"            # Create and annotate

EXAMPLES:
  tatl add \"Fix bug\" project=work +urgent
  tatl add \"Review PR\" due=tomorrow allocation=1h
  tatl add \"Daily standup\" respawn=daily due=09:00
  tatl add \"Start working\" : on
  tatl add \"Meeting\" : onoff 14:00..15:00 : close")]
    Add {
        /// Auto-confirm prompts (create new projects, modify overlapping sessions)
        #[arg(short = 'y', long)]
        yes: bool,
        /// Task description and fields. The description is all text not matching field patterns. Examples: \"fix bug project=work +urgent\", \"Review PR due=tomorrow allocation=1h\"
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// List tasks
    #[command(long_about = "List tasks matching optional filter criteria.

FILTER SYNTAX:
  Field filters (support = and !=):
    id=<n>               - Match by task ID (also supports >, <, >=, <=, !=)
    status=<status>      - Match by status (open, closed, cancelled, deleted)
    project=<name>       - Match by project (supports prefix matching for nested projects)
    stage=<stage>        - Match by stage (proposed, planned, in progress, suspended, active, external, completed, cancelled)
    desc=<pattern>       - Match description containing pattern (case-insensitive)
    description=<pattern> - Alias for desc=
    parent=<id>          - Match by parent task (none=root tasks, any=child tasks, <id>=specific parent)

  Date filters (support =, >, <, >=, <=, !=):
    due=<expr>           - Match by due date (see DATE EXPRESSIONS)
    due>tomorrow         - Tasks due after tomorrow
    due<=eod             - Tasks due by end of day
    due!=none            - Tasks that have a due date
    scheduled=<expr>     - Match by scheduled date
    wait=<expr>          - Match by wait date

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
    project=work +urgent
    +urgent or +important
    not +waiting
    project=work +urgent or project=home +important
    desc=bug status=open
    due=tomorrow stage=planned

DATE EXPRESSIONS (for due=, scheduled=, wait=):
  Relative: tomorrow, +3d, -1w, +2m, +1y
  Absolute: 2024-01-15, 2024-01-15 14:30
  Time-only: 09:00, 14:30
  Intervals: -7d..now, 2024-01-01..2024-01-31

EXAMPLES:
  tatl list
  tatl list project=work +urgent
  tatl list +urgent or +important
  tatl list desc=bug status=open
  tatl list due=tomorrow stage=planned --relative
  tatl list due>tomorrow
  tatl list due!=none")]
    List {
        /// Filter arguments. Multiple filters are ANDed together. Use 'or' for OR, 'not' for NOT. Examples: \"project=work +urgent\", \"+urgent or +important\", \"desc=bug status=open\"
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        filter: Vec<String>,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show due dates as relative time (e.g., \"2 days ago\", \"in 3 days\")
        #[arg(long)]
        relative: bool,
        /// Show all columns regardless of terminal width
        #[arg(long)]
        full: bool,
    },
    /// Show detailed summary of task(s)
    #[command(long_about = "Show detailed information about one or more tasks.

TARGET SYNTAX:
  Single ID:       10
  ID range:        1-5 (tasks 1, 2, 3, 4, 5)
  ID list:         1,3,5 (tasks 1, 3, and 5)
  Filter:          project=work +urgent (same filter syntax as 'tatl list')

The output includes task details, annotations, sessions, and related information.

EXAMPLES:
  tatl show 10
  tatl show 1-5
  tatl show project=work +urgent")]
    Show {
        /// Task ID, ID range, ID list, or filter expression. If omitted, shows the currently active task.
        target: Option<String>,
    },
    /// Modify tasks
    #[command(long_about = "Modify one or more tasks. Target can be a task ID, ID range (e.g., \"1-5\"), ID list (e.g., \"1,3,5\"), or filter expression.

MODIFICATION SYNTAX:
  Field modifications:
    project=<name>       - Assign to project (use \"project=none\" to clear)
    due=<expr>           - Set due date (use \"due=none\" to clear, see DATE EXPRESSIONS)
    scheduled=<expr>      - Set scheduled date (use \"scheduled=none\" to clear)
    wait=<expr>           - Set wait date (use \"wait=none\" to clear)
    allocation=<dur>      - Set time allocation (e.g., \"2h\", \"30m\", use \"allocation=none\" to clear)
    template=<name>       - Set template (use \"template=none\" to clear)
    respawn=<pattern>     - Set respawn rule (use \"respawn=none\" to clear, see RESPAWN PATTERNS)
    parent=<id>           - Set parent task (use \"parent=none\" to clear)
    uda.<key>=<value>     - Set user-defined attribute (use \"uda.<key>=none\" to clear)

  Tag modifications:
    +<tag>                - Add tag
    -<tag>                - Remove tag

  Description:
    Any text not matching field patterns becomes the new description.

RESPAWN PATTERNS:
  Simple: daily, weekly, monthly, yearly
  Interval: 2d, 3w, 2m, 1y
  Weekdays: mon,wed,fri
  Monthdays: 1,15
  Nth weekday: 2nd-tue, 1st-mon, last-fri

  Respawn rules are validated on modification. A preview message shows what will happen when the task is closed.

DATE EXPRESSIONS:
  Relative: tomorrow, +3d, -1w, +2m, +1y
  Absolute: 2024-01-15, 2024-01-15 14:30
  Time-only: 09:00, 14:30

FILTER SYNTAX (for target selection):
  Same as 'tatl list' filter syntax. See 'tatl list --help' for details.

EXAMPLES:
  tatl modify 10 +urgent due=+2d
  tatl modify +urgent due=+1d --yes
  tatl modify 5 respawn=daily due=09:00
  tatl modify 1-5 project=work --yes")]
    Modify {
        /// Task ID, ID range, ID list, or filter expression. If omitted, modifies the currently active task.
        target: Option<String>,
        /// Modification arguments. Field syntax: project=<name>, due=<expr>, +tag, -tag, etc. Any text not matching field patterns becomes the new description.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        /// Apply to all matching tasks without confirmation (also auto-creates new projects if needed)
        #[arg(short = 'y', long)]
        yes: bool,
        /// Force one-by-one confirmation for each task
        #[arg(long)]
        interactive: bool,
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
    /// Remove task from queue without closing
    #[command(long_about = "Remove a task from the queue without closing it. The task remains in open status.")]
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
  Filter:                  project=work +urgent

Use --delete <annotation_id> to remove an annotation.")]
    Annotate {
        /// Task ID, ID range, ID list, or filter (optional when clocked in, defaults to queue[0]). Examples: \"10\", \"1-5\", \"1,3,5\", \"project=work +urgent\"
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
    /// Close task(s) (intent fulfilled)
    #[command(long_about = "Close one or more tasks (intent fulfilled). If task has a respawn rule, a new instance will be created when closed.

TARGET SYNTAX:
  Omit:              Uses queue[0] (current task)
  Task ID:           10
  ID range:          1-5
  ID list:           1,3,5
  Filter:            project=work +urgent

TIME EXPRESSIONS:
  Omit:              Ends session at now
  Time-only:         14:30 (ends session at that time today)
  Date + time:       2024-01-15 14:30

PIPE OPERATOR:
  Use 'close : on' to close the current task and start timing the next task in queue.

EXAMPLES:
  tatl close
  tatl close 10
  tatl close : on")]
    Close {
        /// Task ID, ID range, ID list, or filter (optional, defaults to queue[0]). Examples: \"10\", \"1-5\", \"1,3,5\", \"project=work +urgent\"
        target: Option<String>,
        /// End time expression (optional, defaults to now). Time-only (e.g., \"14:30\") ends session at that time today.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        time_args: Vec<String>,
        /// Complete all matching tasks without confirmation
        #[arg(short = 'y', long)]
        yes: bool,
        /// Force one-by-one confirmation for each task
        #[arg(long)]
        interactive: bool,
    },
    /// Cancel task(s) (intent shifted, won't do)
    #[command(long_about = "Cancel one or more tasks (intent shifted, won't do, etc.). If task has a respawn rule, a new instance will be created when cancelled.

TARGET SYNTAX:
  Omit:              Uses queue[0] (current task)
  Task ID:           10
  ID range:          1-5
  ID list:           1,3,5
  Filter:            project=work +urgent")]
    Cancel {
        /// Task ID, ID range, ID list, or filter (optional, defaults to queue[0]). Examples: \"10\", \"1-5\", \"1,3,5\", \"project=work +urgent\"
        target: Option<String>,
        /// Cancel all matching tasks without confirmation
        #[arg(short = 'y', long)]
        yes: bool,
        /// Force one-by-one confirmation for each task
        #[arg(long)]
        interactive: bool,
    },
    /// Reopen closed or cancelled task(s)
    #[command(long_about = "Reopen one or more closed or cancelled tasks, setting their status back to open.

TARGET SYNTAX:
  Task ID:           10
  ID range:          1-5
  ID list:           1,3,5
  Filter:            project=work status=closed")]
    Reopen {
        /// Task ID, ID range, ID list, or filter. Examples: \"10\", \"1-5\", \"1,3,5\", \"project=work status=closed\"
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
  Filter:            project=work status=closed")]
    Delete {
        /// Task ID, ID range, ID list, or filter. Examples: \"10\", \"1-5\", \"1,3,5\", \"project=work status=closed\"
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
    #[command(long_about = "Send a task to an external party (colleague, supervisor, release window, etc.). The task will be removed from the queue and marked as 'external' stage.

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
    #[command(long_about = "Collect a task that was sent to an external party. Marks all externals for the task as returned. The task returns to normal stage flow (proposed or suspended, depending on whether it has sessions).

After collecting, you can:
  - Re-queue it: tatl enqueue <task_id>
  - Close it: tatl close <task_id>
  - Cancel it: tatl cancel <task_id>

EXAMPLES:
  tatl collect 10")]
    Collect {
        /// Task ID
        task_id: String,
    },
    /// Clone (duplicate) a task
    #[command(long_about = "Create a duplicate of an existing task with optional field overrides.

Copies: description, project, due, scheduled, wait, allocation, template, tags, UDAs.
Does NOT copy: sessions, annotations, externals, queue position, respawn rule.
The new task is always created with open status.

OVERRIDE SYNTAX (same as 'add' and 'modify'):
  project=<name>     - Override project
  due=<expr>         - Override due date
  +<tag>             - Add tag
  -<tag>             - Remove tag
  uda.<key>=<value>  - Override UDA

PIPE OPERATOR:
  tatl close 10 : clone   # Close a task and create a fresh copy

EXAMPLES:
  tatl clone 10
  tatl clone 10 project=other due=+7d
  tatl clone 10 +urgent")]
    Clone {
        /// Source task ID
        task_id: String,
        /// Optional field overrides (same syntax as 'add')
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        /// Auto-confirm prompts (create new projects)
        #[arg(short = 'y', long)]
        yes: bool,
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
    /// Show report with queue, sessions, and statistics
    #[command(long_about = "Display a composite report view showing:
- Current work queue with immediate priorities
- Today's work sessions with running total
- This week's statistics and project breakdown
- Tasks needing attention (overdue, stalled, external)

PERIOD:
  The --period option controls the time range for statistics:
  - week (default): Show this week's data
  - month: Show this month's data
  - year: Show this year's data

EXAMPLES:
  tatl report
  tatl report --period=month")]
    Report {
        /// Time period for statistics (week, month, year)
        #[arg(long, default_value = "week")]
        period: String,
    },
    /// View or configure stage mappings
    #[command(long_about = "View or configure the stage mapping table. Stages are derived from task state
(status, queue membership, session history, active timer, external status) and mapped
to configurable labels with sort order and color.

SUBCOMMANDS:
  tatl stages              Show the current stage mapping table
  tatl stages list         Same as above
  tatl stages set <id> ... Update a mapping row

EXAMPLES:
  tatl stages
  tatl stages set 3 backlog
  tatl stages set 3 color=cyan sort_order=1
  tatl stages set 7 \"working\" sort_order=4 color=green")]
    Stages {
        #[command(subcommand)]
        subcommand: Option<StagesCommands>,
    },
}

#[derive(Subcommand)]
pub enum StagesCommands {
    /// List stage mappings
    List,
    /// Update a stage mapping row
    #[command(long_about = "Update a stage mapping row by ID.

The first positional argument is the row ID. Subsequent arguments can be:
  - A plain string: sets the stage name (e.g., \"backlog\")
  - sort_order=N: sets the sort order
  - color=name: sets the color (valid: black, red, green, yellow, blue, magenta, cyan, white,
    bright_black, bright_red, bright_green, bright_yellow, bright_blue, bright_magenta,
    bright_cyan, bright_white, none)

EXAMPLES:
  tatl stages set 3 backlog
  tatl stages set 3 color=cyan sort_order=1
  tatl stages set 7 \"working\" sort_order=4 color=green")]
    Set {
        /// Row ID to update
        id: i64,
        /// Stage name and/or field=value pairs (sort_order=N, color=name)
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
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
    /// Show task counts by stage per project
    #[command(long_about = "Generate a report showing task counts grouped by project and stage (proposed, planned, in progress, suspended, active, external, completed, cancelled).")]
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
    project=<name>   - Sessions for tasks in project
    +<tag>           - Sessions for tasks with tag
    task=<id>        - Sessions for specific task
  
  Examples:
    tatl sessions list -7d
    tatl sessions list -7d..now
    tatl sessions list project=work
    tatl sessions list -7d project=work")]
    List {
        /// Filter arguments. Date filters: -7d, -7d..now, <start>..<end>. Task filters: project=<name>, +tag, task=<id>. Examples: \"-7d\", \"-7d..now\", \"project=work\"
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
  Task filters:      project=<name>, +tag, task=<id>
  
  Examples:
    tatl sessions report
    tatl sessions report -7d
    tatl sessions report -7d..now project=work
    tatl sessions report 2024-01-01..2024-01-31 +urgent")]
    Report {
        /// Report arguments. Date interval: -7d, -7d..now, <start>..<end>. Task filters: project=<name>, +tag, task=<id>. Examples: \"-7d\", \"-7d..now\", \"-7d project=work\"
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}



/// Split command args on the pipe operator (standalone `:` token).
/// Returns a vector of segments. If no pipe is found, returns a single segment with all args.
fn split_on_pipe(args: &[String]) -> Vec<Vec<String>> {
    let mut segments: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();

    for arg in args {
        if arg == ":" {
            if !current.is_empty() {
                segments.push(current);
                current = Vec::new();
            }
        } else {
            current.push(arg.clone());
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    if segments.is_empty() {
        segments.push(Vec::new());
    }

    segments
}

/// Execute a piped command with the task ID from the previous command.
fn execute_piped_command(task_id: i64, segment: &[String]) -> Result<i64> {
    if segment.is_empty() {
        anyhow::bail!("Empty pipe segment");
    }

    let raw_cmd = segment[0].to_lowercase();
    let cmd = match abbrev::find_unique_command(&raw_cmd, abbrev::PIPE_COMMANDS) {
        Ok(full) => full.to_string(),
        Err(matches) if matches.len() > 1 => {
            let match_list = matches.join(", ");
            anyhow::bail!("Ambiguous pipe command: '{}'. Did you mean one of: {}?", raw_cmd, match_list);
        }
        _ => raw_cmd,
    };
    let rest = &segment[1..];

    match cmd.as_str() {
        "on" => {
            // Special case: a prior stage (e.g., `close` with no explicit target) can return 0
            // to mean "operate on queue[0]". For `on`, that should start timing queue[0].
            if task_id == 0 {
                handle_on(None, rest.to_vec())?;
                Ok(0)
            } else {
                handle_task_on(task_id.to_string(), rest.to_vec())?;
                Ok(task_id)
            }
        }
        "onoff" => {
            // Run onoff for the specific task
            let mut onoff_args = rest.to_vec();
            onoff_args.push(task_id.to_string());
            handle_onoff(onoff_args, false)?;
            Ok(task_id)
        }
        "enqueue" => {
            handle_task_enqueue(task_id.to_string())?;
            Ok(task_id)
        }
        "close" => {
            handle_task_close(Some(task_id.to_string()), None, false, false)?;
            Ok(task_id)
        }
        "cancel" => {
            handle_task_cancel(task_id.to_string(), false, false)?;
            Ok(task_id)
        }
        "annotate" => {
            let note_args = rest.to_vec();
            handle_annotation_add(Some(task_id.to_string()), note_args)?;
            Ok(task_id)
        }
        "send" => {
            if rest.is_empty() {
                anyhow::bail!("'send' requires a recipient. Usage: ... : send <recipient> [message]");
            }
            let recipient = rest[0].clone();
            let request = rest[1..].to_vec();
            handle_send(task_id.to_string(), recipient, request)?;
            Ok(task_id)
        }
        "collect" => {
            handle_collect(task_id.to_string())?;
            Ok(task_id)
        }
        "off" => {
            // Stop timing for the task from previous command
            let conn = DbConnection::connect()
                .context("Failed to connect to database")?;
            
            // Check if there's an open session for this task
            let open_session = SessionRepo::get_open(&conn)?;
            
            if let Some(session) = open_session {
                if session.task_id == task_id {
                    // This task has an open session - close it
                    let end_ts = if rest.is_empty() {
                        chrono::Utc::now().timestamp()
                    } else {
                        let end_expr = rest.join(" ");
                        parse_date_expr(&end_expr)
                            .context("Invalid end time expression")?
                    };
                    
                    // Ensure end_ts is after start_ts (handle micro-sessions)
                    let end_ts = std::cmp::max(end_ts, session.start_ts + 1);
                    
                    if let Err(e) = SessionRepo::close_open(&conn, end_ts) {
                        // If closing fails (e.g., session was already closed/purged),
                        // just continue - idempotent behavior
                        eprintln!("Warning: Could not close session: {}", e);
                    } else {
                        let task = TaskRepo::get_by_id(&conn, task_id)?;
                        let desc = task.as_ref().map(|t| t.description.as_str()).unwrap_or("");
                        let duration = end_ts - session.start_ts;
                        println!("Stopped timing task {}: {} ({}, {})", task_id, desc, format_time(end_ts), format_duration_human(duration));
                    }
                }
                // If open session is for a different task, do nothing (idempotent)
            }
            // If no open session, do nothing (idempotent)
            
            Ok(task_id)
        }
        "dequeue" => {
            // Remove task from queue (uses task_id from previous command)
            handle_dequeue(Some(task_id.to_string()))?;
            Ok(task_id)
        }
        "clone" => {
            // Clone the task from previous command, with optional overrides
            let new_id = handle_clone(task_id.to_string(), rest.to_vec(), false)?;
            Ok(new_id)
        }
        _ => {
            anyhow::bail!(
                "Unknown pipe command: '{}'. Valid: {}",
                cmd,
                abbrev::PIPE_COMMANDS.join(", ")
            );
        }
    }
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
            "projects" | "sessions" | "add" | "list" | "modify" | "annotate" | "close" | "cancel" | "delete" | "show" | "status");
        
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
    // Use `tatl report` command for a consolidated report view.
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
    
    // Check for pipe operator (standalone ":" token)
    let pipe_segments = split_on_pipe(&args);

    if pipe_segments.len() > 1 {
        // Piped command flow: first segment → clap, subsequent segments → piped execution
        let first_segment = &pipe_segments[0];

        let clap_args = std::iter::once("tatl".to_string())
            .chain(first_segment.iter().cloned())
            .collect::<Vec<_>>();
        let cli = match Cli::try_parse_from(clap_args) {
            Ok(cli) => cli,
            Err(e) => {
                e.print()?;
                return Ok(());
            }
        };

        // Execute first command and capture task ID for piping
        let task_id = match cli.command {
            Commands::Add { args: add_args, yes } => {
                handle_task_add(add_args, yes)?
            }
            Commands::Modify { target, args: mod_args, yes, interactive } => {
                let t = target.ok_or_else(|| anyhow::anyhow!("Pipe operator with modify requires a target"))?;
                handle_task_modify(t.clone(), mod_args, yes, interactive)?;
                // Extract task ID from target (only works with single task ID)
                validate_task_id(&t)
                    .map_err(|_| anyhow::anyhow!("Pipe operator with modify requires a single task ID as target"))?
            }
            Commands::Close { target, time_args, yes, interactive } => {
                let end_time = if time_args.is_empty() { None } else { Some(time_args.join(" ")) };
                let close_target = target.clone();
                handle_task_close(target, end_time, yes, interactive)?;

                if let Some(t) = close_target {
                    validate_task_id(&t).unwrap_or(0)
                } else {
                    // Closed queue[0], return 0 to signal "use queue[0]" for next command
                    0
                }
            }
            Commands::Enqueue { task_id: task_id_str } => {
                // Parse task ID(s) - for piping, we'll use the first one
                let task_ids = parse_task_id_list(&task_id_str)
                    .map_err(|e| anyhow::anyhow!("Invalid task ID: {}", e))?;
                if task_ids.is_empty() {
                    anyhow::bail!("No task IDs provided to enqueue");
                }
                handle_task_enqueue(task_id_str)?;
                task_ids[0] // Return first task ID for piping
            }
            Commands::Cancel { target, yes, interactive } => {
                let target_str = target.clone()
                    .ok_or_else(|| anyhow::anyhow!("Pipe operator with cancel requires a task ID as target"))?;
                handle_task_cancel_optional(target, yes, interactive)?;
                // Extract task ID from target (only works with single task ID)
                validate_task_id(&target_str)
                    .map_err(|_| anyhow::anyhow!("Pipe operator with cancel requires a single task ID as target"))?
            }
            Commands::Reopen { target, yes, interactive } => {
                handle_task_reopen(target.clone(), yes, interactive)?;
                // Extract task ID from target (only works with single task ID)
                validate_task_id(&target)
                    .map_err(|_| anyhow::anyhow!("Pipe operator with reopen requires a single task ID as target"))?
            }
            Commands::Annotate { target, note, task, yes: _, interactive: _, delete } => {
                if delete.is_some() {
                    anyhow::bail!("Pipe operator not supported with --delete flag");
                }
                let target_str = target.or(task)
                    .ok_or_else(|| anyhow::anyhow!("Pipe operator with annotate requires a task ID as target"))?;
                handle_annotation_add(Some(target_str.clone()), note)?;
                // Extract task ID from target (only works with single task ID)
                validate_task_id(&target_str)
                    .map_err(|_| anyhow::anyhow!("Pipe operator with annotate requires a single task ID as target"))?
            }
            Commands::Send { task_id: task_id_str, recipient, request } => {
                handle_send(task_id_str.clone(), recipient, request)?;
                validate_task_id(&task_id_str)
                    .map_err(|e| anyhow::anyhow!("Invalid task ID: {}", e))?
            }
            Commands::Collect { task_id: task_id_str } => {
                handle_collect(task_id_str.clone())?;
                validate_task_id(&task_id_str)
                    .map_err(|e| anyhow::anyhow!("Invalid task ID: {}", e))?
            }
            Commands::On { task_id: task_id_opt, time_args } => {
                // For piping, we need a task ID - can't use queue[0]
                let task_id_str = task_id_opt
                    .ok_or_else(|| anyhow::anyhow!("Pipe operator with 'on' requires a task ID"))?;
                handle_task_on(task_id_str.clone(), time_args)?;
                validate_task_id(&task_id_str)
                    .map_err(|e| anyhow::anyhow!("Invalid task ID: {}", e))?
            }
            Commands::Dequeue { task_id: task_id_opt } => {
                // For piping, we need a task ID - can't use queue[0]
                let task_id_str = task_id_opt
                    .ok_or_else(|| anyhow::anyhow!("Pipe operator with 'dequeue' requires a task ID"))?;
                handle_dequeue(Some(task_id_str.clone()))?;
                validate_task_id(&task_id_str)
                    .map_err(|e| anyhow::anyhow!("Invalid task ID: {}", e))?
            }
            Commands::Onoff { args, yes } => {
                // For onoff, we need to extract task ID from args if present
                // onoff can be: "09:00..12:00" or "09:00..12:00 5" or "5 09:00..12:00"
                // We'll look for a numeric argument that's a valid task ID
                let mut task_id_opt: Option<i64> = None;
                for arg in &args {
                    if let Ok(id) = arg.parse::<i64>() {
                        let conn = DbConnection::connect()
                            .context("Failed to connect to database")?;
                        if TaskRepo::get_by_id(&conn, id)?.is_some() {
                            task_id_opt = Some(id);
                            break;
                        }
                    }
                }
                let task_id = task_id_opt
                    .ok_or_else(|| anyhow::anyhow!("Pipe operator with 'onoff' requires a task ID in arguments (e.g., 'onoff 09:00..12:00 5')"))?;
                handle_onoff(args, yes)?;
                task_id
            }
            Commands::Offon { time_args, yes } => {
                // offon when no session is running can work on history, but for piping we need a task ID
                // Check if last argument is a task ID
                let task_id_opt = time_args.last()
                    .and_then(|arg| arg.parse::<i64>().ok())
                    .and_then(|id| {
                        let conn = DbConnection::connect().ok()?;
                        TaskRepo::get_by_id(&conn, id).ok()?.map(|_| id)
                    });
                let task_id = task_id_opt
                    .ok_or_else(|| anyhow::anyhow!("Pipe operator with 'offon' requires a task ID in arguments (e.g., 'offon 14:30 5')"))?;
                handle_offon(time_args, yes)?;
                task_id
            }
            _ => {
                anyhow::bail!("Pipe operator is not supported with this command. Supported commands: add, modify, close, enqueue, cancel, reopen, annotate, send, collect, on, dequeue, onoff, offon, off");
            }
        };

        // Execute pipe segments in sequence
        let mut current_task_id = task_id;
        for segment in &pipe_segments[1..] {
            current_task_id = execute_piped_command(current_task_id, segment)?;
        }

        return Ok(());
    }

    // Normal (non-piped) command flow
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
        Commands::Add { args, yes } => { handle_task_add(args, yes)?; Ok(()) }
        Commands::List { filter, json, relative, full } => {
            handle_task_list(filter, json, relative, full)
        },
        Commands::Show { target } => {
            let resolved = resolve_target_or_active(target, "show")?;
            handle_task_summary(resolved)
        },
        Commands::Modify { target, args, yes, interactive } => {
            let resolved = resolve_target_or_active(target, "modify")?;
            handle_task_modify(resolved, args, yes, interactive)
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
        Commands::Close { target, time_args, yes, interactive } => {
            // Convert time_args to optional end time
            let end_time = if time_args.is_empty() { None } else { Some(time_args.join(" ")) };
            handle_task_close(target, end_time, yes, interactive)
        }
        Commands::Cancel { target, yes, interactive } => {
            handle_task_cancel_optional(target, yes, interactive)
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
        Commands::Clone { task_id, args, yes } => {
            handle_clone(task_id, args, yes)?;
            Ok(())
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
        Commands::Report { period } => {
            handle_report(period)
        }
        Commands::Stages { subcommand } => handle_stages(subcommand),
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

/// Handle the report command
fn handle_report(period: String) -> Result<()> {
    use crate::models::TaskStatus;
    use chrono::{Datelike, Duration, NaiveTime};

    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;

    let now = Local::now();
    let today_start = now.date_naive().and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    let today_start_ts = Local.from_local_datetime(&today_start).single()
        .map(|dt| dt.timestamp())
        .unwrap_or(now.timestamp());

    // Calculate period start based on --period flag
    let period_start_ts = match period.to_lowercase().as_str() {
        "week" => {
            let days_since_monday = now.weekday().num_days_from_monday() as i64;
            let week_start = now.date_naive() - Duration::days(days_since_monday);
            let week_start_dt = week_start.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
            Local.from_local_datetime(&week_start_dt).single()
                .map(|dt| dt.timestamp())
                .unwrap_or(today_start_ts - 7 * 86400)
        }
        "month" => {
            let month_start = now.date_naive().with_day(1).unwrap_or(now.date_naive());
            let month_start_dt = month_start.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
            Local.from_local_datetime(&month_start_dt).single()
                .map(|dt| dt.timestamp())
                .unwrap_or(today_start_ts - 30 * 86400)
        }
        "year" => {
            let year_start = now.date_naive().with_month(1).and_then(|d| d.with_day(1)).unwrap_or(now.date_naive());
            let year_start_dt = year_start.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
            Local.from_local_datetime(&year_start_dt).single()
                .map(|dt| dt.timestamp())
                .unwrap_or(today_start_ts - 365 * 86400)
        }
        _ => {
            eprintln!("Warning: Unknown period '{}', defaulting to 'week'", period);
            let days_since_monday = now.weekday().num_days_from_monday() as i64;
            let week_start = now.date_naive() - Duration::days(days_since_monday);
            let week_start_dt = week_start.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
            Local.from_local_datetime(&week_start_dt).single()
                .map(|dt| dt.timestamp())
                .unwrap_or(today_start_ts - 7 * 86400)
        }
    };

    // Get queue (tasks in stack)
    let stack = StackRepo::get_or_create_default(&conn)?;
    let stack_items = StackRepo::get_items(&conn, stack.id.unwrap())?;

    // Get open session for detecting active task
    let open_session = SessionRepo::get_open(&conn)?;
    let active_task_id = open_session.as_ref().map(|s| s.task_id);

    // Print header
    println!("═══════════════════════════════════════════════════════════════════════════");
    println!("                           TATL DASHBOARD");
    println!("═══════════════════════════════════════════════════════════════════════════");
    println!();

    // SECTION 1: Queue
    println!("📋 QUEUE ({} tasks)", stack_items.len());
    println!("───────────────────────────────────────────────────────────────────────────");

    if stack_items.is_empty() {
        println!("  (no tasks in queue)");
        } else {
        println!(" #  ID   Description                              Project    Priority");
        for (pos, item) in stack_items.iter().take(5).enumerate() {
            if let Ok(Some(task)) = TaskRepo::get_by_id(&conn, item.task_id) {
                let project = if let Some(pid) = task.project_id {
                    ProjectRepo::get_by_id(&conn, pid).ok().flatten()
                        .map(|p| p.name)
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                let priority = crate::cli::priority::calculate_priority(&task, &conn)
                    .unwrap_or(0.0);
                let indicator = if active_task_id == Some(item.task_id) { "▶" } else { " " };
                let desc: String = task.description.chars().take(40).collect();
                println!("{}{:>2}  {:<4} {:<40} {:<10} {:.1}",
                    indicator, pos, item.task_id, desc, project, priority);
            }
        }
        if stack_items.len() > 5 {
            println!("    ... and {} more", stack_items.len() - 5);
        }
    }
    println!();

    // SECTION 2: Today's Sessions
    let today_sessions = SessionRepo::list_all(&conn)?
        .into_iter()
        .filter(|s| s.start_ts >= today_start_ts || s.is_open())
        .collect::<Vec<_>>();

    // Helper to get session duration, using current time for open sessions
    let get_session_duration = |s: &crate::models::Session| -> i64 {
        s.duration_secs().unwrap_or_else(|| now.timestamp() - s.start_ts)
    };

    let today_total_secs: i64 = today_sessions.iter()
        .map(get_session_duration)
        .sum();

    println!("⏰ TODAY'S SESSIONS ({})", format_duration_short(today_total_secs));
    println!("───────────────────────────────────────────────────────────────────────────");

    if today_sessions.is_empty() {
        println!("  (no sessions today)");
    } else {
        for session in today_sessions.iter().take(5) {
            let task = TaskRepo::get_by_id(&conn, session.task_id).ok().flatten();
            let task_desc: String = task.as_ref()
                .map(|t| t.description.chars().take(30).collect())
                .unwrap_or_else(|| format!("Task {}", session.task_id));
            let project = task.as_ref()
                .and_then(|t| t.project_id)
                .and_then(|pid| ProjectRepo::get_by_id(&conn, pid).ok().flatten())
                .map(|p| p.name)
                .unwrap_or_default();

            let start_time = Local.timestamp_opt(session.start_ts, 0).single()
                .map(|dt| dt.format("%H:%M").to_string())
                .unwrap_or_default();
            let end_time = session.end_ts
                .and_then(|ts| Local.timestamp_opt(ts, 0).single())
                .map(|dt| dt.format("%H:%M").to_string())
                .unwrap_or_else(|| "now".to_string());

            let duration = format_duration_short(get_session_duration(session));
            let indicator = if session.is_open() { "[current]" } else { "" };

            println!(" {:>9} {:<5}-{:<5} {:<30} {:<10} {:>8}",
                indicator, start_time, end_time, task_desc, project, duration);
        }
        if today_sessions.len() > 5 {
            println!("    ... and {} more sessions", today_sessions.len() - 5);
        }
    }
    println!();

    // SECTION 3: Period Statistics
    let period_label = match period.to_lowercase().as_str() {
        "week" => "THIS WEEK",
        "month" => "THIS MONTH",
        "year" => "THIS YEAR",
        _ => "THIS WEEK",
    };

    let period_sessions = SessionRepo::list_all(&conn)?
        .into_iter()
        .filter(|s| s.start_ts >= period_start_ts)
        .collect::<Vec<_>>();

    let period_total_secs: i64 = period_sessions.iter()
        .map(get_session_duration)
        .sum();

    // Count completed tasks in period
    let all_tasks = TaskRepo::list_all(&conn)?;
    let completed_in_period = all_tasks.iter()
        .filter(|(t, _)| {
            t.status.is_terminal()
        })
        .count();

    // Calculate time by project
    let mut time_by_project: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for session in &period_sessions {
        let project = TaskRepo::get_by_id(&conn, session.task_id).ok().flatten()
            .and_then(|t| t.project_id)
            .and_then(|pid| ProjectRepo::get_by_id(&conn, pid).ok().flatten())
            .map(|p| p.name)
            .unwrap_or_else(|| "(no project)".to_string());
        *time_by_project.entry(project).or_insert(0) += get_session_duration(session);
    }

    println!("📊 {}", period_label);
    println!("───────────────────────────────────────────────────────────────────────────");
    println!(" Total time:     {:>10}    │  Tasks completed:  {}",
        format_duration_short(period_total_secs), completed_in_period);
    println!();

    if !time_by_project.is_empty() {
        println!(" By project:");
        let mut sorted_projects: Vec<_> = time_by_project.iter().collect();
        sorted_projects.sort_by(|a, b| b.1.cmp(a.1)); // Sort by time descending

        for (project, &secs) in sorted_projects.iter().take(5) {
            let pct = if period_total_secs > 0 {
                (secs as f64 / period_total_secs as f64 * 100.0) as usize
            } else {
                0
            };
            let bar_len = pct / 5; // Scale to max 20 chars
            let bar = "█".repeat(bar_len) + &"░".repeat(20 - bar_len);
            println!("   {:<15} {:>10} {} {:>3}%",
                project.chars().take(15).collect::<String>(),
                format_duration_short(secs), bar, pct);
        }
    }
    println!();

    // SECTION 4: Attention Needed
    println!("⚠️  ATTENTION NEEDED");
    println!("───────────────────────────────────────────────────────────────────────────");

    // Overdue tasks
    let overdue_tasks: Vec<_> = all_tasks.iter()
        .filter(|(t, _)| {
            t.status == TaskStatus::Open
                && t.due_ts.map(|d| d < now.timestamp()).unwrap_or(false)
        })
        .collect();

    // Stalled tasks (has sessions but not in queue)
    let stack_task_ids: std::collections::HashSet<i64> = stack_items.iter()
        .map(|i| i.task_id)
        .collect();
    let tasks_with_sessions: std::collections::HashSet<i64> = period_sessions.iter()
        .map(|s| s.task_id)
        .collect();
    let stalled_tasks: Vec<_> = all_tasks.iter()
        .filter(|(t, _)| {
            t.status == TaskStatus::Open
                && !stack_task_ids.contains(&t.id.unwrap_or(0))
                && tasks_with_sessions.contains(&t.id.unwrap_or(0))
                && !ExternalRepo::has_active_externals(&conn, t.id.unwrap_or(0)).unwrap_or(false)
        })
        .collect();

    // External tasks
    let external_tasks: Vec<_> = all_tasks.iter()
        .filter(|(t, _)| {
            t.status == TaskStatus::Open
                && ExternalRepo::has_active_externals(&conn, t.id.unwrap_or(0)).unwrap_or(false)
        })
        .collect();

    let mut has_attention_items = false;

    if !overdue_tasks.is_empty() {
        has_attention_items = true;
        println!(" Overdue ({}):", overdue_tasks.len());
        for (t, _) in overdue_tasks.iter().take(3) {
            let days = (now.timestamp() - t.due_ts.unwrap_or(0)) / 86400;
            println!("   #{:<4} {} ({} days)", t.id.unwrap_or(0),
                t.description.chars().take(40).collect::<String>(), days);
        }
        if overdue_tasks.len() > 3 {
            println!("   ... and {} more", overdue_tasks.len() - 3);
        }
    }

    if !stalled_tasks.is_empty() {
        has_attention_items = true;
        println!(" Stalled ({}):", stalled_tasks.len());
        for (t, _) in stalled_tasks.iter().take(3) {
            println!("   #{:<4} {}", t.id.unwrap_or(0),
                t.description.chars().take(50).collect::<String>());
        }
        if stalled_tasks.len() > 3 {
            println!("   ... and {} more", stalled_tasks.len() - 3);
        }
    }

    if !external_tasks.is_empty() {
        has_attention_items = true;
        println!(" External ({}):", external_tasks.len());
        for (t, _) in external_tasks.iter().take(3) {
            println!("   #{:<4} {}", t.id.unwrap_or(0),
                t.description.chars().take(50).collect::<String>());
        }
        if external_tasks.len() > 3 {
            println!("   ... and {} more", external_tasks.len() - 3);
        }
    }

    if !has_attention_items {
        println!("  (nothing needs attention)");
    }

    println!();
    println!("═══════════════════════════════════════════════════════════════════════════");

    Ok(())
}

/// Format duration in short form (e.g., "2h 15m")
fn format_duration_short(secs: i64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

fn handle_projects_report(conn: &Connection) -> Result<()> {
    use std::collections::BTreeMap;
    use crate::filter::calculate_task_stage;

    // Load stage map to get distinct stage names ordered by sort_order
    let stage_map = StageRepo::load_map(conn).unwrap_or_default();
    let mut stage_columns: Vec<String> = Vec::new();
    {
        let mut seen = std::collections::HashSet::new();
        let mut sorted = stage_map.clone();
        sorted.sort_by_key(|m| m.sort_order);
        for m in &sorted {
            if seen.insert(m.stage.clone()) {
                stage_columns.push(m.stage.clone());
            }
        }
    }

    // Get all tasks
    let all_tasks = TaskRepo::list_all(conn)?;

    // Build project hierarchy with counts by stage (dynamic)
    let mut project_stats: BTreeMap<String, HashMap<String, i64>> = BTreeMap::new();
    let mut no_project_stats: HashMap<String, i64> = HashMap::new();

    for (task, _tags) in &all_tasks {
        let stage = calculate_task_stage(task, conn)?;

        let project_name = if let Some(pid) = task.project_id {
            let mut stmt = conn.prepare("SELECT name FROM projects WHERE id = ?1")?;
            let full_name: Option<String> = stmt.query_row([pid], |row| row.get::<_, String>(0)).ok();
            full_name.map(|n| n.split('.').next().unwrap_or(&n).to_string())
        } else {
            None
        };

        let stats = if let Some(name) = project_name {
            project_stats.entry(name).or_default()
        } else {
            &mut no_project_stats
        };

        *stats.entry(stage).or_insert(0) += 1;
    }

    // Calculate totals
    let mut total_stats: HashMap<String, i64> = HashMap::new();
    for stats in project_stats.values() {
        for (stage, count) in stats {
            *total_stats.entry(stage.clone()).or_insert(0) += count;
        }
    }
    for (stage, count) in &no_project_stats {
        *total_stats.entry(stage.clone()).or_insert(0) += count;
    }

    // Capitalize stage names for display headers
    let capitalize = |s: &str| -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(c) => c.to_uppercase().to_string() + chars.as_str(),
        }
    };
    let display_names: Vec<String> = stage_columns.iter().map(|n| capitalize(n)).collect();

    // Column widths: minimum 7, or length of header
    let pw = 20;
    let col_widths: Vec<usize> = display_names.iter()
        .map(|name| name.len().max(7))
        .collect();

    // Print header
    print!("{:<pw$}", "Project", pw = pw);
    for (i, name) in display_names.iter().enumerate() {
        let header = if name.len() > col_widths[i] {
            truncate_str(name, col_widths[i])
        } else {
            name.clone()
        };
        print!(" {:>width$}", header, width = col_widths[i]);
    }
    println!(" {:>6}", "Total");

    // Separator
    print!("{}", "─".repeat(pw));
    for w in &col_widths {
        print!(" {}", "─".repeat(*w));
    }
    println!(" {}", "─".repeat(6));

    // Helper to compute total for a stats map
    let stats_total = |stats: &HashMap<String, i64>| -> i64 {
        stats.values().sum()
    };

    // Print project rows
    for (name, stats) in &project_stats {
        print!("{:<pw$}", truncate_str(name, pw), pw = pw);
        for (i, stage) in stage_columns.iter().enumerate() {
            let count = stats.get(stage).copied().unwrap_or(0);
            print!(" {:>width$}", count, width = col_widths[i]);
        }
        println!(" {:>6}", stats_total(stats));
    }

    if stats_total(&no_project_stats) > 0 {
        print!("{:<pw$}", "(no project)", pw = pw);
        for (i, stage) in stage_columns.iter().enumerate() {
            let count = no_project_stats.get(stage).copied().unwrap_or(0);
            print!(" {:>width$}", count, width = col_widths[i]);
        }
        println!(" {:>6}", stats_total(&no_project_stats));
    }

    // Footer separator
    print!("{}", "─".repeat(pw));
    for w in &col_widths {
        print!(" {}", "─".repeat(*w));
    }
    println!(" {}", "─".repeat(6));

    // Totals
    print!("{:<pw$}", "TOTAL", pw = pw);
    for (i, stage) in stage_columns.iter().enumerate() {
        let count = total_stats.get(stage).copied().unwrap_or(0);
        print!(" {:>width$}", count, width = col_widths[i]);
    }
    println!(" {:>6}", stats_total(&total_stats));

    Ok(())
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

fn handle_stages(subcommand: Option<StagesCommands>) -> Result<()> {
    let conn = DbConnection::connect()?;
    match subcommand {
        None | Some(StagesCommands::List) => handle_stages_list(&conn),
        Some(StagesCommands::Set { id, args }) => handle_stages_set(&conn, id, args),
    }
}

fn handle_stages_list(conn: &Connection) -> Result<()> {
    let mappings = StageRepo::list_all(conn)?;

    // Column widths
    let id_w = 4;
    let status_w = 10;
    let queue_w = 5;
    let sess_w = 8;
    let timer_w = 5;
    let ext_w = 8;
    let stage_w = 12;
    let sort_w = 4;
    let color_w = 14;

    println!(
        "{:>id_w$}  {:<status_w$} {:<queue_w$} {:<sess_w$} {:<timer_w$} {:<ext_w$} {:<stage_w$} {:>sort_w$}  {:<color_w$}",
        "#", "Status", "Queue", "Sessions", "Timer", "External", "Stage", "Sort", "Color",
        id_w = id_w, status_w = status_w, queue_w = queue_w, sess_w = sess_w,
        timer_w = timer_w, ext_w = ext_w, stage_w = stage_w, sort_w = sort_w, color_w = color_w,
    );
    println!(
        "{}  {} {} {} {} {} {} {}  {}",
        "─".repeat(id_w), "─".repeat(status_w), "─".repeat(queue_w), "─".repeat(sess_w),
        "─".repeat(timer_w), "─".repeat(ext_w), "─".repeat(stage_w), "─".repeat(sort_w), "─".repeat(color_w),
    );

    for m in &mappings {
        let queue = if m.in_queue == -1 { "*".to_string() } else if m.in_queue == 1 { "yes".to_string() } else { "no".to_string() };
        let sess = if m.has_sessions == -1 { "*".to_string() } else if m.has_sessions == 1 { "yes".to_string() } else { "no".to_string() };
        let timer = if m.has_open_session == -1 { "*".to_string() } else if m.has_open_session == 1 { "yes".to_string() } else { "no".to_string() };
        let ext = if m.has_externals == -1 { "*".to_string() } else if m.has_externals == 1 { "yes".to_string() } else { "no".to_string() };
        let color = m.color.as_deref().unwrap_or("").to_string();

        println!(
            "{:>id_w$}  {:<status_w$} {:<queue_w$} {:<sess_w$} {:<timer_w$} {:<ext_w$} {:<stage_w$} {:>sort_w$}  {:<color_w$}",
            m.id, m.status, queue, sess, timer, ext, m.stage, m.sort_order, color,
            id_w = id_w, status_w = status_w, queue_w = queue_w, sess_w = sess_w,
            timer_w = timer_w, ext_w = ext_w, stage_w = stage_w, sort_w = sort_w, color_w = color_w,
        );
    }

    Ok(())
}

fn handle_stages_set(conn: &Connection, id: i64, args: Vec<String>) -> Result<()> {
    let mut stage_name: Option<String> = None;
    let mut sort_order: Option<i64> = None;
    let mut color: Option<Option<String>> = None;

    for arg in &args {
        if let Some(val) = arg.strip_prefix("sort_order=") {
            sort_order = Some(val.parse::<i64>()
                .map_err(|_| anyhow::anyhow!("Invalid sort_order value: {}", val))?);
        } else if let Some(val) = arg.strip_prefix("color=") {
            if val == "none" || val.is_empty() {
                color = Some(None);
            } else {
                // Validate color name
                let valid_colors = [
                    "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
                    "bright_black", "bright_red", "bright_green", "bright_yellow",
                    "bright_blue", "bright_magenta", "bright_cyan", "bright_white",
                ];
                if !valid_colors.contains(&val) {
                    anyhow::bail!("Invalid color '{}'. Valid colors: {}", val, valid_colors.join(", "));
                }
                color = Some(Some(val.to_string()));
            }
        } else {
            // Plain text = stage name
            stage_name = Some(arg.clone());
        }
    }

    if stage_name.is_none() && sort_order.is_none() && color.is_none() {
        anyhow::bail!("Nothing to update. Provide a stage name, sort_order=N, or color=name.");
    }

    StageRepo::update(
        conn,
        id,
        stage_name.as_deref(),
        sort_order,
        color.as_ref().map(|c| c.as_deref()),
    )?;

    // Print what was updated
    let mut changes = Vec::new();
    if let Some(ref name) = stage_name {
        changes.push(format!("stage → \"{}\"", name));
    }
    if let Some(so) = sort_order {
        changes.push(format!("sort_order → {}", so));
    }
    if let Some(ref c) = color {
        match c {
            Some(name) => changes.push(format!("color → {}", name)),
            None => changes.push("color → none".to_string()),
        }
    }

    println!("Updated row {}: {}", id, changes.join(", "));
    Ok(())
}

fn handle_send(task_id_str: String, recipient: String, request: Vec<String>) -> Result<()> {
    let conn = DbConnection::connect()?;
    let task_id = validate_task_id(&task_id_str)
        .map_err(|e| anyhow::anyhow!("Invalid task ID: {}", e))?;
    
    // Verify task exists and is open
    let task = TaskRepo::get_by_id(&conn, task_id)?
        .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;

    if task.status.is_terminal() {
        user_error(&format!("Cannot send task {}: status is {}", task_id, task.status.as_str()));
    }

    // Check if task is already sent to this recipient
    let existing_externals = ExternalRepo::get_active_for_task(&conn, task_id)?;
    if existing_externals.iter().any(|e| e.recipient == recipient) {
        return Err(anyhow::anyhow!("Task {} is already sent to {}", task_id, recipient));
    }
    
    // Stop active timer if this task is being timed
    let open_session = SessionRepo::get_open(&conn)?;
    if let Some(session) = open_session {
        if session.task_id == task_id {
            let end_ts = chrono::Utc::now().timestamp();
            let end_ts = std::cmp::max(end_ts, session.start_ts + 1);
            SessionRepo::close_open(&conn, end_ts)
                .context("Failed to close session")?;
            let duration = end_ts - session.start_ts;
            println!("Stopped timing task {}: {} ({}, {})",
                task_id, task.description, format_time(end_ts), format_duration_human(duration));
        }
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

    // Auto-enqueue at bottom (Plan 41: collect re-queues)
    let stack = StackRepo::get_or_create_default(&conn)?;
    StackRepo::enqueue(&conn, stack.id.unwrap(), task_id)?;

    println!("Collected task {}: {}", task_id, task.description);
    println!("  Returned from: {}", externals.iter().map(|e| e.recipient.as_str()).collect::<Vec<_>>().join(", "));
    println!("  Added to queue");
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

/// Resolve an optional target to a task ID string, falling back to the active session's task.
fn resolve_target_or_active(target: Option<String>, command_name: &str) -> Result<String> {
    if let Some(t) = target {
        return Ok(t);
    }
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;
    let session = SessionRepo::get_open(&conn)?;
    if let Some(s) = session {
        Ok(s.task_id.to_string())
    } else {
        anyhow::bail!("No task ID provided and no session is running. Usage: tatl {} <id>", command_name)
    }
}

fn handle_task_add(args: Vec<String>, auto_yes: bool) -> Result<i64> {
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
    
    // Resolve project (handle clearing with project=none or project=)
    let project_id = if let Some(project_name) = parsed.project {
        if project_name == "none" {
            // project=none or project= (empty) means no project
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
                            std::process::exit(0);
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
    
    // Resolve parent
    let parent_id = if let Some(parent_str) = &parsed.parent {
        if parent_str == "none" {
            None
        } else {
            let pid = validate_task_id(parent_str)
                .map_err(|e| anyhow::anyhow!("Invalid parent ID: {}", e))?;
            let parent_task = TaskRepo::get_by_id(&conn, pid)?
                .ok_or_else(|| anyhow::anyhow!("Parent task {} not found", pid))?;
            if parent_task.status.is_terminal() {
                anyhow::bail!("Cannot set parent to task {} (status: {})", pid, parent_task.status.as_str());
            }
            Some(pid)
        }
    } else {
        None
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
        parent_id,
    )
    .context("Failed to create task")?;

    let task_id = task.id.unwrap();
    println!("Created task {}: {}", task_id, description);

    Ok(task_id)
}

fn handle_clone(task_id_str: String, args: Vec<String>, auto_yes: bool) -> Result<i64> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;

    let source_id = validate_task_id(&task_id_str)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let source = TaskRepo::get_by_id(&conn, source_id)?
        .ok_or_else(|| anyhow::anyhow!("Task {} not found", source_id))?;
    let source_tags = TaskRepo::get_tags(&conn, source_id)?;

    // Parse override arguments (if any)
    let overrides = if !args.is_empty() {
        let parsed = match parse_task_args(args) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        };
        Some(parsed)
    } else {
        None
    };

    // Start with source values, apply overrides
    let description = if let Some(ref o) = overrides {
        if !o.description.is_empty() {
            join_description(&o.description)
        } else {
            source.description.clone()
        }
    } else {
        source.description.clone()
    };

    // Resolve project: override > source
    let project_id = if let Some(ref o) = overrides {
        if let Some(ref project_name) = o.project {
            if project_name == "none" || project_name.is_empty() {
                None
            } else {
                let project = ProjectRepo::get_by_name(&conn, project_name)?;
                if let Some(p) = project {
                    Some(p.id.unwrap())
                } else if auto_yes {
                    if let Err(e) = validate_project_name(project_name) {
                        user_error(&e);
                    }
                    let project = ProjectRepo::create(&conn, project_name)?;
                    println!("Created project '{}' (id: {})", project.name, project.id.unwrap());
                    Some(project.id.unwrap())
                } else {
                    match prompt_create_project(project_name, &conn)? {
                        Some(true) => {
                            if let Err(e) = validate_project_name(project_name) {
                                user_error(&e);
                            }
                            let project = ProjectRepo::create(&conn, project_name)?;
                            println!("Created project '{}' (id: {})", project.name, project.id.unwrap());
                            Some(project.id.unwrap())
                        }
                        Some(false) => None,
                        None => {
                            println!("Cancelled.");
                            std::process::exit(0);
                        }
                    }
                }
            }
        } else {
            source.project_id
        }
    } else {
        source.project_id
    };

    let due_ts = if let Some(ref o) = overrides {
        if let Some(ref due) = o.due {
            Some(parse_date_expr(due).context("Failed to parse due date")?)
        } else {
            source.due_ts
        }
    } else {
        source.due_ts
    };

    let scheduled_ts = if let Some(ref o) = overrides {
        if let Some(ref scheduled) = o.scheduled {
            Some(parse_date_expr(scheduled).context("Failed to parse scheduled date")?)
        } else {
            source.scheduled_ts
        }
    } else {
        source.scheduled_ts
    };

    let wait_ts = if let Some(ref o) = overrides {
        if let Some(ref wait) = o.wait {
            Some(parse_date_expr(wait).context("Failed to parse wait date")?)
        } else {
            source.wait_ts
        }
    } else {
        source.wait_ts
    };

    let alloc_secs = if let Some(ref o) = overrides {
        if let Some(ref allocation) = o.allocation {
            Some(parse_duration(allocation).context("Failed to parse allocation duration")?)
        } else {
            source.alloc_secs
        }
    } else {
        source.alloc_secs
    };

    // Merge UDAs: start with source, apply overrides
    let mut udas = source.udas.clone();
    if let Some(ref o) = overrides {
        for (k, v) in &o.udas {
            if v.is_empty() {
                udas.remove(k);
            } else {
                udas.insert(k.clone(), v.clone());
            }
        }
    }

    // Merge tags: start with source, apply adds/removes
    let mut tags: Vec<String> = source_tags;
    if let Some(ref o) = overrides {
        for tag in &o.tags_remove {
            tags.retain(|t| t != tag);
        }
        for tag in &o.tags_add {
            if !tags.contains(tag) {
                tags.push(tag.clone());
            }
        }
    }

    // Template: override > source
    let template = if let Some(ref o) = overrides {
        if o.template.is_some() { o.template.clone() } else { source.template.clone() }
    } else {
        source.template.clone()
    };

    // Always clear respawn on clone (one-shot operation)
    let respawn: Option<String> = None;

    // Parent: override > source
    let parent_id = if let Some(ref o) = overrides {
        if let Some(ref parent_str) = o.parent {
            if parent_str == "none" {
                None
            } else {
                let pid = validate_task_id(parent_str)
                    .map_err(|e| anyhow::anyhow!("Invalid parent ID: {}", e))?;
                TaskRepo::get_by_id(&conn, pid)?
                    .ok_or_else(|| anyhow::anyhow!("Parent task {} not found", pid))?;
                Some(pid)
            }
        } else {
            source.parent_id
        }
    } else {
        source.parent_id
    };

    let new_task = TaskRepo::create_full(
        &conn,
        &description,
        project_id,
        due_ts,
        scheduled_ts,
        wait_ts,
        alloc_secs,
        template,
        respawn,
        &udas,
        &tags,
        parent_id,
    )
    .context("Failed to create cloned task")?;

    let new_id = new_task.id.unwrap();
    println!("Cloned task {} → new task {}: {}", source_id, new_id, description);

    Ok(new_id)
}

struct ListRequest {
    filter_tokens: Vec<String>,
    sort_columns: Vec<String>,
    group_columns: Vec<String>,
    hide_columns: Vec<String>,
    color_column: Option<String>,
    fill_column: Option<String>,
    save_alias: Option<String>,
}

fn parse_list_request(tokens: Vec<String>) -> ListRequest {
    let mut filter_tokens = Vec::new();
    let mut sort_columns = Vec::new();
    let mut group_columns = Vec::new();
    let mut hide_columns = Vec::new();
    let mut color_column: Option<String> = None;
    let mut fill_column: Option<String> = None;
    let mut save_alias: Option<String> = None;
    
    for token in tokens {
        if let Some(spec) = token.strip_prefix("sort:") {
            sort_columns.extend(spec.split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()));
        } else if let Some(spec) = token.strip_prefix("group:") {
            group_columns.extend(spec.split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()));
        } else if let Some(spec) = token.strip_prefix("hide:") {
            hide_columns.extend(spec.split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()));
        } else if let Some(spec) = token.strip_prefix("color:") {
            if color_column.is_none() && !spec.is_empty() {
                color_column = Some(spec.to_lowercase());
            }
        } else if let Some(spec) = token.strip_prefix("fill:") {
            if fill_column.is_none() && !spec.is_empty() {
                fill_column = Some(spec.to_lowercase());
            }
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
        color_column,
        fill_column,
        save_alias,
    }
}

fn is_view_name_token(token: &str) -> bool {
    !token.contains(':') && !token.starts_with('+') && !token.starts_with('-') && token.parse::<i64>().is_err()
}

fn looks_like_filter(token: &str) -> bool {
    token.contains('=') || token.contains('>') || token.contains('<')
        || token.starts_with('+') || token.starts_with('-') || token == "waiting"
}

fn handle_task_list(filter_args: Vec<String>, json: bool, relative: bool, full: bool) -> Result<()> {
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
            request.color_column = view.color_column;
            request.fill_column = view.fill_column;
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
            &request.color_column,
            &request.fill_column,
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
            color_column: request.color_column,
            fill_column: request.fill_column,
            full_width: full,
        };
        let table = format_task_list_table(&conn, &tasks, &options)?;
        print!("{}", table);
    }
    
    Ok(())
}

/// Handle task modify with optional --on flag
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
    
    // Resolve project (handle clearing with project=none)
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
    
    // Resolve parent
    let parent_id = if let Some(parent_str) = &parsed.parent {
        if parent_str == "none" {
            Some(None) // Clear parent
        } else {
            let pid = validate_task_id(parent_str)
                .map_err(|e| anyhow::anyhow!("Invalid parent ID: {}", e))?;
            TaskRepo::get_by_id(conn, pid)?
                .ok_or_else(|| anyhow::anyhow!("Parent task {} not found", pid))?;
            // Cycle detection
            TaskRepo::validate_no_cycle(conn, task_id, pid)?;
            Some(Some(pid))
        }
    } else {
        None // Don't change
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
        conn,
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
        parent_id,
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
    let mut eligible_ids = Vec::new();
    let mut missing_ids = Vec::new();
    let mut ineligible = Vec::new();
    
    for task_id in &task_ids {
        match TaskRepo::get_by_id(&conn, *task_id)? {
            Some(task) => {
                if task.status == TaskStatus::Open {
                    eligible_ids.push(*task_id);
                } else {
                    ineligible.push((*task_id, task.status));
                }
            }
            None => missing_ids.push(*task_id),
        };
    }
    
    if !missing_ids.is_empty() {
        user_error(&format!("Task(s) not found: {}", 
            missing_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", ")));
    }
    
    if !ineligible.is_empty() {
        let details = ineligible.iter()
            .map(|(id, status)| format!("{} ({})", id, status.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        user_error(&format!(
            "Cannot add task(s) to the queue because they are not open: {}. Reopen the task(s) to make them eligible.",
            details
        ));
    }
    
    if eligible_ids.is_empty() {
        user_error("No eligible tasks to enqueue");
    }
    
    // Enqueue all tasks in order
    let stack = StackRepo::get_or_create_default(&conn)?;
    let stack_id = stack.id.unwrap();
    
    for task_id in eligible_ids {
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
    let session = session_opt.expect("checked above");
    
    // Parse end time (defaults to "now")
    let mut end_ts = if time_args.is_empty() {
        chrono::Utc::now().timestamp()
    } else {
        let end_expr = time_args.join(" ");
        parse_date_expr(&end_expr)
            .context("Invalid end time expression")?
    };

    // Ensure monotonicity (end must be after start). This can happen in tests (or if the user
    // ends a session at a time-only expression that resolves before the session start).
    if end_ts <= session.start_ts {
        end_ts = session.start_ts + 1;
    }
    
    // Close session
    let closed = SessionRepo::close_open(&conn, end_ts)
        .context("Failed to close session")?;
    
    if let Some(session) = closed {
        let task_id = session.task_id;
        // Get task description for better message
        let task = TaskRepo::get_by_id(&conn, task_id)?;
        let desc = task.as_ref().map(|t| t.description.as_str()).unwrap_or("");
        let duration = end_ts - session.start_ts;
        println!("Stopped timing task {}: {} ({}, {})", task_id, desc, format_time(end_ts), format_duration_human(duration));

        // Invariant 3: external-waiting tasks are removed from queue when timer stops
        if ExternalRepo::has_active_externals(&conn, task_id)? {
            let stack = StackRepo::get_or_create_default(&conn)?;
            StackRepo::remove_task(&conn, stack.id.unwrap(), task_id)?;
        }
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
    
    // Capture raw expressions for time-only disambiguation
    let (stop_expr, start_expr_opt) = if let Some(sep_pos) = arg_str.find("..") {
        (
            arg_str[..sep_pos].trim().to_string(),
            Some(arg_str[sep_pos + 2..].trim().to_string())
        )
    } else {
        (arg_str.clone(), None)
    };
    
    // Get current session (anchor for interpreting time-only expressions)
    let current_session = SessionRepo::get_open(conn)?
        .expect("Session should exist - checked in caller");
    
    // Parse time arguments - check for interval syntax, then align time-only expressions to the session timeline
    let (mut stop_ts, mut start_ts_opt) = parse_offon_time_args(&arg_str)?;
    stop_ts = align_time_only_to_anchor(stop_ts, &stop_expr, current_session.start_ts);
    if stop_ts <= current_session.start_ts {
        user_error(&format!(
            "Stop time must be after the running session start at {}.",
            format_time(current_session.start_ts)
        ));
    }
    
    if let Some(start_ts) = start_ts_opt {
        let resume_ts = align_time_only_to_anchor(start_ts, start_expr_opt.as_deref().unwrap_or(""), stop_ts);
        if resume_ts <= stop_ts {
            user_error("Resume time must be after the stop time. Provide a later time or include a date.");
        }
        start_ts_opt = Some(resume_ts);
    }
    
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
    let alloc = resume_task.as_ref().and_then(|t| t.alloc_secs);
    let context = crate::cli::output::format_on_context(conn, resume_task_id, alloc)?;
    if !context.is_empty() {
        print!("{}", context);
    }

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

/// Detect simple time-only expressions (no explicit date or relative keywords)
fn is_time_only_expression(expr: &str) -> bool {
    let expr = expr.trim();
    if expr.is_empty() {
        return false;
    }
    
    let lower = expr.to_lowercase();
    let has_date_hint = lower.contains('-')
        || lower.contains('t')
        || lower.contains("today")
        || lower.contains("tomorrow")
        || lower.contains("eod")
        || lower.contains("eow")
        || lower.contains("eom")
        || lower.starts_with('+')
        || lower.starts_with("in ")
        || lower.contains("next ");
    
    if has_date_hint {
        return false;
    }
    
    lower.contains(':')
        || lower.ends_with("am")
        || lower.ends_with("pm")
        || lower == "noon"
        || lower == "midnight"
}

/// Align a time-only timestamp to be after the provided anchor (by adding 24h if needed)
fn align_time_only_to_anchor(ts: i64, expr: &str, anchor_ts: i64) -> i64 {
    if !is_time_only_expression(expr) {
        return ts;
    }
    
    let mut aligned = ts;
    while aligned <= anchor_ts {
        aligned += 86_400; // 24h in seconds
    }
    aligned
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

/// Handle `tatl dequeue [<task_id>]` - Remove from queue without closing
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
    let task = match TaskRepo::get_by_id(conn, task_id)? {
        Some(task) => task,
        None => user_error(&format!("Task {} not found", task_id)),
    };
    if task.status != TaskStatus::Open {
        user_error(&format!(
            "Task {} is {} and cannot be started from the queue. Reopen the task to make it eligible.",
            task_id,
            task.status.as_str()
        ));
    }
    
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
        println!("Started timing task {}: {} ({})", task_id, desc, format_time(start_ts));
        let alloc = task.as_ref().and_then(|t| t.alloc_secs);
        let context = crate::cli::output::format_on_context(conn, task_id, alloc)?;
        if !context.is_empty() {
            print!("{}", context);
        }
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
    
    // Check if task exists and is eligible for queue
    let task = match TaskRepo::get_by_id(&conn, task_id)? {
        Some(task) => task,
        None => user_error(&format!("Task {} not found", task_id)),
    };
    if task.status != TaskStatus::Open {
        user_error(&format!(
            "Task {} is {} and cannot be added to the queue. Reopen the task to make it eligible.",
            task_id,
            task.status.as_str()
        ));
    }
    let task_desc = task.description.clone();
    
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

    // If we're switching tasks "at now" (no explicit time) and the existing open session's
    // start time is in the future relative to now (possible in tests that use time-only
    // expressions like "09:00"), ensure we use a strictly increasing timestamp.
    //
    // This prevents close_open() from failing with end_ts <= start_ts.
    let effective_start_ts = if end_ts_opt.is_none() {
        if let Some(s) = &existing_session {
            if start_ts <= s.start_ts {
                s.start_ts + 1
            } else {
                start_ts
            }
        } else {
            start_ts
        }
    } else {
        start_ts
    };
    
    // If session is running, close it at the effective start time
    if existing_session.is_some() {
        SessionRepo::close_open(&tx, effective_start_ts)
            .context("Failed to close existing session")?;
    }
    
    // Check for overlap prevention (before creating new session)
    // Note: This might need to be done outside transaction if it queries other sessions
    // For now, we'll do it within the transaction
    check_and_amend_overlaps_transactional(&tx, effective_start_ts)?;
    
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
        SessionRepo::create(&tx, task_id, effective_start_ts)
            .context("Failed to start session")?;
        tx.commit()?;
        println!("Started timing task {}: {} ({})", task_id, task_desc, format_time(effective_start_ts));
        let context = crate::cli::output::format_on_context(&conn, task_id, task.alloc_secs)?;
        if !context.is_empty() {
            print!("{}", context);
        }
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

fn handle_task_close(
    mut id_or_filter_opt: Option<String>,
    mut at_opt: Option<String>,
    yes: bool,
    interactive: bool,
) -> Result<()> {
    let conn = DbConnection::connect()
        .context("Failed to connect to database")?;

    // Disambiguation: allow `tatl close <time>` (with no explicit target).
    //
    // Clap will otherwise treat the first positional as `target` because it's optional and
    // `time_args` is `trailing_var_arg`. If the "target" looks like a time expression and
    // cannot be parsed as a task selector, interpret it as the end time instead.
    if at_opt.is_none() {
        if let Some(t) = id_or_filter_opt.clone() {
            let looks_like_time = t.contains(':') || t.contains('T') || t.contains('-');
            let looks_like_task_selector =
                t.parse::<i64>().is_ok()
                || t.contains(',')
                // Treat "-" as a task range only if it doesn't look like a datetime (dates contain '-')
                || (t.contains('-') && !t.contains('T') && !t.contains(':'))
                || t.contains('=') || t.contains('+') || t.contains("..");

            if looks_like_time && !looks_like_task_selector {
                at_opt = Some(t);
                id_or_filter_opt = None;
            }
        }
    }
    
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
            println!("This will close {} task(s).", task_ids.len());
            print!("Close all tasks? (y/n/i): ");
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
                    return handle_close_interactive(&conn, &task_ids, end_ts);
                }
                _ => {
                    println!("Invalid input. Cancelled.");
                    return Ok(());
                }
            }
        } else if interactive {
            // Force interactive mode
            return handle_close_interactive(&conn, &task_ids, end_ts);
        }
    }
    
    // Complete all tasks
    for task_id in &task_ids {
        // Verify task exists
        if TaskRepo::get_by_id(&conn, *task_id)?.is_none() {
            eprintln!("Error: Task {} not found", task_id);
            continue; // Continue processing other tasks
        }
        
        // Check if session is running for this task - close it if it exists
        let mut effective_end_ts = end_ts;
        if let Some(session) = &open_session {
            if session.task_id == *task_id {
                // Close the session
                effective_end_ts = std::cmp::max(end_ts, session.start_ts + 1);
                SessionRepo::close_open(&conn, effective_end_ts)
                    .context("Failed to close session")?;
                let duration = effective_end_ts - session.start_ts;
                let task_desc = TaskRepo::get_by_id(&conn, *task_id)?
                    .map(|t| t.description)
                    .unwrap_or_default();
                println!("Stopped timing task {}: {} ({}, {})",
                    task_id, task_desc, format_time(effective_end_ts), format_duration_human(duration));
            }
        }
        // Note: We allow closing tasks even if no session is running

        // Get task before closing (to check respawn rule)
        let task = TaskRepo::get_by_id(&conn, *task_id)?
            .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;

        // Mark task as closed (intent fulfilled)
        TaskRepo::close(&conn, *task_id)
            .context("Failed to close task")?;

        // Invariant 4: terminal lifecycle cleanup - clear active externals
        ExternalRepo::mark_all_returned_for_task(&conn, *task_id)?;

        // Handle respawn if task has respawn rule
        if let Some(new_task_id) = respawn_task(&conn, &task, effective_end_ts)? {
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

        println!("Closed task {}: {}", task_id, task.description);
    }

    Ok(())
}

fn handle_close_interactive(conn: &Connection, task_ids: &[i64], end_ts: i64) -> Result<()> {
    use std::io::{self, Write};

    let open_session = SessionRepo::get_open(conn)?;

    for task_id in task_ids {
        // Get task description for display
        let task = TaskRepo::get_by_id(conn, *task_id)?;
        if task.is_none() {
            eprintln!("Error: Task {} not found", task_id);
            continue; // Continue processing other tasks
        }
        let task = task.unwrap();

        // Prompt for confirmation
        print!("Close task {} ({})? (y/n): ", task_id, task.description);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("Skipped task {}.", task_id);
            continue;
        }

        // Close the session if this is the running task
        let mut effective_end_ts = end_ts;
        if let Some(session) = &open_session {
            if session.task_id == *task_id {
                effective_end_ts = std::cmp::max(end_ts, session.start_ts + 1);
                SessionRepo::close_open(conn, effective_end_ts)
                    .context("Failed to close session")?;
                let duration = effective_end_ts - session.start_ts;
                println!("Stopped timing task {}: {} ({}, {})",
                    task_id, task.description, format_time(effective_end_ts), format_duration_human(duration));
            }
        }

        // Mark task as closed (intent fulfilled)
        TaskRepo::close(conn, *task_id)
            .context("Failed to close task")?;

        // Invariant 4: terminal lifecycle cleanup - clear active externals
        ExternalRepo::mark_all_returned_for_task(conn, *task_id)?;

        // Handle respawn if task has respawn rule
        if let Some(new_task_id) = respawn_task(conn, &task, effective_end_ts)? {
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

        println!("Closed task {}: {}", task_id, task.description);
    }

    Ok(())
}

/// Handle task cancel with optional target (defaults to queue[0])
fn handle_task_cancel_optional(target: Option<String>, yes: bool, interactive: bool) -> Result<()> {
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
    
    handle_task_cancel(id_or_filter, yes, interactive)
}

fn handle_task_cancel(id_or_filter: String, yes: bool, interactive: bool) -> Result<()> {
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
            println!("This will cancel {} task(s).", task_ids.len());
            print!("Cancel all tasks? (y/n/i): ");
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
                    return handle_cancel_interactive(&conn, &task_ids);
                }
                _ => {
                    println!("Invalid input. Cancelled.");
                    return Ok(());
                }
            }
        } else if interactive {
            return handle_cancel_interactive(&conn, &task_ids);
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
        
        // Get task before cancelling (to check respawn rule)
        let task = TaskRepo::get_by_id(&conn, *task_id)?
            .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;

        TaskRepo::cancel(&conn, *task_id)
            .context("Failed to cancel task")?;

        // Invariant 4: terminal lifecycle cleanup - clear active externals
        ExternalRepo::mark_all_returned_for_task(&conn, *task_id)?;

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

        println!("Cancelled task {}: {}", task_id, task.description);
    }
    
    Ok(())
}

fn handle_cancel_interactive(conn: &Connection, task_ids: &[i64]) -> Result<()> {
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
        
        print!("Cancel task {} ({})? (y/n): ", task_id, task.description);
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

        TaskRepo::cancel(conn, *task_id)
            .context("Failed to cancel task")?;

        // Invariant 4: terminal lifecycle cleanup - clear active externals
        ExternalRepo::mark_all_returned_for_task(conn, *task_id)?;

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

        println!("Cancelled task {}: {}", task_id, task.description);
    }
    
    Ok(())
}

/// Handle task reopen (set status back to open)
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
        
        if task.status == crate::models::TaskStatus::Open {
            println!("Task {} is already open", task_id);
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
        
        if task.status == crate::models::TaskStatus::Open {
            println!("Task {} is already open, skipping.", task_id);
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

        // Orphan children before deleting
        TaskRepo::orphan_children(conn, task_ids[0])?;
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
                // Orphan children before deleting
                TaskRepo::orphan_children(conn, *task_id)?;
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

        // Orphan children before deleting
        TaskRepo::orphan_children(conn, *task_id)?;
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

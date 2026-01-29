# TATL Command Reference

Complete reference for all Tatl commands with examples and usage patterns.

## Table of Contents

- [Task Commands](#task-commands)
- [Project Commands](#project-commands)
- [Timing Commands](#timing-commands)
- [Queue Commands](#queue-commands)
- [Session Commands](#session-commands)
- [Respawning Tasks](#respawning-tasks)
- [Filter Syntax](#filter-syntax)
- [Date Expressions](#date-expressions)
- [Duration Format](#duration-format)

---

## Task Commands

### `tatl add [-y] <description> [attributes...] [ : <command> [args]...]`

Add a new task with optional attributes.

**Description:** The task description is the first argument or all non-attribute tokens.

**Options:**
- `-y` - Auto-confirm prompts (create new projects, modify overlapping sessions)

**Notes:**
- Multiple pipe commands can be chained: `tatl add "Task" : onoff 09:00..12:00 : finish`
- Pipe commands execute sequentially, passing the task ID from one to the next

**Attributes:**
- `project=<name>` - Assign to project
- `due=<expr>` - Set due date
- `scheduled=<expr>` - Set scheduled date
- `wait=<expr>` - Set wait date
- `allocation=<duration>` - Set time allocation
- `template=<name>` - Use template
- `respawn=<pattern>` - Set respawn rule (creates new instance on completion)
- `+<tag>` - Add tag
- `uda.<key>=<value>` - Set user-defined attribute

**Examples:**
```bash
# Simple task
tatl add Fix bug in authentication

# Task with project and tags
tatl add Review PR project=work +code-review +urgent

# Task with due date and allocation
tatl add Write documentation project=docs due=tomorrow allocation=2h

# Task with respawn rule (creates new instance when completed)
tatl add "Daily standup" respawn=daily due=09:00

# Task with UDA
tatl add Customer call uda.client=acme uda.priority=high

# Task with : on (automatically starts timing)
tatl add "Start working on feature" : on
tatl add "Fix urgent bug" project=work +urgent : on

# Task with : on <time> (start timing at earlier time)
tatl add "Meeting started at 2pm" project=meetings : on 14:00
tatl add "Forgot to start timer" project=work : on 09:30

# Task with --enqueue (adds to queue without starting timing)
tatl add "Review documentation" project=docs : enqueue
tatl add "Write tests" project=work due=tomorrow allocation=2h : enqueue

# Task with : onoff (create task and add historical session)
tatl add "Emergency meeting" project=meetings : onoff 14:00..15:00
tatl add "Support request" +support : onoff 10:30..11:00

# Task with : finish (create already completed task)
tatl add "Already done task" project=work : finish
tatl add "Fixed bug yesterday" project=work : onoff 14:00..15:00 : finish

# Task with : close (create already closed task)
tatl add "Cancelled request" project=work : close
tatl add "Started but abandoned" : onoff 09:00..10:00 : close

# Task with new project (interactive prompt)
tatl add "New feature" project=newproject
# Prompts: "This is a new project 'newproject'. Add new project? [y/n/c] (default: y):"
# - y: Create project and continue (default)
# - n: Skip project, create task without it
# - c: Cancel task creation

# Task with -y (non-interactive)
tatl add -y "New feature" project=newproject
# Automatically creates project if it doesn't exist
```

### `tatl list [filter] [options]`

List tasks matching optional filter with display customization.

**Options:**
- `--json` - Output in JSON format
- `--relative` - Show due dates as relative time (e.g., "2 days ago", "in 3 days")
- `--full` - Show all columns regardless of terminal width

**Display Modifiers:**
- `sort:<column>` - Sort by column (prefix with `-` for descending)
- `group:<column>` - Group tasks by column value
- `hide:<column>` - Hide specified column(s)
- `color:<column>` - Apply text color based on column value
- `fill:<column>` - Apply background color based on column value

**Color Types:**
- **Categorical** (project, status, kanban, tags): Semantic or hash-based colors
- **Numeric** (priority, alloc, clock): Gradient from green → yellow → red
- **Date** (due, scheduled): Heat map from green (far) → red (near/overdue)

**Note:** Colors only appear in terminal (TTY) output. Piped output has no ANSI codes.

**Examples:**
```bash
# List all tasks
tatl list

# List with filter
tatl list project=work
tatl list +urgent
tatl list status=pending

# Sort and group
tatl list sort:project,priority group:kanban

# Color output
tatl list color:project          # Hash-based colors per project
tatl list color:kanban           # Semantic colors for stages
tatl list fill:status            # Background color by status
tatl list color:priority         # Priority gradient

# Combine options
tatl list group:project color:project   # Colored group headers
tatl list sort:-priority color:kanban   # Sorted with row colors

# Hide columns
tatl list hide:tags,status

# JSON output
tatl list --json
tatl list project=work +urgent --json
```

### `tatl modify <id|filter> [attributes...] [--yes] [--interactive]`

Modify one or more tasks.

**Options:**
- `--yes` - Apply to all matching tasks without confirmation (also auto-creates new projects if needed)
- `--interactive` - Confirm each task one by one

**Examples:**
```bash
# Modify single task
tatl modify 10 +urgent due=+2d

# Modify multiple tasks (with confirmation)
tatl modify project=work description=Updated description

# Modify with --yes flag
tatl modify +urgent due=+1d --yes

# Modify with new project (prompts to create)
tatl modify 10 project=newproject

# Clear attributes
tatl 10 modify project=none due=none allocation=none
```

### `tatl finish [<id|filter>] [--at <expr>] [--next] [--yes] [--interactive]`

Complete one or more tasks.

**Behavior:**
1. If task has running session: closes session at `--at` or now
2. If task has no running session: marks task as completed (no session to close)
3. Marks task as completed
4. Removes from clock stack
5. If `--next` and clock stack non-empty: starts session for new clock[0]

**Notes:**
- `tatl finish` (without ID/filter) requires clock[0] and a running session
- `tatl finish <id>` or `tatl finish <filter>` works even if task is not clocked in
- If a session exists for the task, it will be closed when completing

**Options:**
- `--at <expr>` - End session at specific time (only applies if session exists)
- `--next` - Automatically start next task in clock stack (only if session was closed)
- `--yes` - Complete all matching tasks without confirmation
- `--interactive` - Confirm each task one by one

**Examples:**
```bash
# Complete current task (requires clocked in)
tatl finish

# Complete specific task (works even if not clocked in)
tatl finish 10

# Complete with --next
tatl finish --next

# Complete at specific time
tatl finish 10 --at 17:00

# Complete multiple tasks
tatl finish +urgent --yes
```

### `tatl close <id|filter> [--yes] [--interactive]`

Close one or more tasks (sets status to `closed`).

**Notes:**
- `tatl close <id>` or `tatl close <filter>` works even if task is not clocked in
- If a session exists for the task, it will be closed when closing

**Options:**
- `--yes` - Close all matching tasks without confirmation
- `--interactive` - Confirm each task one by one

**Examples:**
```bash
# Close a task
tatl close 10

# Close multiple tasks
tatl close project=work --yes
```

### `tatl annotate [<id>] <note...> [--task <id>] [--delete <annotation_id>]`

Add or delete annotation to/from a task.

**Behavior:**
- If `<id>` is provided and valid: annotate that task.
- If `<id>` is missing or invalid and a task is clocked in: annotate the LIVE task.
- If no task is clocked in: error.
- Links annotation to current session if clock is running.

**Options:**
- `--task <id>` - Override task selection
- `--delete <annotation_id>` - Delete a specific annotation

**Examples:**
```bash
# Annotate specific task
tatl annotate 10 Found the bug in auth module

# Annotate current LIVE task
tatl annotate Investigating flaky tests

# Multiple words in annotation
tatl annotate 10 This is a longer note with multiple words

# Delete annotation
tatl annotate 10 --delete 5
```

### `tatl show <id|filter>`

Show detailed summary of task(s).

**Examples:**
```bash
# Show single task
tatl show 10

# Show task range
tatl show 1-3

# Show task list
tatl show 1,3,5

# Show with filter
tatl show project=work
```

### `tatl delete <id|filter> [--yes] [--interactive]`

Permanently delete task(s).

**Options:**
- `--yes` - Delete all matching tasks without confirmation
- `--interactive` - Confirm each task one by one

**Examples:**
```bash
# Delete single task
tatl delete 10

# Delete with confirmation
tatl delete 10 --yes

# Delete multiple tasks
tatl delete +old --yes
```

---

## Project Commands

### `tatl projects add <name>`

Create a new project.

**Examples:**
```bash
# Simple project
tatl projects add work

# Nested project (dot notation)
tatl projects add admin.email
tatl projects add sales.northamerica.texas
```

### `tatl projects list [--archived]`

List all projects.

**Options:**
- `--archived` - Include archived projects

**Examples:**
```bash
# List active projects
tatl projects list

# List all projects including archived
tatl projects list --archived
```

### `tatl projects rename <old_name> <new_name> [--force]`

Rename a project.

**Behavior:**
- If target name exists and `--force` not provided: error
- If target name exists and `--force` provided: merge projects (all tasks moved to target)

**Options:**
- `--force` - Merge projects if target exists

**Examples:**
```bash
# Rename project
tatl projects rename temp work

# Merge projects
tatl projects rename temp work --force
```

### `tatl projects archive <name>`

Archive a project.

**Examples:**
```bash
tatl projects archive old-project
```

### `tatl projects report`

Display task counts by project and kanban status.

Shows a table with columns for each kanban status (Proposed, Stalled, Queued, External, Done) and rows for each project.

**Examples:**
```bash
# Show project report
tatl projects report
```

**Output:**
```
Projects Report
═══════════════════════════════════════════════════════════════════
Project                   Proposed   Stalled   Queued   External   Done  Total
───────────────────────── ──────── ──────── ──────── ──────── ────── ──────
work                            3        1        2        0      5     11
work.email                      1        0        0        0      2      3
personal                        2        0        1        1      3      7
───────────────────────── ──────── ──────── ──────── ──────── ────── ──────
TOTAL                           6        1        3        1     10     21
```

---

## Timing Commands

The task queue controls which tasks are active. The task at position 0 (queue[0]) is the "active" task. Queue operations (enqueue, pick, dequeue) affect which task is active. `on`/`off` controls timing.

### `tatl on [<id>] [<start>|<start..end>]`

Start timing the current queue[0] task, or a specific task.

**Behavior:**
- If `<id>` provided: pushes task to top and starts timing
- If `<id>` omitted: uses queue[0]
- If no time arguments: starts at "now"
- If single time: starts at specified time
- If interval (`start..end`): creates closed session

**Examples:**
```bash
# Start now (uses queue[0])
tatl on

# Start at specific time (uses queue[0])
tatl on 09:00

# Create closed interval (uses queue[0])
tatl on 09:00..11:00

# Push task 5 to top and start timing
tatl on 5

# Push task 10 to top and start at specific time
tatl on 10 09:00
```

### `tatl off [<end>]`

Stop the currently running session.

**Examples:**
```bash
# Stop now
tatl off

# Stop at specific time
tatl off 17:00
```

### `tatl offon <stop>[..<start>] [<task_id>] [-y]`

Stop current session and resume (with optional break). Useful for capturing breaks after the fact.

**Behavior:**
- If a session is running: stops it at `<stop>` and starts a new one (at `<start>` or now)
- If no session is running: operates on history (finds and modifies overlapping sessions)

**Arguments:**
- `<stop>` - Time to stop the current session
- `<stop>..<start>` - Stop at first time, resume at second time
- `<task_id>` - Optional task ID for the new session (defaults to queue[0])
- `-y` - Skip confirmation for history modifications

**Examples:**
```bash
# Interrupted at 14:30, resuming now
tatl offon 14:30

# Interrupted at 14:30, resuming at 15:00 (30 min break)
tatl offon 14:30..15:00

# Remove 14:30-15:00 from history (modifies overlapping sessions)
tatl offon 14:30..15:00 -y

# Split session at 14:30
tatl offon 14:30 -y
```

### `tatl onoff <start>..<end> [<task_id>] [-y]`

Add a historical session for a task. Replaces `sessions add`.

**Behavior:**
- Creates a closed session for the specified interval
- Defaults to queue[0] task if task_id not provided
- If overlapping sessions exist: prompts for confirmation, then clears overlapping time and inserts new session

**Arguments:**
- `<start>..<end>` - Time interval for the session (required)
- `<task_id>` - Optional task ID (defaults to queue[0])
- `-y` - Skip confirmation for overlapping session modifications

**Examples:**
```bash
# Add session for queue[0] from 09:00 to 12:00 today
tatl onoff 09:00..12:00

# Add session for task 10 from 09:00 to 12:00
tatl onoff 09:00..12:00 10

# Insert session into overlapping time without confirmation
tatl onoff 14:00..15:00 5 -y
```

### `tatl enqueue <id|id,id,...|range|mixed>`

Add task(s) to end of queue (do it later).

**Arguments:**
- `<id>` - Single task ID
- `<id,id,...>` - Comma-separated list of task IDs
- `<start-end>` - Range of task IDs (e.g., `30-31`)
- Mixed syntax - Combine lists and ranges (e.g., `1,3-5,10`)

**Examples:**
```bash
# Enqueue single task
tatl enqueue 10

# Enqueue multiple tasks
tatl enqueue 1,3,5

# Enqueue range
tatl enqueue 30-31
```

### `tatl dequeue [<task_id>]`

Remove task from queue without finishing.

**Behavior:**
- If `<task_id>` provided: removes that task from queue
- If omitted: removes queue[0]

**Examples:**
```bash
# Remove queue[0]
tatl dequeue

# Remove specific task
tatl dequeue 5
```

### `tatl queue sort <field>`

Sort the queue by a specified field.

**Arguments:**
- `<field>` - Field to sort by: `priority`, `due`, `scheduled`, `alloc`, `id`, `description`
- Prefix with `-` for descending order (e.g., `-priority`, `-due`)

**Behavior:**
- Ascending sorts put smallest values first (earlier dates, lower priorities)
- Descending sorts put largest values first (later dates, higher priorities)
- Tasks with missing values for the sort field are placed at the end

**Examples:**
```bash
# Sort by due date (earliest first)
tatl queue sort due

# Sort by priority (highest first, descending)
tatl queue sort -priority

# Sort by allocation time (shortest first)
tatl queue sort alloc

# Sort by scheduled date
tatl queue sort scheduled
```

### `tatl list`

Display the current task queue with full task details.

**Options:**
- `--json` - Output in JSON format

**Examples:**
```bash
# List queue
tatl list

# JSON output
tatl list --json
```

---

## Session Commands

### `tatl sessions list [<filter>...] [--json]`

List session history.

**Behavior:**
- If filter arguments provided: lists sessions for tasks matching the filter
- If filter omitted: lists all sessions
- Filters sessions by task attributes (project, tags, etc.)
- Supports same filter syntax as `tatl list`

**Session Date Filters:**

Filter sessions by start or end time using `start:` and `end:` prefixes:

| Filter | Description |
|--------|-------------|
| `start:<date>` | Sessions starting on or after date |
| `start:<date>..<date>` | Sessions starting within date range |
| `end:<date>` | Sessions ending on or after date |
| `end:<date>..<date>` | Sessions ending within date range |

Date expressions support:
- Relative dates: `-7d`, `-1w`, `today`, `yesterday`
- Absolute dates: `2024-01-15`, `2024-01-15T14:30`
- Time-only: `09:00`, `14:30`

**Options:**
- `<filter>...` - Filter arguments (e.g., "project=work +urgent")
- `--json` - Output in JSON format

**Examples:**
```bash
# List all sessions
tatl sessions list

# List sessions for specific task
tatl sessions list 10

# Filter by project
tatl sessions list project=work

# Filter by tags
tatl sessions list +urgent

# Multiple filter arguments
tatl sessions list project=work +urgent

# Filter by session start date
tatl sessions list start:today       # Sessions starting today
tatl sessions list start:-7d         # Sessions starting in last 7 days
tatl sessions list start:2024-01-01  # Sessions starting on or after Jan 1

# Filter by session end date
tatl sessions list end:today         # Sessions ending today
tatl sessions list end:-7d           # Sessions ending in last 7 days

# Filter by date range (interval syntax)
tatl sessions list start:2024-01-01..2024-01-31  # Start date range
tatl sessions list end:-7d..-1d                  # End date range
tatl sessions list start:-7d..                   # Starting 7+ days ago (open-ended)
tatl sessions list start:..today                 # Starting up to today

# Combine date filters with task filters
tatl sessions list start:-7d project=work
tatl sessions list start:today +urgent
tatl sessions list end:today project=work +billable

# JSON output
tatl sessions list --json
tatl sessions list project=work --json
```

### `tatl sessions show [--task <id|filter>]`

Show detailed session information.

**Behavior:**
- If `--task` provided: shows most recent session for task or filter
- If `--task` omitted: shows current running session

**Examples:**
```bash
# Show current session
tatl sessions show

# Show most recent session for task
tatl sessions show --task 10
```

### `tatl sessions modify <session_id> <interval> [--yes] [--force]`

Modify session start and/or end times using interval syntax.

**Interval Syntax:**
- `<start>..<end>` - Set both start and end times
- `<start>..` - Set start time only (keep current end)
- `..<end>` - Set end time only (keep current start)

**Options:**
- `--yes` - Apply modification without confirmation
- `--force` - Allow modification even with conflicts (may require manual conflict resolution)

**Overlap Detection:**
- Before applying modifications, checks for conflicts with other sessions
- Reports all conflicting sessions with details
- Prevents modification by default if conflicts exist (use `--force` to override)

**Examples:**
```bash
# Set both start and end
tatl sessions modify 5 09:00..17:00

# Set start time only (keep current end)
tatl sessions modify 5 09:00..

# Set end time only (keep current start)
tatl sessions modify 5 ..17:00

# Modify with confirmation bypass
tatl sessions modify 5 --yes 09:00..17:00

# Force modification despite conflicts
tatl sessions modify 5 --force 10:00..11:00
```

### `tatl sessions delete <session_id> [--yes]`

Delete a session.

**Syntax:** CLAP-native: `tatl sessions delete <session_id> [--yes]`

**Options:**
- `--yes` - Delete without confirmation

**Behavior:**
- Cannot delete running session (must clock out first)
- Annotations linked to session will have their `session_id` set to NULL
- Events referencing the session are preserved

**Examples:**
```bash
# Delete session (with confirmation)
tatl sessions delete 5

# Delete session (without confirmation)
tatl sessions delete 5 --yes
```

**Confirmation Prompt:**
```
Delete session 5?
  Task: 10 (Fix bug in authentication)
  Start: 2024-01-15 09:00:00
  End: 2024-01-15 11:00:00
  Duration: 2h0m0s
  Linked annotations: 2

Are you sure? (y/n):
```

### `tatl sessions report [<start>] [<end>] [<filter>...]`

Generate a time report summarizing hours by project.

**Syntax:**
- Single date: `tatl sessions report -7d` (last 7 days to now)
- Date range: `tatl sessions report 2024-01-01 2024-01-31`
- Interval syntax: `tatl sessions report -7d..now`
- With filter: `tatl sessions report -7d project=work`

**Arguments:**
- `<start>` - Start date for report period (date expression)
- `<end>` - End date for report period (defaults to now)
- `<filter>...` - Optional task filter (same syntax as `tatl list`)

**Behavior:**
- Aggregates session time by project hierarchy
- Shows percentage of total for each project
- Sessions are clipped to report period boundaries
- Filters apply to tasks (only sessions for matching tasks are included)

**Examples:**
```bash
# Last 7 days
tatl sessions report -7d

# Specific date range
tatl sessions report 2024-01-01 2024-01-31

# Using interval syntax
tatl sessions report -7d..now
tatl sessions report 2024-01-01..2024-01-31

# With task filter
tatl sessions report -7d project=work
tatl sessions report -30d +urgent

# Combining all options
tatl sessions report -7d..now project=work +billable
```

---

## Report

### `tatl report [--period <week|month|year>]`

Display a composite report view with queue, sessions, statistics, and attention items.

**Sections:**
1. **Queue** - Current work queue showing top tasks with priorities
2. **Today's Sessions** - Time tracked today with running total
3. **Period Statistics** - Summary stats and project breakdown for selected period
4. **Attention Needed** - Overdue, stalled, and external tasks requiring action

**Options:**
- `--period <period>` - Time period for statistics (default: `week`)
  - `week` - Current week (Monday to now)
  - `month` - Current month
  - `year` - Current year

**Examples:**
```bash
# Show report with this week's statistics
tatl report

# Show report with this month's statistics
tatl report --period=month

# Show report with this year's statistics
tatl report --period=year
```

**Sample Output:**
```
═══════════════════════════════════════════════════════════════════════════
                           TATL DASHBOARD
═══════════════════════════════════════════════════════════════════════════

📋 QUEUE (3 tasks)
───────────────────────────────────────────────────────────────────────────
 Q   ID   Description                              Project    Priority
 ▶  12   Fix auth bug                             work       11.2
 1  15   Review PR                                work        8.5
 2   8   Update docs                              docs        5.1

⏰ TODAY'S SESSIONS (2h 15m)
───────────────────────────────────────────────────────────────────────────
           09:00-10:30 Fix auth bug                   work        1h 30m
           10:45-11:30 Code review                    work           45m
 [current] 11:45-now   Fix auth bug                   work           23m

📊 THIS WEEK
───────────────────────────────────────────────────────────────────────────
 Total time:     12h 30m    │  Tasks completed:  5

 By project:
   work            8h 15m ████████████████░░░░  66%
   home            2h 45m █████░░░░░░░░░░░░░░░  22%
   docs            1h 30m ███░░░░░░░░░░░░░░░░░  12%

⚠️  ATTENTION NEEDED
───────────────────────────────────────────────────────────────────────────
 Overdue (2):     #5 Submit report (3 days), #9 Pay invoice (1 day)
 Stalled (1):     #7 Waiting on feedback
 External (1):    #11 Sent to @manager

═══════════════════════════════════════════════════════════════════════════
```

---

## Respawning Tasks

Tasks with a `respawn` rule automatically create a new instance when completed or closed. This differs from traditional recurrence:

| Recurrence (Traditional) | Respawning (TATL) |
|-------------------------|-------------------|
| Pre-generates many instances | One active instance at a time |
| Missed tasks pile up | Single obligation persists |
| Time-based trigger | Completion-based trigger |
| Fixed due dates | Due dates relative to completion |

### Creating Respawning Tasks

```bash
# Daily task - respawns tomorrow when completed
tatl add "Daily standup" respawn=daily due=09:00

# Weekly task
tatl add "Weekly review" respawn=weekly due=friday

# Specific weekdays
tatl add "Team sync" respawn=mon,wed,fri due=10:00

# Specific days of month
tatl add "Timesheet" respawn=14,30 due=17:00

# Nth weekday of month
tatl add "Monthly board meeting" respawn=2nd-tue due=14:00

# Custom intervals
tatl add "Check-in" respawn=3d
tatl add "Quarterly review" respawn=3m
```

### Respawn Patterns

| Pattern | Example | Description |
|---------|---------|-------------|
| `daily` | `respawn=daily` | Every day |
| `weekly` | `respawn=weekly` | Every 7 days |
| `monthly` | `respawn=monthly` | Same day each month |
| `yearly` | `respawn=yearly` | Same date each year |
| `Nd` | `respawn=3d` | Every N days |
| `Nw` | `respawn=2w` | Every N weeks |
| `Nm` | `respawn=6m` | Every N months |
| `Ny` | `respawn=1y` | Every N years |
| `day,day,...` | `respawn=mon,fri` | Specific weekdays |
| `N,N,...` | `respawn=1,15` | Specific days of month |
| `Nth-day` | `respawn=2nd-tue` | Nth weekday of month |

### Respawn Behavior

When you finish or close a task with a respawn rule:

```bash
tatl finish
# Finished task 5
# ↻ Respawned as task 6, due: 2026-01-23 09:00
```

- **Due date**: Calculated from completion date, not original due date
- **Attributes**: All attributes carried forward (project, tags, allocation)
- **Status**: New instance starts as `pending`
- **Delete**: Deleting a task ends the respawn chain (no new instance)

---

## Filter Syntax

Filters support AND, OR, and NOT operations with implicit AND.

### Filter Terms

- `1` - Task ID
- `status=<status>` - Task status (pending, completed, closed, deleted)
- `project=<name>` - Project name (supports prefix matching for nested projects)
- `+<tag>` - Has tag
- `-<tag>` - Does not have tag
- `due=<expr>` - Due date (any, none, or date expression)
- `due>expr`, `due<expr`, `due>=expr`, `due<=expr`, `due!=expr` - Date comparisons
- `scheduled=<expr>` - Scheduled date
- `wait=<expr>` - Wait date
- `desc=<pattern>` - Description contains pattern (case-insensitive substring match)
- `waiting` - Derived: wait_ts is set and in the future
- `kanban=<status>` - Derived kanban status (proposed, stalled, queued, external, done)

### Operators

- **AND** (implicit): Multiple terms are ANDed together
- **OR** (explicit): Use `or` keyword
- **NOT** (explicit): Use `not` keyword

**Precedence:** `not` > `and` > `or`

### Examples

```bash
# AND (implicit)
tatl list project=work +urgent
tatl list status=pending due=tomorrow

# OR (explicit)
tatl list +urgent or +important
tatl list project=work or project=home

# NOT
tatl list not +waiting
tatl list not project=work

# Complex filters
tatl list project=work +urgent or project=home +important
tatl list status=pending not +waiting
tatl list (project=work or project=home) +urgent  # Note: parentheses not yet supported
```

---

## Date Expressions

Date expressions support absolute dates, relative dates, and time-only expressions.

### Absolute Dates

```bash
2026-01-15
2026-01-15T09:00
2026-01-15T09:00:00
```

### Relative Dates

```bash
today
tomorrow
+1d      # 1 day from now
+2w      # 2 weeks from now
+3m      # 3 months from now
+1y      # 1 year from now
-1d      # 1 day ago
1w       # 1 week from now
1week    # 1 week from now
2weeks   # 2 weeks from now
in 1 week
next week
```

### Time-Only Expressions

Time-only expressions (e.g., `09:00`) resolve to the nearest occurrence:
- If past option is closer: use past
- If future option is closer: use future
- If equally close: use future
- Window: 8 hours past, 16 hours future

```bash
09:00
17:30
```

### End-of-Period Expressions

```bash
eod      # End of day
eow      # End of week
eom      # End of month
```

### Examples

```bash
# Due dates
tatl add Review PR due=tomorrow
tatl add Fix bug due=+2d
tatl add Meeting due:2026-01-15T14:00

# Scheduled dates
tatl add Prepare presentation scheduled="next Monday"
tatl add Follow up scheduled:+1w

# Wait dates
tatl add Start project wait:2026-02-01
```

---

## Duration Format

Durations use unit suffixes: `d` (days), `h` (hours), `m` (minutes), `s` (seconds).

**Format:** `<number><unit>` with units in order: days, hours, minutes, seconds.

**Examples:**
```bash
1h       # 1 hour
2h30m    # 2 hours 30 minutes
1d2h     # 1 day 2 hours
30m      # 30 minutes
45s      # 45 seconds
1h15m30s # 1 hour 15 minutes 30 seconds
```

---

## Troubleshooting

### Common Issues

**Error: Stack is empty**
- Solution: Add a task to the stack first with `task <id> enqueue`

**Error: No session is currently running**
- Solution: Start a session with `task clock in` or `task <id> clock in`

**Error: Task not found**
- Solution: Verify task ID with `tatl list`

**Error: Project not found**
- Solution: Create project with `tatl projects add <name>`

**Error: Filter parse error**
- Solution: Check filter syntax, ensure proper spacing around `or` and `not`

### Database Issues

**Database location:**
- Default: `~/.tatl/ledger.db`
- Override: Create `~/.tatl/rc` with `data.location=/path/to/db`

**Database corruption:**
- Backup database regularly
- If corruption occurs, restore from backup

### Performance

For large datasets (1000+ tasks):
- Use specific filters to limit results
- Indexes are automatically created for common queries
- List operations should complete in < 1 second

---

## Exit Codes

- `0` - Success
- `1` - User error (invalid input, missing resource, etc.)
- `2` - Internal error (database corruption, unexpected failure)

---

## See Also

- `README.md` - Quick start guide
- `design/Plan_01_Build_Team_Handoff_Package.md` - Complete design specification
- `design/Design_Decisions.md` - Implementation decisions

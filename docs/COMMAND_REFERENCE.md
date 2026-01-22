# Tatl Ninja Command Reference

Complete reference for all Tatl commands with examples and usage patterns.

## Table of Contents

- [Task Commands](#task-commands)
- [Project Commands](#project-commands)
- [Timing Commands](#timing-commands)
- [Session Commands](#session-commands)
- [Status Command](#status-command)
- [Respawning Tasks](#respawning-tasks)
- [Filter Syntax](#filter-syntax)
- [Date Expressions](#date-expressions)
- [Duration Format](#duration-format)

---

## Tatl Commands

### `tatl add [--on] [--onoff <start>..<end>] [--enqueue] [-y] <description> [attributes...]`

Add a new task with optional attributes.

**Description:** The task description is the first argument or all non-attribute tokens.

**Options:**
- `--on` - Automatically start timing after creating task (pushes to queue[0] and starts timing)
- `--onoff <start>..<end>` - Create task and add historical session for the specified interval
- `--enqueue` - Automatically enqueue task to queue after creating (adds to end, does not start timing)
- `-y` - Auto-confirm prompts (create new projects, modify overlapping sessions)

**Note:** If `--onoff` is specified, it takes precedence over `--on` and `--enqueue`.

**Attributes:**
- `project:<name>` - Assign to project
- `due:<expr>` - Set due date
- `scheduled:<expr>` - Set scheduled date
- `wait:<expr>` - Set wait date
- `allocation:<duration>` - Set time allocation
- `template:<name>` - Use template
- `respawn:<pattern>` - Set respawn rule (creates new instance on completion)
- `+<tag>` - Add tag
- `uda.<key>:<value>` - Set user-defined attribute

**Examples:**
```bash
# Simple task
tatl add Fix bug in authentication

# Tatl with project and tags
tatl add Review PR project:work +code-review +urgent

# Tatl with due date and allocation
tatl add Write documentation project:docs due:tomorrow allocation:2h

# Tatl with respawn rule (creates new instance when completed)
tatl add "Daily standup" respawn:daily due:09:00

# Tatl with UDA
tatl add Customer call uda.client:acme uda.priority:high

# Task with --on (automatically starts timing)
tatl add --on Start working on feature
tatl add --on "Fix urgent bug" project:work +urgent

# Task with --enqueue (adds to queue without starting timing)
tatl add --enqueue "Review documentation" project:docs
tatl add --enqueue "Write tests" project:work due:tomorrow allocation:2h

# Task with --onoff (create task and add historical session)
tatl add "Emergency meeting" --onoff 14:00..15:00 project:meetings
tatl add "Support request" --onoff 10:30..11:00 +support

# Task with new project (interactive prompt)
tatl add "New feature" project:newproject
# Prompts: "This is a new project 'newproject'. Add new project? [y/n/c] (default: y):"
# - y: Create project and continue (default)
# - n: Skip project, create task without it
# - c: Cancel task creation

# Task with -y (non-interactive)
tatl add -y "New feature" project:newproject
# Automatically creates project if it doesn't exist
```

### `tatl list [filter] [--json] [--relative] [--add-alias <name>]`

List tasks matching optional filter.

**Options:**
- `--json` - Output in JSON format
- `--relative` - Show due dates as relative time
- `--add-alias <name>` - Save current list options as a named view

**Examples:**
```bash
# List all tasks
tatl list

# List with filter
tatl list project:work
tatl list +urgent
tatl list status:pending

# Sort and group
tatl list sort:project,priority group:kanban

# Save a list view alias
tatl list project:work sort:project --add-alias mywork
tatl list mywork

# JSON output
tatl list --json
tatl list project:work +urgent --json
```

### `tatl modify <id|filter> [attributes...] [--yes] [--interactive]`

Modify one or more tasks.

**Options:**
- `--yes` - Apply to all matching tasks without confirmation (also auto-creates new projects if needed)
- `--interactive` - Confirm each task one by one

**Examples:**
```bash
# Modify single task
tatl modify 10 +urgent due:+2d

# Modify multiple tasks (with confirmation)
tatl modify project:work description:Updated description

# Modify with --yes flag
tatl modify +urgent due:+1d --yes

# Modify with new project (prompts to create)
tatl modify 10 project:newproject

# Clear attributes
tatl 10 modify project:none due:none allocation:none
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
tatl close project:work --yes
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

# Show tatl list
tatl show 1,3,5

# Show with filter
tatl show project:work
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

# Push tatl 10 to top and start at specific time
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

# Add session for tatl 10 from 09:00 to 12:00
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

### `tatl sessions list [<filter>...] [--json] [--add-alias <name>]`

List session history.

**Behavior:**
- If filter arguments provided: lists sessions for tasks matching the filter
- If filter omitted: lists all sessions
- Filters sessions by task attributes (project, tags, etc.)
- Supports same filter syntax as `tatl list`
- Special `start:<date>` filter: filters sessions by start date (e.g., `start:today`, `start:-7d`, `start:2026-01-19`)

**Options:**
- `<filter>...` - Filter arguments (e.g., "project:work +urgent")
- `--json` - Output in JSON format
- `--add-alias <name>` - Save current list options as a named view
- `--task <id|filter>` - Legacy flag (backward compatibility, use filter arguments instead)

**Examples:**
```bash
# List all sessions
tatl sessions list

# List sessions for specific task
tatl sessions list 10

# Filter by project
tatl sessions list project:work

# Filter by tags
tatl sessions list +urgent

# Multiple filter arguments
tatl sessions list project:work +urgent

# Filter by date (sessions from specific date or relative date)
tatl sessions list start:today
tatl sessions list start:yesterday
tatl sessions list start:-7d          # Last 7 days
tatl sessions list start:2026-01-19  # Specific date

# Combine task filters with date filters
tatl sessions list project:work start:-7d
tatl sessions list +urgent start:today

# Sort/group
tatl sessions list sort:start group:task

# Save a list view alias
tatl sessions list project:work sort:start --add-alias worksessions
tatl sessions list worksessions

# JSON output
tatl sessions list --json
tatl sessions list project:work --json

# Legacy --task flag (still supported)
tatl sessions list --tatl 10
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
tatl sessions show --tatl 10
```

### `tatl sessions modify <session_id> [start:<expr>] [end:<expr>] [--yes] [--force]`

Modify session start and/or end times.

**Syntax:** CLAP-native: `tatl sessions modify <session_id> [start:<expr>] [end:<expr>]`

**Fields:**
- `start:<expr>` - Modify start time (date expression)
- `end:<expr>` - Modify end time (date expression)
- `end:none` - Clear end time (make session open, only for closed sessions)
- `end:now` - Set end time to current time (close session, only for open sessions)

**Options:**
- `--yes` - Apply modification without confirmation
- `--force` - Allow modification even with conflicts (may require manual conflict resolution)

**Overlap Detection:**
- Before applying modifications, checks for conflicts with other sessions
- Reports all conflicting sessions with details
- Prevents modification by default if conflicts exist (use `--force` to override)

**Behavior:**
- Cannot clear end time of a running session (it's already open)
- Cannot modify running session's end time to `none`
- Can modify running session's start time (but checks for conflicts)

**Examples:**
```bash
# Modify start time
tatl sessions 5 modify start:09:00

# Modify end time
tatl sessions 5 modify end:17:00

# Modify both
tatl sessions 5 modify start:09:00 end:17:00

# Close an open session
tatl sessions 5 modify end:now

# Make a closed session open (clear end time)
tatl sessions 5 modify end:none

# Modify with confirmation bypass
tatl sessions 5 modify start:09:00 --yes

# Force modification despite conflicts
tatl sessions 5 modify start:10:00 --force
```

**Conflict Example:**
```bash
$ tatl sessions 5 modify start:10:00
Error: Session modification would create conflicts:

  Session 5 (Task 10): 2024-01-15 10:00:00 - 2024-01-15 11:00:00
  Conflicts with:
    - Session 3 (Task 8): 2024-01-15 10:00:00 - 2024-01-15 12:00:00

Use --force to override (may require resolving conflicts manually).
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

---

## Status Command

### `tatl status [--json]`

Show dashboard with system status and actionable information.

**Description:** Displays a consolidated view of the current system state, including clock status, clock stack, today's sessions, and overdue tasks.

**Options:**
- `--json` - Output in JSON format

**Sections:**
1. **Clock Status** - Shows whether clocked in/out, current task description, and duration if clocked in
2. **Clock Stack (Top 3)** - Displays the top 3 tasks in the clock stack with full details
3. **Priority Tasks (Top 3)** - Displays the top 3 priority tasks NOT in the clock stack, sorted by urgency score
4. **Today's Sessions** - Summary of sessions today (count and total duration)
5. **Overdue Tasks** - Count of overdue tasks, or next overdue date if none

**Priority Calculation:**
Priority is calculated using a Taskwarrior-style urgency algorithm that considers:
- **Due date proximity**: Tasks due soon or overdue get higher priority
  - Overdue tasks: High urgency (15.0 - days_overdue * 0.5, min 1.0)
  - Due within 7 days: Urgency increases as deadline approaches (12.0 - days_until_due)
  - Due within 30 days: Moderate urgency (5.0 - days_until_due / 10.0)
  - Due far in future: Low urgency (2.0 / (1.0 + days_until_due / 30.0))
- **Allocation remaining**: Tasks with less time remaining get higher priority
  - < 25% remaining: +3.0 urgency
  - < 50% remaining: +1.5 urgency
  - > 50% remaining: +0.5 urgency
- **Task age**: Older tasks get a small boost (+0.1 per 30 days, max +2.0)
- **Status**: Only pending tasks are included in priority calculation

Priority tasks exclude tasks already in the clock stack, as those are already being worked on.

**Examples:**
```bash
# Show dashboard
tatl status

# JSON output
tatl status --json
```

**Output Format:**
```
=== Clock Status ===
Clocked IN on task 1: Fix bug (2h30m)

=== Clock Stack (Top 3) ===
[0] 1: Fix bug project:work +urgent due:2026-01-15 alloc:2h
[1] 2: Review PR project:work +code-review
[2] 3: Write docs project:docs

=== Priority Tasks (Top 3) ===
4: Critical bug fix project:work +urgent due:2026-01-10 (priority: 15.2)
5: Documentation update project:docs due:2026-01-20 (priority: 8.5)
6: Code review project:work +code-review (priority: 1.5)

=== Today's Sessions ===
5 session(s), 4h30m

=== Overdue Tasks ===
2 task(s) overdue
```

**JSON Output:**
The `--json` flag outputs structured data:
```json
{
  "clock": {
    "state": "in",
    "task_id": 1,
    "duration_secs": 9000
  },
  "clock_stack": [
    {
      "position": 0,
      "id": 1,
      "description": "Fix bug",
      "status": "pending",
      "project_id": 1,
      "tags": ["urgent"],
      "due_ts": 1705276800,
      "allocation_secs": 7200
    }
  ],
  "today_sessions": {
    "count": 5,
    "total_duration_secs": 16200
  },
            "overdue": {
                "count": 2,
                "next_overdue_ts": null
            },
            "priority_tasks": [
                {
                    "id": 4,
                    "description": "Critical bug fix",
                    "status": "pending",
                    "project_id": 1,
                    "tags": ["urgent"],
                    "due_ts": 1705276800,
                    "allocation_secs": null,
                    "priority": 15.2
                }
            ]
        }
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
tatl add "Daily standup" respawn:daily due:09:00

# Weekly task
tatl add "Weekly review" respawn:weekly due:friday

# Specific weekdays
tatl add "Team sync" respawn:weekdays:mon,wed,fri due:10:00

# Specific days of month
tatl add "Timesheet" respawn:monthdays:14,30 due:17:00

# Nth weekday of month
tatl add "Monthly board meeting" respawn:nth:2:tue due:14:00

# Custom intervals
tatl add "Check-in" respawn:every:3d
tatl add "Quarterly review" respawn:every:3m
```

### Respawn Patterns

| Pattern | Example | Description |
|---------|---------|-------------|
| `daily` | `respawn:daily` | Every day |
| `weekly` | `respawn:weekly` | Every 7 days |
| `monthly` | `respawn:monthly` | Same day each month |
| `yearly` | `respawn:yearly` | Same date each year |
| `every:Nd` | `respawn:every:3d` | Every N days |
| `every:Nw` | `respawn:every:2w` | Every N weeks |
| `every:Nm` | `respawn:every:6m` | Every N months |
| `every:Ny` | `respawn:every:1y` | Every N years |
| `weekdays:` | `respawn:weekdays:mon,fri` | Specific weekdays |
| `monthdays:` | `respawn:monthdays:1,15` | Specific days of month |
| `nth:N:day` | `respawn:nth:2:tue` | Nth weekday of month |

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
- `status:<status>` - Task status (pending, completed, closed, deleted)
- `project:<name>` - Project name (supports prefix matching for nested projects)
- `+<tag>` - Has tag
- `-<tag>` - Does not have tag
- `due:<expr>` - Due date (any, none, or date expression)
- `scheduled:<expr>` - Scheduled date
- `wait:<expr>` - Wait date
- `waiting` - Derived: wait_ts is set and in the future
- `kanban:<status>` - Derived kanban status (proposed, paused, queued, working, next, live, done)

### Operators

- **AND** (implicit): Multiple terms are ANDed together
- **OR** (explicit): Use `or` keyword
- **NOT** (explicit): Use `not` keyword

**Precedence:** `not` > `and` > `or`

### Abbreviations

Filter tokens allow unambiguous abbreviations:
- `st:pending` → `status:pending`
- `proj:work` → `project:work`
- Ambiguous prefixes error with suggestions.

### Examples

```bash
# AND (implicit)
tatl list project:work +urgent
tatl list status:pending due:tomorrow

# OR (explicit)
tatl list +urgent or +important
tatl list project:work or project:home

# NOT
tatl list not +waiting
tatl list not project:work

# Complex filters
tatl list project:work +urgent or project:home +important
tatl list status:pending not +waiting
tatl list (project:work or project:home) +urgent  # Note: parentheses not yet supported
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
tatl add Review PR due:tomorrow
tatl add Fix bug due:+2d
tatl add Meeting due:2026-01-15T14:00

# Scheduled dates
tatl add Prepare presentation scheduled:next Monday
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

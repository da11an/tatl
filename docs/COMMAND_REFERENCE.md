# Tatl Ninja Command Reference

Complete reference for all Tatl commands with examples and usage patterns.

## Table of Contents

- [Task Commands](#task-commands)
- [Project Commands](#project-commands)
- [Clock Commands](#clock-commands)
- [Session Commands](#session-commands)
- [Status Command](#status-command)
- [Recurrence Commands](#recurrence-commands)
- [Filter Syntax](#filter-syntax)
- [Date Expressions](#date-expressions)
- [Duration Format](#duration-format)

---

## Tatl Commands

### `tatl add [--clock-in] [--enqueue] [--auto-create-project] <description> [attributes...]`

Add a new task with optional attributes.

**Description:** The task description is the first argument or all non-attribute tokens.

**Options:**
- `--clock-in` - Automatically clock in after creating task (pushes to clock[0] and starts timing)
- `--enqueue` - Automatically enqueue task to clock stack after creating (adds to end, does not start timing)
- `--auto-create-project` - Automatically create project if it doesn't exist (non-interactive mode)

**Note:** If both `--clock-in` and `--enqueue` are specified, `--clock-in` takes precedence.

**Attributes:**
- `project:<name>` - Assign to project
- `due:<expr>` - Set due date
- `scheduled:<expr>` - Set scheduled date
- `wait:<expr>` - Set wait date
- `allocation:<duration>` - Set time allocation
- `template:<name>` - Use template
- `recur:<rule>` - Set recurrence rule
- `+<tag>` - Add tag
- `uda.<key>:<value>` - Set user-defined attribute

**Examples:**
```bash
# Simple task
task add Fix bug in authentication

# Tatl with project and tags
task add Review PR project:work +code-review +urgent

# Tatl with due date and allocation
task add Write documentation project:docs due:tomorrow allocation:2h

# Tatl with template and recurrence
task add Daily standup template:meeting recur:daily

# Tatl with UDA
task add Customer call uda.client:acme uda.priority:high

# Tatl with --clock-in (automatically starts timing)
task add --clock-in Start working on feature
task add --clock-in "Fix urgent bug" project:work +urgent

# Tatl with --enqueue (adds to clock stack without starting timing)
task add --enqueue "Review documentation" project:docs
task add --enqueue "Write tests" project:work due:tomorrow allocation:2h

# Tatl with new project (interactive prompt)
task add "New feature" project:newproject
# Prompts: "This is a new project 'newproject'. Add new project? [y/n/c] (default: y):"
# - y: Create project and continue (default)
# - n: Skip project, create task without it
# - c: Cancel task creation

# Tatl with --auto-create-project (non-interactive)
task add --auto-create-project "New feature" project:newproject
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
task list

# List with filter
task list project:work
task list +urgent
task list status:pending

# Sort and group
task list sort:project,priority group:kanban

# Save a list view alias
task list project:work sort:project --add-alias mywork
task list mywork

# JSON output
task list --json
task list project:work +urgent --json
```

### `tatl modify <id|filter> [attributes...] [--yes] [--interactive]`

Modify one or more tasks.

**Options:**
- `--yes` - Apply to all matching tasks without confirmation (also auto-creates new projects if needed)
- `--interactive` - Confirm each task one by one

**Examples:**
```bash
# Modify single task
task modify 10 +urgent due:+2d

# Modify multiple tasks (with confirmation)
task modify project:work description:Updated description

# Modify with --yes flag
task modify +urgent due:+1d --yes

# Modify with new project (prompts to create)
task modify 10 project:newproject

# Clear attributes
task 10 modify project:none due:none allocation:none
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
- `task finish` (without ID/filter) requires clock[0] and a running session
- `task finish <id>` or `task finish <filter>` works even if task is not clocked in
- If a session exists for the task, it will be closed when completing

**Options:**
- `--at <expr>` - End session at specific time (only applies if session exists)
- `--next` - Automatically start next task in clock stack (only if session was closed)
- `--yes` - Complete all matching tasks without confirmation
- `--interactive` - Confirm each task one by one

**Examples:**
```bash
# Complete current task (requires clocked in)
task finish

# Complete specific task (works even if not clocked in)
task finish 10

# Complete with --next
task finish --next

# Complete at specific time
task finish 10 --at 17:00

# Complete multiple tasks
task finish +urgent --yes
```

### `tatl close <id|filter> [--yes] [--interactive]`

Close one or more tasks (sets status to `closed`).

**Notes:**
- `task close <id>` or `task close <filter>` works even if task is not clocked in
- If a session exists for the task, it will be closed when closing

**Options:**
- `--yes` - Close all matching tasks without confirmation
- `--interactive` - Confirm each task one by one

**Examples:**
```bash
# Close a task
task close 10

# Close multiple tasks
task close project:work --yes
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
task annotate 10 Found the bug in auth module

# Annotate current LIVE task
task annotate Investigating flaky tests

# Multiple words in annotation
task annotate 10 This is a longer note with multiple words

# Delete annotation
task annotate 10 --delete 5
```

### `tatl show <id|filter>`

Show detailed summary of task(s).

**Examples:**
```bash
# Show single task
task show 10

# Show task range
task show 1-3

# Show task list
task show 1,3,5

# Show with filter
task show project:work
```

### `tatl delete <id|filter> [--yes] [--interactive]`

Permanently delete task(s).

**Options:**
- `--yes` - Delete all matching tasks without confirmation
- `--interactive` - Confirm each task one by one

**Examples:**
```bash
# Delete single task
task delete 10

# Delete with confirmation
task delete 10 --yes

# Delete multiple tasks
task delete +old --yes
```

---

## Project Commands

### `tatl projects add <name>`

Create a new project.

**Examples:**
```bash
# Simple project
task projects add work

# Nested project (dot notation)
task projects add admin.email
task projects add sales.northamerica.texas
```

### `tatl projects list [--archived]`

List all projects.

**Options:**
- `--archived` - Include archived projects

**Examples:**
```bash
# List active projects
task projects list

# List all projects including archived
task projects list --archived
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
task projects rename temp work

# Merge projects
task projects rename temp work --force
```

### `tatl projects archive <name>`

Archive a project.

**Examples:**
```bash
task projects archive old-project
```

---

## Clock Commands

The clock stack is a queue of tasks. The task at position 0 (clock[0]) is the "active" task. Clock operations (pick, next, drop) affect which task is active. Clock in/out controls timing.

### `tatl clock list`

Display the current clock stack with full task details.

**Options:**
- `--json` - Output in JSON format

**Output:**
- Shows clock stack position, task ID, description, status, project, tags, and due date
- Tasks are sorted by clock stack position (0 = active task)

**Examples:**
```bash
# List clock stack with full details
task clock list

# JSON output
task clock list --json
```

### `tatl clock enqueue <id|id,id,...|range|mixed>`

Add task(s) to end of clock stack (do it later).

**Arguments:**
- `<id>` - Single task ID
- `<id,id,...>` - Comma-separated list of task IDs (enqueued in listed order)
- `<start-end>` - Range of task IDs (e.g., `30-31` expands to 30, 31)
- Mixed syntax - Combine lists and ranges (e.g., `1,3-5,10`)

**Examples:**
```bash
# Enqueue single task
task clock enqueue 10

# Enqueue multiple tasks
task clock enqueue 1,3,5

# Enqueue range
task clock enqueue 30-31

# Enqueue mixed (ranges and individual IDs)
task clock enqueue 1,3-5,10

# Enqueue with spaces
task clock enqueue 1, 3, 5
```

### `tatl clock pick <index>`

Move task at position to top of clock stack.

**Examples:**
```bash
task clock pick 2
```

### `tatl clock next [<n>]`

Move to the next task by n positions (default: 1).

**Behavior:**
- If clock is running: closes current session and starts new one for new clock[0]
- If clock is not running: only reorders clock stack

**Examples:**
```bash
# Move once
task clock next

# Move 2 positions
task clock next 2
```

### `tatl clock drop <index>`

Remove task from clock stack at position.

**Forms:**
- `task stack drop <index>` (canonical form)
- `task stack <index> drop` (alternative syntax, equivalent)

**Examples:**
```bash
# Canonical form
task stack drop 1

# Alternative syntax (equivalent)
task stack 1 drop
```

### `tatl clock clear`

Clear all tasks from clock stack.

**Examples:**
```bash
task clock clear
```

### `tatl clock in [<id>] [<start>|<start..end>]`

Start timing the current clock[0] task, or a specific task.

**Behavior:**
- If `<id>` provided: pushes task to top and starts timing
- If `<id>` omitted: uses clock[0]
- If no time arguments: starts at "now"
- If single time: starts at specified time
- If interval (`start..end`): creates closed session

**Notes:**
- Task ID is always treated as a task ID (not a clock stack position)
- To clock in by position, use: `task clock pick <index> && task clock in`

**Overlap Prevention:**
If another session starts before the end time of a closed interval, the interval's end time is automatically amended.

**Examples:**
```bash
# Start now (uses clock[0])
task clock in

# Start at specific time (uses clock[0])
task clock in 09:00

# Create closed interval (uses clock[0])
task clock in 09:00..11:00
task clock in today..eod

# Push task 5 to top and start timing (new positional syntax)
task clock in 5

# Push task 10 to top and start at specific time
task clock in 10 09:00

# Push task 10 to top and create interval
task clock in 10 09:00..11:00
```

### `tatl clock out [<end>]`

Stop the currently running session.

**Examples:**
```bash
# Stop now
task clock out

# Stop at specific time
task clock out 17:00
```

---

## Session Commands

### `tatl sessions list [<filter>...] [--json] [--add-alias <name>]`

List session history.

**Behavior:**
- If filter arguments provided: lists sessions for tasks matching the filter
- If filter omitted: lists all sessions
- Filters sessions by task attributes (project, tags, etc.)
- Supports same filter syntax as `task list`

**Options:**
- `<filter>...` - Filter arguments (e.g., "project:work +urgent")
- `--json` - Output in JSON format
- `--add-alias <name>` - Save current list options as a named view
- `--task <id|filter>` - Legacy flag (backward compatibility, use filter arguments instead)

**Examples:**
```bash
# List all sessions
task sessions list

# List sessions for specific task
task sessions list 10

# Filter by project
task sessions list project:work

# Filter by tags
task sessions list +urgent

# Multiple filter arguments
task sessions list project:work +urgent

# Sort/group
task sessions list sort:start group:task

# Save a list view alias
task sessions list project:work sort:start --add-alias worksessions
task sessions list worksessions

# JSON output
task sessions list --json
task sessions list project:work --json

# Legacy --task flag (still supported)
task sessions list --task 10
```

### `tatl sessions show [--task <id|filter>]`

Show detailed session information.

**Behavior:**
- If `--task` provided: shows most recent session for task or filter
- If `--task` omitted: shows current running session

**Examples:**
```bash
# Show current session
task sessions show

# Show most recent session for task
task sessions show --task 10
```

### `tatl sessions modify <session_id> [start:<expr>] [end:<expr>] [--yes] [--force]`

Modify session start and/or end times.

**Syntax:** CLAP-native: `task sessions modify <session_id> [start:<expr>] [end:<expr>]`

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
task sessions 5 modify start:09:00

# Modify end time
task sessions 5 modify end:17:00

# Modify both
task sessions 5 modify start:09:00 end:17:00

# Close an open session
task sessions 5 modify end:now

# Make a closed session open (clear end time)
task sessions 5 modify end:none

# Modify with confirmation bypass
task sessions 5 modify start:09:00 --yes

# Force modification despite conflicts
task sessions 5 modify start:10:00 --force
```

**Conflict Example:**
```bash
$ task sessions 5 modify start:10:00
Error: Session modification would create conflicts:

  Session 5 (Task 10): 2024-01-15 10:00:00 - 2024-01-15 11:00:00
  Conflicts with:
    - Session 3 (Task 8): 2024-01-15 10:00:00 - 2024-01-15 12:00:00

Use --force to override (may require resolving conflicts manually).
```

### `tatl sessions delete <session_id> [--yes]`

Delete a session.

**Syntax:** CLAP-native: `task sessions delete <session_id> [--yes]`

**Options:**
- `--yes` - Delete without confirmation

**Behavior:**
- Cannot delete running session (must clock out first)
- Annotations linked to session will have their `session_id` set to NULL
- Events referencing the session are preserved

**Examples:**
```bash
# Delete session (with confirmation)
task sessions delete 5

# Delete session (without confirmation)
task sessions delete 5 --yes
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
task status

# JSON output
task status --json
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

## Recurrence Commands

### `tatl recur run [--until <date_expr>]`

Generate recurring task instances.

**Behavior:**
- Finds all seed tasks (tasks with `recur` field)
- Generates instances up to `--until` date (default: 30 days from now)
- Idempotent: running multiple times produces same results

**Options:**
- `--until <date_expr>` - Generate instances up to this date

**Examples:**
```bash
# Generate instances for next 30 days
task recur run

# Generate instances until specific date
task recur run --until 2026-12-31
task recur run --until +90d
```

### Creating Recurring Tasks

Recurring tasks are created with the `recur` attribute:

```bash
# Daily task
task add Daily standup recur:daily template:meeting

# Weekly task (Mondays)
task add Weekly review recur:weekly byweekday:mon

# Monthly task (1st of month)
task add Monthly report recur:monthly bymonthday:1

# Custom interval
task add Check email recur:every:2h
```

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
task list project:work +urgent
task list status:pending due:tomorrow

# OR (explicit)
task list +urgent or +important
task list project:work or project:home

# NOT
task list not +waiting
task list not project:work

# Complex filters
task list project:work +urgent or project:home +important
task list status:pending not +waiting
task list (project:work or project:home) +urgent  # Note: parentheses not yet supported
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
task add Review PR due:tomorrow
task add Fix bug due:+2d
task add Meeting due:2026-01-15T14:00

# Scheduled dates
task add Prepare presentation scheduled:next Monday
task add Follow up scheduled:+1w

# Wait dates
task add Start project wait:2026-02-01
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
- Solution: Verify task ID with `task list`

**Error: Project not found**
- Solution: Create project with `task projects add <name>`

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

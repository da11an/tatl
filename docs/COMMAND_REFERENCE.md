# Task Ninja Command Reference

Complete reference for all Task Ninja commands with examples and usage patterns.

## Table of Contents

- [Task Commands](#task-commands)
- [Project Commands](#project-commands)
- [Stack Commands](#stack-commands)
- [Clock Commands](#clock-commands)
- [Session Commands](#session-commands)
- [Recurrence Commands](#recurrence-commands)
- [Filter Syntax](#filter-syntax)
- [Date Expressions](#date-expressions)
- [Duration Format](#duration-format)

---

## Task Commands

### `task add <description> [attributes...]`

Add a new task with optional attributes.

**Description:** The task description is the first argument or all non-attribute tokens.

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

# Task with project and tags
task add Review PR project:work +code-review +urgent

# Task with due date and allocation
task add Write documentation project:docs due:tomorrow allocation:2h

# Task with template and recurrence
task add Daily standup template:meeting recur:daily

# Task with UDA
task add Customer call uda.client:acme uda.priority:high
```

### `task list [filter] [--json]`

List tasks matching optional filter.

**Note:** You can also use the filter-before-command pattern: `task <filter> list` (e.g., `task project:work list`)

**Options:**
- `--json` - Output in JSON format

**Examples:**
```bash
# List all tasks
task list

# List with filter
task project:work list
task +urgent list
task status:pending list

# JSON output
task list --json
task project:work +urgent list --json
```

### `task <id|filter> modify [attributes...] [--yes] [--interactive]`

Modify one or more tasks.

**Note:** You can also use: `task modify <id|filter> [attributes...]` (top-level form)

**Options:**
- `--yes` - Apply to all matching tasks without confirmation
- `--interactive` - Confirm each task one by one

**Examples:**
```bash
# Modify single task
task 10 modify +urgent due:+2d

# Modify multiple tasks (with confirmation)
task project:work modify description:Updated description

# Modify with --yes flag
task +urgent modify due:+1d --yes

# Clear attributes
task 10 modify project:none due:none allocation:none
```

### `task [<id|filter>] done [--at <expr>] [--next] [--yes] [--interactive]`

Complete one or more tasks.

**Note:** You can also use: `task done [<id|filter>]` (top-level form)

**Behavior:**
1. Closes running session at `--at` or now
2. Marks task as completed
3. Removes from stack
4. If `--next` and stack non-empty: starts session for new stack[0]

**Options:**
- `--at <expr>` - End session at specific time
- `--next` - Automatically start next task in stack
- `--yes` - Complete all matching tasks without confirmation
- `--interactive` - Confirm each task one by one

**Examples:**
```bash
# Complete current task (if clocked in)
task done

# Complete specific task
task 10 done

# Complete with --next
task done --next

# Complete at specific time
task 10 done --at 17:00

# Complete multiple tasks
task +urgent done --yes
```

### `task [<id>] annotate <note...>`

Add annotation to a task.

**Note:** You can also use: `task annotate <note...>` (top-level form, requires task ID in filter or defaults to stack[0])

**Behavior:**
- If `<id>` provided: annotates specified task
- If `<id>` omitted and clock running: annotates current task and links to session
- If `<id>` omitted and clock not running: error

**Examples:**
```bash
# Annotate specific task
task 10 annotate Found the bug in auth module

# Annotate current task (if clocked in)
task annotate Waiting for API response

# Multiple words in annotation
task 10 annotate This is a longer note with multiple words
```

### `task <id> annotate --delete <annotation_id>`

Delete a specific annotation.

**Examples:**
```bash
task 10 annotate --delete 5
```

---

## Project Commands

### `task projects add <name>`

Create a new project.

**Examples:**
```bash
# Simple project
task projects add work

# Nested project (dot notation)
task projects add admin.email
task projects add sales.northamerica.texas
```

### `task projects list [--archived]`

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

### `task projects rename <old_name> <new_name> [--force]`

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

### `task projects archive <name>`

Archive a project.

**Examples:**
```bash
task projects archive old-project
```

---

## Stack Commands

### `task stack show`

Display the current stack.

**Examples:**
```bash
task stack show
```

### `task stack enqueue <id>` / `task <id> enqueue`

Add task to end of stack (do it later).

**Forms:**
- `task stack enqueue <id>` (canonical form)
- `task <id> enqueue` (syntactic sugar, equivalent)

**Examples:**
```bash
# Canonical form
task stack enqueue 10

# Syntactic sugar (equivalent)
task 10 enqueue
task 11 enqueue
```

### `task stack pick <index>` / `task stack <index> pick`

Move task at position to top of stack.

**Forms:**
- `task stack pick <index>` (canonical form)
- `task stack <index> pick` (alternative syntax, equivalent)

**Examples:**
```bash
# Canonical form
task stack pick 2

# Alternative syntax (equivalent)
task stack 2 pick
```

### `task stack roll [<n>]`

Rotate stack by n positions (default: 1).

**Behavior:**
- If clock is running: closes current session and starts new one for new stack[0]
- If clock is not running: only reorders stack

**Examples:**
```bash
# Rotate once
task stack roll

# Rotate 2 positions
task stack roll 2
```

### `task stack drop <index>` / `task stack <index> drop`

Remove task from stack at position.

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

### `task stack clear [--clock-out]`

Clear all tasks from stack.

**Options:**
- `--clock-out` - Stop clock if running

**Examples:**
```bash
task stack clear
task stack clear --clock-out
```

---

## Clock Commands

### `task clock in [<start>|<start..end>]` / `task <id> clock in [<start>|<start..end>]`

Start timing the current stack[0] task, or push a specific task to top and start timing.

**Forms:**
- `task clock in` (starts timing stack[0])
- `task <id> clock in` (pushes task to top and starts timing)

**Behavior:**
- If no arguments: starts at "now"
- If single time: starts at specified time
- If interval (`start..end`): creates closed session

**Overlap Prevention:**
If another session starts before the end time of a closed interval, the interval's end time is automatically amended.

**Examples:**
```bash
# Start now
task clock in

# Start at specific time
task clock in 09:00

# Create closed interval
task clock in 09:00..11:00
task clock in today..eod

# Push task 5 to top and start timing
task 5 clock in
```

Push task to stack[0] and start timing.

**Behavior:**
- Moves task to top of stack
- Closes existing session if running
- Creates new session (open or closed interval)

**Examples:**
```bash
# Push to top and start now
task 10 clock in

# Push to top and start at specific time
task 10 clock in 09:00

# Push to top and create interval
task 10 clock in 09:00..11:00
```

### `task clock out [<end>]`

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

### `task [<id>] sessions list [--json]` / `task sessions list [--json]`

List session history.

**Note:** You can also use: `task <id> sessions list` or `task <filter> sessions list` to filter by task

**Behavior:**
- If `<id>` provided: lists sessions for specific task
- If `<id>` omitted: lists all sessions

**Options:**
- `--json` - Output in JSON format

**Examples:**
```bash
# List all sessions
task sessions list

# List sessions for specific task
task 10 sessions list

# JSON output
task sessions list --json
```

### `task [<id>] sessions show` / `task sessions show`

Show detailed session information.

**Note:** You can also use: `task <id> sessions show` or `task <filter> sessions show` to filter by task

**Behavior:**
- If `<id>` provided: shows most recent session for task
- If `<id>` omitted: shows current running session

**Examples:**
```bash
# Show current session
task sessions show

# Show most recent session for task
task 10 sessions show
```

---

## Recurrence Commands

### `task recur run [--until <date_expr>]`

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
- `status:<status>` - Task status (pending, completed, deleted)
- `project:<name>` - Project name (supports prefix matching for nested projects)
- `+<tag>` - Has tag
- `-<tag>` - Does not have tag
- `due:<expr>` - Due date (any, none, or date expression)
- `scheduled:<expr>` - Scheduled date
- `wait:<expr>` - Wait date
- `waiting` - Derived: wait_ts is set and in the future

### Operators

- **AND** (implicit): Multiple terms are ANDed together
- **OR** (explicit): Use `or` keyword
- **NOT** (explicit): Use `not` keyword

**Precedence:** `not` > `and` > `or`

### Examples

```bash
# AND (implicit)
task project:work +urgent list
task status:pending due:tomorrow list

# OR (explicit)
task +urgent or +important list
task project:work or project:home list

# NOT
task not +waiting list
task not project:work list

# Complex filters
task project:work +urgent or project:home +important list
task status:pending not +waiting list
task (project:work or project:home) +urgent list  # Note: parentheses not yet supported
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
yesterday
+1d      # 1 day from now
+2w      # 2 weeks from now
+3m      # 3 months from now
+1y      # 1 year from now
-1d      # 1 day ago
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
- Default: `~/.taskninja/tasks.db`
- Override: Create `~/.taskninja/rc` with `data.location=/path/to/db`

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

# TATL - Task and Time Ledger

A command-line task and time tracking tool built with Rust and SQLite. TATL focuses on **doing work, not managing work** - simple semantics for tracking what you're working on and how long you spend on it.

## Philosophy

TATL is designed around a simple insight: **most task management is procrastination in disguise**. Instead of elaborate organizational systems, TATL provides:

- **A work queue**: What's next? Just look at `queue[0]`
- **Start/stop timing**: `tatl on`, `tatl off` - that's it
- **Respawning tasks**: Repeating obligations create new instances only when you complete them
- **Immutable history**: Every change is recorded. No data is ever lost.

## Features

- **Task Management**: Create, modify, close, cancel, clone, reopen, and delete tasks
- **Projects**: Hierarchical project organization (e.g., `work`, `work.email`)
- **Tags**: Flexible tagging with `+tag` / `-tag` syntax
- **Scheduling**: Due dates, scheduled dates, and wait times with natural date expressions
- **Time Tracking**: Simple `on`/`off` timing with break capture (`offon`) and historical sessions (`onoff`)
- **Task Queue**: Work queue semantics - `queue[0]` is always "what's next"
- **Respawning**: Tasks with respawn rules create a new instance when completed
- **Templates**: Standardized task creation via `template=<name>`
- **UDAs**: User-defined attributes for custom task properties
- **Annotations**: Timestamped notes linked to tasks and sessions
- **Filters**: Powerful filter expressions with AND, OR, NOT operators and comparison operators
- **Stages**: Derived task stages (proposed, planned, in progress, active, suspended, external, completed, cancelled) with customizable labels, sort order, and colors
- **Externals**: Send tasks to external parties and track their return
- **Sessions Report**: Time reports with project breakdowns and date range filtering
- **Pipe Operator**: Chain commands with ` : ` (e.g., `tatl add "Task" : enqueue : on`)
- **Command Abbreviations**: Unambiguous prefixes work everywhere (e.g., `enq` for `enqueue`)
- **Immutable History**: Complete audit trail of all task changes via event log

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/da11an/tatl.git
cd tatl

# Build release version
cargo build --release

# Install to ~/.cargo/bin/
cargo install --path .
```

The `tatl` command will be available in your PATH.

### Local Development

```bash
# Build and use directly
cargo build --release
./target/release/tatl list

# Or create an alias
alias tatl='./target/release/tatl'
```

See `INSTALL.md` for detailed installation options.

## Quick Start

```bash
# Add tasks
tatl add "Fix the auth bug" project=work +urgent
tatl add "Review PR" project=work due=tomorrow

# View your tasks
tatl list

# Start working - pick a task and go
tatl on 1           # Push task 1 to queue[0] and start timing
tatl on             # Start timing queue[0]

# Take a break? Capture it
tatl offon 14:30    # I was interrupted at 14:30, resuming now

# Done with the task
tatl close          # Complete queue[0], stop timing

# Log time you forgot to track
tatl onoff 09:00..12:00 2    # Add 3-hour session to task 2

# Create and start timing in one command
tatl add "Quick task" : on          # Create and start timing
tatl add "Meeting" : on 14:00       # Create and start timing at 14:00
tatl add "Past work" : onoff 09:00..12:00  # Create with historical session
```

## Core Concepts

### The Task Queue

The queue is your "currently working on" list. Position 0 is always "what's next":

```bash
tatl list           # Shows queue with positions
tatl on             # Start timing queue[0]
tatl on 5           # Move task 5 to queue[0] and start
tatl enqueue 3      # Add task 3 to bottom of queue
tatl enqueue 1,3,5  # Add multiple tasks
tatl dequeue        # Remove queue[0] from queue
```

### Time Tracking

Simple start/stop semantics:

```bash
tatl on             # Start timing queue[0]
tatl on 5           # Start timing task 5 (moves to queue[0])
tatl off            # Stop timing

# Capture breaks retroactively
tatl offon 14:30              # I left at 14:30, resuming now
tatl offon 14:30..15:00       # 30-minute break

# Log historical sessions
tatl onoff 09:00..12:00       # Log session for queue[0]
tatl onoff 09:00..12:00 5     # Log session for task 5
```

When you start timing a task, TATL shows context to help you get oriented:

```
Started timing task 5: Fix the auth bug (14:30)
  - Check if the token refresh is being called before expiry
  - Look at the retry logic in auth_middleware.rs
  Timer: 2h15m0s / 4h0m0s [====================-----] 56%
```

### Respawning (Not Recurrence)

Traditional recurrence creates multiple task instances upfront. TATL uses **respawning** instead:

- Only one active instance exists at a time
- New instance is created **when you complete** the current one
- Missed deadlines don't pile up as separate tasks
- Next due date is calculated from completion date

```bash
# Create a respawning task
tatl add "Daily standup" respawn=daily due=09:00
tatl add "Weekly review" respawn=weekly due=friday
tatl add "Timesheet" respawn=14,30 due=17:00

# When you close it...
tatl close
# Output:
# Closed task 1: Daily standup
# Respawned as task 2, due: 2026-01-23 09:00

# Respawn patterns:
# respawn=daily              - Every day
# respawn=weekly             - Every week
# respawn=monthly            - Every month
# respawn=yearly             - Every year
# respawn=3d                 - Every 3 days
# respawn=2w                 - Every 2 weeks
# respawn=mon,wed,fri        - Specific weekdays
# respawn=1,15               - Specific days of month
# respawn=2nd-tue            - 2nd Tuesday of month
```

### Task Stages

Tasks have derived stages based on their orthogonal state. Stages are never stored directly - they are computed from a combination of status, queue position, session history, and external state.

| Stage | Condition |
|-------|-----------|
| `proposed` | Open, not in queue, no sessions |
| `planned` | Open, in queue, no sessions |
| `suspended` | Open, not in queue, has past sessions |
| `in progress` | Open, in queue, has sessions |
| `active` | Open, in queue, session running now |
| `external` | Open, has active external |
| `completed` | Closed |
| `cancelled` | Cancelled |

```bash
tatl list stage=planned             # Show planned tasks
tatl list stage=suspended           # Show tasks needing attention
tatl list stage=external            # Show tasks with external parties
tatl list stage=planned,suspended   # Comma-separated OR
```

Stage labels, sort order, and colors are customizable via the `stages` command:

```bash
tatl stages                         # View stage mapping table
tatl stages set 7 "working"        # Rename "in progress" to "working"
tatl stages set 7 color=green      # Change stage color
tatl stages set 7 sort_order=3     # Change sort position
```

## Command Reference

### Tasks

```bash
# Create
tatl add "Description" project=name +tag due=tomorrow
tatl add "Quick task" : on          # Create and start timing
tatl add "Meeting" : on 14:00       # Create and start timing at 14:00
tatl add "Past work" : onoff 09:00..12:00  # Create with historical session

# Read
tatl list                           # All open tasks
tatl list project=work +urgent      # With filters
tatl show 5                         # Detailed view
tatl show                           # Show currently active task

# Update
tatl modify 5 +urgent due=+2d       # Add tag, change due date
tatl modify project=work            # Modify currently active task
tatl annotate 5 "Found the issue"   # Add note
tatl clone 5                        # Clone task with all attributes
tatl clone 5 project=other +new     # Clone with overrides

# Complete
tatl close                          # Close queue[0], stop timing
tatl close 5                        # Close specific task
tatl cancel 5                       # Cancel (intent shifted)
tatl reopen 5                       # Reopen a closed/cancelled task
tatl delete 5                       # Permanently delete
```

### Time Tracking

```bash
tatl on                     # Start timing queue[0]
tatl on 5                   # Start timing task 5
tatl on 09:00               # Start at specific time
tatl off                    # Stop timing
tatl off 17:00              # Stop at specific time

# Break capture
tatl offon 14:30            # Was interrupted at 14:30, resuming now
tatl offon 14:30..15:00     # Capture break period

# Historical sessions
tatl onoff 09:00..12:00     # Add session for queue[0]
tatl onoff 09:00..12:00 5   # Add session for task 5
```

### Queue Management

```bash
tatl list                   # View queue (Q column shows position)
tatl enqueue 5              # Add task to queue
tatl enqueue 1,3,5          # Add multiple tasks
tatl dequeue                # Remove queue[0] from queue
tatl dequeue 5              # Remove specific task
```

### Projects

```bash
tatl projects add work
tatl projects add work.email        # Nested project
tatl projects list
tatl projects rename old new
tatl projects archive old-project
tatl projects unarchive old-project
tatl projects report                # Task counts by project and stage
```

### Externals

Send tasks to external parties (colleagues, supervisors, release windows):

```bash
tatl send 5 colleague "Please review this PR"
tatl send 3 Release_5.2             # Send to release window
tatl externals                       # List all external tasks
tatl externals colleague            # Filter by recipient
tatl collect 5                       # Collect task back
```

### Sessions

```bash
tatl sessions list                  # All sessions
tatl sessions list project=work     # With task filter
tatl sessions list -7d              # Sessions from last 7 days
tatl sessions list start:-7d        # Sessions starting on/after 7 days ago
tatl sessions list start:today      # Sessions starting today
tatl sessions list end:today        # Sessions ending today
tatl sessions list start:2024-01-01..2024-01-31  # Start date range
tatl sessions list end:-7d..-1d     # End date range
tatl sessions list start:-7d project=work  # Combine date and task filters
tatl sessions modify 5 09:00..17:00  # Adjust both times
tatl sessions modify 5 ..17:00      # Adjust end time only
tatl sessions modify 5 09:00..      # Adjust start time only
tatl sessions delete 5 -y           # Delete session
tatl sessions report -7d            # Time report for last 7 days
tatl sessions report -7d..now project=work  # Report with filter
```

### Report

```bash
tatl report                    # Show dashboard with this week's stats
tatl report --period=week      # Same as above (default)
tatl report --period=month     # Show this month's stats
tatl report --period=year      # Show this year's stats
```

The report shows:
- Current work queue with priorities
- Today's sessions and time tracked
- Period statistics with project breakdown
- Tasks needing attention (overdue, stalled, external)

### Pipe Operator

Chain commands using ` : ` (space-colon-space). The first command produces a task ID, subsequent commands inherit it:

```bash
tatl add "Task" : enqueue : on      # Create, queue, start timing
tatl add "Done" : close             # Create already closed
tatl add "Meeting" : onoff 14:00..15:00 : close  # Historical + close
tatl add "Clone source" : clone     # Create and clone
```

Supported pipe commands: `on`, `off`, `onoff`, `enqueue`, `dequeue`, `close`, `cancel`, `annotate`, `send`, `collect`, `clone`.

### Command Abbreviations

Unambiguous prefixes are expanded automatically:

```bash
tatl l                  # → tatl list
tatl mod 5 +urgent      # → tatl modify 5 +urgent
tatl ann 5 "Note"       # → tatl annotate 5 "Note"
tatl enq 5              # → tatl enqueue 5
tatl add "Task" : enq   # Abbreviations work in pipes too
```

## Filter Syntax

```bash
# Equality filters
tatl list project=work +urgent
tatl list status=open

# Comparison operators (for dates and numeric fields)
tatl list due>tomorrow           # Tasks due after tomorrow
tatl list due<=eod               # Tasks due by end of day
tatl list due!=none              # Tasks that have a due date
tatl list activity>-7d           # Active in the last 7 days
tatl list modified>2026-01-01    # Modified after a date
tatl list created>-30d           # Created in the last 30 days

# Stage filter
tatl list stage=planned
tatl list stage=suspended,external   # Comma-separated OR

# OR (explicit)
tatl list +urgent or +important

# NOT
tatl list not +waiting

# Description search
tatl list desc=meeting           # Tasks with "meeting" in description
tatl list desc="code review"     # Phrase search

# External filter
tatl list external=colleague     # Tasks sent to specific recipient

# Complex combinations
tatl list project=work status=open not +blocked
```

### Filter Fields

| Field | Operators | Example |
|-------|-----------|---------|
| `id` | `=`, `!=`, `>`, `<`, `>=`, `<=` | `id>10` |
| `status` | `=`, `!=` | `status=open` |
| `stage` | `=`, `!=` | `stage=active` |
| `project` | `=`, `!=` | `project=work` |
| `due` | `=`, `!=`, `>`, `<`, `>=`, `<=` | `due<=eod` |
| `scheduled` | `=`, `!=`, `>`, `<`, `>=`, `<=` | `scheduled>tomorrow` |
| `wait` | `=`, `!=`, `>`, `<`, `>=`, `<=` | `wait=none` |
| `created` | `=`, `!=`, `>`, `<`, `>=`, `<=` | `created>-7d` |
| `modified` | `=`, `!=`, `>`, `<`, `>=`, `<=` | `modified>-1d` |
| `activity` | `=`, `!=`, `>`, `<`, `>=`, `<=` | `activity>-7d` |
| `desc` | `=`, `!=` | `desc=meeting` |
| `external` | `=`, `!=` | `external=bob` |
| `+tag` / `-tag` | presence/absence | `+urgent` |

Date fields support `any` and `none` with `=`/`!=` (e.g., `due=any`, `due=none`).

### Display Options

```bash
# Sort by column
tatl list sort:project
tatl list sort:-priority    # Descending

# Group by column
tatl list group:project
tatl list group:stage

# Hide columns
tatl list hide:tags
tatl list hide:status,stage

# Color output (text color by column value)
tatl list color:project     # Hash-based colors per project
tatl list color:stage       # Semantic colors for stages
tatl list color:priority    # Gradient (green to red)

# Fill output (background color by column value)
tatl list fill:status       # Semantic colors for status
tatl list fill:project      # Hash-based colors per project

# Combine display options
tatl list group:project color:project    # Colored group headers
tatl list sort:priority color:stage      # Sorted with colored rows

# Output formats
tatl list --json            # JSON output
tatl list --relative        # Relative timestamps
tatl list --full            # Show all columns
```

**Note:** Colors only appear in terminal (TTY) output. Piped output has no ANSI codes.

## Configuration

TATL stores data in `~/.tatl/`:

```
~/.tatl/
├── ledger.db    # SQLite database (all data)
└── rc           # Configuration file (optional)
```

### Configuration File

Create `~/.tatl/rc` to customize:

```
# Custom database location
data.location=/path/to/my/tasks.db
```

## Database

All data is stored in a single SQLite database (`~/.tatl/ledger.db`):

- **Tasks**: Core task data and metadata
- **Sessions**: Time tracking entries
- **Events**: Immutable audit log of all changes
- **Projects**: Project hierarchy
- **Annotations**: Timestamped notes
- **Externals**: Tasks sent to external parties
- **Stage Map**: Customizable stage derivation rules

The database is created automatically on first use and migrations are applied automatically on upgrade (currently at schema version 10).

## Development

```bash
# Build
cargo build

# Test
cargo test

# Run with debug output
RUST_LOG=debug cargo run -- list

# Format and lint
cargo fmt
cargo clippy
```

### Design Documentation

The `design/` directory contains implementation plans and design decisions for each feature iteration.

## License

MIT

# TATL - Task and Time Ledger

A command-line task and time tracking tool built with Rust and SQLite. TATL focuses on **doing work, not managing work** - simple semantics for tracking what you're working on and how long you spend on it.

## Philosophy

TATL is designed around a simple insight: **most task management is procrastination in disguise**. Instead of elaborate organizational systems, TATL provides:

- **A work queue**: What's next? Just look at `queue[0]`
- **Start/stop timing**: `tatl on`, `tatl off` - that's it
- **Respawning tasks**: Repeating obligations create new instances only when you complete them
- **Immutable history**: Every change is recorded. No data is ever lost.

## Features

### Implemented

- **Task Management**: Create, modify, list, complete, close, and delete tasks
- **Projects**: Hierarchical project organization (e.g., `work`, `work.email`)
- **Tags**: Flexible tagging with `+tag` / `-tag` syntax
- **Scheduling**: Due dates, scheduled dates, and wait times with natural date expressions
- **Time Tracking**: Simple `on`/`off` timing with break capture (`offon`) and historical sessions (`onoff`)
- **Task Queue**: Work queue semantics - `queue[0]` is always "what's next"
- **Respawning**: Tasks with respawn rules create a new instance when completed
- **UDAs**: User-defined attributes for custom task properties
- **Annotations**: Timestamped notes linked to tasks and sessions
- **Filters**: Powerful filter expressions with AND, OR, NOT operators
- **Kanban Status**: Derived statuses (proposed, stalled, queued, external, done)
- **Externals**: Send tasks to external parties and track their return
- **Immutable History**: Complete audit trail of all task changes via event log

### Potential Future Work

- Templates for standardized task creation
- Time reports and analytics
- Import/export functionality
- Sync between devices

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
tatl add "Fix the auth bug" project:work +urgent
tatl add "Review PR" project:work due:tomorrow

# View your tasks
tatl list

# Start working - pick a task and go
tatl on 1           # Push task 1 to queue[0] and start timing
tatl on             # Start timing queue[0]

# Take a break? Capture it
tatl offon 14:30    # I was interrupted at 14:30, resuming now

# Done with the task
tatl finish         # Complete queue[0], stop timing

# Log time you forgot to track
tatl onoff 09:00..12:00 2    # Add 3-hour session to task 2
```

## Core Concepts

### The Task Queue

The queue is your "currently working on" list. Position 0 is always "what's next":

```bash
tatl list           # Shows queue with positions
tatl on             # Start timing queue[0]
tatl on 5           # Move task 5 to queue[0] and start
tatl enqueue 3      # Add task 3 to bottom of queue
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

### Respawning (Not Recurrence)

Traditional recurrence creates multiple task instances upfront. TATL uses **respawning** instead:

- Only one active instance exists at a time
- New instance is created **when you complete** the current one
- Missed deadlines don't pile up as separate tasks
- Next due date is calculated from completion date

```bash
# Create a respawning task
tatl add "Daily standup" respawn:daily due:09:00
tatl add "Weekly review" respawn:weekly due:friday
tatl add "Timesheet" respawn:monthdays:14,30 due:17:00

# When you finish it...
tatl finish
# Output:
# Finished task 1
# ↻ Respawned as task 2, due: 2026-01-23 09:00

# Respawn patterns:
# respawn:daily              - Every day
# respawn:weekly             - Every week
# respawn:monthly            - Every month
# respawn:every:3d           - Every 3 days
# respawn:weekdays:mon,wed,fri  - Specific weekdays
# respawn:monthdays:1,15     - Specific days of month
# respawn:nth:2:tue          - 2nd Tuesday of month
```

### Kanban Status

Tasks have derived Kanban statuses based on their state:

| Status | Meaning |
|--------|---------|
| `proposed` | Not in queue, no work done yet |
| `stalled` | Has sessions but not currently in queue |
| `queued` | In queue (Q column shows position: 0, 1, 2... or ▶ if timing) |
| `external` | Sent to external party for review/approval |
| `done` | Completed or closed |

```bash
tatl list kanban:queued      # Show queued tasks
tatl list kanban:stalled     # Show tasks needing attention
tatl list kanban:external    # Show tasks with external parties
```

## Command Reference

### Tasks

```bash
# Create
tatl add "Description" project:name +tag due:tomorrow
tatl add "Quick task" --on          # Create and start timing
tatl add "Meeting" --on=14:00       # Create and start timing at 14:00
tatl add "Past work" --onoff 09:00..12:00  # Create with historical session

# Read
tatl list                           # All pending tasks
tatl list project:work +urgent      # With filters
tatl show 5                         # Detailed view

# Update
tatl modify 5 +urgent due:+2d       # Add tag, change due date
tatl annotate 5 "Found the issue"   # Add note

# Complete
tatl finish                         # Complete queue[0]
tatl finish 5                       # Complete specific task
tatl close 5                        # Close without completing
tatl reopen 5                       # Reopen a closed task
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
tatl list                   # View queue
tatl enqueue 5              # Add task to queue
tatl enqueue 1,3,5          # Add multiple tasks
tatl dequeue                # Remove queue[0]
tatl dequeue 5              # Remove specific task
```

### Projects

```bash
tatl projects add work
tatl projects add work.email        # Nested project
tatl projects list
tatl projects rename old new
tatl projects archive old-project
tatl projects unarchive old-project # Restore archived project
tatl projects report                # Task counts by project and status
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
tatl sessions list project:work     # With task filter
tatl sessions list -7d              # Sessions from last 7 days
tatl sessions list -7d..now         # Sessions from date interval
tatl sessions list -7d project:work # Combine filters
tatl sessions modify 5 start:09:00..end:17:00  # Adjust both times
tatl sessions modify 5 end:17:00    # Adjust end time only
tatl sessions modify 5 start:09:00  # Adjust start time only
tatl sessions delete 5 -y           # Delete session
tatl sessions report -7d            # Time report for last 7 days
tatl sessions report -7d..now project:work  # Report with filter
```

## Filter Syntax

```bash
# AND (implicit)
tatl list project:work +urgent

# OR (explicit)
tatl list +urgent or +important

# NOT
tatl list not +waiting

# Description search
tatl list desc:meeting           # Tasks with "meeting" in description
tatl list desc:"code review"     # Phrase search

# Complex
tatl list project:work status:pending not +blocked

# Kanban status
tatl list kanban:queued
tatl list kanban:stalled
tatl list kanban:external
```

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

The database is created automatically on first use and migrations are applied automatically on upgrade.

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

The `design/` directory contains implementation plans and decisions:

- `Plan_21_Rename_to_Tatl.md` - Migration from task-ninja to tatl
- `Plan_22_CLI_Syntax_Review.md` - CLI design decisions
- `Plan_23_Break_Capture_Workflow.md` - offon/onoff implementation
- `Plan_24_Respawn_Model.md` - Respawn vs recurrence

## Troubleshooting

### Common Errors

| Error | Solution |
|-------|----------|
| "Queue is empty" | Add a task with `tatl enqueue <id>` |
| "No session running" | Start with `tatl on` or `tatl on <id>` |
| "Task not found" | Check ID with `tatl list` |
| "Project not found" | Create with `tatl projects add <name>` |

### Database

- **Location**: `~/.tatl/ledger.db`
- **Override**: Set `data.location` in `~/.tatl/rc`
- **Backup**: Copy the `.db` file periodically

## License

MIT

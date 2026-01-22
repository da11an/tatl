# TATL - Task and Time Ledger

A powerful command-line task and time tracking tool built with Rust and SQLite - your ledger for tasks and time.

## Features

- **Task Management**: Add, modify, list, complete, and annotate tasks
- **Projects**: Organize tasks with hierarchical projects (e.g., `work`, `work.email`)
- **Tags**: Flexible tagging system with `+tag` / `-tag` syntax
- **Scheduling**: Due dates, scheduled dates, and wait times
- **Time Tracking**: Simple `on`/`off` timing with break capture (`offon`) and historical sessions (`onoff`)
- **Task Queue**: Work queue with "do it now" vs "do it later" semantics
- **Recurrence**: Recurring tasks with templates and flexible recurrence rules
- **UDAs**: User-defined attributes for custom task properties
- **Annotations**: Timestamped notes linked to tasks and sessions
- **Immutable History**: Complete audit trail of all task changes

## Installation

### From Source (Rust)

```bash
# Clone the repository
git clone <repository-url>
cd tatl

# Build release version
cargo build --release

# Install to ~/.cargo/bin/ (or $CARGO_HOME/bin)
cargo install --path .
```

The `tatl` command will be available in your PATH (typically `~/.cargo/bin/tatl`).

**Note:** If you have Taskwarrior installed, it also uses the `tatl` command. You can:
- Use the full path: `~/.cargo/bin/tatl`
- Create an alias: `alias tatl='~/.cargo/bin/tatl'`
- Add `~/.cargo/bin` to the beginning of your PATH to prioritize this installation
- Rename the binary by modifying `Cargo.toml` if you prefer a different name

### Local Development Installation

For local testing without installing globally:

```bash
# Build in release mode
cargo build --release

# Use directly
./target/release/tatl list

# Or create an alias in your current shell
alias tatl='./target/release/tatl'
tatl list
```

See `INSTALL.md` for more detailed installation options and conflict resolution with Taskwarrior.

## Quick Start

```bash
# Add a new task
tatl add fix the bug project:work +urgent

# List tasks
tatl list
tatl list project:work
tatl list +urgent

# Start working on a task
tatl on 10      # Push task 10 to top and start timing
tatl on         # Start timing queue[0]

# Add task to queue (do it later)
tatl enqueue 11

# Add annotation while working
tatl annotate 10 Found the issue in auth module

# Stop timing and capture a break
tatl off                    # Stop now
tatl offon 14:30            # Interrupted at 14:30, resume now
tatl offon 14:30..15:00     # 30 min break, resume at 15:00

# Add historical session
tatl onoff 09:00..12:00     # Add session for queue[0]
tatl onoff 09:00..12:00 10  # Add session for task 10

# Complete a task
tatl finish     # Finish queue[0]
tatl finish 10  # Finish specific task

# View session history
tatl sessions list
```

## Database

The database is stored at `~/.tatl/ledger.db` by default. You can override this location by creating a configuration file at `~/.tatl/rc`:

```
data.location=/path/to/your/tasks.db
```

The database is created automatically on first use.

## Command Examples

### Tasks

```bash
# Add task with project, tags, and due date
tatl add Review PR project:work +code due:tomorrow

# Add task and start timing immediately
tatl add --on "Urgent fix" project:work +urgent

# Add task with historical session
tatl add "Yesterday's meeting" --onoff 09:00..10:00 project:meetings

# Modify task
tatl modify 10 +urgent due:+2d

# List with filters
tatl list project:work +urgent
tatl list +urgent or +important
tatl list not +waiting

# Annotate task
tatl annotate 10 Started investigation

# Complete task
tatl finish     # Finish queue[0]
tatl finish 10  # Finish specific task
```

### Projects

```bash
# Create projects
tatl projects add work
tatl projects add admin.email  # Nested project

# List projects
tatl projects list
tatl projects list --archived

# Rename project
tatl projects rename work office

# Archive project
tatl projects archive old-project
```

### Time Tracking

```bash
# Start timing
tatl on           # Start queue[0]
tatl on 10        # Push task 10 to top and start
tatl on 09:00     # Start at specific time

# Stop timing
tatl off          # Stop now
tatl off 17:00    # Stop at specific time

# Break capture (stop and resume)
tatl offon 14:30              # Interrupted at 14:30, resume now
tatl offon 14:30..15:00       # 30 min break

# Add historical session
tatl onoff 09:00..12:00       # Add session for queue[0]
tatl onoff 09:00..12:00 10    # Add session for task 10

# Insert session into existing time (splits overlapping sessions)
tatl onoff 14:00..15:00 5 -y  # Insert meeting for task 5

# Modify history (remove time from sessions)
tatl offon 14:30..15:00 -y    # Remove this interval from overlapping sessions
```

### Task Queue

```bash
# View queue
tatl list

# Add to queue
tatl enqueue 10
tatl enqueue 1,3,5    # Multiple tasks

# Remove from queue
tatl dequeue          # Remove queue[0]
tatl dequeue 5        # Remove specific task
```

### Sessions

```bash
# List all sessions
tatl sessions list

# List sessions with filter
tatl sessions list project:work

# Modify session times
tatl sessions modify 5 start:09:00 end:17:00

# Delete a session
tatl sessions delete 5 -y
```

### Recurrence

```bash
# Generate recurring task instances
tatl recur run
tatl recur run --until +30d

# Recurrence rules
tatl add Daily standup recur:daily template:meeting
tatl add Weekly review recur:weekly byweekday:mon
tatl add Monthly report recur:monthly bymonthday:1
```

## Filter Syntax

Filters support AND, OR, and NOT operations:

```bash
# AND (implicit)
tatl list project:work +urgent

# OR (explicit)
tatl list +urgent or +important

# NOT
tatl list not +waiting

# Complex filters
tatl list project:work +urgent or project:home +important
tatl list status:pending not +waiting
```

## Configuration

Create `~/.tatl/rc` to customize behavior:

```
data.location=/custom/path/to/tasks.db
```

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run with debug output
RUST_LOG=debug cargo run -- <command>

# Format code
cargo fmt

# Lint
cargo clippy
```

## Design Documentation

See the `design/` directory for complete specifications:

- `Plan_01_Build_Team_Handoff_Package.md` - Complete design specification
- `Review_01_Design_Issues_and_Recommendations.md` - Design review and resolved issues
- `Design_Decisions.md` - Implementation decisions log

## Command Reference

See `docs/COMMAND_REFERENCE.md` for complete command documentation with examples.

## Troubleshooting

### Common Issues

**Error: Queue is empty**
- Solution: Add a task to the queue first with `tatl enqueue <id>`

**Error: No session is currently running**
- Solution: Start a session with `tatl on` or `tatl on <id>`

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

## License

MIT

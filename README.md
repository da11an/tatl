# TATL - Task and Time Ledger

A powerful command-line task and time tracking tool built with Rust and SQLite - your ledger for tasks and time.

## Features

- **Task Management**: Add, modify, list, complete, and annotate tasks
- **Projects**: Organize tasks with hierarchical projects (e.g., `work`, `work.email`)
- **Tags**: Flexible tagging system with `+tag` / `-tag` syntax
- **Scheduling**: Due dates, scheduled dates, and wait times
- **Time Tracking**: Built-in clock with session tracking
- **Clock Stack**: Work queue with "do it now" vs "do it later" semantics
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
./target/release/tatl clock list

# Or create an alias in your current shell
alias tn='./target/release/tatl'
tn clock list
```

See `INSTALL.md` for more detailed installation options and conflict resolution with Taskwarrior.

## Quick Start

```bash
# Add a new task
task add fix the bug project:work +urgent

# List tasks
task list
task list project:work
task list +urgent

# Start working on a task (do it now)
task clock in --task 10  # or: task clock in (uses clock[0])

# Add task to queue (do it later)
task clock enqueue 11

# Add annotation while working
task annotate 10 Found the issue in auth module

# Complete a task
task done  # or: task done 10

# View session history
task sessions list
task sessions list --task 10
task sessions show --task 10
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
task add Review PR project:work +code due:tomorrow

# Modify task
task modify 10 +urgent due:+2d

# List with filters
task list project:work +urgent
task list +urgent or +important
task list not +waiting

# Annotate task
task annotate 10 Started investigation

# Complete task
task done  # Completes current task (if clocked in)
task done 10
```

### Projects

```bash
# Create projects
task projects add work
task projects add admin.email  # Nested project

# List projects
task projects list
task projects list --archived

# Rename project
task projects rename work office

# Archive project
task projects archive old-project
```

### Clock Stack

```bash
# View clock stack
task clock list  # Shows clock stack with full task details

# Do it now: push to top and start clock
task clock in --task 10  # or: task clock in (uses clock[0])

# Do it later: add to end of queue
task clock enqueue 11

# Manage clock stack
task clock pick 2    # Move position 2 to top
task clock roll      # Rotate once
task clock drop 1    # Remove from position 1
task clock clear     # Clear all

# Clock operations
task clock in                    # Start current clock[0]
task clock in 09:00..11:00      # Create closed interval (uses clock[0])
task clock in --task 10 09:00..11:00  # Create closed interval for specific task
task clock out                   # Stop current session
```

### Sessions

```bash
# List all sessions
task sessions list

# List sessions for specific task
task sessions list --task 10

# Show current session
task sessions show

# Show most recent session for task
task sessions show --task 10

# Modify session start/end times
task sessions modify 5 start:09:00
task sessions modify 5 end:17:00
task sessions modify 5 start:09:00 end:17:00

# Close an open session
task sessions modify 5 end:now

# Make a closed session open (clear end time)
task sessions modify 5 end:none

# Delete a session
task sessions delete 5
task sessions delete 5 --yes
```
```

### Recurrence

```bash
# Generate recurring task instances
task recur run
task recur run --until +30d

# Recurrence rules
task add Daily standup recur:daily template:meeting
task add Weekly review recur:weekly byweekday:mon
task add Monthly report recur:monthly bymonthday:1
```

## Filter Syntax

Filters support AND, OR, and NOT operations:

```bash
# AND (implicit)
task list project:work +urgent

# OR (explicit)
task list +urgent or +important

# NOT
task list not +waiting

# Complex filters
task list project:work +urgent or project:home +important
task list status:pending not +waiting
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

## License

MIT

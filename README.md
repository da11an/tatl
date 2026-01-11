# Task Ninja 🥷

A powerful command-line task management tool built with Rust and SQLite - your stealthy companion for getting things done.

## Features

- **Task Management**: Add, modify, list, complete, and annotate tasks
- **Projects**: Organize tasks with hierarchical projects (e.g., `work`, `work.email`)
- **Tags**: Flexible tagging system with `+tag` / `-tag` syntax
- **Scheduling**: Due dates, scheduled dates, and wait times
- **Time Tracking**: Built-in clock with session tracking
- **Stack Management**: Work queue with "do it now" vs "do it later" semantics
- **Recurrence**: Recurring tasks with templates and flexible recurrence rules
- **UDAs**: User-defined attributes for custom task properties
- **Annotations**: Timestamped notes linked to tasks and sessions
- **Immutable History**: Complete audit trail of all task changes

## Installation

### From Source (Rust)

```bash
# Clone the repository
git clone <repository-url>
cd task-ninja

# Build release version
cargo build --release

# Install to ~/.cargo/bin/ (or $CARGO_HOME/bin)
cargo install --path .
```

The `task` command will be available in your PATH (typically `~/.cargo/bin/task`).

**Note:** If you have Taskwarrior installed, it also uses the `task` command. You can:
- Use the full path: `~/.cargo/bin/task`
- Create an alias: `alias task-ninja='~/.cargo/bin/task'`
- Add `~/.cargo/bin` to the beginning of your PATH to prioritize this installation
- Rename the binary by modifying `Cargo.toml` if you prefer a different name

### Local Development Installation

For local testing without installing globally:

```bash
# Build in release mode
cargo build --release

# Use directly
./target/release/task stack show

# Or create an alias in your current shell
alias tn='./target/release/task'
tn stack show
```

See `INSTALL.md` for more detailed installation options and conflict resolution with Taskwarrior.

## Quick Start

```bash
# Add a new task
task add fix the bug project:work +urgent

# List tasks
task list
task project:work list
task +urgent list

# Start working on a task (do it now)
task 10 clock in

# Add task to queue (do it later)
task 11 enqueue

# Add annotation while working
task annotate Found the issue in auth module

# Complete a task
task done

# View session history
task sessions list
task 10 sessions show
```

## Database

The database is stored at `~/.taskninja/tasks.db` by default. You can override this location by creating a configuration file at `~/.taskninja/rc`:

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
task 10 modify +urgent due:+2d

# List with filters
task project:work +urgent list
task +urgent or +important list
task not +waiting list

# Annotate task
task 10 annotate Started investigation
task annotate  # If clocked in, annotates current task

# Complete task
task done  # Completes current task (if clocked in)
task 10 done
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

### Stack and Clock

```bash
# View stack
task stack show

# Do it now: push to top and start clock
task 10 clock in

# Do it later: add to end of queue
task 11 enqueue

# Manage stack
task stack 2 pick    # Move position 2 to top
task stack roll      # Rotate once
task stack 1 drop    # Remove from position 1
task stack clear     # Clear all

# Clock operations
task clock in                    # Start current stack[0]
task clock in 09:00..11:00      # Create closed interval
task clock out                   # Stop current session
```

### Sessions

```bash
# List all sessions
task sessions list

# List sessions for specific task
task 10 sessions list

# Show current session
task sessions show

# Show most recent session for task
task 10 sessions show
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
task project:work +urgent list

# OR (explicit)
task +urgent or +important list

# NOT
task not +waiting list

# Complex filters
task project:work +urgent or project:home +important list
task status:pending not +waiting list
```

## Configuration

Create `~/.taskninja/rc` to customize behavior:

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

## License

MIT

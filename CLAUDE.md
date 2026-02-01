# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build

# Test
cargo test               # Run all tests
cargo test <test_name>   # Run specific test by name
cargo test --test <file> # Run tests in specific file (e.g., cargo test --test respawn_tests)
RUST_LOG=debug cargo test <test_name> -- --nocapture  # Run with debug output

# Lint and format
cargo fmt                # Format code
cargo clippy             # Run linter

# Run (two binaries: tatl and generate-man)
cargo run --bin tatl -- <args>      # Run with arguments (e.g., cargo run --bin tatl -- list)
RUST_LOG=debug cargo run --bin tatl -- <args>  # Run with debug logging
```

## Architecture Overview

TATL is a CLI task/time tracking tool built with Rust and SQLite (rusqlite). The core philosophy is "doing work, not managing work" - simple semantics focused on execution.

### Module Structure

```
src/
├── cli/           # Command parsing and execution
│   ├── commands.rs         # Main command dispatch (~4500 lines, clap derive)
│   ├── commands_sessions.rs # Session subcommands (sessions list/modify/delete/report)
│   ├── parser.rs           # Task argument parsing (extracts project=, +tag, due=, etc.)
│   └── output.rs           # Table/JSON output formatting, stage derivation, colors
├── models/        # Data structures (Task, Project, Session, Stack, Annotation, External)
├── repo/          # Database access layer - one repo per model (static methods on &Connection)
├── db/            # Connection management and migrations (currently v9)
├── filter/        # Filter expression parser (boolean algebra) and evaluator
├── respawn/       # Respawn rule parser and next-date generator
└── utils/         # Date/duration parsing, fuzzy matching
```

### Key Concepts

- **Work Queue**: Tasks are managed via a queue (stack). `queue[0]` is always "what's next". Commands: `enqueue`, `dequeue`, `on` (moves to queue[0] and starts timing).

- **Time Tracking**: `on`/`off` commands with break capture (`offon 14:30` = interrupted at 14:30, resuming now) and historical sessions (`onoff 09:00..12:00`). At most one open session globally (enforced by unique index).

- **Pipe Operator**: Space-colon-space (` : `) chains commands on the same task. `add "Task" : on : annotate "Note"`. First segment produces a task ID, subsequent segments inherit it. Supported piped commands defined in `execute_piped_command()`.

- **Respawning**: Unlike recurrence, respawning creates a new task instance only when the current one is completed. Only one active instance at a time. Patterns: `daily`, `weekly`, `monthdays:1,15`, `every:3d`, etc.

- **Externals**: Tasks can be sent to external parties (`send`) and collected back (`collect`). Active externals affect stage derivation.

- **Immutable Events**: All task changes are recorded in `task_events` for audit trail.

### Orthogonal State Model

**Status** (lifecycle, stored in DB): `open`, `closed`, `cancelled`, `deleted`

**Stage** (derived, never stored): Computed from orthogonal facts in precedence order:

| Priority | Condition | Stage |
|----------|-----------|-------|
| 1 | status == closed | `completed` |
| 2 | status == cancelled | `cancelled` |
| 3 | open session for this task | `active` |
| 4 | active external exists | `external` |
| 5 | in queue + has sessions | `in progress` |
| 6 | in queue + no sessions | `planned` |
| 7 | not in queue + has sessions | `suspended` |
| 8 | not in queue + no sessions | `proposed` |

Stage derivation is backed by the `stage_map` SQLite table (migration v9), which maps
(status, in_queue, has_sessions, has_open_session, has_externals) → stage label + sort_order + color.
Users can customize stage names, sort order, and colors via `tatl stages set`.

Two code paths perform the lookup:
- `cli/output.rs::calculate_stage_status()` — uses pre-loaded `StageMapping` cache for bulk display
- `filter/evaluator.rs::calculate_task_stage()` — uses `StageRepo::lookup()` for filter evaluation

### CLI Commands

Commands use clap derive (`Commands` enum in commands.rs). Key lifecycle commands:
- `close` — intent fulfilled (status → closed, triggers respawn)
- `cancel` — intent shifted (status → cancelled, triggers respawn)
- `reopen` — return to open status

Target selection supports single IDs, ranges (`1-5`), comma-separated (`1,3,5`), and filter expressions.

Field arguments parsed by `cli/parser.rs`: `project=`, `due=`, `scheduled=`, `wait=`, `allocation=`, `template=`, `respawn=`, `+tag`, `-tag`, `uda.<key>=`.

### Filter System

Full boolean expression parser (`filter/parser.rs`):
- Implicit AND between terms: `project=work +urgent`
- Explicit OR: `+urgent or +important`
- NOT: `not +waiting`
- Comparison operators: `=`, `!=`, `>`, `<`, `>=`, `<=`
- Fields: `id`, `status`, `stage`, `project`, `due`, `scheduled`, `wait`, `desc`, `external`
- Tags: `+tag` (has), `-tag` (doesn't have)

### Data Flow

1. CLI parses command via clap → `cli/commands.rs`
2. Pipe operator checked: `split_on_pipe()` splits on ` : ` tokens
3. Commands use repos (`repo/*.rs`) to read/write data
4. Repos interact with SQLite via `db/connection.rs`
5. Complex queries use filter engine (`filter/parser.rs` + `filter/evaluator.rs`)

### Testing

Tests use two patterns:

1. **AcceptanceTestContext** from `tests/acceptance_framework.rs`:
   - Creates temp directory with isolated database
   - Sets `HOME` env var to temp directory
   - Provides Given/When/Then builder pattern for BDD-style tests

2. **Direct assert_cmd** pattern:
   - `setup_test_env()` creates temp dir, writes `.tatl/rc` config
   - `get_task_cmd()` returns Command with HOME set
   - Uses `test_env::lock_test_env()` mutex to serialize tests

To run a single test with output visible:
```bash
cargo test test_name -- --nocapture
```

### Configuration

Minimal: `~/.tatl/rc` file with `data.location=<path>` as the only current option. List view aliases (filter + sort + group + hide + color + fill) are stored in the `list_views` SQLite table.

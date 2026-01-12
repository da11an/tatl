## CLAP-Native Grammar Redesign - Planning Document

This document outlines the redesign of Task Ninja's CLI grammar to be fully CLAP-native, eliminating pre-clap parsing complexity (except for abbreviation expansion).

**Key Decisions:**
- **No backward compatibility** - breaking changes are acceptable
- **Keep abbreviation expansion** - minimal pre-clap parsing for abbreviations only
- **Fully CLAP-native** - all other parsing handled by CLAP
- **Implicit defaults optional** - `task 1` → `task show 1` as optional extension
- **Unify terminology** - replace "stack" with "clock" (the stack is the clock stack)

---

### Current State Analysis

#### Pre-Clap Parsing Patterns

The current implementation has **extensive pre-clap parsing** to support Taskwarrior-style syntax:

1. **Filter-Before-Verb Patterns:**
   - `task <id|filter> done`
   - `task <id|filter> delete`
   - `task <id|filter> modify`
   - `task <id|filter> annotate`
   - `task <id|filter> summary`
   - `task <filter> list`

2. **Resource-Before-Action Patterns:**
   - `task <id> clock in`
   - `task <id> enqueue`
   - `task <id|filter> sessions list/show`
   - `task sessions <session_id> modify/delete`

3. **Alternative Syntax Patterns:**
   - `task stack <index> pick/drop` (alternative to `task stack pick <index>`)
   - `task <id>` (defaults to summary, implicit command)

4. **Implicit Commands:**
   - `task <id>` → `task <id> summary`

#### Current Pre-Clap Parsing Complexity

- **~250 lines** of pattern matching code in `run()`
- **Order-dependent** parsing (must check sessions before delete, etc.)
- **Fragile** - adding new patterns requires careful ordering
- **Hard to test** - pre-clap logic is intertwined with command execution
- **Maintenance burden** - each new pattern adds complexity

#### What CLAP Already Handles Well

- Top-level commands: `add`, `list`, `modify`, `delete`, `done`, `annotate`
- Subcommands: `projects`, `clock`, `recur`, `sessions`
- Flags: `--yes`, `--json`, `--help`, etc.
- Argument parsing and validation
- Help generation
- Error messages

#### Terminology Change: Stack → Clock

**Current:** Separate "stack" and "clock" concepts
- Stack: queue of tasks (stack[0] is active)
- Clock: timing operations (in/out)

**Proposed:** Unify under "clock" terminology
- The stack is the clock stack - it's just the queue of tasks being timed
- All stack operations become clock operations
- Simplifies conceptual model: clock = queue + timing

---

### Proposed CLAP-Native Grammar

#### Design Principles

1. **Verb-First**: Commands follow `task <verb> [resource] [arguments]` pattern
2. **Explicit Resources**: Resources (tasks, sessions) are explicit arguments, not implicit filters
3. **CLAP Subcommands**: Use CLAP's subcommand system for all command grouping
4. **Fully CLAP-Native**: All parsing handled by CLAP (except abbreviation expansion)
5. **No Backward Compatibility**: Breaking changes are acceptable
6. **Unified Clock Model**: Stack and clock are unified under "clock" terminology

#### Grammar Patterns

##### 1. Task Commands (Verb-First)

**Current (Taskwarrior-style):**
```bash
task 1 modify +urgent
task project:work list
task +urgent done
```

**Proposed (CLAP-native):**
```bash
task modify 1 +urgent
task list --filter project:work
task done --filter +urgent
# OR
task done 1  # Single ID is positional
task list project:work  # Filter as positional arg
```

**CLAP Structure:**
```rust
Commands::Modify {
    target: String,  // ID or filter (positional)
    args: Vec<String>,  // Modification args
    yes: bool,
    interactive: bool,
}

Commands::List {
    filter: Option<String>,  // Optional positional filter
    json: bool,
}

Commands::Done {
    target: Option<String>,  // Optional ID or filter
    at: Option<String>,
    next: bool,
    yes: bool,
    interactive: bool,
}
```

##### 2. Task Resource Commands (Explicit Resource)

**Current:**
```bash
task 1 clock in
task 1 enqueue
task 1 sessions list
```

**Proposed:**
```bash
task clock in --task 1
task clock enqueue 1  # Unified clock command
task sessions list --task 1
```

**CLAP Structure:**
```rust
Commands::Clock {
    #[command(subcommand)]
    subcommand: ClockCommands,
    /// Task ID (optional, defaults to clock[0] for in/out)
    #[arg(long)]
    task: Option<String>,
}

// Unified clock commands (replaces both stack and clock)
#[derive(Subcommand)]
pub enum ClockCommands {
    /// Show current clock stack
    Show {
        #[arg(long)]
        json: bool,
    },
    /// Add task to end of clock stack
    Enqueue {
        task_id: i64,
    },
    /// Move task at position to top
    Pick {
        index: i32,
    },
    /// Rotate clock stack
    Roll {
        #[arg(default_value = "1")]
        n: i32,
    },
    /// Remove task at position
    Drop {
        index: i32,
    },
    /// Clear all tasks from clock stack
    Clear,
    /// Start timing (optionally with task)
    In {
        /// Task ID (optional, defaults to clock[0])
        #[arg(long)]
        task: Option<i64>,
    },
    /// Stop timing
    Out,
}

Commands::Sessions {
    #[command(subcommand)]
    subcommand: SessionsCommands,
    /// Task ID or filter (optional)
    #[arg(long)]
    task: Option<String>,
}
```

##### 3. Session Commands (Resource-First with Subcommands)

**Current:**
```bash
task sessions 5 modify start:09:00
task sessions 5 delete
```

**Proposed:**
```bash
task sessions modify 5 start:09:00
task sessions delete 5
# OR
task session 5 modify start:09:00
task session 5 delete
```

**CLAP Structure:**
```rust
SessionsCommands {
    Modify {
        session_id: i64,  // Positional
        args: Vec<String>,
        yes: bool,
        force: bool,
    },
    Delete {
        session_id: i64,  // Positional
        yes: bool,
    },
}
```

##### 4. Clock Commands (Unified Stack + Clock)

**Current:**
```bash
task stack show
task stack enqueue 1
task stack pick 2
task stack roll
task stack drop 1
task stack clear
task clock in
task clock out
```

**Proposed:**
```bash
task clock show          # Show clock stack
task clock enqueue 1     # Add to end
task clock pick 2        # Move to top
task clock roll          # Rotate
task clock drop 1        # Remove
task clock clear         # Clear all
task clock in            # Start timing (uses clock[0])
task clock in --task 1   # Start timing with specific task
task clock out           # Stop timing
```

**Benefits:**
- Unified terminology (no separate "stack" concept)
- All clock-related operations in one place
- Clearer mental model: clock = queue + timing

##### 5. Default Behavior (Optional Extension)

**Current:**
```bash
task 1  # Implicitly shows summary
```

**Proposed (Primary):**
```bash
task show 1      # Explicit (required)
task summary 1   # Explicit (required)
```

**Proposed (Optional Extension):**
If we want to support implicit defaults, handle via minimal pre-clap:
```rust
// After abbreviation expansion, before CLAP parsing
if args.len() == 1 {
    if parse_task_id_spec(&args[0]).is_ok() || validate_task_id(&args[0]).is_ok() {
        // Prepend "show" to make it explicit
        args.insert(0, "show".to_string());
    }
}
```

**Recommendation:** Start with explicit commands, add implicit defaults as optional extension if needed.

---

### Migration Strategy

#### Approach: Full CLAP-Native (Breaking Changes Acceptable)

**Decision:** No backward compatibility needed - breaking changes are acceptable.

**Implementation:**
1. Redesign entire command structure to be CLAP-native
2. Keep abbreviation expansion (minimal pre-clap, ~20 lines)
3. Optional: Add implicit defaults as extension (minimal pre-clap, ~10 lines)
4. Remove all other pre-clap parsing (~250 lines → ~30 lines)

**What Stays (Pre-Clap):**
- Abbreviation expansion (required)
- Optional: Implicit defaults `task 1` → `task show 1` (optional extension)

**What Goes (CLAP-Native):**
- All filter-before-verb patterns
- All resource-before-action patterns
- All alternative syntax patterns
- All manual flag parsing

**Benefits:**
- Eliminates ~220 lines of pre-clap parsing
- Cleaner, more maintainable code
- Better help generation
- Easier to extend
- Unified clock terminology

**Timeline:**
- Major version bump (v2.0)
- Migration guide for users
- Clear breaking changes documentation

---

### Detailed CLAP-Native Grammar Design

#### Command Structure

```rust
#[derive(Parser)]
#[command(name = "task")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    // Task Management
    Add {
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    List {
        /// Filter expression (optional)
        filter: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Show {
        /// Task ID, ID range, or filter
        target: String,
    },
    Modify {
        /// Task ID or filter
        target: String,
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        interactive: bool,
    },
    Done {
        /// Task ID or filter (optional, defaults to current task)
        target: Option<String>,
        #[arg(long)]
        at: Option<String>,
        #[arg(long)]
        next: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        interactive: bool,
    },
    Delete {
        /// Task ID or filter
        target: String,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        interactive: bool,
    },
    Annotate {
        /// Task ID or filter
        target: String,
        #[arg(trailing_var_arg = true)]
        note: Vec<String>,
        #[arg(long)]
        delete: Option<String>,
    },
    
    // Resource Management
    Projects {
        #[command(subcommand)]
        subcommand: ProjectCommands,
    },
    /// Clock management commands
    /// The clock stack is a queue of tasks. The task at position 0 (clock[0]) is the "active" task.
    /// Clock operations (pick, roll, drop) affect which task is active. Clock in/out controls timing.
    Clock {
        #[command(subcommand)]
        subcommand: ClockCommands,
        /// Task ID (optional, defaults to clock[0] for in/out)
        #[arg(long)]
        task: Option<String>,
    },
    Sessions {
        #[command(subcommand)]
        subcommand: SessionsCommands,
        /// Task ID or filter (optional)
        #[arg(long)]
        task: Option<String>,
    },
    Recur {
        #[command(subcommand)]
        subcommand: RecurCommands,
    },
}

// Unified Clock Commands (replaces both StackCommands and ClockCommands)
#[derive(Subcommand)]
pub enum ClockCommands {
    /// Show current clock stack
    Show {
        #[arg(long)]
        json: bool,
    },
    /// Add task to end of clock stack
    Enqueue {
        task_id: i64,
    },
    /// Move task at position to top
    Pick {
        index: i32,
    },
    /// Rotate clock stack
    Roll {
        #[arg(default_value = "1")]
        n: i32,
    },
    /// Remove task at position
    Drop {
        index: i32,
    },
    /// Clear all tasks from clock stack
    Clear,
    /// Start timing (optionally with task)
    In {
        /// Task ID (optional, defaults to clock[0])
        #[arg(long)]
        task: Option<i64>,
    },
    /// Stop timing
    Out,
}
```

#### Example Commands

**Task Operations:**
```bash
# Current → Proposed
task 1 modify +urgent          → task modify 1 +urgent
task project:work list         → task list project:work
task +urgent done              → task done --filter +urgent
task 1 delete                  → task delete 1
task 1 annotate "note"         → task annotate 1 "note"
task 1                         → task show 1
```

**Clock Operations (Unified):**
```bash
# Current → Proposed
task stack show                → task clock show
task stack enqueue 1           → task clock enqueue 1
task stack pick 2              → task clock pick 2
task stack roll                → task clock roll
task stack drop 1              → task clock drop 1
task stack clear               → task clock clear
task 1 clock in                → task clock in --task 1
task clock in                  → task clock in  # Uses clock[0]
task clock out                 → task clock out  # No change
```

**Session Operations:**
```bash
# Current → Proposed
task sessions 5 modify start:09:00  → task sessions modify 5 start:09:00
task sessions 5 delete              → task sessions delete 5
task 1 sessions list                → task sessions list --task 1
task sessions list                  → task sessions list  # No change
```

---

### Implementation Plan

#### Phase 1: Analysis and Design
- [x] Document all current pre-clap patterns
- [x] Design CLAP-native equivalents
- [x] Create migration examples
- [x] Unify stack/clock terminology

#### Phase 2: Implementation
- [ ] Redesign CLAP command structure
- [ ] Merge stack commands into clock commands
- [ ] Update all command handlers
- [ ] Keep abbreviation expansion (minimal pre-clap)
- [ ] Optional: Add implicit defaults extension

#### Phase 3: Testing
- [ ] Update all integration tests
- [ ] Test new CLAP-native syntax
- [ ] Verify abbreviation expansion still works
- [ ] Test implicit defaults (if implemented)

#### Phase 4: Documentation
- [ ] Update command reference
- [ ] Create migration guide (old → new syntax)
- [ ] Update README examples
- [ ] Document breaking changes

---

### Trade-offs and Considerations

#### Advantages of CLAP-Native Grammar

1. **Maintainability:**
   - All parsing in one place (CLAP)
   - No order-dependent pattern matching
   - Easier to add new commands
   - Better error messages from CLAP

2. **Extensibility:**
   - Adding new commands is straightforward
   - CLAP handles argument validation
   - Automatic help generation
   - Better IDE support

3. **Consistency:**
   - All commands follow same pattern
   - Predictable syntax
   - Easier to learn

4. **Testing:**
   - CLAP parsing is easier to test
   - Less custom parsing logic
   - Better error handling

#### Disadvantages

1. **Breaking Changes:**
   - Users must learn new syntax
   - Scripts need updating
   - Muscle memory adjustment
   - **Acceptable** - breaking changes are planned

2. **Verbosity:**
   - Some commands become longer (`task modify 1` vs `task 1 modify`)
   - Less "natural" for Taskwarrior users
   - **Trade-off** - clearer, more maintainable code

3. **Migration Effort:**
   - Documentation updates
   - User education
   - Migration guide needed
   - **One-time cost** - long-term maintenance benefit

---

### Alternative: Enhanced CLAP with Custom Parsers

Instead of eliminating pre-clap, we could use CLAP's custom parsing features:

#### Using CLAP's `ValueParser` for Filters

```rust
Commands::Modify {
    #[arg(value_parser = parse_task_target)]
    target: String,  // Custom parser handles IDs and filters
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

// Custom parser that handles both IDs and filters
fn parse_task_target(s: &str) -> Result<TaskTarget, String> {
    if let Ok(id) = s.parse::<i64>() {
        Ok(TaskTarget::Id(id))
    } else {
        // Try parsing as filter
        parse_filter(vec![s.to_string()])
            .map(TaskTarget::Filter)
            .map_err(|e| format!("Invalid task target: {}", e))
    }
}
```

#### Using CLAP's Default Subcommand for Implicit Commands

CLAP v4 supports default subcommands, which could handle `task 1` → `task show 1`:

```rust
#[derive(Subcommand)]
pub enum Commands {
    // ... other commands ...
    
    // Default subcommand - handles task <id> without explicit command
    #[command(subcommand)]
    #[command(default_value = "show")]
    Default(DefaultCommands),
}

#[derive(Subcommand)]
pub enum DefaultCommands {
    Show {
        #[arg(value_parser = parse_task_target)]
        target: String,
    },
}
```

However, this requires the first argument to be a subcommand. A better approach:

#### Using CLAP's `ArgAction` and Custom Parsing

```rust
#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    
    // Handle implicit: task 1 → task show 1
    // If no subcommand and first arg looks like task ID, treat as show
    #[arg(value_parser = parse_implicit_target)]
    pub implicit_target: Option<String>,
}

fn parse_implicit_target(s: &str) -> Result<String, String> {
    // Validate it's a task ID or filter
    if parse_task_id_spec(s).is_ok() || validate_task_id(s).is_ok() {
        Ok(s.to_string())
    } else {
        Err("Not a valid task target".to_string())
    }
}
```

#### Using CLAP's `flatten` for Shared Arguments

For commands that share common flags:

```rust
#[derive(Args)]
pub struct CommonFlags {
    #[arg(long)]
    yes: bool,
    #[arg(long)]
    interactive: bool,
}

Commands::Modify {
    #[command(flatten)]
    flags: CommonFlags,
    target: String,
    args: Vec<String>,
}
```

#### Using CLAP's `conflicts_with` for Mutually Exclusive Options

```rust
Commands::Annotate {
    target: String,
    #[arg(trailing_var_arg = true, conflicts_with = "delete")]
    note: Vec<String>,
    #[arg(long, conflicts_with = "note")]
    delete: Option<String>,
}
```

#### Using CLAP's `requires` for Dependent Arguments

```rust
Commands::Sessions {
    #[command(subcommand)]
    subcommand: SessionsCommands,
    #[arg(long, requires = "subcommand")]
    task: Option<String>,
}
```

#### Using CLAP's `default_value_t` for Defaults

```rust
Commands::Done {
    target: Option<String>,
    #[arg(long, default_value = "now")]
    at: String,
}
```

#### Using CLAP's `value_enum` for Restricted Values

```rust
#[derive(ValueEnum, Clone)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
}

Commands::List {
    filter: Option<String>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    format: OutputFormat,
}
```

---

### Recommendations

#### Approach: Full CLAP-Native (No Backward Compatibility)

1. **Redesign command structure** to be verb-first
2. **Keep abbreviation expansion** (minimal pre-clap, ~20 lines)
3. **Unify stack/clock terminology** (all become clock commands)
4. **Optional: Implicit defaults** as extension (~10 lines pre-clap)
5. **Remove all other pre-clap parsing** (~220 lines eliminated)

#### Syntax Migration Examples

**Core Commands:**
```bash
# Old → New
task 1 modify +urgent          → task modify 1 +urgent
task project:work list         → task list project:work
task +urgent done              → task done --filter +urgent
task 1 delete                  → task delete 1
task 1 annotate "note"         → task annotate 1 "note"
task 1                         → task show 1  # (or implicit extension)
```

**Clock Commands (Unified):**
```bash
# Old → New
task stack show                → task clock show
task stack enqueue 1           → task clock enqueue 1
task stack pick 2              → task clock pick 2
task stack roll                → task clock roll
task stack drop 1              → task clock drop 1
task stack clear               → task clock clear
task 1 clock in                → task clock in --task 1
task clock in                  → task clock in  # Uses clock[0]
```

**Session Commands:**
```bash
# Old → New
task sessions 5 modify start:09:00  → task sessions modify 5 start:09:00
task sessions 5 delete              → task sessions delete 5
task 1 sessions list                → task sessions list --task 1
```

#### Implementation Priority

1. **High Priority:**
   - Redesign core commands (modify, delete, done, list, show)
   - Unify stack/clock commands
   - Make sessions commands CLAP-native
   - Update help documentation

2. **Medium Priority:**
   - Clock commands with --task flag
   - Filter parsing improvements
   - Update all command handlers

3. **Low Priority:**
   - Abbreviation expansion (keep as-is, already minimal)
   - Implicit defaults (optional extension)

---

### Decisions Made

1. **Backward Compatibility:**
   - ✅ **No backward compatibility** - breaking changes are acceptable
   - ✅ Provide migration guide
   - ✅ Clear documentation of breaking changes

2. **Filter Syntax:**
   - ✅ Keep current filter syntax as positional argument
   - ✅ `task list project:work` (filter as positional)
   - ✅ `task done --filter +urgent` (filter as flag, for clarity)

3. **Implicit Commands:**
   - ✅ **Optional extension** - `task 1` → `task show 1`
   - ✅ Implement as minimal pre-clap (~10 lines)
   - ✅ Can be disabled if not needed

4. **Resource Commands:**
   - ✅ `task clock in --task 1` (CLAP-native with flag)
   - ✅ More intuitive and CLAP-native
   - ✅ Consistent with other resource commands

5. **Abbreviations:**
   - ✅ Keep current abbreviation system (minimal pre-clap)
   - ✅ Already well-tested and working
   - ✅ ~20 lines of code, acceptable

6. **Terminology:**
   - ✅ **Unify stack/clock** - all become clock commands
   - ✅ Stack is just the clock stack
   - ✅ Simplifies conceptual model

---

### Next Steps

1. **Review and Feedback:**
   - Get user input on proposed syntax
   - Discuss trade-offs
   - Decide on migration strategy

2. **Prototype:**
   - Implement CLAP-native structure
   - Test with real commands
   - Measure complexity reduction

3. **Documentation:**
   - Create migration guide
   - Update command reference
   - Add syntax comparison

4. **Implementation:**
   - Phase 1: Core commands
   - Phase 2: Resource commands
   - Phase 3: Cleanup

---

## Summary

A CLAP-native grammar will:
- **Eliminate ~220 lines** of pre-clap parsing (keep ~30 lines for abbreviations + optional implicit defaults)
- **Improve maintainability** by centralizing parsing in CLAP
- **Enhance extensibility** by making new commands easier to add
- **Unify terminology** by merging stack/clock into unified clock commands
- **Require breaking changes** (acceptable - no backward compatibility needed)
- **Improve consistency** across all commands

**Approach:** Full CLAP-native with minimal pre-clap:
- Keep abbreviation expansion (~20 lines)
- Optional: Implicit defaults extension (~10 lines)
- Remove all other pre-clap parsing (~220 lines eliminated)

**Terminology Change:** Stack → Clock
- All stack operations become clock operations
- Unified conceptual model: clock = queue + timing
- Simpler mental model for users

---

## Appendix: CLAP Feature Reference

### Key CLAP Features for This Redesign

1. **`#[arg(value_parser)]`**: Custom parsing for task IDs and filters
2. **`#[arg(trailing_var_arg = true)]`**: Handle variable-length argument lists
3. **`#[command(flatten)]`**: Share common flags across commands
4. **`#[arg(conflicts_with)]`**: Enforce mutually exclusive options
5. **`#[arg(requires)]`**: Enforce dependent arguments
6. **`#[arg(default_value_t)]`**: Provide sensible defaults
7. **`#[derive(ValueEnum)]`**: Restrict values to enum variants
8. **`#[command(subcommand_required = false)]`**: Optional subcommands
9. **`#[arg(allow_hyphen_values = true)]`**: Allow negative numbers in filters

### Code Complexity Comparison

**Current (Pre-Clap):**
- ~250 lines of pattern matching
- Order-dependent checks
- Manual flag parsing
- Custom error handling
- Hard to test

**Proposed (CLAP-Native):**
- ~50 lines of CLAP definitions
- Order-independent
- Automatic flag parsing
- CLAP error handling
- Easy to test

**Estimated Reduction:** ~80% less parsing code

## Enqueue Help Documentation - Planning Document

This document scopes where the `enqueue` command should be documented, given its functional role in the Task Ninja CLI.

---

### Current State

**Enqueue is NOT in Clap Help:**
- `task --help` does not show `enqueue` as a command
- This is because `enqueue` is handled via **pre-clap parsing**, not as a Clap subcommand

**Enqueue IS in Documentation:**
- `docs/COMMAND_REFERENCE.md` - Has a section for `task <id> enqueue` (line 241)
- `README.md` - Mentions enqueue in examples (line 75)

**Similar Commands:**
- `task <id> modify` - **IS** in Clap help (as top-level "modify" command)
- `task <id> done` - **IS** in Clap help (as top-level "done" command)
- `task <id> delete` - **IS** in Clap help (as top-level "delete" command)
- `task <id> annotate` - **IS** in Clap help (as top-level "annotate" command)
- `task <id> summary` - **IS** in Clap help (as top-level "summary" command)
- `task <id> enqueue` - **NOT** in Clap help ✗

**Why the Discrepancy:**
- `modify`, `done`, `delete`, `annotate`, `summary` are **both**:
  1. Top-level Clap commands (can be used as `task modify`, `task done`, etc.)
  2. Task subcommands (can be used as `task <id> modify`, `task <id> done`, etc.)
- `enqueue` is **only** a task subcommand (no top-level usage: `task enqueue` doesn't make sense)

---

### Where Enqueue Functions

**Functional Role:**
- `enqueue` is a **stack operation** - it adds a task to the end of the work queue
- Pattern: `task <id> enqueue` (or `task <id> enq` with abbreviation)
- It's part of the stack management workflow, not a standalone task operation

**Related Commands:**
- `task stack show` - View the stack
- `task stack pick <index>` - Move task to top
- `task stack roll <n>` - Rotate stack
- `task stack drop <index>` - Remove from stack
- `task <id> clock in` - Push task to top and start timing (alternative to enqueue)

**Stack Workflow:**
1. `task <id> enqueue` - Add task to end of queue (do it later)
2. `task <id> clock in` - Push task to top and start timing (do it now)
3. `task stack show` - View queue
4. `task stack roll` - Rotate to next task
5. `task done` - Complete current task

---

### Documentation Locations

#### 1. **Clap Help System** (`task --help`)

**Current State:** Enqueue is NOT shown

**Selected Options:**
- **Option B: Add to Stack Commands Help** ✓
  - Add `Enqueue` variant to `StackCommands` enum
  - This makes `task stack enqueue <id>` a valid command
  - Will show in `task stack --help` automatically
  - **Rationale:** This is the canonical form; `task <id> enqueue` is syntactic sugar

- **Option C: Add Note in Stack Help** ✓
  - Add documentation text to `Stack` command description mentioning `task <id> enqueue`
  - Example: "Use `task <id> enqueue` (or `task stack enqueue <id>`) to add tasks to the stack"
  - **Rationale:** Documents the syntactic sugar form for discoverability

- **Option D: Create "Task Subcommands" Section** ✓
  - Add a new help section that lists task subcommands
  - Could be shown when `task --help` is called
  - Would include: `enqueue`, `modify`, `done`, `delete`, `annotate`, `summary`
  - **Rationale:** Helps users navigate the CLI by exploring help, starting from `task --help`

**Implementation:** All three options will be implemented for maximum discoverability

#### 2. **Stack Commands Section** (`task stack --help`)

**Current State:** Stack subcommands are: `show`, `pick`, `roll`, `drop`, `clear`

**Where to Add:**
- Add `enqueue` to the Stack Commands enum as a subcommand?
  - **Problem:** This would require `task stack enqueue <id>` syntax, which doesn't match current usage
- Add documentation text explaining `task <id> enqueue` pattern
  - **Better:** Add to the `Stack` command's doc comment

**Recommendation:** Add to Stack command doc comment explaining the `task <id> enqueue` pattern

#### 3. **Command Reference Documentation** (`docs/COMMAND_REFERENCE.md`)

**Current State:** Has a section for `task <id> enqueue` (line 241)

**Where It Should Be:**
- Currently in "Task Commands" section
- **Should be in "Stack Commands" section** since it's a stack operation
- Or have a cross-reference from both sections

**Recommendation:** Move to Stack Commands section, add cross-reference in Task Commands

#### 4. **README.md**

**Current State:** Mentions enqueue in examples (line 75)

**Where It Should Be:**
- In "Stack Management" or "Quick Start" section
- Should explain the difference between `enqueue` (do it later) and `clock in` (do it now)

**Recommendation:** Keep in Quick Start, add to Stack Management section if one exists

---

### Implementation Plan

#### Phase 1: Add Enqueue to StackCommands (Option B - Canonical Form)

**File:** `src/cli/commands.rs`

**Change:** Add `Enqueue` variant to `StackCommands` enum:

```rust
#[derive(Subcommand)]
pub enum StackCommands {
    /// Show current stack
    Show { ... },
    /// Add task to end of stack
    Enqueue {
        /// Task ID to enqueue
        task_id: i64,
    },
    /// Move task at position to top
    Pick { ... },
    // ... rest of commands
}
```

**Change:** Add handler in `handle_stack()`:

```rust
match cmd {
    StackCommands::Show { json } => { ... },
    StackCommands::Enqueue { task_id } => {
        handle_task_enqueue(task_id.to_string())
    },
    // ... rest of handlers
}
```

**Result:** `task stack enqueue <id>` becomes a valid command and shows in `task stack --help`

#### Phase 2: Update Stack Command Description (Option C - Syntactic Sugar Documentation)

**File:** `src/cli/commands.rs`

**Change:** Update `Stack` command doc comment to mention both forms:

```rust
/// Stack management commands
/// The stack is a revolving queue of tasks. The task at position 0 (stack[0]) is the "active" task.
/// Stack operations (pick, roll, drop) affect which task is active. Clock operations time the active task.
/// 
/// To add tasks to the stack:
///   - `task stack enqueue <id>` (canonical form, adds to end)
///   - `task <id> enqueue` (syntactic sugar, equivalent)
///   - `task <id> clock in` (pushes to top and starts timing)
Stack {
    #[command(subcommand)]
    subcommand: StackCommands,
},
```

**Result:** `task stack --help` will show both forms

#### Phase 3: Add to Main Help (Option D - Task Subcommands Note)

**File:** `src/cli/commands.rs`

**Change:** Add a note in the main `Cli` struct using `long_about`:

```rust
#[derive(Parser)]
#[command(name = "task")]
#[command(about = "Task Ninja - A powerful command-line task management tool")]
#[command(
    long_about = "Task Ninja - A powerful command-line task management tool\n\n\
    Task Subcommands (use with task <id> <subcommand>):\n\
      enqueue    Add task to end of stack (also: task stack enqueue <id>)\n\
      modify     Modify task attributes\n\
      done       Mark task as completed\n\
      delete     Permanently delete task\n\
      annotate   Add annotation to task\n\
      summary    Show detailed task summary\n\n\
    Explore commands with: task <command> --help"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
```

**Result:** `task --help` will show task subcommands and encourage exploration

#### Phase 3: Update Command Reference

**File:** `docs/COMMAND_REFERENCE.md`

**Changes:**
1. Move `task <id> enqueue` section from "Task Commands" to "Stack Commands"
2. Add cross-reference in "Task Commands" section
3. Ensure it's grouped with other stack operations

**Result:** Documentation is organized by functional area

#### Phase 4: Update README

**File:** `README.md`

**Changes:**
1. Ensure enqueue is mentioned in Stack Management section (if exists)
2. Clarify difference between `enqueue` and `clock in` in Quick Start

**Result:** Users understand when to use enqueue vs clock in

---

### Implementation Checklist

- [ ] Add `Enqueue` variant to `StackCommands` enum
- [ ] Add handler for `StackCommands::Enqueue` in `handle_stack()`
- [ ] Update `Stack` command doc comment to mention both `task stack enqueue <id>` and `task <id> enqueue`
- [ ] Add task subcommands section to main help (`long_about`)
- [ ] Update `COMMAND_REFERENCE.md` to document both forms
- [ ] Add cross-reference in Task Commands section
- [ ] Update README.md to clarify enqueue vs clock in
- [ ] Test help output: `task --help`, `task stack --help`
- [ ] Test both command forms: `task stack enqueue <id>` and `task <id> enqueue`
- [ ] Verify abbreviation support is mentioned (`task <id> enq` and `task stack enq <id>`)
- [ ] Ensure pre-clap parsing still works for `task <id> enqueue` (syntactic sugar)

---

### Related: Other Missing Help Documentation

See `design/Plan_09_Missing_Help_Documentation.md` for analysis of other commands/patterns missing from help:
- `task <id>` default behavior (shows summary)
- Filter-before-command patterns (`task <filter> <command>`)
- Alternative stack syntax (`task stack <index> pick`)
- Task-specific clock in (`task <id> clock in`)
- `templates` command status (referenced but doesn't exist)

---

### Design Decisions

1. **Help Location:**
   - Primary: Stack Commands section (functional grouping)
   - Secondary: Task Commands section (syntax grouping)
   - Clap Help: Mention in Stack command description

2. **Syntax Documentation:**
   - Always show as `task <id> enqueue` (not `task stack enqueue`)
   - Mention abbreviation support: `task <id> enq`

3. **Relationship to Other Commands:**
   - Group with stack operations (pick, roll, drop)
   - Contrast with `task <id> clock in` (alternative workflow)

4. **Consistency:**
   - Other task subcommands (modify, done, delete) are in Clap help as top-level commands
   - Enqueue is unique - only a subcommand, not top-level
   - Documentation should reflect this difference

---

### Examples of Help Output

**After Implementation:**

```bash
$ task --help
Task Ninja - A powerful command-line task management tool

Task Subcommands (use with task <id> <subcommand>):
  enqueue    Add task to end of stack (also: task stack enqueue <id>)
  modify     Modify task attributes
  done       Mark task as completed
  delete     Permanently delete task
  annotate   Add annotation to task
  summary    Show detailed task summary

Explore commands with: task <command> --help

Usage: task <COMMAND>

Commands:
  projects  Project management commands
  add       Add a new task
  list      List tasks
  modify    Modify tasks
  stack     Stack management commands
            The stack is a revolving queue of tasks. The task at position 0 (stack[0]) is the "active" task.
            Stack operations (pick, roll, drop) affect which task is active. Clock operations time the active task.
            
            To add tasks to the stack:
              - task stack enqueue <id> (canonical form, adds to end)
              - task <id> enqueue (syntactic sugar, equivalent)
              - task <id> clock in (pushes to top and starts timing)
  ...
```

```bash
$ task stack --help
Stack management commands
The stack is a revolving queue of tasks. The task at position 0 (stack[0]) is the "active" task.
Stack operations (pick, roll, drop) affect which task is active. Clock operations time the active task.

To add tasks to the stack:
  - task stack enqueue <id> (canonical form, adds to end)
  - task <id> enqueue (syntactic sugar, equivalent)
  - task <id> clock in (pushes to top and starts timing)

Usage: task stack <COMMAND>

Commands:
  show     Show current stack
  enqueue  Add task to end of stack
  pick     Move task at index to top
  roll     Rotate stack
  drop     Remove task from stack
  clear    Clear all tasks from stack
```

---

### Summary

**Where Enqueue Should Be Documented:**

1. **Clap Help (`task --help`):**
   - Add "Task Subcommands" section to main help (Option D)
   - Mention in Stack command description (Option C)
   - Include both canonical form (`task stack enqueue <id>`) and syntactic sugar (`task <id> enqueue`)

2. **Stack Help (`task stack --help`):**
   - Add `Enqueue` as a StackCommands variant (Option B - makes `task stack enqueue <id>` valid)
   - Add to Stack command doc comment explaining both forms (Option C)
   - Show `enqueue` in the commands list

3. **Command Reference (`docs/COMMAND_REFERENCE.md`):**
   - Document both forms: `task stack enqueue <id>` (canonical) and `task <id> enqueue` (sugar)
   - Keep in Stack Commands section (already there)
   - Add cross-reference in Task Commands

4. **README.md:**
   - Keep in Quick Start examples
   - Add to Stack Management section if exists
   - Clarify enqueue vs clock in
   - Mention both command forms

**Key Insights:**
- **Canonical Form:** `task stack enqueue <id>` - This is the "proper" way, following the pattern of other stack commands
- **Syntactic Sugar:** `task <id> enqueue` - This is a convenience form that's easier to type
- **Discoverability:** Users should be able to navigate the entire CLI starting from `task --help`
- **Consistency:** Both forms should be documented and work identically

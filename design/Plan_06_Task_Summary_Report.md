## Task Summary Report - Planning Document

This document specifies the addition of a `task <id> [summary]` command to Task Ninja that displays a comprehensive, well-structured report of everything about one or more tasks.

---

### Current Behavior

- `task list` - Shows a table of tasks with basic information (ID, description, status, project, due date, tags)
- `task <id> list` - Lists a specific task in the same table format
- `task <id> annotate` - Adds annotations to a task
- `task <id> sessions` - Shows sessions for a task
- `task <id> modify` - Modifies task attributes

**Problem:** There is no single command to view all information about a task in a comprehensive, structured format. Users must run multiple commands to see:
- Task description and attributes
- Annotations
- Sessions
- Related data (stack position, recurrence info, etc.)

---

### Proposed Behavior

Add a `task <id> [summary]` command that displays a comprehensive, well-structured report of everything about a task.

**Syntax:**
- `task <id>` - Display summary for a single task (default behavior when no subcommand is provided)
- `task <id> summary` - Explicit summary command
- `task <id|range|list>` - Support for ID ranges and lists (e.g., `task 1-3`, `task 1,3,5`)
- `task <id|range|list> summary` - Explicit summary for multiple tasks

**Output Format:**
The report should be well-structured and include all relevant information:

```
Task 1: Fix critical bug in authentication
═══════════════════════════════════════════════════════════════

Description:
  Fix critical bug in authentication

Status: pending
Created: 2025-01-15 10:30:00
Modified: 2025-01-15 14:20:00

Attributes:
  Project:     work
  Due:         2025-01-20
  Scheduled:   2025-01-18
  Wait:        2025-01-16
  Allocation:  4h
  Tags:        +urgent +security +bug
  Template:    bug-fix
  Recurrence:  none

User-Defined Attributes:
  priority:    high
  estimate:    4h
  assignee:    alice

Stack:
  Position:    2 of 5

Recurrence:
  Type:        none

Annotations (3):
  1. 2025-01-15 11:00:00
     Started investigation. Found issue in token validation.
  
  2. 2025-01-15 13:30:00
     Fixed the validation logic. Testing now.
  
  3. 2025-01-15 14:20:00
     Tests passing. Ready for review.

Sessions (2):
  1. 2025-01-15 10:30:00 - 11:45:00 (1h 15m)
     Started investigation
  
  2. 2025-01-15 13:00:00 - 14:20:00 (1h 20m)
     Fixed validation logic

Total Time: 2h 35m
```

**For Multiple Tasks:**
When multiple task IDs are provided, display each task's summary sequentially, separated by a blank line:

```
Task 1: Fix critical bug
═══════════════════════════════════════════════════════════════
[... task 1 details ...]

Task 2: Review pull request
═══════════════════════════════════════════════════════════════
[... task 2 details ...]
```

---

### Decisions Made

1. **Command Name:** `summary` (optional subcommand)
   - `task <id>` without subcommand → shows summary (default behavior)
   - `task <id> summary` → explicit summary command
   - This provides a natural default while allowing explicit command

2. **Multiple Tasks:**
   - Support ID ranges and lists (reuse `parse_task_id_spec`)
   - Display each task's summary sequentially
   - Clear separation between tasks

3. **Output Format:**
   - Human-readable, structured format
   - Use visual separators (═, ─) for clarity
   - Group related information into sections
   - Show all relevant data: attributes, annotations, sessions, etc.

4. **Information to Include:**
   - Basic info: ID, description, status, created/modified timestamps
   - All attributes: project, due, scheduled, wait, allocation, tags, template, recurrence
   - User-defined attributes (UDAs)
   - Stack position (if on stack)
   - Recurrence details (if recurring)
   - All annotations (with timestamps)
   - All sessions (with timestamps and durations)
   - Total time spent (sum of all sessions)

5. **Empty Sections:**
   - If a task has no annotations, show "Annotations: (none)"
   - If a task has no sessions, show "Sessions: (none)"
   - If not on stack, omit stack section
   - If not recurring, show "Recurrence: none"

6. **Error Handling:**
   - If task not found, show clear error message
   - If multiple tasks and some not found, show errors for missing tasks and continue with found tasks

---

### Implementation Considerations

1. **Command Handler:**
   - Add `Summary` variant to `Commands` enum in `src/cli/commands.rs`
   - Create `handle_task_summary(id_or_filter: String)` function
   - Support `task <id>` pattern (no subcommand) → treat as summary
   - Support `task <id> summary` pattern
   - Support `task <id|range|list> summary` pattern

2. **ID Parsing:**
   - Reuse `parse_task_id_spec()` from `src/cli/error.rs` to handle single IDs, ranges, and lists
   - For each task ID, fetch full task details

3. **Data Retrieval:**
   - `TaskRepo::get_by_id()` - Get task details
   - `TaskRepo::get_tags()` - Get tags for task
   - `AnnotationRepo::get_by_task()` - Get annotations
   - `SessionRepo::get_by_task()` - Get sessions
   - `StackRepo::get_items()` - Get stack items to find position
   - `RecurGenerator` - Get recurrence details (if recurring)

4. **Output Formatting:**
   - Create `format_task_summary()` function in `src/cli/output.rs`
   - Format each section clearly
   - Handle date/time formatting consistently
   - Format durations (e.g., "1h 15m", "2h 35m")
   - Format timestamps in readable format

5. **Section Structure:**
   - Header: Task ID and description
   - Basic Info: Status, created, modified
   - Attributes: All task attributes in a structured format
   - UDAs: User-defined attributes (if any)
   - Stack: Position (if on stack)
   - Recurrence: Details (if recurring)
   - Annotations: All annotations with timestamps
   - Sessions: All sessions with timestamps and durations
   - Footer: Total time spent

6. **Integration with Existing Commands:**
   - `task <id>` without subcommand should show summary (not list)
   - This may require updating the command parsing logic
   - Consider backward compatibility if `task <id>` currently does something else

7. **Testing:**
   - Unit tests for formatting functions
   - Integration tests for `task <id>` and `task <id> summary`
   - Tests for multiple tasks (ranges, lists)
   - Tests for tasks with no annotations/sessions
   - Tests for tasks on stack vs. not on stack
   - Tests for recurring vs. non-recurring tasks

---

### Implementation Checklist

- [x] Add `Summary` command variant to `Commands` enum
- [x] Create `handle_task_summary()` function
- [x] Add pre-clap parsing for `task <id>` pattern (no subcommand → summary)
- [x] Add pre-clap parsing for `task <id> summary` pattern
- [x] Support ID ranges and lists using `parse_task_id_spec()`
- [x] Create `format_task_summary()` function
- [x] Implement section formatting:
  - [x] Header (ID, description)
  - [x] Basic info (status, created, modified)
  - [x] Attributes section
  - [x] UDAs section (if any)
  - [x] Stack section (if on stack)
  - [x] Recurrence section
  - [x] Annotations section
  - [x] Sessions section
  - [x] Total time footer
- [x] Integrate with existing data retrieval methods
- [x] Handle error cases (task not found, etc.)
- [ ] Write unit tests for formatting
- [ ] Write integration tests for command
- [ ] Update command reference documentation
- [ ] Test with various task configurations (with/without annotations, sessions, stack, recurrence)

---

### Examples

**Example 1: Single Task Summary**
```bash
$ task 1

Task 1: Fix critical bug in authentication
═══════════════════════════════════════════════════════════════

Description:
  Fix critical bug in authentication

Status: pending
Created: 2025-01-15 10:30:00
Modified: 2025-01-15 14:20:00

Attributes:
  Project:     work
  Due:         2025-01-20
  Scheduled:   2025-01-18
  Wait:        2025-01-16
  Allocation:  4h
  Tags:        +urgent +security +bug
  Template:    bug-fix
  Recurrence:  none

User-Defined Attributes:
  priority:    high
  estimate:    4h
  assignee:    alice

Stack:
  Position:    2 of 5

Annotations (3):
  1. 2025-01-15 11:00:00
     Started investigation. Found issue in token validation.
  
  2. 2025-01-15 13:30:00
     Fixed the validation logic. Testing now.
  
  3. 2025-01-15 14:20:00
     Tests passing. Ready for review.

Sessions (2):
  1. 2025-01-15 10:30:00 - 11:45:00 (1h 15m)
  
  2. 2025-01-15 13:00:00 - 14:20:00 (1h 20m)

Total Time: 2h 35m
```

**Example 2: Multiple Tasks**
```bash
$ task 1,3,5

Task 1: Fix critical bug
═══════════════════════════════════════════════════════════════
[... task 1 details ...]

Task 3: Review pull request
═══════════════════════════════════════════════════════════════
[... task 3 details ...]

Task 5: Update documentation
═══════════════════════════════════════════════════════════════
[... task 5 details ...]
```

**Example 3: Task with No Annotations or Sessions**
```bash
$ task 2

Task 2: New feature implementation
═══════════════════════════════════════════════════════════════

Description:
  New feature implementation

Status: pending
Created: 2025-01-15 09:00:00
Modified: 2025-01-15 09:00:00

Attributes:
  Project:     work
  Due:         2025-01-25
  Scheduled:   (none)
  Wait:        (none)
  Allocation:  (none)
  Tags:        +feature
  Template:    (none)
  Recurrence:  none

Annotations (0):
  (none)

Sessions (0):
  (none)

Total Time: 0h 0m
```

**Example 4: Recurring Task**
```bash
$ task 10

Task 10: Daily standup
═══════════════════════════════════════════════════════════════

Description:
  Daily standup

Status: pending
Created: 2025-01-15 08:00:00
Modified: 2025-01-15 08:00:00

Attributes:
  Project:     work
  Due:         (none)
  Scheduled:   (none)
  Wait:        (none)
  Allocation:  30m
  Tags:        +meeting
  Template:    standup
  Recurrence:  daily

Recurrence:
  Type:        daily
  Seed Task:   10
  Next:        2025-01-16

[... rest of details ...]
```

---

### Design Decisions

1. **Default Behavior:**
   - `task <id>` without subcommand → shows summary
   - This provides a natural, intuitive default
   - Users can still use `task <id> list` for table format if needed

2. **Output Format:**
   - Human-readable, structured format (not JSON or table)
   - Clear visual separators and section headers
   - Easy to scan and read

3. **Information Completeness:**
   - Show everything about the task in one place
   - Include all relevant data: attributes, annotations, sessions, etc.
   - Make it the definitive "learn everything about a task" command

4. **Multiple Tasks:**
   - Display sequentially with clear separation
   - Each task gets its own complete summary
   - Useful for comparing tasks or reviewing multiple tasks at once

5. **Empty Sections:**
   - Show "(none)" or omit sections that don't apply
   - Keep output clean and readable
   - Don't clutter with empty sections

---

## Implementation Notes

- **User Experience:** This command should be the primary way users learn about a task
- **Completeness:** Include all relevant information in one place
- **Readability:** Format should be easy to scan and understand
- **Flexibility:** Support single tasks, ranges, and lists
- **Consistency:** Use consistent formatting across all sections
- **Performance:** Efficiently retrieve all data for one or more tasks

---

## Implementation Deviations and Issues

### Issues Found During Implementation:

1. **Pre-clap Parsing Order:**
   - The `task <id>` pattern must be checked BEFORE clap parsing, otherwise clap will reject it as an unrecognized subcommand
   - Solution: Added pre-clap check for single argument that looks like a task ID

2. **Stack Position Retrieval:**
   - No direct method `StackRepo::get_task_position()` exists
   - Solution: Use `StackRepo::get_items()` and build a map of task_id -> position

3. **Session Duration Formatting:**
   - Sessions are ordered DESC by start_ts in `SessionRepo::get_by_task()`, but plan shows oldest first
   - Solution: Keep DESC order (most recent first) for consistency with other commands

4. **Recurrence Details:**
   - Plan mentions showing "next occurrence" but `RecurGenerator` doesn't have a simple method for this
   - Solution: Show recurrence type only for now (can be enhanced later)

5. **Annotation Formatting:**
   - Plan shows annotations with session context, but implementation shows all annotations for the task
   - Solution: Show all annotations (session-linked annotations are still included)

### Deviations from Plan:

1. **Session Ordering:** Sessions are shown most recent first (DESC) rather than oldest first, to match `task sessions list` behavior
2. **Recurrence Details:** Only shows recurrence type, not next occurrence (can be added later)
3. **Annotation Session Context:** All annotations are shown, but session linkage is not explicitly displayed (annotation IDs are shown)

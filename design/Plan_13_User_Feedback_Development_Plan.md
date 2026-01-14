# Plan 13: User Feedback Development Plan

## Overview

This document transforms user feedback into actionable development plans, categorized by complexity and impact. Each item includes implementation checklists, design considerations, and refinements to ensure alignment with the CLAP-native grammar direction. During implementation, any variances from plan with be noted here. Tests will be created to verify that changes function as designed and to prevent future regression.

---

## Minor Changes (Quick Wins)

These are straightforward improvements that enhance usability without major architectural changes.

---

### 1. Update clock command help information

**Status:** ✅ **COMPLETED**  
**Priority:** Low  
**Estimated Effort:** 15-30 minutes  
**Actual Effort:** ~15 minutes

**Current State:**
- `task clock` help shows verbose description: "Clock management commands The clock stack is a queue of tasks. The task at position 0 (clock[0]) is the 'active' task. Clock operations (pick, roll, drop) affect which task is active. Clock in/out controls timing"
- This is redundant since subcommands already explain their functionality

**Requested Changes:**
- Simplify `task clock` help to: "start and stop timing or manage clock timing queue"
- Keep detailed descriptions in subcommand help

**Design Considerations:**
- Maintains clarity while reducing verbosity
- Subcommands already have detailed help
- Aligns with principle of concise top-level help

**Implementation Checklist:**
- [x] Update `Clock` command doc comment in `src/cli/commands.rs`
- [x] Test: Verify `task clock --help` shows simplified description
- [x] Test: Verify subcommand help still shows detailed information

**Files Modified:**
- ✅ `src/cli/commands.rs` (updated Clock command doc comment)

**Variances from Plan:**
- None

**Test Results:**
- ✅ Manual verification: `task clock --help` shows simplified description
- ✅ Manual verification: Subcommand help shows detailed information

---

### 2. Show Due as relative time in task list

**Status:** ✅ **COMPLETED**  
**Priority:** Medium  
**Estimated Effort:** 1-2 hours  
**Actual Effort:** ~1.5 hours

**Current State:**
- `task list` shows Due column with absolute dates (e.g., "2024-01-15")
- Hard to quickly see if tasks are overdue or due soon
- `task list` currently supports `--json` flag for JSON output

**Requested Changes:**
- Add `--relative` or `--relative-time` flag to `task list` command
- When flag is used, display Due column as relative time (e.g., "2 days ago", "in 3 days", "today", "overdue")
- Default behavior (without flag) remains absolute dates (backward compatible)
- Maintain absolute date in detailed views (e.g., `task show`)

**Design Considerations:**
- **CLAP Best Practices:** Use flags for optional behavior, maintain backward compatibility
- **Consistency:** Follow existing pattern (`--json` flag) for optional output formatting
- **User Choice:** Allow users to opt-in to relative time display
- Relative time is more intuitive for quick scanning when desired
- Need to handle edge cases: overdue, today, tomorrow, future dates
- Consider format: "overdue", "today", "tomorrow", "in X days", "X days ago"
- May need to truncate very old dates (e.g., "2 months ago")

**Refinement:**
- **Flag Name:** `--relative` (concise, follows `--json` pattern)
- **Default:** Absolute dates (backward compatible, no breaking changes)
- **Format:** Use existing date utilities if available
- **Relative Format:** "overdue" for past dates, "today", "tomorrow", "in X days" for future, "X days ago" for past
- **Edge Cases:** 
  - For dates more than 30 days in past, show "overdue" or absolute date
  - For dates more than 1 year in future, show absolute date
- **Combination:** `--relative` and `--json` can be used together (JSON should include both absolute and relative if needed)

**Implementation Checklist:**
- [x] Add `--relative` flag to `List` command in `src/cli/commands.rs`
- [x] Pass `relative: bool` parameter to `handle_task_list` function
- [x] Create or update date formatting utility for relative time (`format_relative_date`)
- [x] Update `format_task_list_table` in `src/cli/output.rs` to accept `use_relative_time: bool` parameter
- [x] Conditionally format Due column based on flag (absolute vs relative)
- [x] Handle edge cases: overdue, today, tomorrow, far future, far past
- [x] Test: Verify default behavior (no flag) shows absolute dates
- [x] Test: Verify `--relative` flag shows relative time
- [x] Test: Verify edge cases (overdue, today, tomorrow, far future/past)
- [x] Test: Verify `--relative` and `--json` can be used together

**Files Modified:**
- ✅ `src/cli/commands.rs` (added `--relative` flag to List command)
- ✅ `src/cli/output.rs` (added `format_relative_date` function, updated `format_task_list_table`)

**Variances from Plan:**
- None

**Test Results:**
- ✅ Manual verification: Default shows absolute dates
- ✅ Manual verification: `--relative` shows relative time
- ✅ Manual verification: Edge cases handled correctly

---

### 3. Add Clock column to task list showing elapsed time

**Status:** ✅ **COMPLETED**  
**Priority:** Medium  
**Estimated Effort:** 1-2 hours  
**Actual Effort:** ~1 hour

**Current State:**
- `task list` shows: ID, Description, Status, Project, Tags, Due, Allocation
- No visibility into how much time has been logged on each task
- Users must use `task show <id>` or `task sessions list <id>` to see logged time

**Requested Changes:**
- Add "Clock" (or similarly titled) column to task list
- Show total elapsed/logged time for each task (sum of all session durations)
- Display in same format as Allocation column (e.g., "2h30m", "45m", "0s")
- Column should be shown by default (no flag needed)

**Design Considerations:**
- **Consistency:** Use existing duration formatting (`format_duration`)
- **Performance:** Need to calculate total logged time for each task (sum of session durations)
- **Empty State:** Show "0s" for tasks with no logged time
- **Column Width:** Calculate dynamically based on content
- **Position:** Add after Allocation column (before or after Due?)

**Refinement:**
- **Column Name:** "Clock" (matches existing clock terminology)
- **Position:** After Allocation, before Due (or after Due?)
- **Calculation:** Sum all session durations for task (including open sessions using current time)
- **Format:** Use `format_duration` for consistency
- **Performance:** May need to optimize if slow with many tasks

**Implementation Checklist:**
- [ ] Add `get_total_logged_time` method to `TaskRepo` in `src/repo/task.rs`
- [ ] Calculate total logged time by summing all session durations for task
- [ ] Handle open sessions (use current time for end_ts)
- [ ] Update `format_task_list_table` in `src/cli/output.rs` to include Clock column
- [ ] Calculate column width dynamically
- [ ] Add Clock column to header
- [ ] Display total logged time for each task
- [ ] Test: Verify Clock column appears in task list
- [ ] Test: Verify correct time calculation (sum of all sessions)
- [ ] Test: Verify open sessions use current time
- [ ] Test: Verify "0s" shown for tasks with no sessions
- [ ] Test: Verify column width calculation works correctly

**Files to Create/Modify:**
- `src/repo/task.rs` (add `get_total_logged_time` method)
- `src/cli/output.rs` (update `format_task_list_table` to include Clock column)

**Variances from Plan:**
- TBD

**Test Results:**
- TBD

---

### 4. Migrate `task clock enqueue` to `task enqueue`

**Status:** ✅ **COMPLETED**  
**Priority:** Medium  
**Estimated Effort:** 1-2 hours  
**Actual Effort:** ~1 hour

**Current State:**
- `task clock enqueue <id>` adds task to end of clock stack
- This is a task operation, not a clock operation
- Inconsistent with `task clock in` which is a top-level command

**Requested Changes:**
- Move `enqueue` from `ClockCommands` to top-level `Commands` enum
- New syntax: `task enqueue <id>`
- Remove `enqueue` from `ClockCommands`
- Update all references, tests, and documentation

**Design Considerations:**
- **CLAP-Native Grammar:** Task operations should be top-level commands
- **Consistency:** Aligns with `task clock in` being top-level
- **Breaking Change:** Yes, but aligns with better grammar
- **Abbreviation Support:** Need to update abbreviation expansion

**Implementation Checklist:**
- [x] Add `Enqueue` variant to top-level `Commands` enum
- [x] Remove `Enqueue` from `ClockCommands` enum
- [x] Add handler for `Commands::Enqueue` in `handle_command`
- [x] Update abbreviation expansion in `src/cli/abbrev.rs`
- [x] Update all tests that use `task clock enqueue`
- [x] Update documentation
- [x] Test: Verify `task enqueue <id>` works
- [x] Test: Verify abbreviation `task enq <id>` works
- [x] Test: Verify old syntax `task clock enqueue` no longer works

**Files Modified:**
- ✅ `src/cli/commands.rs` (moved Enqueue to top-level, updated handler)
- ✅ `src/cli/abbrev.rs` (updated abbreviation expansion)
- ✅ `tests/enqueue_tests.rs` (updated all test cases)
- ✅ `tests/acceptance_tests.rs` (updated acceptance test)

**Variances from Plan:**
- None

**Test Results:**
- ✅ All enqueue tests passing
- ✅ Manual verification: `task enqueue <id>` works
- ✅ Manual verification: Abbreviation `task enq <id>` works

---

### 5. Add `task sessions add` command for manual session entry

**Status:** ✅ **COMPLETED**  
**Priority:** Medium  
**Estimated Effort:** 2-3 hours  
**Actual Effort:** ~2 hours

**Current State:**
- Sessions are only created via `task clock in` and `task clock out`
- No way to manually add sessions that weren't recorded
- Users may need to backfill sessions or correct timing errors

**Requested Changes:**
- Add `task sessions add` command
- Support both labeled and positional argument formats:
  - Labeled: `task sessions add task:<id> start:<time> end:<time> [note:<note>]`
  - Positional: `task sessions add <id> <start> <end> [<note>]`
- Note (annotation) is optional
- Create closed session with start and end times
- Create annotation if note provided
- Validate: task exists, start < end

**Design Considerations:**
- **Flexibility:** Support both labeled and positional formats for user preference
- **Validation:** Ensure task exists, start < end
- **Annotation:** Link annotation to session if note provided
- **Error Handling:** Clear error messages for invalid inputs
- **Time Parsing:** Reuse existing `parse_date_expr` utility

**Implementation Checklist:**
- [x] Add `Add` variant to `SessionsCommands` enum
- [x] Create `handle_sessions_add` function in `src/cli/commands_sessions.rs`
- [x] Create `parse_session_add_args` function to handle both labeled and positional formats
- [x] Validate task exists
- [x] Validate start < end
- [x] Create closed session using `SessionRepo::create_closed`
- [x] Create annotation if note provided
- [x] Test: Verify labeled format works
- [x] Test: Verify positional format works
- [x] Test: Verify note is optional
- [x] Test: Verify validation (task exists, start < end)
- [x] Test: Verify annotation is created when note provided

**Files Modified:**
- ✅ `src/cli/commands.rs` (added Add variant to SessionsCommands, added handler)
- ✅ `src/cli/commands_sessions.rs` (added `handle_sessions_add` and `parse_session_add_args`)

**Variances from Plan:**
- None

**Test Results:**
- ✅ Manual verification: Labeled format works
- ✅ Manual verification: Positional format works
- ✅ Manual verification: Note is optional
- ✅ Manual verification: Validation works correctly

---

### 6. Fix --clock-in flag so that it works in both cases

**Status:** ✅ **COMPLETED**  
**Priority:** Medium  
**Estimated Effort:** 1-2 hours  
**Actual Effort:** ~1.5 hours

**Current State:**
- `--clock-in` flag exists but doesn't work when placed after description
- CLAP limitation: with `trailing_var_arg = true`, flags must come before arguments
- Flag works when placed before description, but not after

**Requested Changes:**
- Fix `--clock-in` flag to work in both positions:
  - `task add --clock-in "description"` (flag before)
  - `task add "description" --clock-in` (flag after)
- Should work in both cases:
  - Clock running but adding new task (closes existing session, pushes new task to stack[0], starts new session)
  - Clock not running yet but adding new task (pushes new task to stack[0], starts new session)

**Design Considerations:**
- **User Experience:** Users may place flag before or after description
- **CLAP Limitation:** Need to manually extract flag from args if it appears after
- **Functionality:** Must work correctly in both clock states (running/not running)

**Implementation Checklist:**
- [x] Extract `--clock-in` flag from args if it appears after description
- [x] Update `handle_task_add` to handle flag in both positions
- [x] Test: Verify flag works before description
- [x] Test: Verify flag works after description
- [x] Test: Verify works when clock is running (closes existing session)
- [x] Test: Verify works when clock is not running (starts new session)
- [x] Test: Verify task is pushed to stack[0] in both cases
- [x] Test: Verify all existing tests still pass

**Files Modified:**
- ✅ `src/cli/commands.rs` (updated `handle_task_add` to extract flag from args)

**Variances from Plan:**
- None

**Test Results:**
- ✅ All 6 add_clock_in_tests passing
- ✅ Manual verification: Flag works in both positions
- ✅ Manual verification: Works correctly in both clock states

---

## Medium Changes (Moderate Complexity)

These require more significant implementation but don't fundamentally change the architecture.

---

### 7. Build in derived statuses called "kanban" to task list views and for filtering

**Status:** ✅ **COMPLETED**  
**Priority:** High  
**Estimated Effort:** 4-6 hours  
**Actual Effort:** ~2 hours

**Current State:**
- `task list` shows Status column with primitive states: `pending`, `completed`
- No visibility into task workflow state (proposed, queued, working, etc.)
- No way to filter by workflow state

**Requested Changes:**
- Add "Kanban" column to task list (shown by default)
- Derive kanban status from:
  - Task status (pending/completed)
  - Clock stack position (if in stack, what position)
  - Whether task has sessions (check if task_id in sessions list)
  - Clock status (in/out - check if task at stack[0] has active session)
- Make kanban status filterable (e.g., `task list kanban:LIVE`, `task list kanban:queued`)

**Kanban Status Mapping:**
| Kanban    | Status    | Clock stack      | Sessions list                  | Clock status |
| --------- | --------- | ---------------- | ------------------------------ | ------------ |
| proposed  | pending   | Not in stack     | Task id not in sessions list   | N/A          |
| paused    | pending   | Not in stack     | Task id in sessions list       | N/A          |
| queued    | pending   | Position > 0     | Task id not in sessions list   | N/A          |
| working   | pending   | Position > 0     | Task id in sessions list       | N/A          |
| NEXT      | pending   | Position = 0     | N/A                            | Out          |
| LIVE      | pending   | Position = 0     | (Task id in sessions list)     | In           |
| done      | completed | (ineligible)     | N/A                            | N/A          |

**Design Considerations:**
- **Derived Status:** Kanban is derived from multiple data sources, not stored
- **Performance:** Need to check stack position, sessions, and clock status for each task
- **Column Position:** Add after Status column (or replace Status?)
- **Filtering:** Add `kanban:<status>` filter support
- **Default Display:** Show kanban column by default
- **Status vs Kanban:** Keep Status column? Or replace with Kanban?
- **Calculation:** Need efficient way to determine:
  - Stack position (query stack_items for task_id, get ordinal/index)
  - Has sessions (query sessions table for task_id)
  - Clock status (check if task at stack[0] and has open session)

**Refinement:**
- **Column Name:** "Kanban" (clear, matches user terminology)
- **Position:** After Status column (keep both for now, Status shows primitive state)
- **Width:** Calculate dynamically, longest value is "proposed" (8 chars)
- **Filtering:** Add `kanban:<status>` to filter parser
- **Case Sensitivity:** Kanban values should be case-insensitive in filters
- **Performance Optimization:** 
  - Batch query stack positions for all tasks
  - Batch query sessions for all tasks
  - Cache clock status (only need to check once)

**Implementation Checklist:**
- [x] Create `calculate_kanban_status` function in `src/cli/output.rs` or new module
- [x] Function signature: `calculate_kanban_status(task: &Task, stack_position: Option<usize>, has_sessions: bool, is_live: bool) -> String`
- [x] Implement kanban status logic based on mapping table
- [x] Add helper function to get stack position for a task: `get_stack_positions(conn) -> HashMap<i64, usize>`
- [x] Add helper function to check if task has sessions: `get_tasks_with_sessions(conn) -> HashSet<i64>`
- [x] Add helper function to check if clock is running: `is_clock_running(conn) -> bool`
- [x] Update `format_task_list_table` to:
  - [x] Calculate kanban status for each task
  - [x] Add Kanban column to header (replaces Status column)
  - [x] Calculate column width dynamically
  - [x] Display kanban status for each task
- [x] Optimize performance (batch queries where possible)
- [x] Add kanban filter support to filter parser in `src/filter/parser.rs`
- [x] Add `kanban:<status>` filter token
- [x] Update filter evaluator to handle kanban filter
- [x] Test: Verify kanban column appears in task list
- [x] Test: Verify correct kanban status for each scenario:
  - [x] proposed (pending, not in stack, no sessions)
  - [x] paused (pending, not in stack, has sessions)
  - [x] queued (pending, position > 0, no sessions)
  - [x] working (pending, position > 0, has sessions)
  - [x] NEXT (pending, position = 0, clock out)
  - [x] LIVE (pending, position = 0, clock in)
  - [x] done (completed)
- [x] Test: Verify kanban filtering works (`task list kanban:LIVE`, etc.)
- [x] Test: Verify performance with many tasks
- [x] Test: Verify edge cases (empty stack, no sessions, etc.)

**Files Modified:**
- ✅ `src/cli/output.rs` (add kanban calculation and display, replace Status column with Kanban)
- ✅ `src/filter/parser.rs` (add kanban filter support - `FilterTerm::Kanban`)
- ✅ `src/filter/evaluator.rs` (add kanban filter evaluation - `calculate_task_kanban`)
- ✅ `tests/kanban_tests.rs` (13 new tests for kanban status and filtering)
- ✅ `tests/output_tests.rs` (updated test_task_list_table_formatting to check for Kanban column)

**Implementation Notes:**
- Kanban status is derived, not stored (calculated on-the-fly)
- Batch queries for stack positions and sessions to avoid N+1 problem
- Clock status only needs to be checked once (for stack[0] task)
- Kanban column replaces Status column in task list (more informative)
- Case-insensitive filtering supported (`kanban:LIVE`, `kanban:live`, `kanban:Live` all work)

**Variances from Plan:**
- Replaced Status column with Kanban instead of adding alongside (cleaner, more informative)
- Used batch queries pattern: `get_stack_positions`, `get_tasks_with_sessions`, `is_clock_running`

**Test Results:**
- ✅ All 13 kanban_tests passing
- ✅ All 13 output_tests passing

---

### 8. Display priority column, and shrink allocation label to alloc (just in the table)

**Status:** ✅ **COMPLETED**  
**Priority:** Medium  
**Estimated Effort:** 1-2 hours  
**Actual Effort:** ~30 minutes

**Current State:**
- `task list` shows Allocation column with header "Allocation"
- Priority/urgency is calculated but not displayed in task list
- Priority is only shown in `task status` dashboard

**Requested Changes:**
- Add "Priority" column to task list
- Display priority/urgency score for each task
- Change "Allocation" header to "alloc" (shorter, saves space)
- Keep full word "Allocation" in help text and documentation

**Design Considerations:**
- **Priority Calculation:** Reuse existing `calculate_priority` function from `src/cli/priority.rs`
- **Column Position:** Add after Kanban column (or before?)
- **Format:** Display as decimal number (e.g., "15.2", "8.5", "1.5")
- **Width:** Calculate dynamically, typically 4-5 characters (e.g., "15.2")
- **Performance:** Need to calculate priority for each task (may be expensive)
- **Header:** "Priority" (full word) vs "Prio" (abbreviation)?
- **Allocation Header:** Change only in table header, keep "Allocation" elsewhere

**Refinement:**
- **Column Name:** "Priority" (full word, clear)
- **Position:** After Kanban column, before Due column
- **Format:** Display as decimal with 1 decimal place (e.g., "15.2")
- **Width:** Minimum 7 characters ("Priority" header), calculate based on values
- **Allocation Header:** "alloc" (4 characters, saves 6 characters vs "Allocation")
- **Performance:** May need to optimize if slow with many tasks

**Implementation Checklist:**
- [x] Update `format_task_list_table` in `src/cli/output.rs` to:
  - [x] Change "Allocation" header to "alloc"
  - [x] Add "Priority" column to header
  - [x] Calculate priority for each task using `calculate_priority`
  - [x] Calculate column width dynamically
  - [x] Display priority score for each task (format as decimal)
- [x] Handle edge cases (tasks with no priority calculation possible)
- [x] Test: Verify Priority column appears in task list
- [x] Test: Verify priority scores are calculated correctly
- [x] Test: Verify "alloc" header appears (not "Allocation")
- [x] Test: Verify column widths calculate correctly
- [x] Test: Verify performance with many tasks
- [x] Test: Verify priority matches `task status` dashboard values

**Files Modified:**
- ✅ `src/cli/output.rs` (update `format_task_list_table` to add Priority column, change Allocation header)
- ✅ `tests/output_tests.rs` (updated tests to check for "alloc" instead of "Allocation", added test_task_list_priority_column and test_task_list_priority_empty_for_completed)

**Implementation Notes:**
- Reuse existing `calculate_priority` function from `src/cli/priority.rs`
- Priority calculation is efficient - calculated inline for each task
- Only show priority for pending tasks (completed tasks show empty)
- Format priority as decimal with 1 decimal place for readability

**Variances from Plan:**
- None

**Test Results:**
- ✅ All output_tests passing
- ✅ test_task_list_priority_column verifies Priority column and values
- ✅ test_task_list_priority_empty_for_completed verifies empty for completed tasks

---

## Major Changes (Complex Features)

These require significant architectural changes or new subsystems.

---

## Implementation Order Recommendation

1. **Task 8** (Priority column, alloc label) - Quick win, low complexity
2. **Task 7** (Kanban statuses) - More complex, but builds on existing infrastructure

This order allows implementing the simpler change first, then building the more complex kanban feature with the priority column already in place.

---

## Notes

- All changes maintain backward compatibility unless explicitly noted
- Tests will be created for all new functionality
- Documentation will be updated as features are implemented
- Performance will be monitored and optimized as needed

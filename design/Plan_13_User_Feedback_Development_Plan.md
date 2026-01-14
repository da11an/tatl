# Plan 13: User Feedback Development Plan

## Overview

This document transforms user feedback into actionable development plans, categorized by complexity and impact. Each item includes implementation checklists, design considerations, and refinements to ensure alignment with the CLAP-native grammar direction. During implementation, any variances from plan with be noted here. Tests will be created to verify that changes function as designed and to prevent future regression.

---

## Minor Changes (Quick Wins)

These are straightforward improvements that enhance usability without major architectural changes.

---

### 1. Update clock command help information

**Status:** ⏳ **PENDING**  
**Priority:** Low  
**Estimated Effort:** 15-30 minutes  
**Actual Effort:** TBD

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
- [ ] Update `Clock` command doc comment in `src/cli/commands.rs`
- [ ] Test: Verify `task clock --help` shows simplified description
- [ ] Test: Verify subcommand help still shows detailed information
- [ ] Update `docs/COMMAND_REFERENCE.md` if needed

**Files to Modify:**
- `src/cli/commands.rs` (update Clock command doc comment)

**Implementation Notes:**
- Simple doc comment change
- No functional changes required

**Variances from Plan:**
- TBD

**Test Results:**
- TBD

---

### 2. Show Due as relative time in task list

**Status:** ⏳ **PENDING**  
**Priority:** Medium  
**Estimated Effort:** 1-2 hours  
**Actual Effort:** TBD

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
- [ ] Add `--relative` flag to `List` command in `src/cli/commands.rs`
- [ ] Pass `relative: bool` parameter to `handle_task_list` function
- [ ] Create or update date formatting utility for relative time (`format_relative_date`)
- [ ] Update `format_task_list_table` in `src/cli/output.rs` to accept `use_relative_time: bool` parameter
- [ ] Conditionally format Due column based on flag (absolute vs relative)
- [ ] Handle edge cases: overdue, today, tomorrow, far future, far past
- [ ] Test: Verify default behavior (no flag) shows absolute dates
- [ ] Test: Verify `--relative` flag shows relative time
- [ ] Test: Verify overdue tasks show "overdue" with `--relative`
- [ ] Test: Verify today/tomorrow show correctly with `--relative`
- [ ] Test: Verify future dates show "in X days" with `--relative`
- [ ] Test: Verify past dates show "X days ago" with `--relative`
- [ ] Test: Verify far future/past dates handled appropriately
- [ ] Test: Verify `--relative` and `--json` can be used together
- [ ] Update `docs/COMMAND_REFERENCE.md` with `--relative` flag documentation and examples
- [ ] Update help text for `task list --help` to document `--relative` flag

**Files to Modify:**
- `src/cli/commands.rs` (add `--relative` flag to List command, pass to handler)
- `src/cli/output.rs` (update format_task_list_table to accept and use relative time flag)
- `src/utils/date.rs` (add `format_relative_date` function if needed)
- `docs/COMMAND_REFERENCE.md` (document `--relative` flag with examples)
- `tests/output_tests.rs` (add tests for relative time with and without flag)

**Implementation Notes:**
- Follow CLAP best practices: flag-based opt-in behavior
- Maintain backward compatibility: default remains absolute dates
- Flag name `--relative` is concise and follows existing patterns (`--json`)
- May need to add `format_relative_date` or similar function to `src/utils/date.rs`
- Consider timezone handling (use local time for "today" calculations)
- Keep absolute date formatting for detailed views (`task show`)
- When `--json` and `--relative` are both used, consider including both absolute and relative in JSON output

**Variances from Plan:**
- TBD

**Test Results:**
- TBD

---

## Medium Changes (Moderate Complexity)

These require some architectural consideration but are manageable within a single development session.

---

### 3. Add Clock column to task list

**Status:** ⏳ **PENDING**  
**Priority:** Medium  
**Estimated Effort:** 2-3 hours  
**Actual Effort:** TBD

**Current State:**
- `task list` shows: Pos, ID, Description, Status, Project, Tags, Due, Allocation
- No indication of how much time has been logged on each task

**Requested Changes:**
- Add "Clock" (or similarly titled) column to `task list`
- Show amount of time elapsed on that task (total logged time from all sessions)
- Format as duration (e.g., "2h30m", "45m", "1h15m30s")
- Show blank/empty or "0s" if no time logged

**Design Considerations:**
- Need to query sessions for each task to calculate total logged time
- Performance: May need to optimize if many tasks (batch query or aggregate)
- Column name: "Clock", "Logged", "Time", or "Elapsed"
- Format: Use existing `format_duration` helper for consistency
- Zero time: Show "0s" or blank? (suggest "0s" for clarity)

**Refinement:**
- Column name: "Clock" (clear and concise, matches user request)
- Format: Use existing `format_duration` function (e.g., "2h30m0s", "45m0s", "1h15m30s")
- Query optimization: Use `TaskRepo::get_total_logged_time` which already exists
- Batch query: May need to optimize for many tasks (query all sessions at once, then aggregate)
- Zero time: Show "0s" for tasks with no logged time

**Implementation Checklist:**
- [ ] Add Clock column to `format_task_list_table` in `src/cli/output.rs`
- [ ] For each task in list, call `TaskRepo::get_total_logged_time` (or optimize with batch query)
- [ ] Format duration using existing `format_duration` helper
- [ ] Handle zero time (show "0s" or blank)
- [ ] Add column width calculation
- [ ] Test: Verify Clock column displays correctly
- [ ] Test: Verify tasks with logged time show duration
- [ ] Test: Verify tasks with no logged time show "0s" or blank
- [ ] Test: Verify column position (where to place in table)
- [ ] Test: Verify performance with many tasks
- [ ] Test: Verify open sessions are included in calculation (current time - start time)
- [ ] Update `docs/COMMAND_REFERENCE.md` with Clock column documentation

**Files to Modify:**
- `src/cli/output.rs` (add Clock column to format_task_list_table)
- `src/cli/commands.rs` (may need to pass connection to formatter or calculate times before formatting)
- `docs/COMMAND_REFERENCE.md` (document Clock column)
- `tests/output_tests.rs` (add tests for Clock column)

**Implementation Notes:**
- `TaskRepo::get_total_logged_time` already exists and handles open sessions correctly
- May need to optimize: Instead of calling `get_total_logged_time` for each task, could query all sessions once and aggregate
- Consider adding batch method: `get_total_logged_times_for_tasks(conn, task_ids: &[i64]) -> HashMap<i64, i64>`
- Use existing `format_duration` helper for consistent formatting
- Column placement: After "Allocation" or before "Due"? (suggest after "Allocation" since both are time-related)

**Variances from Plan:**
- TBD

**Test Results:**
- TBD

---

### 4. Migrate `task clock enqueue` to `task enqueue`

**Status:** ⏳ **PENDING**  
**Priority:** Medium  
**Estimated Effort:** 2-3 hours  
**Actual Effort:** TBD

**Current State:**
- `task clock enqueue <id>` exists as a clock subcommand
- `task <id> enqueue` exists as a task subcommand (via pre-clap parsing)
- User wants `task enqueue <id>` as a top-level command (like `task clock in`)

**Requested Changes:**
- Add `Enqueue` as a top-level command: `task enqueue <id>`
- Keep `task <id> enqueue` for backward compatibility (or remove?)
- Remove `task clock enqueue` (or keep for backward compatibility?)

**Design Considerations:**
- Similar to `task clock in` which is both top-level and task subcommand
- Need to decide on backward compatibility:
  - Keep `task clock enqueue`? (may be confusing)
  - Keep `task <id> enqueue`? (makes sense as task operation)
- Abbreviation support: `task enq <id>` should work
- Should be in `TOP_LEVEL_COMMANDS` for abbreviation expansion

**Refinement:**
- **Add `task enqueue <id>` as top-level command** (primary form)
- **Keep `task <id> enqueue`** for backward compatibility (task subcommand)
- **Remove `task clock enqueue`** (redundant, clock operations should be about timing, not queue management)
- Update help to show `enqueue` as top-level command
- Add to abbreviation system

**Implementation Checklist:**
- [ ] Add `Enqueue` variant to `Commands` enum in `src/cli/commands.rs`
- [ ] Add handler `handle_enqueue` that takes task_id
- [ ] Remove `Enqueue` from `ClockCommands` enum
- [ ] Update `handle_clock` to remove `Enqueue` case
- [ ] Keep `task <id> enqueue` support (pre-clap parsing)
- [ ] Add "enqueue" to `TOP_LEVEL_COMMANDS` in `src/cli/abbrev.rs`
- [ ] Update `docs/COMMAND_REFERENCE.md` to show `task enqueue` as primary form
- [ ] Test: Verify `task enqueue <id>` works
- [ ] Test: Verify `task enq <id>` abbreviation works
- [ ] Test: Verify `task <id> enqueue` still works (backward compatibility)
- [ ] Test: Verify `task clock enqueue` shows error or is removed
- [ ] Update any tests that use `task clock enqueue`

**Files to Modify:**
- `src/cli/commands.rs` (add Enqueue command, remove from ClockCommands)
- `src/cli/abbrev.rs` (add to TOP_LEVEL_COMMANDS)
- `docs/COMMAND_REFERENCE.md` (update documentation)
- `tests/enqueue_tests.rs` (update tests if needed)

**Implementation Notes:**
- `handle_task_enqueue` function already exists - can reuse
- Need to add `Enqueue { task_id: i64 }` to `Commands` enum
- Remove `ClockCommands::Enqueue` variant
- Update help text to reflect new structure

**Variances from Plan:**
- TBD

**Test Results:**
- TBD

---

## Major Features (Significant Development)

These require substantial design and implementation work, potentially new subsystems.

---

### 5. Add `task sessions add` command for manual session entry

**Status:** ⏳ **PENDING**  
**Priority:** High  
**Estimated Effort:** 4-6 hours  
**Actual Effort:** TBD

**Current State:**
- Sessions are created automatically when clocking in/out
- No way to manually add sessions that weren't recorded (e.g., forgot to clock in, worked offline)

**Requested Changes:**
- Add `task sessions add` command
- Syntax: `task sessions add task:<task id> start:<time or datetime> end:<time or datetime> note:<note>`
- Support both labeled arguments (`task:1 start:9am end:10am`) and positional arguments
- Note (annotation) should be optional

**Design Considerations:**
- **Argument parsing:** Need flexible parser for labeled vs positional
- **Time parsing:** Support various formats (9am, 09:00, 2024-01-15 09:00, relative times)
- **Validation:** Ensure start < end, task exists, times are valid
- **Annotation:** Link note to session (via annotations table with session_id)
- **Positional vs labeled:** How to distinguish? (e.g., `task sessions add 1 9am 10am "note"` vs `task sessions add task:1 start:9am end:10am note:"note"`)

**Refinement:**
- **Syntax options:**
  1. Labeled only: `task sessions add task:1 start:9am end:10am [note:"note"]`
  2. Positional: `task sessions add <task_id> <start> <end> [note]`
  3. Hybrid: Support both (prefer labeled, fallback to positional)
- **Time formats:** Support common formats (9am, 09:00, 2024-01-15 09:00, relative like "2 hours ago")
- **Validation:**
  - Task must exist
  - Start < end
  - Times must be valid
  - Handle timezone (use local time by default)
- **Annotation:** Create annotation linked to session (if note provided)
- **Error handling:** Clear error messages for invalid inputs

**Implementation Checklist:**
- [ ] Add `Add` variant to `SessionsCommands` enum in `src/cli/commands.rs`
- [ ] Design argument parser for labeled/positional arguments
- [ ] Implement time/datetime parsing (reuse existing date utilities)
- [ ] Add `handle_sessions_add` function
- [ ] Validate task exists
- [ ] Validate start < end
- [ ] Create closed session via `SessionRepo::create_closed`
- [ ] Create annotation if note provided (link to session)
- [ ] Update `docs/COMMAND_REFERENCE.md` with examples
- [ ] Test: Labeled arguments work (`task sessions add task:1 start:9am end:10am`)
- [ ] Test: Positional arguments work (`task sessions add 1 9am 10am`)
- [ ] Test: Note is optional
- [ ] Test: Note creates annotation linked to session
- [ ] Test: Invalid task ID shows error
- [ ] Test: Start > end shows error
- [ ] Test: Various time formats work
- [ ] Test: Date + time formats work
- [ ] Test: Relative time formats work (if supported)

**Files to Create/Modify:**
- `src/cli/commands.rs` (add Add variant to SessionsCommands, implement handler)
- `src/cli/commands_sessions.rs` (add handle_sessions_add function)
- `src/cli/parser.rs` (may need to add session argument parser)
- `src/repo/session.rs` (verify create_closed works, may need annotation linking)
- `src/repo/annotation.rs` (may need to add session_id linking)
- `docs/COMMAND_REFERENCE.md` (document new command)
- `tests/sessions_tests.rs` (add comprehensive tests)

**Implementation Notes:**
- Reuse `parse_date_expr` from `src/utils/date.rs` for time parsing
- May need to extend date parser to handle time-only formats (9am, 09:00)
- Annotation linking: Check if annotations table has session_id column, or use a different mechanism
- Consider using clap's argument parsing for labeled arguments, or custom parser
- Positional arguments: Use `trailing_var_arg` and parse manually

**Variances from Plan:**
- TBD

**Test Results:**
- TBD

---

## Summary

### Completed Items
- None yet

### In Progress
- None yet

### Pending Items
1. Update clock command help information (Minor, Low priority)
2. Show Due as relative time in task list (Minor, Medium priority)
3. Add Clock column to task list (Medium, Medium priority)
4. Migrate `task clock enqueue` to `task enqueue` (Medium, Medium priority)
5. Add `task sessions add` command (Major, High priority)

### Estimated Total Effort
- Minor Changes: ~2-3 hours
- Medium Changes: ~4-6 hours
- Major Features: ~4-6 hours
- **Total: ~10-15 hours**

### Recommended Implementation Order
1. **Task 1** (Update clock help) - Quick win, low effort
2. **Task 4** (Migrate enqueue) - Medium effort, improves command structure
3. **Task 2** (Relative time) - Improves UX, medium effort
4. **Task 3** (Clock column) - Useful feature, medium effort
5. **Task 5** (Sessions add) - High value, most complex, do last

---

## Notes

- All tasks maintain backward compatibility where possible
- Abbreviation support should be added for new top-level commands
- Documentation should be updated for all changes
- Tests should be comprehensive for new features

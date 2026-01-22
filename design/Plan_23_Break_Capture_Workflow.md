# Plan 23: Break Capture Workflow - `offon` and `onoff` Commands

## Executive Summary

This plan introduces two new top-level commands (`offon` and `onoff`) to streamline the workflow for capturing breaks and manually adding sessions. These commands address the common scenario where a user is interrupted while working and needs to record the break before resuming, as well as providing a more ergonomic way to add historical sessions.

---

## Problem Statement

### Use Case 1: Unexpected Interruption

**Scenario:** You're working on a task, get pulled away unexpectedly, and don't sign off. Later, you want to capture that break before resuming work.

**Current Workflow:**
```bash
tatl off 14:30    # Stop current session at 14:30
tatl on           # Resume work now (starts new session)
```

**Problems:**
- Requires two separate commands
- Easy to forget the second step
- Verbose for a common operation

### Use Case 2: Manual Session Entry

**Scenario:** You want to add a historical session for a task (e.g., logging time from yesterday).

**Current Workflow:**
```bash
tatl sessions add 10 2026-01-20T09:00 2026-01-20T12:00
# or
tatl sessions add task:10 start:2026-01-20T09:00 end:2026-01-20T12:00
```

**Problems:**
- Requires `sessions` subcommand
- Verbose syntax with `start:`/`end:` prefixes
- Doesn't default to stack[0] task

---

## Proposed Solution

### Command 1: `offon` - Stop Current Session and Resume

**Purpose:** Atomically stop the current session and start a new one, optionally with a break period.

**Syntax:**
```bash
# Stop at <stop>, resume now (defaults to stack[0])
tatl offon <stop>

# Stop at <stop>, resume at <start> (interval notation)
tatl offon <stop>..<start>

# Stop at <stop>, resume now, for specific task
tatl offon <stop> <task_id>

# Stop at <stop>, resume at <start>, for specific task
tatl offon <stop>..<start> <task_id>
```

**Examples:**
```bash
# Interrupted at 14:30, resuming now
tatl offon 14:30

# Interrupted at 14:30, resuming at 15:00 (30 min break)
tatl offon 14:30..15:00

# Interrupted at 14:30, resuming now on task 5
tatl offon 14:30 5

# Interrupted at 14:30, resuming at 15:00 on task 5
tatl offon 14:30..15:00 5
```

**Behavior:**
1. If a session is currently running:
   - Close it at `<stop>` time
   - If `<start>` is provided, start new session at `<start>`
   - If `<start>` is omitted, start new session now
   - Default task is stack[0] unless `<task_id>` is provided
2. If no session is running:
   - Error: "No session is currently running"
3. Atomic operation: Both close and open must succeed or both fail

### Command 2: `onoff` - Add Historical Session

**Purpose:** Add a closed session for a task (replaces `sessions add`).

**Syntax:**
```bash
# Add session for stack[0] from <start> to <end>
tatl onoff <start>..<end>

# Add session for specific task from <start> to <end>
tatl onoff <start>..<end> <task_id>

# Add session with note
tatl onoff <start>..<end> [<task_id>] note:<text>
```

**Examples:**
```bash
# Add session for stack[0] from 09:00 to 12:00 today
tatl onoff 09:00..12:00

# Add session for task 10 from 09:00 to 12:00
tatl onoff 09:00..12:00 10

# Add session with note
tatl onoff 09:00..12:00 10 note:Fixed critical bug
```

**Behavior:**
1. Creates a closed session (not open)
2. Defaults to stack[0] task if task_id not provided
3. Requires interval notation (`<start>..<end>`) - both times required
4. Validates start < end
5. Creates annotation if note provided
6. **If the interval overlaps existing sessions**: Prompts for confirmation, then clears the overlapping time from affected sessions and inserts the new session (see Command 4 below)

### Command 3: `offon` Applied to History

**Purpose:** Remove time from historical sessions by automatically finding and modifying any overlapping sessions.

**Syntax:**
```bash
# Remove interval from history (finds all overlapping sessions)
tatl offon <stop>..<start>

# Split at single time point
tatl offon <time>
```

**Examples:**
```bash
# Remove 14:30-15:00 from all overlapping sessions
# If session 09:00-17:00 exists, result: 09:00-14:30 and 15:00-17:00
tatl offon 14:30..15:00

# Split all sessions at 14:30
# If session 09:00-17:00 exists, result: 09:00-14:30 and 14:30-17:00
tatl offon 14:30
```

**Behavior:**
1. **Finds all sessions overlapping with the provided interval/time**
2. **Applies modification based on overlap type:**
   - **Overlaps endpoints:** Shortens the intervals (truncates at overlap boundaries)
   - **Entirely includes interval(s):** Removes those intervals completely
   - **Falls within an interval:** Splits the interval (creates two sessions)
   - **Single time point:** Splits all overlapping sessions at that point
3. **No session_id needed** - automatically discovers affected sessions
4. **Allows reassociation** - after splitting, you can reassociate one interval with another task

**Overlap Scenarios:**

| Scenario | Example | Result |
|----------|---------|--------|
| **Overlaps start** | Session: 09:00-12:00<br>Remove: 10:00-11:00 | Session: 09:00-10:00, 11:00-12:00 (split) |
| **Overlaps end** | Session: 09:00-12:00<br>Remove: 11:00-13:00 | Session: 09:00-11:00 (shortened) |
| **Entirely within** | Session: 09:00-17:00<br>Remove: 14:30-15:00 | Session: 09:00-14:30, 15:00-17:00 (split) |
| **Entirely includes** | Sessions: 10:00-11:00, 11:00-12:00<br>Remove: 09:00-13:00 | Both sessions removed |
| **Single time point** | Session: 09:00-17:00<br>Split: 14:30 | Session: 09:00-14:30, 14:30-17:00 (split) |
| **No overlap** | Point or interval overlaps nothing | Alert user |

**User Confirmation:** When modifying existing sessions, the system prompts for confirmation before making changes. Use `-y` to bypass confirmation.

### Command 4: `onoff` Applied to History (Insertion)

**Purpose:** Insert a new session into an interval that overlaps with existing sessions. This handles the scenario where an interruption happened but wasn't recorded at the time - you need to insert a session for what actually happened during that time.

**Syntax:**
```bash
# Insert session for task into interval, modifying overlapping sessions
tatl onoff <start>..<end> <task_id>
```

**Examples:**
```bash
# You were working on task 10 (09:00-17:00) but got pulled into a meeting for task 5 (14:00-15:00)
# Original: Session for task 10: 09:00-17:00
# Command:
tatl onoff 14:00..15:00 5

# Result after confirmation:
# - Task 10 session: 09:00-14:00 (truncated)
# - Task 5 session: 14:00-15:00 (inserted)
# - Task 10 session: 15:00-17:00 (new, continuation)
```

**Behavior:**
1. Finds all sessions overlapping with the provided interval
2. **If overlaps exist**: Prompts for confirmation, showing what will be modified
3. Clears the interval from all overlapping sessions (same logic as `offon`)
4. Creates a new session for the specified task in the cleared interval
5. Result: The new session replaces whatever was in that time slot

**Difference from `offon`:**
- `offon <interval>`: Removes time, leaves gap (no new session)
- `onoff <interval> <task_id>`: Removes time AND inserts new session for specified task

### Command 5: `--onoff` Flag on `add`

**Purpose:** Create a new task AND log a historical session for it in one command. Useful when an interruption happened, you need to record what happened, and the task didn't previously exist.

**Syntax:**
```bash
tatl add "<description>" --onoff <start>..<end> [other fields...]
```

**Examples:**
```bash
# Got pulled into unexpected meeting, need to log it
tatl add "Emergency planning meeting" --onoff 14:00..15:00 project:meetings

# Got interrupted by support request
tatl add "Support: Fix customer login issue" --onoff 10:30..11:00 +support
```

**Behavior:**
1. Creates the task with specified description and fields
2. Creates a closed session for the new task in the specified interval
3. If the interval overlaps existing sessions, prompts for confirmation and modifies them
4. Equivalent to: `tatl add "..." && tatl onoff <interval> <new_task_id>`

**Interaction with existing flags:**
- `--on`: Start timing NOW (task added, session started)
- `--onoff <interval>`: Add HISTORICAL session (task added, session already closed)
- Can combine with other add options like `project:`, `+tag`, `due:`, etc.

---

## Design Decisions

### Decision 1: Interval Notation

**Decision:** Use `..` interval notation consistently.

**Rationale:**
- Aligns with Plan 22 decision to use interval syntax
- More concise than `start:`/`end:` prefixes
- Consistent with time range filters

### Decision 2: Default Task

**Decision:** Default to stack[0] for both commands.

**Rationale:**
- Most common use case is working on current task
- Reduces typing for frequent operations
- Can be overridden with explicit task_id

### Decision 3: Atomic Operations

**Decision:** `offon` must be atomic (both close and open succeed or both fail).

**Rationale:**
- Prevents inconsistent state
- Ensures accurate time tracking
- Matches user mental model (one operation)

### Decision 4: `onoff` Replaces `sessions add`

**Decision:** `onoff` becomes the primary way to add historical sessions. `sessions add` is removed.

**Rationale:**
- More ergonomic syntax
- Top-level command (faster to type)
- Consistent with `offon` naming pattern

### Decision 5: User Confirmation for History Modifications

**Decision:** Require user confirmation when modifying existing sessions. Support `-y` flag to bypass.

**Rationale:**
- Modifying history is potentially destructive
- User should see what will change before it happens
- Consistent with other destructive operations (delete, finish, etc.)
- `-y` flag allows scripting and fast confirmation

### Decision 6: `onoff` for Insertion (Clearing + Adding)

**Decision:** `onoff` can insert a session by clearing overlapping time and adding the new session.

**Rationale:**
- Common scenario: got interrupted, need to insert the interruption into recorded time
- Single command instead of `offon` + `onoff`
- Intuitive: "I was on, then off (the original task), then on (the new task)"

### Decision 7: `--onoff` Flag on `add`

**Decision:** Add `--onoff <interval>` flag to `add` command for creating task with historical session.

**Rationale:**
- Common scenario: unexpected interruption, need to create task AND log time
- Eliminates two-step process (add task, then log session)
- Consistent with `--on` flag pattern
- Handles overlap confirmation automatically

### Decision 8: History Modification

**Decision:** Support `offon` on historical sessions for splitting/editing.

**Rationale:**
- Common need to correct past sessions
- More intuitive than `sessions modify` for splitting
- Extends the command's utility

---

## Implementation Details

### Phase 1: Core `offon` Command

#### 1.1 Command Definition

Add to `src/cli/commands.rs`:

```rust
/// Stop current session and resume (with optional break)
Offon {
    /// Time expression or interval (e.g., "14:30" or "14:30..15:00")
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    time_args: Vec<String>,
    /// Task ID (optional, defaults to stack[0])
    task_id: Option<i64>,
}
```

#### 1.2 Argument Parsing

Parse time arguments:
- Single time: `14:30` → stop at 14:30, resume now
- Interval: `14:30..15:00` → stop at 14:30, resume at 15:00
- Task ID: Optional positional argument after time

#### 1.3 Implementation Logic

```rust
fn handle_offon(time_args: Vec<String>, task_id: Option<i64>) -> Result<()> {
    // 1. Parse time arguments
    let (stop_time, start_time) = parse_offon_time_args(time_args)?;
    
    // 2. Check if current session exists
    match get_current_session() {
        Ok(current_session) => {
            // Current session mode: stop and resume
            // 3. Close current session at stop_time
            close_session_at(current_session.id, stop_time)?;
            
            // 4. Determine resume task (task_id or stack[0])
            let resume_task = task_id.unwrap_or_else(|| get_stack_top()?);
            
            // 5. Start new session at start_time (or now if None)
            let start_ts = start_time.unwrap_or_else(|| now());
            start_session(resume_task, start_ts)?;
        }
        Err(_) => {
            // History mode: find and modify overlapping sessions
            handle_offon_history(time_args)?;
        }
    }
    
    Ok(())
}
```

#### 1.4 Error Handling

**Current Session Mode:**
- No current session: Automatically switches to history mode
- Invalid time format: "Invalid time expression: {expr}"
- Task not found: "Task {id} not found"
- Stack empty: "No tasks in queue. Enqueue a task first or specify a task ID."

**History Mode:**
- No overlapping sessions: "No sessions found overlapping with the specified time/interval ({time})."
- Invalid time format: "Invalid time expression: {expr}"

### Phase 2: Core `onoff` Command

#### 2.1 Command Definition

Add to `src/cli/commands.rs`:

```rust
/// Add historical session (replaces sessions add)
Onoff {
    /// Time interval (required, e.g., "09:00..12:00")
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    time_args: Vec<String>,
    /// Task ID (optional, defaults to stack[0])
    task_id: Option<i64>,
    /// Note for the session
    #[arg(long)]
    note: Option<String>,
}
```

#### 2.2 Argument Parsing

Parse interval:
- Must be interval format: `<start>..<end>`
- Parse both start and end times
- Extract optional task_id and note

#### 2.3 Implementation Logic

```rust
fn handle_onoff(time_args: Vec<String>, task_id: Option<i64>, note: Option<String>) -> Result<()> {
    // 1. Parse interval (must have ..)
    let (start_ts, end_ts) = parse_interval(time_args)?;
    
    // 2. Determine task (task_id or stack[0])
    let task = task_id.unwrap_or_else(|| get_stack_top()?);
    
    // 3. Validate task exists
    validate_task_exists(task)?;
    
    // 4. Validate start < end
    if start_ts >= end_ts {
        return Err("Start time must be before end time");
    }
    
    // 5. Create closed session
    let session = create_closed_session(task, start_ts, end_ts)?;
    
    // 6. Create annotation if note provided
    if let Some(note_text) = note {
        create_annotation(task, note_text, session.id)?;
    }
    
    Ok(())
}
```

### Phase 3: History Modification

#### 3.1 Extended `offon` Logic

When no current session is running, `offon` operates on history:

```rust
fn handle_offon_history(time_args: Vec<String>) -> Result<()> {
    // 1. Parse time argument (single time or interval)
    let (remove_start, remove_end) = parse_time_or_interval(time_args)?;
    
    // 2. Find all sessions overlapping with the interval
    let overlapping_sessions = find_overlapping_sessions(remove_start, remove_end)?;
    
    if overlapping_sessions.is_empty() {
        return Err("No sessions found overlapping with the specified time/interval");
    }
    
    // 3. Process each overlapping session
    for session in overlapping_sessions {
        modify_session_for_removal(session, remove_start, remove_end)?;
    }
    
    Ok(())
}
```

#### 3.2 Overlap Detection

```rust
fn find_overlapping_sessions(start: i64, end: i64) -> Result<Vec<Session>> {
    // Query all sessions that overlap with [start, end]
    // Overlap condition: session.start < end && session.end > start
    let sessions = SessionRepo::list_all(conn)?;
    
    sessions.into_iter()
        .filter(|s| {
            let s_start = s.start_ts;
            let s_end = s.end_ts.unwrap_or(i64::MAX); // Open sessions extend to infinity
            
            s_start < end && s_end > start
        })
        .collect()
}
```

#### 3.3 Session Modification Logic

```rust
fn modify_session_for_removal(session: Session, remove_start: i64, remove_end: i64) -> Result<()> {
    let s_start = session.start_ts;
    let s_end = session.end_ts.unwrap_or(i64::MAX);
    
    // Determine overlap type and modify accordingly
    if remove_start <= s_start && remove_end >= s_end {
        // Entirely includes: remove session completely
        SessionRepo::delete(&conn, session.id)?;
        
    } else if remove_start > s_start && remove_end < s_end {
        // Falls within: split into two sessions
        // First part: s_start to remove_start
        update_session(session.id, s_start, remove_start)?;
        // Second part: remove_end to s_end
        create_closed_session(session.task_id, remove_end, s_end)?;
        
    } else if remove_start <= s_start {
        // Overlaps start: truncate at remove_end
        if remove_end < s_end {
            update_session(session.id, remove_end, s_end)?;
        } else {
            // Entirely includes (already handled above, but for safety)
            SessionRepo::delete(&conn, session.id)?;
        }
        
    } else if remove_end >= s_end {
        // Overlaps end: truncate at remove_start
        if remove_start > s_start {
            update_session(session.id, s_start, remove_start)?;
        } else {
            // Entirely includes (already handled above, but for safety)
            SessionRepo::delete(&conn, session.id)?;
        }
        
    } else {
        // Single time point: split at that point
        update_session(session.id, s_start, remove_start)?;
        create_closed_session(session.task_id, remove_start, s_end)?;
    }
    
    Ok(())
}
```

#### 3.4 Single Time Point Handling

When a single time is provided (not an interval):

```rust
fn parse_time_or_interval(time_args: Vec<String>) -> Result<(i64, i64)> {
    if time_args.len() == 1 {
        // Single time point
        let time = parse_date_expr(&time_args[0])?;
        Ok((time, time)) // Same start and end = split point
    } else {
        // Interval
        parse_interval(time_args)
    }
}
```

---

## Edge Cases and Validation

### Edge Case 1: No Current Session (History Mode)

**Scenario:** User runs `tatl offon 14:30` but no session is running.

**Handling:** Automatically switch to history mode - find and modify any sessions overlapping with 14:30. If no overlapping sessions found, error: "No sessions found overlapping with the specified time (14:30)."

### Edge Case 2: Stack Empty (Current Session Mode)

**Scenario:** User runs `tatl offon 14:30` with a current session, but stack is empty when trying to resume.

**Handling:** Error: "No tasks in queue. Enqueue a task first or specify a task ID."

### Edge Case 3: No Overlapping Sessions

**Scenario:** User runs `tatl offon 20:00..21:00` but no sessions exist in that time range.

**Handling:** Error: "No sessions found overlapping with the specified time/interval (20:00-21:00)."

### Edge Case 4: Overlapping Sessions

**Scenario:** `offon` creates a session that overlaps with existing sessions.

**Handling:** 
- Check for overlaps (same as `sessions modify`)
- Error if overlap detected (unless `--force` flag added)
- Or: Auto-adjust adjacent sessions (future enhancement)

### Edge Case 5: Micro-Session Rules

**Scenario:** `offon` creates a very short session (< 30 seconds).

**Handling:** Apply existing micro-session merge/purge rules from `SessionRepo`.

---

## Testing Scenarios

### Test 1: Basic `offon` - Stop and Resume Now

```bash
# Setup: Task 1 is clocked in
tatl add "Test task"
tatl enqueue 1
tatl on

# Test: Stop at 14:30, resume now
tatl offon 14:30

# Verify:
# - Session 1: started -> 14:30 (closed)
# - Session 2: now -> (open)
```

### Test 2: `offon` with Break Period

```bash
# Setup: Task 1 is clocked in
tatl on

# Test: Stop at 14:30, resume at 15:00
tatl offon 14:30..15:00

# Verify:
# - Session 1: started -> 14:30 (closed)
# - Session 2: 15:00 -> (open)
```

### Test 3: `onoff` - Add Historical Session

```bash
# Setup: Task 1 in stack
tatl add "Test task"
tatl enqueue 1

# Test: Add session 09:00-12:00
tatl onoff 09:00..12:00

# Verify:
# - Session created: 09:00-12:00 (closed)
# - Task 1 has session
```

### Test 4: `onoff` with Specific Task

```bash
# Test: Add session for task 5
tatl onoff 09:00..12:00 5

# Verify:
# - Session created for task 5
```

### Test 5: `offon` on History - Split Session

```bash
# Setup: Session exists (09:00-17:00)
# Test: Remove 14:30-15:00 (automatically finds session)
tatl offon 14:30..15:00

# Verify:
# - Original session: 09:00-14:30 (updated)
# - New session: 15:00-17:00 (created)
```

### Test 6: `offon` on History - Remove Entire Session

```bash
# Setup: Sessions exist (09:00-12:00, 13:00-17:00)
# Test: Remove 09:00-13:00 (overlaps both)
tatl offon 09:00..13:00

# Verify:
# - First session: removed (entirely included)
# - Second session: 13:00-17:00 (truncated at start)
```

### Test 7: `offon` on History - Split at Single Time Point

```bash
# Setup: Session exists (09:00-17:00)
# Test: Split at 14:30
tatl offon 14:30

# Verify:
# - Original session: 09:00-14:30 (updated)
# - New session: 14:30-17:00 (created)
```

### Test 8: `offon` on History - Multiple Overlapping Sessions

```bash
# Setup: Multiple sessions (09:00-12:00, 11:00-14:00, 13:00-17:00)
# Test: Remove 11:30-13:30
tatl offon 11:30..13:30

# Verify:
# - First session: 09:00-11:30 (truncated)
# - Second session: removed (entirely included)
# - Third session: 13:30-17:00 (truncated)
```

### Test 9: History Mode - No Overlapping Sessions

```bash
# Test: offon with no session running and no overlapping sessions
tatl offon 20:00..21:00

# Expected: Error "No sessions found overlapping with the specified time/interval (20:00-21:00)"
```

### Test 10: History Mode - Reassociate After Split

```bash
# Setup: Session 1 (09:00-17:00) for task 10
# Test: Split at 14:30
tatl offon 14:30

# Result: Two sessions (09:00-14:30, 14:30-17:00) both for task 10
# User can then reassociate one with another task using sessions modify
```

### Test 11: Error - Invalid Interval

```bash
# Test: onoff with invalid interval
tatl onoff 14:30

# Expected: Error "Interval required (use <start>..<end>)"
```

### Test 12: `onoff` Insertion - Replace Overlapping Time

```bash
# Setup: Session for task 10 (09:00-17:00)
# Test: Insert meeting (task 5) that happened 14:00-15:00
tatl onoff 14:00..15:00 5

# Confirmation prompt:
# "This will modify 1 session(s). Continue? [y/N]"
# User: y

# Result:
# - Task 10 session: 09:00-14:00 (truncated)
# - Task 5 session: 14:00-15:00 (inserted)
# - Task 10 session: 15:00-17:00 (new, continuation)
```

### Test 13: `onoff` Insertion with `-y` Flag

```bash
# Setup: Session for task 10 (09:00-17:00)
# Test: Insert without confirmation
tatl onoff 14:00..15:00 5 -y

# Result: Same as above, no prompt
```

### Test 14: `add --onoff` - Create Task with Historical Session

```bash
# Test: Create new task and log session
tatl add "Emergency planning meeting" --onoff 14:00..15:00 project:meetings

# Result:
# - Task created (e.g., ID 20)
# - Session created: 14:00-15:00 for task 20
# - If overlapping sessions existed, prompt for confirmation first
```

### Test 15: `add --onoff` with Overlap

```bash
# Setup: Session for task 10 (09:00-17:00)
# Test: Create new task and insert into overlapping time
tatl add "Support request" --onoff 10:30..11:00 +support

# Confirmation prompt:
# "This will modify 1 session(s). Continue? [y/N]"
# User: y

# Result:
# - Task created (e.g., ID 21)
# - Task 10 session: 09:00-10:30 (truncated)
# - Task 21 session: 10:30-11:00 (inserted)
# - Task 10 session: 11:00-17:00 (new, continuation)
```

### Test 16: User Confirmation - Decline

```bash
# Setup: Session for task 10 (09:00-17:00)
# Test: Try to modify but decline
tatl offon 14:00..15:00

# Confirmation prompt:
# "This will modify 1 session(s). Continue? [y/N]"
# User: n

# Result: No changes made, original session intact
```

---

## Migration and Compatibility

### Backward Compatibility

**Decision:** Keep `sessions add` for backward compatibility, but document `onoff` as preferred.

**Rationale:**
- Existing scripts may use `sessions add`
- Gradual migration path
- No breaking changes

### Documentation Updates

1. Update `COMMAND_REFERENCE.md`:
   - Add `offon` command documentation
   - Add `onoff` command documentation
   - Mark `sessions add` as "legacy" (prefer `onoff`)

2. Update `README.md`:
   - Add examples of break capture workflow
   - Show `offon` and `onoff` in quick start

3. Update help text:
   - Add examples to command help
   - Show interval syntax

---

## Implementation Priority

### Phase 1: Core Commands (High Priority) ✓
- [x] Implement `offon` for current session (stop and resume)
- [x] Implement `onoff` for historical sessions (simple add, no overlap)
- [x] Add interval parsing (`<start>..<end>`)
- [x] Add tests for basic functionality
- [x] Update documentation

### Phase 2: History Modification (High Priority) ✓
- [x] Implement automatic history mode for `offon` (when no current session)
- [x] Implement overlap detection and session finding
- [x] Implement session modification logic (split, truncate, remove)
- [x] Handle single time point splitting
- [x] Add user confirmation prompt for modifications
- [x] Add `-y` flag to bypass confirmation
- [x] Add tests for history modification scenarios

### Phase 3: Insertion Mode (Medium Priority) ✓
- [x] Implement `onoff` insertion mode (clear overlapping + insert)
- [x] Handle multi-session overlap with insertion
- [x] Preserve task association on split remainder sessions
- [x] Add tests for insertion scenarios

### Phase 4: `--onoff` Flag on `add` (Medium Priority) ✓
- [x] Add `--onoff <interval>` flag to `add` command
- [x] Handle overlap detection and confirmation
- [x] Create task and session atomically
- [x] Add tests for add with historical session

### Phase 5: Polish (Low Priority) ✓
- [x] Improve confirmation messages with session details (shows duration, modification type)
- [x] Update README.md to reflect CLI changes
- [ ] Add validation for micro-sessions (deferred - existing micro-session rules apply)
- [ ] Performance optimization for large histories (deferred - not needed for single user)

---

## Open Questions

### Question 1: Command Naming Conflict

**Issue:** Both `offon` and `onoff` could be abbreviated to `oo`.

**Options:**
- A: Use different abbreviations (`of` for `offon`, `on` for `onoff` - but `on` conflicts with existing command)
- B: No abbreviations (type full commands)
- C: Use different names (`break` and `add-session`?)

**Recommendation:** Option B - No abbreviations. The commands are short enough, and abbreviations would be confusing.

### Question 2: `sessions add` Deprecation

**Issue:** Should `sessions add` be deprecated immediately or kept indefinitely?

**Options:**
- A: Deprecate immediately (remove in next major version)
- B: Keep indefinitely (backward compatibility)
- C: Keep but mark as legacy in docs only

**Recommendation:** Option C - Keep but mark as legacy. Single user, can remove later if desired.

### Question 3: History Modification Auto-Discovery

**Issue:** Automatically finding overlapping sessions could modify multiple sessions at once. Is this desired behavior?

**Options:**
- A: Always modify all overlapping sessions (current design)
- B: Require confirmation if multiple sessions would be affected
- C: Only modify if exactly one session overlaps (error if multiple)

**Recommendation:** Option A - Always modify all overlapping sessions. This is the most intuitive behavior - you specify the time to remove, and it removes it from all affected sessions. User can review results with `sessions list` if needed.

---

## Success Criteria

1. ✅ `offon` successfully stops current session and resumes
2. ✅ `offon` automatically switches to history mode when no current session
3. ✅ `offon` finds and modifies all overlapping sessions in history
4. ✅ `onoff` successfully adds historical sessions (simple mode)
5. ✅ `onoff` clears overlapping time and inserts new session (insertion mode)
6. ✅ User confirmation required for history modifications
7. ✅ `-y` flag bypasses confirmation
8. ✅ `add --onoff` creates task with historical session
9. ✅ Both commands default to stack[0] correctly
10. ✅ Interval notation works correctly
11. ✅ Single time point splits sessions correctly
12. ✅ All edge cases handled gracefully
13. ✅ Tests pass
14. ✅ Documentation updated

---

## Appendix: Command Syntax Reference

### `offon` Syntax

**Current Session Mode** (when a session is running):
```bash
# Stop at <stop>, resume now
tatl offon <stop>

# Stop at <stop>, resume at <start>
tatl offon <stop>..<start>

# Stop at <stop>, resume now, specific task
tatl offon <stop> <task_id>

# Stop at <stop>, resume at <start>, specific task
tatl offon <stop>..<start> <task_id>
```

**History Mode** (when no session is running - automatically finds overlapping sessions):
```bash
# Remove interval from all overlapping sessions
tatl offon <stop>..<start>

# Split all overlapping sessions at single time point
tatl offon <time>
```

### `onoff` Syntax

**Simple Mode** (no overlapping sessions):
```bash
# Add session for stack[0]
tatl onoff <start>..<end>

# Add session for specific task
tatl onoff <start>..<end> <task_id>

# Add session with note
tatl onoff <start>..<end> [<task_id>] --note "Note text"
```

**Insertion Mode** (overlapping sessions exist - prompts for confirmation):
```bash
# Clear overlap and insert session for task (requires confirmation)
tatl onoff <start>..<end> <task_id>

# Skip confirmation
tatl onoff <start>..<end> <task_id> -y
```

### `add --onoff` Syntax

```bash
# Create task and add historical session
tatl add "<description>" --onoff <start>..<end> [fields...]

# Create task, add session, clear overlaps (requires confirmation if overlaps)
tatl add "<description>" --onoff <start>..<end> -y [fields...]
```

---

## Related Plans

- **Plan 22:** CLI Syntax Review (interval notation decision)
- **Plan 10:** Session Modification and Deletion (overlap detection)
- **Plan 13:** User Feedback (micro-session rules)

---

## Next Steps

1. ☑ Review and approve plan
2. ☑ Implement Phase 1 (core commands)
3. ☑ Implement Phase 2 (history modification)
4. ☑ Implement Phase 3 (insertion mode)
5. ☑ Implement Phase 4 (`--onoff` flag on add)
6. ☑ Implement Phase 5 (polish: improved messages, README update)
7. ☑ Add tests
8. ☑ Update documentation

**Status: COMPLETE** (2026-01-21)

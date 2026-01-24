# Plan 31: CLI Exploration and Issues

## Summary

This document captures findings from a systematic exploration of the tatl CLI from a user discoverability standpoint. The goal was to identify bugs, unexpected behavior, unimplemented features, inconsistencies in syntax, and potential user surprises.

---

## Critical Bugs

### 1. ID Range Syntax Not Implemented
**Severity:** High

**Observation:** The help documentation for `show`, `modify`, `finish`, `delete`, and other commands shows ID range syntax (e.g., `1..5`) as valid, but it fails with "Invalid filter token".

**Example:**
```bash
$ tatl show 1..3
Error: Filter parse error: Invalid filter token: 1..3

$ tatl modify 1..3 +tag -y
Error: Filter parse error: Invalid filter token: 1..3
```

**Help Documentation (from `tatl show --help`):**
```
TARGET SYNTAX:
  Single ID:       10
  ID range:        1..5 (tasks 1, 2, 3, 4, 5)   # <-- Documented but not working
  ID list:         1,3,5 (tasks 1, 3, and 5)
```

**Impact:** Users following documentation will hit errors. ID list syntax (`1,3,5`) works correctly.

**Decision:** Distinguish between numerical RANGES and date, datetime, or time INTERVALS.

- Ranges notation (1-5) for indexes, numeric ids, etc.
- Interval notation (e.g. 1..5) for date, datetime, or time intervals.

Update behavior and documentation to match this decision. This should apply throughout the CLI and user documentation.

---

### 2. kanban:NEXT and kanban:LIVE Filters Don't Match
**Severity:** High

**Observation:** When a task is at queue[0] and actively being timed (shown with `▶` indicator), filtering by `kanban:LIVE` or `kanban:NEXT` returns no results.

**Example:**
```bash
$ tatl on 2
Started timing task 2: Task two

$ tatl list | grep "Task two"
2    ▶    Task two    pending queued   work   ...   1m25s

$ tatl list kanban:LIVE
No tasks found.

$ tatl list kanban:NEXT
No tasks found.
```

**Analysis:** The display correctly shows the `▶` indicator and elapsed time, but the kanban filter logic doesn't recognize these states. The kanban column shows `queued` instead of `LIVE` even when the task is actively running.

**Root Cause:** Likely the kanban field value stored/computed doesn't include LIVE/NEXT as separate states - they may be display-only concepts.

**Decision:** The LIVE and NEXT kanban stages have been intentionally removed. Their related filters should also be removed. It is not now possible to filter by queue position, e.g. queue:1 or queue:2-4, but this could cover that removed capability. Please add this capability if it is consistent with the overall behavior of the CLI. Note outcome in Implementation document.

---

### 3. kanban:paused vs kanban:stalled Naming Inconsistency
**Severity:** Medium

**Observation:** The help documentation references `kanban:paused` but the actual kanban status is `stalled`.

**Help Documentation (from `tatl list --help`):**
```
kanban:<status>  - Match by kanban status (proposed, paused, queued, NEXT, LIVE, done)
```

**Actual Behavior:**
```bash
$ tatl list kanban:paused
No tasks found.

$ tatl list kanban:stalled
ID   Q    Description Status  Kanban  Project ...
2         Task two    pending stalled work    ...
```

**Impact:** Users following documentation will get no results when filtering for paused tasks.

**Decision:** Update documentation to match the actual kanban stages (stalled is now correct, not paused. The code is right, not the documentation on this).

---

### 4. Sessions with Negative Duration Display
**Severity:** Medium

**Observation:** Sessions can display with negative duration values, indicating data corruption or timezone issues.

**Example:**
```bash
$ tatl sessions list
Session ID Task   Description Start               End                 Duration
4          3      Task three  2026-01-24 10:00:00 2026-01-24 00:05:12 -48s
7          15     Test task   2026-01-24 09:00:00 2026-01-24 00:10:19 -41s
```

**Analysis:** The end timestamp (00:05:12) is earlier than the start timestamp (10:00:00), which should be impossible for valid sessions. This may indicate:
- Timezone handling issues
- Improper session splitting during `onoff` operations
- Data entry edge cases with historical sessions

**Decision:** Do not allow sessions with non-chronological start and ends. Figure out what scenarios allow these broken sessions to be created to make sure that is not allowed.

---

### 5. queue sort `-field` Syntax Fails
**Severity:** Medium

**Observation:** The documented syntax for descending sort (`-priority`, `-due`) doesn't work because clap interprets the `-` as a flag prefix.

**Help Documentation (from `tatl queue sort --help`):**
```
tatl queue sort -priority    - Sort by priority (descending)
```

**Actual Behavior:**
```bash
$ tatl queue sort -priority
error: unexpected argument '-p' found

  tip: to pass '-p' as a value, use '-- -p'
```

**Workaround:** Using `-- -priority` works:
```bash
$ tatl queue sort -- -priority
Queue sorted by priority (descending)
```

**Fix:** Either document the `--` separator requirement, or change the syntax to `priority:desc` or `--desc priority`.

**Decision:** Drop the queue sort capability from the code and documentation. This feature is not a good idea. Instead uses, can just run `tatl enqueue <comma separated list of ids>` to sort.

---

## Unimplemented or Incomplete Features

### 6. sessions show Doesn't Accept Session ID
**Severity:** Low

**Observation:** `sessions show` only shows the current active session. It doesn't accept a session ID to show details of any session.

**Help Text (misleading):**
```
sessions  show    Show detailed session information
```

**Actual Behavior:**
```bash
$ tatl sessions show 1
error: unexpected argument '1' found
Usage: tatl sessions show
```

**Expectation:** Based on the help, users might expect to view details of any session by ID.

---

### 7. sessions report Date Range Syntax Not Working
**Severity:** Medium

**Observation:** The documented `start:DATE..end:DATE` syntax for sessions report fails.

**Help Documentation:**
```
Date range: -7d, -7d..now, start:2024-01-01..end:2024-01-31
```

**Actual Behavior:**
```bash
$ tatl sessions report start:2026-01-24..end:2026-01-25
Error: Filter parse error: Invalid filter token: start:2026-01-24..end:2026-01-25

$ tatl sessions report start:2026-01-24
Error: Filter parse error: Invalid filter token: start:2026-01-24
```

The simple `-7d` syntax works, but the explicit `start:`/`end:` format does not.

**Decision:** `start:`/`end:` format was intentionally removed. Update syntax to reflect. We are going all in on interval syntax for intervals.

---

### 8. Template Field Has No Associated Commands
**Severity:** Low

**Observation:** The `template:<name>` field can be set on tasks, but there are no commands to manage templates (list, create, apply).

```bash
$ tatl add "Task" template:mytemplate
Created task 19: Task

$ tatl show 19
...
  Template:    mytemplate
...
```

The field is stored but appears non-functional. No `tatl templates` subcommand exists.

**Question:** Is this feature planned for future implementation, or should the field be removed from documentation?

**Decision:** Document what you found, but take no action at this time. We don't need it, but I'd want to make sure we clean it up everywhere it may exist (documentation, database, tasks, etc.). Just note a plan in the Implementation document.

---

### 9. Priority Cannot Be Set Manually
**Severity:** Low

**Observation:** The task list displays a Priority column, but there's no `priority:` field syntax to set it.

```bash
$ tatl modify 1 priority:5
Error: Unrecognized field token 'priority:5'
```

**Analysis:** Priority appears to be auto-calculated (possibly based on due date, allocation, tags, etc.). This should be documented more clearly, or the ability to set priority manually should be added.

**Decision:** Document more clearly that the priority score is automatically calculated. If you can expose the scoring in the `task show <id>` output, that would be helpful.

---

## Input Validation Issues

### 10. Empty Project Name Creates "none" Project
**Severity:** Medium

**Observation:** Using `project:` with no value creates a project literally named "none".

```bash
$ tatl add -y 'test' project:
Created project 'none' (id: 5)
Created task 11: test
```

**Expected:** Error message indicating project name is required.

**Decision:** Using project with no value in add or modify should assign no project and if modifying remove any associated projects from the task.

---

### 11. Empty Tag Is Silently Accepted
**Severity:** Low

**Observation:** Using `+` with no tag name creates a task with an empty tag.

```bash
$ tatl add -y 'test' +
Created task 12: test

$ tatl list
...
12   test   pending proposed   +   ...
```

The tag column shows `+` as a tag name.

**Expected:** Error message indicating tag name is required.

**Decision:** Error message indicating tag name is required.

---

### 12. Invalid Date Gives "Internal error"
**Severity:** Low

**Observation:** Invalid date expressions show "Internal error" rather than a user-friendly error.

```bash
$ tatl add -y 'test' due:invaliddate
Internal error: Failed to parse due date

Caused by:
   Unsupported date expression: invaliddate
```

**Expected:** User error (not "Internal error") with suggestions for valid date formats.

**Decision:** User error (not "Internal error") with suggestions for valid date formats.

---

## Inconsistencies

### 13. Error Message Format Inconsistency
**Severity:** Low

**Observation:** Different error types use different prefixes inconsistently:
- User errors: `Error: ...`
- System errors: `Internal error: ...`

Some errors that should be user errors are shown as internal errors (like invalid date above).

**Decision:** Ensure capitalization and error messaging is accurate and follows consistent patterns.

---

### 14. Man Page Missing Commands
**Severity:** Low

**Observation:** The man page (`tatl.1`) doesn't list `send`, `collect`, or `externals` commands in the subcommands section.

```
$ man ./man/man1/tatl.1
...
SUBCOMMANDS
       tatl-projects(1)
       tatl-add(1)
       ...
       (no send, collect, or externals listed)
```

**Decision:** Fix this.

---

### 15. sessions list vs sessions report Filtering
**Severity:** Low

**Observation:** Different filter syntax between list and report:

- `tatl sessions list project:work` - Works ✓
- `tatl sessions list +urgent` - Works ✓
- `tatl sessions list -7d` - Works ✓
- `tatl sessions report project:work` - Works ✓ (but less reliable)
- `tatl sessions report +urgent` - Works ✓
- `tatl sessions report -7d..now` - Works ✓
- `tatl sessions report start:DATE` - Fails ✗

The `start:`/`end:` prefix syntax from the help doesn't work in practice.

**Decision:** Unify the filter syntax behavior between list and report. But remember that we are dropping start and end terms. For an open interval, you can use ..DATE (like end:DATE) or DATE.. (like start:DATE). If a single date makes sense, that should be allowed instead of an interval.

---

## Minor Issues & User Surprises

### 16. Micro-Session Warning Shows Negative Duration
**Severity:** Low

**Observation:** When turning off a session that was started in the "past" (e.g., `--on=09:00`), the micro-session warning shows negative duration.

```bash
$ tatl add -y "Test" --on=09:00
Created task 15: Test
Started timing task 15: Test

$ tatl off
Warning: Micro-session detected (-31781s). This session may be merged or purged...
```

**Decision:** Debug and fix.

---

### 17. `waiting` Filter Includes Closed Tasks
**Severity:** Low

**Observation:** The `waiting` filter shows closed tasks that have a wait_ts set.

```bash
$ tatl list waiting
ID   Q    Description  Status Kanban Project ...
8         Waiting task closed done   ...
```

**Question:** Should `waiting` exclude completed/closed tasks? That's a weird state to get into in the first place. When a task is completed or closed, its waiting status should probably be cleared. Please resolve by ensuring that closing or completing clears the waiting.

---

### 18. Abbreviation Ambiguity: `de` matches both dequeue and delete
**Severity:** Low (Correctly handled)

**Observation:** This is actually working correctly - the CLI warns about ambiguity:

```bash
$ tatl de 5
Error: Ambiguous command 'de'. Did you mean one of: dequeue, delete?
```

This is good behavior, just noting it for completeness.

---

## Summary Table

| # | Issue | Severity | Type |
|---|-------|----------|------|
| 1 | ID range syntax (1..5) not implemented | High | Bug |
| 2 | kanban:NEXT/LIVE filters don't match | High | Bug |
| 3 | kanban:paused vs stalled naming | Medium | Inconsistency |
| 4 | Sessions with negative duration | Medium | Bug |
| 5 | queue sort -field syntax fails | Medium | Bug |
| 6 | sessions show no argument | Low | Incomplete |
| 7 | sessions report start: syntax fails | Medium | Bug |
| 8 | Template field not functional | Low | Incomplete |
| 9 | Priority not settable | Low | Missing feature |
| 10 | Empty project: creates "none" | Medium | Validation |
| 11 | Empty + tag accepted | Low | Validation |
| 12 | Invalid date shows "Internal error" | Low | UX |
| 13 | Error format inconsistency | Low | Inconsistency |
| 14 | Man page missing commands | Low | Documentation |
| 15 | sessions report filter inconsistency | Low | Inconsistency |
| 16 | Negative micro-session warning | Low | Display bug |
| 17 | waiting includes closed tasks | Low | Unclear behavior |
| 18 | 'de' abbreviation ambiguous | Low | Working correctly |

---

## Recommendations

### Priority 1 (Breaking User Workflows)
1. **Fix ID range parsing** - Implement `1..5` syntax as documented, or remove from help
2. **Fix kanban filter states** - Make LIVE, NEXT work as documented, or clarify they are display-only
3. **Update kanban:paused → kanban:stalled** - Align help text with actual values

### Priority 2 (Confusing User Experience)
4. **Fix queue sort syntax** - Either document `--` requirement or use alternative syntax
5. **Fix sessions report date syntax** - Implement start:/end: or remove from docs
6. **Validate empty project/tag** - Reject `project:` and `+` without values
7. **Improve error messages** - Use "Error:" consistently for user input errors

### Priority 3 (Documentation & Polish)
8. **Update man page** - Add send, collect, externals commands
9. **Clarify sessions show** - Either add argument support or update help text
10. **Document priority calculation** - Explain how priority score is derived
11. **Decide on template feature** - Implement or remove from field list

---

## Test Commands Used

All tests were run with isolated database:
```bash
TATL_BIN="$(pwd)/target/debug/tatl"
HOME=/tmp/tatl_test_explore $TATL_BIN <command>
```

This ensures no interference with real user data.

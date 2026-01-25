# Plan 32: README and CLI Documentation Deviations

This document catalogs deviations between the README.md, COMMAND_REFERENCE.md documentation, and actual CLI behavior discovered during verification testing.

## Summary

Testing revealed **3 critical bugs**, **3 README documentation issues**, **3 COMMAND_REFERENCE.md issues**, and **2 display/UX issues**.

---

## Critical Bugs

### 1. Session Close Failures

**Symptom:** `tatl off`, `tatl finish`, and `tatl offon` frequently fail with "Failed to close session" error.

**Reproduction:**
```bash
tatl on 1
tatl off        # Sometimes fails
tatl finish     # Fails with "Failed to close session"
tatl offon 14:30  # Fails with "Failed to close session"
```

**Impact:** Core timing functionality is unreliable. Users cannot consistently stop timing or complete tasks.

**Recommended Fix:** Investigate session close logic in the session repository. The error appears related to micro-session handling and session state management.

**Decision:** Need to test is this holds true at human time scales.

---

### 2. `add --on=<time>` Fails

**Symptom:** `tatl add --on=14:00` creates the task but fails to start timing.

**Reproduction:**
```bash
tatl add "Meeting" --on=14:00 project:work -y
# Output: Created task 4: Meeting
#         Error: Failed to start timing task
```

**Expected:** Task is created and timing starts at 14:00.

**Impact:** The `--on=<time>` feature documented in README doesn't work.

**Recommended Fix:** Debug the session start logic when a specific time is provided with `--on=<time>`.

**Decision:** Do this. But recognize that this should only work for times in the past. There are no future-looking start specifications.

---

### 3. Negative Duration Sessions with `on <time>`

**Symptom:** `tatl on 09:00` creates sessions with start time AFTER end time, resulting in negative durations.

**Reproduction:**
```bash
tatl on 1
tatl on 09:00    # If current time is after 09:00
tatl sessions list
# Shows: 2026-01-24 15:31:12 - 2026-01-24 09:00:00 (-12s)
```

**Expected:** Session should start at 09:00 (either today if in the future, or interpret as "start session with start time set to 09:00 today").

**Impact:** Time tracking data becomes corrupted with negative durations.

**Recommended Fix:** The time parsing for `on <time>` needs to properly handle the case where the specified time has already passed today. Either reject it, or interpret it correctly.

**Decision:** `tatl on <time>` should always default to a time before now. There is no starting of tasks in the future. It should actually reject times that are in the future. The documentation should be clear that specifying times allow specifications in the past, not the future.
---

## README Documentation Issues

### 1. `projects report` Shows Old Kanban Status Names

**Current Output:**
```
Project                   Proposed   Queued   Paused   NEXT   LIVE   Done  Total
```

**Expected (per README):**
The README was updated to document `proposed, stalled, queued, external, done` but `projects report` still displays `Paused`, `NEXT`, `LIVE`.

**Location:** `src/cli/commands.rs` or `src/cli/output.rs` - the `projects report` command.

**Recommended Fix:** Update `projects report` to use the new kanban status names: `proposed`, `stalled`, `queued`, `external`, `done`.

**Decision:** Agreed.

---

### 2. `sessions modify` Syntax Mismatch

**README Documents:**
```bash
tatl sessions modify 5 start:09:00..end:17:00  # Adjust both times
tatl sessions modify 5 end:17:00    # Adjust end time only
tatl sessions modify 5 start:09:00  # Adjust start time only
```

**Actual CLI Behavior:**
```bash
tatl sessions modify 5 start:09:00..end:17:00
# Error: Failed to parse start time: start:09:00

tatl sessions modify 5 09:00..17:00
# This is the actual expected syntax (plain intervals)
```

**CLI Help Shows:**
```
INTERVAL SYNTAX:
  start:<time>..end:<time>  - Modify both start and end
  start:<time>              - Modify only start time
  end:<time>                - Modify only end time
```

**Status:** The CLI help and COMMAND_REFERENCE.md document the `start:<time>..end:<time>` syntax, but it doesn't actually work. The CLI appears to expect plain interval syntax.

**Recommended Fix:** Either:
- A) Update the CLI to accept `start:` and `end:` prefixes as documented
- B) Update all documentation to reflect the actual plain interval syntax

**Decision:** B - update documentation to exclusively reference the plain interval syntax.

---

### 3. `-y` Flag Position for `sessions modify`

**README Documents:**
```bash
tatl sessions modify 5 09:00..17:00 -y
```

**Actual Behavior:**
```bash
tatl sessions modify 5 09:00..17:00 -y
# Error: Invalid argument: -y. Use interval syntax...
```

**Issue:** The `-y` flag is being interpreted as part of the interval arguments rather than as a flag.

**Recommended Fix:** Fix argument parsing so `-y`/`--yes` is recognized regardless of position, or document the required position.

**Decision:** Agreed

---

## COMMAND_REFERENCE.md Documentation Issues

### 1. `--add-alias` Option Doesn't Exist

**Documented:**
```bash
tatl list project:work sort:project --add-alias mywork
tatl sessions list project:work sort:start --add-alias worksessions
```

**Actual:** The `--add-alias` option is not present in the CLI help for `list` or `sessions list`.

**Recommended Fix:** Either implement the feature or remove from documentation.

**Decision:** Check if alias:<alias name> works in these scenarios. Present the comparative merits of both syntaxes, but do not implement yet.

---

### 2. `sessions list start:<date>` Filter Inconsistent

**Documented:**
```bash
tatl sessions list start:today      # Works
tatl sessions list start:-7d        # Returns no results
tatl sessions list -7d              # Works correctly
```

**Issue:** The `start:` prefix syntax works for `today` but not for relative dates like `-7d`. The plain date format (`-7d`) works.

**Recommended Fix:** Either fix `start:` prefix to work consistently, or update documentation to use plain date filter syntax.

**Decision:** You should be able to filter either the start or end dates in the table. If a single date is provided or relative date, it should be assumed that the intent is for after that date. But it should also accept intervals of dates or times, whether relative or absolute. This would allow you to zero in on a particular time range.

---

### 3. Legacy `sessions modify` Syntax Not Implemented

**Documented as "Legacy syntax":**
```bash
tatl sessions modify 5 start:09:00 end:17:00
```

**Actual:** This syntax doesn't work. Only interval syntax is accepted.

**Recommended Fix:** Remove legacy syntax from documentation since it's not implemented, or implement it.

**Decision:** On document and support interval syntax, either as 09:00..17:00 (two sided) or one sided for editing only the start or end 09:00.. or ..17:00.

---

## Display/UX Issues

### 1. "Recurrence" Label Instead of "Respawn"

**Current `tatl show` Output:**
```
  Recurrence:  none
```

**Expected:**
```
  Respawn:     none
```

**Context:** TATL uses "respawn" terminology, not "recurrence". The display label should match.

**Location:** Likely in `src/cli/commands.rs` in the show command output formatting.

**Decision:** Update to Respawn

---

### 2. Task Kanban Status After Collect

**Observed:** Task 1 had sessions, was sent external, collected back, but shows `proposed` instead of `stalled`.

**Expected:** Tasks with session history should show `stalled` after collection, not `proposed`.

**Investigation Needed:** The session for task 1 may have been purged (micro-session). If so, this is expected behavior. Otherwise, the kanban calculation after collect may have a bug.

**Decision:** Session was probably purged (microsession). Assume this isn't a problem.

---

## Verification Test Results Summary

| Feature | README | CLI | Status |
|---------|--------|-----|--------|
| `tatl add` basic | Documented | Works | OK |
| `tatl add --on` | Documented | Works | OK |
| `tatl add --on=<time>` | Documented | Fails | BUG |
| `tatl add --onoff` | Documented | Works | OK |
| `tatl add --enqueue` | Documented | Works | OK |
| `tatl list` | Documented | Works | OK |
| `tatl on` | Documented | Works | OK |
| `tatl on <time>` | Documented | Creates bad data | BUG |
| `tatl off` | Documented | Sometimes fails | BUG |
| `tatl offon` | Documented | Sometimes fails | BUG |
| `tatl onoff` | Documented | Works | OK |
| `tatl finish` | Documented | Sometimes fails | BUG |
| `tatl finish --next` | Documented | Fails | BUG |
| `tatl close` | Documented | Works | OK |
| `tatl reopen` | Documented | Works | OK |
| `tatl delete` | Documented | Works | OK |
| `tatl enqueue` | Documented | Works | OK |
| `tatl dequeue` | Documented | Works | OK |
| `tatl modify` | Documented | Works | OK |
| `tatl annotate` | Documented | Works | OK |
| `tatl show` | Documented | Works | OK |
| `tatl projects add` | Documented | Works | OK |
| `tatl projects list` | Documented | Works | OK |
| `tatl projects rename` | Documented | Works | OK |
| `tatl projects archive` | Documented | Works | OK |
| `tatl projects unarchive` | Documented | Works | OK |
| `tatl projects report` | Documented | Wrong labels | DOC |
| `tatl send` | Documented | Works | OK |
| `tatl collect` | Documented | Works | OK |
| `tatl externals` | Documented | Works | OK |
| `tatl sessions list` | Documented | Works | OK |
| `tatl sessions list -7d` | Documented | Works | OK |
| `tatl sessions list start:-7d` | Documented | Returns empty | DOC |
| `tatl sessions modify` | Documented | Syntax wrong | DOC |
| `tatl sessions delete` | Documented | Works | OK |
| `tatl sessions report` | Documented | Works | OK |
| Filter syntax | Documented | Works | OK |
| Kanban filters | Documented | Works | OK |
| Respawn | Documented | Works | OK |

---

## Implementation Plan

Based on decisions above, the following work items are planned:

### Phase 1: Time Specification Fixes
- [ ] 1.1 Fix `tatl on <time>` to reject future times and only accept past times
- [ ] 1.2 Fix `tatl add --on=<time>` to work with past times
- [ ] 1.3 Add tests for time specification behavior

### Phase 2: Display/Label Fixes
- [ ] 2.1 Update `projects report` kanban column headers (Paused→Stalled, remove NEXT/LIVE, add External)
- [ ] 2.2 Change "Recurrence" label to "Respawn" in `tatl show` output
- [ ] 2.3 Add tests for display output

### Phase 3: Sessions Command Fixes
- [ ] 3.1 Fix `-y` flag parsing for `sessions modify` (allow at end of command)
- [ ] 3.2 Update CLI help text for `sessions modify` to show plain interval syntax
- [ ] 3.3 Add tests for sessions modify

### Phase 4: Sessions List Date Filtering
- [ ] 4.1 Implement `start:` and `end:` date filters for `sessions list`
- [ ] 4.2 Support interval syntax for date ranges (e.g., `start:2024-01-01..2024-01-31`)
- [ ] 4.3 Add tests for sessions list date filtering

### Phase 5: Documentation Updates
- [ ] 5.1 Update COMMAND_REFERENCE.md CLI help text for sessions modify
- [ ] 5.2 Document `--add-alias` comparison (alias: syntax vs --add-alias flag)
- [ ] 5.3 Final documentation review

### Deferred
- Session close failures: Test at human time scales before implementing fix

---

## Implementation Checklist

### Phase 1: Time Specification Fixes
- [ ] 1.1 Fix `tatl on <time>` to reject future times
- [ ] 1.2 Fix `tatl add --on=<time>` for past times
- [ ] 1.3 Add tests for time specification

### Phase 2: Display/Label Fixes
- [x] 2.1 Update `projects report` kanban headers
- [x] 2.2 Change "Recurrence" to "Respawn" in show
- [x] 2.3 Add tests for display

### Phase 3: Sessions Command Fixes
- [ ] 3.1 Fix `-y` flag parsing for `sessions modify`
- [ ] 3.2 Update CLI help for sessions modify
- [ ] 3.3 Add tests for sessions modify

### Phase 4: Sessions List Date Filtering
- [ ] 4.1 Implement start:/end: date filters
- [ ] 4.2 Support interval syntax for date ranges
- [ ] 4.3 Add tests for date filtering

### Phase 5: Documentation
- [ ] 5.1 Update COMMAND_REFERENCE.md
- [ ] 5.2 Document alias comparison
- [ ] 5.3 Final review

# Plan 33: Improve Visibility and Display

This plan addresses visibility improvements for task and session displays, screen adaptation, and a potential dashboard feature.

---

## 1. Bold/Highlight Reference IDs

**Goal:** Make task IDs in `tatl list` and session IDs in `tatl sessions list` stand out visually, since these are the primary reference values for commands.

**Current State:**
```
ID   Q    Description   Status  Kanban   Project    Tags Due  Alloc Priority Clock
──── ──── ───────────── ─────── ──────── ────────── ──── ──── ───── ──────── ─────
1    ▶    Test task one pending queued   work                       1.0      0s
```

**Proposed Options:**

### Option A: ANSI Bold
Use ANSI escape codes to make the ID column bold:
```
\x1b[1m1\x1b[0m    ▶    Test task one pending queued   work
```
- Pro: Simple, widely supported
- Con: May not work in all terminals, breaks if output is piped/redirected

### Option B: Visual Brackets/Markers
Add visual markers around IDs:
```
[1]  ▶    Test task one pending queued   work
```
- Pro: Works everywhere, including redirected output
- Con: Takes slightly more horizontal space

### Option C: Column Header Emphasis
Keep IDs plain but emphasize column header:
```
*ID*  Q    Description   Status  Kanban
────  ──── ───────────── ─────── ──────
1     ▶    Test task one pending queued
```

**Decision:** Use ANSI bold only for interactive terminals (TTY detection). Plain output when piped.
---

## 2. Adaptive Screen Width

**Goal:** Intelligently truncate columns when terminal width is less than default table width.

**Current Behavior:** Tables overflow or wrap awkwardly on narrow terminals.

**Proposed Algorithm:**

1. **Detect terminal width** via `COLUMNS` env var or `ioctl` call
2. **Define column priorities:**
   - Essential (never truncate): ID, Q (queue position)
   - Important (truncate last): Description, Project
   - Secondary (truncate first): Status, Kanban, Due, Priority
   - Optional (hide if needed): Alloc, Clock, Tags

3. **Truncation strategy:**
   - First pass: Reduce Description to min 15 chars
   - Second pass: Hide lowest priority columns
   - Third pass: Truncate Project names
   - Always preserve ID and Q columns

**Column Priority Table:**

| Column | Priority | Min Width | Truncate Strategy |
|--------|----------|-----------|-------------------|
| ID | 1 (highest) | 4 | Never truncate |
| Q | 1 | 4 | Never truncate |
| Description | 2 | 15 | Truncate with ellipsis |
| Project | 3 | 8 | Truncate with ellipsis |
| Status | 4 | 7 | Hide if needed |
| Kanban | 4 | 8 | Hide if needed |
| Due | 5 | 10 | Hide if needed |
| Priority | 6 | 8 | Hide if needed |
| Tags | 7 | 6 | Hide if needed |
| Alloc | 8 | 5 | Hide if needed |
| Clock | 8 | 5 | Hide if needed |

**Decision:**
- Add `--full` flag to force all column visibility regardless of terminal width
- Config file column settings is nice-to-have (users can already configure views)
---

## 3. Fix `projects report` Kanban Stages

**Goal:** Update to reflect current kanban stages and aggregate to top-level projects.

**Current Output (incorrect):**
```
Project                   Proposed   Queued   Paused   NEXT   LIVE   Done  Total
work                             0        0        0      1      0      0      1
work.email                       0        0        1      0      0      0      1
```

**Issues:**
1. Uses old stage names (Paused, NEXT, LIVE) instead of (Stalled, External)
2. Shows child projects separately - too granular/busy

**Proposed Output:**
```
Project                   Proposed  Stalled   Queued  External   Done  Total
───────────────────────── ──────── ──────── ──────── ──────── ────── ──────
work                             0        1        1        0      0      2
home                             1        0        0        0      0      1
───────────────────────── ──────── ──────── ──────── ──────── ────── ──────
TOTAL                            1        1        1        0      0      3
```

**Changes:**
- Rename "Paused" → "Stalled"
- Remove "NEXT" and "LIVE" (fold into "Queued")
- Add "External" column
- Aggregate child projects (work.email) into parent (work)

**Decision:**
- Add `--detailed` flag to show child project breakdown (adds rows, not columns)
- Keep actively-timing task counted in "Queued" column (don't separate)
---

## 4. Fix Play Symbol (▶) Alignment

**Goal:** Ensure the play symbol doesn't offset column alignment.

**Current State:** Verified - alignment is correct in standard terminals.

**Investigation Results:**
- The play symbol `▶` (U+25B6) has "Neutral" East Asian width property
- Most terminals render it as 1 cell wide, matching ASCII characters
- Column width calculation uses byte length (3 for ▶) but format padding uses char count (1)
- This mismatch doesn't cause alignment issues since padding is applied consistently
- Tested successfully: "▶" and numeric queue positions align correctly in output

**Proposed Fix:** No fix needed for standard terminals. If users report issues:
- Option 1: Use `unicode-width` crate to calculate actual display width
- Option 2: Use ASCII alternative: `>` or `*` for timing indicator

**Decision:**
- No changes needed - alignment verified working
- ASCII fallback (`>`) available if specific terminals report issues
- Future: Could add `unicode-width` dependency for edge cases if needed
---

## 5. Dashboard / Report View

**Goal:** Create a composite view showing immediately actionable tasks and summary information.

**Proposed Command:** `tatl dashboard` or `tatl report`

**Proposed Layout:**

```
═══════════════════════════════════════════════════════════════════════════
                           TATL DASHBOARD
═══════════════════════════════════════════════════════════════════════════

📋 QUEUE (3 tasks)
───────────────────────────────────────────────────────────────────────────
 #  ID   Description              Project    Due         Priority
 0  12   ▶ Fix auth bug           work       today       11.2
 1  15   Review PR                work       tomorrow     8.5
 2   8   Update docs              docs       +3d          5.1

⏰ TODAY'S SESSIONS (2h 15m)
───────────────────────────────────────────────────────────────────────────
 09:00-10:30  Fix auth bug              work        1h 30m
 10:45-11:30  Code review               work           45m
 [current]    Fix auth bug              work           23m

📊 THIS WEEK
───────────────────────────────────────────────────────────────────────────
 Total time:     12h 30m    │  Tasks completed:  5
 Avg per day:     2h 30m    │  Tasks created:    8

 By project:
   work           8h 15m ████████████████░░░░  66%
   home           2h 45m █████░░░░░░░░░░░░░░░  22%
   docs           1h 30m ███░░░░░░░░░░░░░░░░░  12%

⚠️ ATTENTION NEEDED
───────────────────────────────────────────────────────────────────────────
 Overdue (2):     #5 Submit report (3 days), #9 Pay invoice (1 day)
 Stalled (1):     #7 Waiting on feedback (5 days idle)
 External (1):    #11 Sent to @manager (2 days)

═══════════════════════════════════════════════════════════════════════════
```

**Sections:**
1. **Queue** - Current work queue with immediate priorities
2. **Today's Sessions** - Time tracked today with running total
3. **This Week** - Summary statistics and project breakdown
4. **Attention Needed** - Overdue, stalled, and external tasks

**Implementation:**
Compose from existing commands:
- `tatl list group:-kanban sort:q,-priority hide:priority,status,kanban status:pending`
- `tatl sessions report -7d`
- `tatl sessions list today`

**Decision:**
- Dashboard is NOT the default for `tatl` with no arguments (keep `list` as default)
- All sections are initially included; can optimize later based on usage
- No `--json` support needed at this point
- Support `--period=<week|month|year>` option for time range

---

## 6. Session Date Column Filters

**Goal:** Allow filtering sessions by date columns using interval syntax.

**Current Syntax:**
```bash
tatl sessions list -7d              # Works
tatl sessions list start:today      # Partially works
```

**Proposed Enhanced Syntax:**
```bash
# Filter by start date
tatl sessions list start:today
tatl sessions list start:-7d
tatl sessions list start:2024-01-01..2024-01-31

# Filter by end date
tatl sessions list end:today
tatl sessions list end:-7d..now

# Filter by date range (either start or end falls within)
tatl sessions list date:2024-01-01..2024-01-31

# Combined with other filters
tatl sessions list start:-7d project:work +urgent
```

**Implementation:**
- `start:<date>` - Sessions that started on/after date
- `start:<date>..<date>` - Sessions that started within interval
- `end:<date>` - Sessions that ended on/before date
- `end:<date>..<date>` - Sessions that ended within interval
- `date:<interval>` - Sessions that overlap with interval

**Decision:**
- `start:<date>` means "on or after this date" (inclusive lower bound)
- Only `start:` and `end:` filters needed (no overlapping `date:` filter)

---

## Implementation Checklist

### Phase 1: Quick Fixes
- [x] 3.1 Update `projects report` kanban column headers
- [x] 3.2 Aggregate child projects into parent
- [x] 4.1 Verify and fix play symbol alignment (verified: alignment correct in standard terminals)

### Phase 2: ID Visibility
- [x] 1.1 Implement chosen ID highlighting approach (ANSI bold for ID column)
- [x] 1.2 Add TTY detection for ANSI codes (`is_tty()` function, only applies bold in TTY mode)

### Phase 3: Adaptive Width
- [x] 2.1 Add terminal width detection (`get_terminal_width()` from COLUMNS env var)
- [x] 2.2 Implement column priority system (priority 1-8, hides lowest priority first)
- [x] 2.3 Add `--full` flag for complete output

### Phase 4: Session Filters
- [x] 6.1 Implement `start:` interval filter (supports dates and intervals like `start:-7d` or `start:2024-01-01..2024-01-31`)
- [x] 6.2 Implement `end:` interval filter (supports dates and intervals)
- [x] 6.3 Update documentation (README.md and COMMAND_REFERENCE.md updated)

### Phase 5: Dashboard
- [x] 5.1 Design final layout (Queue, Today's Sessions, Period Statistics, Attention Needed)
- [x] 5.2 Implement `tatl dashboard` command
- [x] 5.3 Add configuration options (`--period=week|month|year`)

---

## Decisions Summary

| Item | Decision |
|------|----------|
| ID Highlighting | ANSI bold for TTY, plain when piped |
| Adaptive Width | `--full` flag to show all columns |
| Projects Report | `--detailed` flag for child breakdown; aggregate by default |
| Play Symbol | Fix spacing; ASCII fallback if needed |
| Dashboard | Separate command, not default; support `--period` |
| Session Filters | `start:` = "on or after"; only start/end filters |

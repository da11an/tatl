# Plan 22: CLI Syntax Review and Recommendations

## Executive Summary

This document provides a comprehensive analysis of Tatl's CLI syntax, identifying patterns, inconsistencies, and opportunities for streamlining. The CLI is generally well-designed with good Taskwarrior-style ergonomics, but there are areas for improvement.

---

## Current CLI Architecture

### Command Structure Overview

```
tatl <command> [subcommand] [target] [args...] [flags]
```

**Top-Level Commands:**
- `add`, `list`, `show`, `modify`, `annotate`, `finish`, `close`, `delete`, `enqueue`, `status`
- `projects` (subcommands: `add`, `list`, `rename`, `archive`, `unarchive`)
- `clock` (subcommands: `list`, `in`, `out`, `pick`, `next`, `drop`, `clear`)
- `sessions` (subcommands: `list`, `show`, `modify`, `delete`, `add`, `report`)
- `recur` (subcommands: `run`)

### Syntax Patterns

| Pattern | Example | Supported? | Description |
|---------|---------|-----------|-------------|
| Command-first | `tatl add "fix bug"` | ✅ | Standard CLAP subcommand |
| ID-first (task subcommands) | `tatl 10 modify +urgent` | ✅ | Normalized to command-first |
| ID-first (other) | `tatl 1 show` | ❌ | Not supported |
| Filter-based | `tatl list project:work` | ✅ | Filter as argument |
| Implicit target | `tatl finish` | ✅ | Uses clock[0] |
| Implicit show | `tatl 10` | ✅ | Single ID becomes `show 10` |

**ID-first syntax supported for:** `modify`, `enqueue`, `finish`, `close`, `delete`, `annotate`

**ID-first syntax NOT supported for:** `show`, `list`, `add`, `sessions`, `projects`, `clock`

---

## Strengths

### 1. **Taskwarrior-Compatible Syntax**
- `project:name`, `due:tomorrow`, `+tag`, `-tag` syntax is familiar
- Field abbreviations (`proj:`, `sc:`, `du:`) reduce typing
- Command abbreviations (`l` → `list`, `mod` → `modify`)

### 2. **Flexible Input Patterns (Limited)**
- ID-first syntax: `tatl 10 modify +urgent` → normalized to `tatl modify 10 +urgent`
- Supported for: `modify`, `enqueue`, `finish`, `close`, `delete`, `annotate`
- NOT supported for: `show`, `list`, `add`, `sessions` (CLAP-first design)
- Special case: `tatl 10 clock in` → `tatl clock in 10`

### 3. **Context-Aware Defaults**
- `tatl finish` uses current clocked task
- `tatl annotate <note>` annotates live task
- `tatl clock in` uses clock[0]

### 4. **Consistent Flag Patterns**
- `--yes` for non-interactive batch operations
- `--interactive` for one-by-one confirmation
- `--json` for machine-readable output

---

## Inconsistencies and Issues

### Issue 1: Naming Inconsistency - `finish` vs `done`

**Problem:** The command is `finish` but documentation and error messages sometimes reference "done" (e.g., kanban status `done`).

**Current:**
```bash
tatl finish 10          # Command
tatl list kanban:done   # Filter uses "done"
```

**Recommendation:** Keep `finish` as command (avoids `done` as common word), but ensure documentation uses consistent terminology. Consider adding `done` as an alias.

---

### Issue 2: Asymmetric `enqueue` Placement

**Problem:** `enqueue` exists in TWO places with different semantics:

```bash
tatl enqueue 10           # Top-level command
tatl clock enqueue 10     # Under clock subcommand (alias)
```

The `clock enqueue` variant is documented but `enqueue` is also top-level.

**Recommendation:** 
- Keep top-level `enqueue` (convenient shorthand)
- Document both explicitly
- Consider deprecating `clock enqueue` in favor of just `enqueue`

---

### Issue 3: Target Argument Inconsistency

**Problem:** Some commands require target, others make it optional with different defaults:

| Command | Target | Default if Omitted |
|---------|--------|-------------------|
| `finish` | Optional | clock[0] (must be clocked in) |
| `close` | **Required** | N/A |
| `delete` | **Required** | N/A |
| `annotate` | Optional | Live task |
| `modify` | **Required** | N/A |
| `show` | **Required** | N/A |

**Inconsistency:** Why does `finish` allow omission but `close` does not? Both could reasonably default to clock[0].

**Recommendation:** Consider allowing `close` and `delete` to also default to clock[0] when no target specified, with appropriate confirmation.

**Decision:** Recommendation accepted.
---

### Issue 4: `show` vs Implicit Show

**Problem:** Both work:
```bash
tatl show 10    # Explicit
tatl 10         # Implicit (converted to "show 10")
```

But the implicit form only works for single bare IDs. Inconsistent with:
```bash
tatl 10,11,12   # Does NOT work as implicit show
tatl 1-5        # Does NOT work as implicit show
```

**Recommendation:** Extend implicit show to support ID lists and ranges, or document the limitation clearly.

**Decision:** Recommendation accepted.

---

### Issue 5: Sessions Subcommand Placement

**Problem:** `sessions` is a top-level command requiring a `--task` flag OR filter arguments:

```bash
tatl sessions list --task 10      # Legacy
tatl sessions list 10             # Filter-style
tatl sessions list project:work   # Filter-style
```

**Inconsistency:** Why isn't this `tatl 10 sessions list` to match other task-centric patterns?

**Recommendation:** Support task-ID-first syntax for sessions:
```bash
tatl 10 sessions list   # Proposed
tatl 10 sessions show   # Proposed
```

---

### Issue 6: Clock `in` Task ID Argument Position

**Problem:** `clock in` accepts task ID as first positional, but clock operations like `pick` and `drop` take an **index** not a task ID:

```bash
tatl clock in 10        # 10 is task ID
tatl clock pick 2       # 2 is stack INDEX
tatl clock drop 1       # 1 is stack INDEX
```

This is confusing. Users might think `clock pick 10` picks task 10.

**Recommendation:**
- Document this distinction prominently
- Consider `clock pick --index 2` or `clock pick @2` syntax for indices
- Or: `clock in --task 10` to be explicit

**Decision:**
- Drop UX support for index at all.
- Exclusively use task id in the user interface.

---

### Issue 7: Time Interval Syntax Inconsistency

**Problem:** Clock intervals use `..` but date ranges in filters aren't supported:

```bash
tatl clock in 09:00..11:00    # Works
tatl list due:2026-01-01..2026-01-31   # Doesn't work
```

**Recommendation:** Either add range support to filters or document that `..` is specific to clock intervals.

**Decision:** Support range filters. And exclusively use range syntax `..` if a range is used.

---

### Issue 8: `--at` Flag Only on `finish`

**Problem:** `finish --at 17:00` sets session end time, but `close` has no equivalent:

```bash
tatl finish 10 --at 17:00   # Works
tatl close 10 --at 17:00    # Doesn't exist
```

**Recommendation:** Add `--at` to `close` for consistency, or document why it's finish-only (close is typically non-session-related).

**Decision:** Drop the at flag altogether. Instead, allow a single argument: a single time or datetime, or a range of time or datetime. E.g. 17:00 or 16:00..17:00 or 2026-01-01T16:00..2026-01-01T17:00

This is in line with streamlining date, time, or datetimes or ranges of them.
Also keeping the syntax tight and fast.
Flags should be used to increase mileage (like --clock-in when adding a task) not verbosity

---

### Issue 9: Project Creation Prompts

**Problem:** `add` prompts for new projects but `modify` also prompts. The `--auto-create-project` flag only exists on `add`:

```bash
tatl add --auto-create-project "task" project:new     # Non-interactive
tatl modify 10 project:new                             # Interactive prompt
tatl modify 10 project:new --yes                       # Creates silently
```

**Inconsistency:** `--yes` on `modify` auto-creates projects, but `--auto-create-project` is the flag on `add`.

**Recommendation:** Harmonize: Either add `--auto-create-project` to `modify`, or document that `--yes` implies auto-create.

**Decision:** Standardize of -y or -n for bypassing the interactive prompts with preset answers. Apply to all follow up prompt scenarios.
---

### Issue 10: Missing `projects delete` Command

**Problem:** Projects can be archived but not deleted:

```bash
tatl projects add work
tatl projects archive work
tatl projects delete work   # Doesn't exist!
```

**Recommendation:** Add `projects delete` for cleanup (with cascading behavior options).

**Decision:** Recommendation accepted. However, do not delete nested projects if a parent project is deleted: e.g. deleting `home` should not cascade to delete `home.cleaning`

---

### Issue 11: Report Command Limited to Sessions

**Problem:** `sessions report` exists but there's no general reporting:

```bash
tatl sessions report -7d      # Time report
tatl report                   # Doesn't exist
```

**Recommendation:** Consider top-level `report` command with subcommands:
```bash
tatl report time -7d          # Time report
tatl report tasks             # Task summary
tatl report projects          # Project summary
```

**Decision:** Go with report, drop summary syntax

**Question:** However, if we're introducing the key word time, do we drop sessions. We have three words that all share the same space: sessions, time, clock. Can we consolidate without introducing ambiguity to 1 or 2?

---

### Issue 12: Bug - `summary` in TASK_SUBCOMMANDS but command is `show`

**Problem:** In `src/cli/abbrev.rs`, `TASK_SUBCOMMANDS` includes `"summary"` but the actual clap command is `show`. This causes `tatl 1 summary` to be normalized to `tatl summary 1`, which then fails.

```rust
// abbrev.rs line 55-57
pub const TASK_SUBCOMMANDS: &[&str] = &[
    "enqueue", "modify", "finish", "close", "delete", "annotate", "summary"  // ← "summary" should be "show"
];
```

**Impact:** `tatl 1 summary` fails; `tatl 1 show` also fails (because `show` isn't in TASK_SUBCOMMANDS).

**Fix:** Change `"summary"` to `"show"` in TASK_SUBCOMMANDS.

**Decision:** Fix Accepted

---

### Issue 13: Inconsistent Alias/View Syntax

**Problem:** Views are saved with `alias:name` in filter, but `--add-alias` is also mentioned:

```bash
tatl list project:work alias:mywork   # Inline syntax
tatl list project:work --add-alias mywork   # Flag syntax (documented but check if implemented)
```

**Recommendation:** Pick one syntax and use consistently. Flag syntax is more discoverable.

**Decision:** Use `alias:name` syntax consistently. Let's avoid --commands except where filter style syntax doesn't make sense.

---

## Streamlining Recommendations

### Recommendation A: Unified Target Syntax

Standardize on a single target pattern across all task commands:

```bash
# All these should work identically:
tatl <command> <id>           # Single ID
tatl <command> <id>,<id>      # ID list  
tatl <command> <start>-<end>  # ID range
tatl <command> <filter>       # Filter expression
```

Currently this works for most commands but `enqueue` has quirks.

**Decision:** Yes, standardize behavior, and consolidate code where appropriate to suppose this.

---

### Recommendation B: Consistent Defaults

| Command | Current Default | Proposed Default |
|---------|-----------------|------------------|
| `finish` | clock[0] | clock[0] (keep) |
| `close` | None (required) | clock[0] with confirmation |
| `delete` | None (required) | None (keep - destructive) |
| `annotate` | Live task | Live task (keep) |

**Decision:** Agree

---

### Recommendation C: Simplify Clock Subcommands

Consider merging related operations:

**Current:**
```bash
tatl clock list
tatl clock in
tatl clock out
tatl clock pick <index>
tatl clock next [n]
tatl clock drop <index>
tatl clock clear
tatl enqueue <id>
```

**Alternative (if streamlining):**
- Remove `clock` prefix for timing: `tatl in`, `tatl out`
- Keep stack ops under `clock`: `tatl clock pick`, `tatl clock drop`

This would give:
```bash
tatl on 10        # Start timing task 10
tatl off          # Stop timing
tatl enqueue 10   # Add to stack
tatl clock list   # View stack
tatl clock pick 2 # Reorder stack
```

**Decision:** Remove clock commands in favor of:
- Remove `tatl clock list`. Kanban column tells you what's in queue already.
- Merge kanban `working` stage into `queue` stage for simplicity and clarity
- Instead of `tatl clock pick 2` you can just `tatl in <task id>` when you're ready to work. We drop goofing around with clock stack commands when the clock isn't live. We don't want playing with the task list to become a distraction.
- Instead of `tatl drop <index>` use `tatl dequeue <optional task id>` Default to stack[0]. This does not finish or close the task, just drops it from the clock stack and as a result the queue view in kanban.
- No `tatl clock next` remains, as you can just `tatl in <task id>` just as easily. This reduces the commmand surface without losing functionality -- reduced mental load, learning/decision ramp. Use a few powerful commands rather than many narrow commands.
- Use `on` instead of `in` and `off` instead of `out`. This corresponds to phrases like timer on, or on task, and timer off, or off task. The --on and --off flags should be made available to any non-destructive single task commands, like `tatl add ...` and `tatl modify <task id>`.
---

### Recommendation D: Add Short Aliases

Create single-letter aliases for common operations:

| Alias | Command |
|-------|---------|
| `a` | `add` |
| `l` | `list` |
| `s` | `status` |
| `f` | `finish` |
| `i` | `on` |
| `o` | `off` |
| `e` | `enqueue` |
| `d` | `dequeue` |
| `m` | `modify` |

Note: Some of these work via abbreviation already, but formal aliases ensure they never become ambiguous.

**Decision:** I'm okay with this if it is clearly documented. I began editing based on edits elsewhere in the document. But we need to circle back to this list (maybe do it last) after we see where the syntax goes.

---

### Recommendation E: Standardize Time Arguments

Create consistent syntax for time-related arguments:

| Context | Current | Proposed | Decision | Note |
|---------|---------|----------|----------|------|
| Due date | `due:tomorrow` | `due:tomorrow` (keep) | `due:tomorrow` (keep) | |
| Clock start | `tatl clock in 09:00` | `tatl clock in start:09:00` | `tatl on` (now) or `tatl on 09:00` | While this becomes part of an interval, at this point it's a clock operation |
| Clock end | `tatl clock out 09:00` | `tatl clock out end:09:00` | `tatl off` (now) or `tatl off 09:00` | Just a clock operation at this point, not directly manipulating the interval -- that happens behind the scene |
| Clock interval | `09:00..11:00` | `start:09:00 end:11:00` | `09:00..11:00` | |
| Session modify | `start:09:00 end:17:00` | `start:09:00 end:17:00` (keep) | `09:00..17:00` or `..17:00` or `09:00..` | either-sided or full interval |
| Session add |  |  | `09:00..17:00` or `2026-01-01T12:00..2026-01-01T13:00` | requires full interval notation |

The interval syntax is nice but inconsistent with the rest of the CLI.

**Decision:** Lean into the interval syntax. Whenever an interval is needed require it. If you are editing one side of an interval, require a one-sided interval, e.g. `..13:00`. This forces users to acknowledge when something is an interval.

---

## Consolidated Decisions

### Naming & Terminology
| Decision | Details |
|----------|---------|
| **Timer commands** | `on` / `off` (not `in` / `out` or `clock in` / `clock out`) |
| **Queue commands** | `enqueue` / `dequeue` (top-level, not under `clock`) |
| **Sessions vs Time** | **Open question** - consolidate `sessions`, `time`, `clock` terminology |

### Syntax Principles
| Principle | Decision |
|-----------|----------|
| **Interval notation** | Use `..` consistently: `09:00..17:00`, `..17:00`, `09:00..` |
| **Range filters** | Support `due:2026-01-01..2026-01-31` syntax |
| **Target arguments** | Positional time/datetime/range, NOT `--at` flags |
| **Prompt bypass** | Standardize on `-y` / `-n` for all interactive prompts |
| **View aliases** | Use `alias:name` syntax (filter-style, not `--add-alias`) |
| **Task identification** | Exclusively task IDs, no index-based operations |

### Commands to Add
- `on [<task_id>] [<time>]` - Start timing (replaces `clock in`)
- `off [<time>]` - Stop timing (replaces `clock out`)
- `dequeue [<task_id>]` - Remove from queue without finishing (replaces `clock drop`)
- `projects delete <project_id>` - Delete project (no cascade to nested)
- `report time` / `report tasks` / `report projects` - Top-level reporting

### Commands to Remove
- `clock in` / `clock out` → replaced by `on` / `off`
- `clock list` → kanban queue column serves this purpose
- `clock pick` → just use `on <task_id>` when ready
- `clock next` → just use `on <task_id>`
- `clock drop` → replaced by `dequeue`
- `clock clear` → remove (use `dequeue` iteratively or filter-based)
- `sessions report` → replaced by `report time`

### Flags to Add
- `--on` flag for `add` and `modify` (start timing after operation)
- `--off` flag where appropriate

### Kanban Simplification
- Merge `working` stage into `queue` stage
- Queue column shows what was formerly in clock stack

---

## Open Question: sessions / time / clock Consolidation

Three words share overlapping semantic space:
- **sessions** - historical time records
- **time** - duration/reporting concept  
- **clock** - timer operations

**Options:**

| Option | Sessions Becomes | Timer Commands | Report Command |
|--------|------------------|----------------|----------------|
| A: Keep `sessions` | `sessions list/show/modify/delete/add` | `on`/`off` | `report time` |
| B: Use `time` | `time list/show/modify/delete/add` | `on`/`off` | `report time` (ambiguous?) |
| C: Merge into `on`/`off` | `on list`, `on show`, `on modify` | `on`/`off` | `report time` |

**Recommendation:** Option A - Keep `sessions` for CRUD on historical records, use `on`/`off` for live timer, use `report time` for reporting. Clear separation:
- `on`/`off` = control the timer NOW
- `sessions` = manage historical time RECORDS
- `report time` = ANALYZE time data

**Decision:** Go with recommendation for now.

---

## Revised Implementation Priority

### Phase 1: Bug Fixes & Quick Wins
| # | Item | Effort | Notes |
|---|------|--------|-------|
| 1.1 | Fix `summary` → `show` in abbrev.rs | Low | One-line fix |
| 1.2 | Extend implicit show to ID lists/ranges | Low | `tatl 1,2,3` and `tatl 1-5` |
| 1.3 | Add `-y`/`-n` flags uniformly | Low | Standardize prompt bypass |

### Phase 2: Timer Command Overhaul
| # | Item | Effort | Notes |
|---|------|--------|-------|
| 2.1 | Replace `clock in` with `on` command | Medium | Direct replacement, no deprecation |
| 2.2 | Replace `clock out` with `off` command | Medium | Direct replacement, no deprecation |
| 2.3 | Add `dequeue` command | Medium | Drop from queue without finishing |
| 2.4 | Add `--on` flag to `add` | Low | Timer start after add |
| 2.5 | Add `--on` flag to `modify` | Low | Timer start after modify |
| 2.6 | Remove all `clock` subcommands | Medium | Clean removal: `in`, `out`, `pick`, `next`, `drop`, `clear`, `list`, `enqueue` |

### Phase 3: Interval Syntax Unification
| # | Item | Effort | Notes |
|---|------|--------|-------|
| 3.1 | Implement one-sided intervals `..17:00`, `09:00..` | Medium | For session modify |
| 3.2 | Support range filters `due:2026-01-01..2026-01-31` | Medium | Filter parser change |
| 3.3 | Remove `--at` flag from `finish` | Low | Use positional time arg |
| 3.4 | Update `sessions modify` to use interval syntax | Medium | Replace `start:`/`end:` |
| 3.5 | Update `sessions add` to require interval | Low | Consistency |

### Phase 4: Default & Target Improvements
| # | Item | Effort | Notes |
|---|------|--------|-------|
| 4.1 | `close` defaults to clock[0] | Low | Match `finish` behavior |
| 4.2 | Standardize target patterns across commands | Medium | Consolidate parsing code |
| 4.3 | Add `show` to TASK_SUBCOMMANDS for ID-first | Low | `tatl 10 show` works |

### Phase 5: New Features
| # | Item | Effort | Notes |
|---|------|--------|-------|
| 5.1 | Add `projects delete` | Medium | No cascade to nested |
| 5.2 | Add `report` command with subcommands | High | `time`, `tasks`, `projects` |
| 5.3 | Merge kanban `working` → `queue` | Medium | Simplify kanban model |

### Phase 6: Documentation
| # | Item | Effort | Notes |
|---|------|--------|-------|
| 6.1 | Finalize single-letter aliases | Low | After syntax stabilizes |
| 6.2 | Update COMMAND_REFERENCE.md | Medium | Reflect all changes |
| 6.3 | Update README.md | Low | Quick start examples |

---

## Summary Table (Updated with Decisions)

| # | Issue | Decision | Phase |
|---|-------|----------|-------|
| 1 | `finish` vs `done` naming | Keep as-is (low priority) | - |
| 2 | `enqueue` in two places | Deprecate `clock enqueue` | 6.4 |
| 3 | Target argument inconsistency | `close` defaults to clock[0] | 4.1 |
| 4 | Implicit show limited | Extend to ID lists/ranges | 1.2 |
| 5 | Sessions placement | Keep filter-style (reconsider later) | - |
| 6 | Clock ID vs index confusion | **Remove index-based UX entirely** | 2.7 |
| 7 | Time interval syntax | **Support `..` ranges everywhere** | 3.x |
| 8 | `--at` only on finish | **Remove `--at`, use positional time** | 3.3 |
| 9 | Project creation flags | **Standardize `-y`/`-n`** | 1.3 |
| 10 | Missing `projects delete` | Add command, no cascade | 5.1 |
| 11 | Limited reporting | Add `report` command | 5.2 |
| 12 | `summary` vs `show` bug | **Fix abbrev.rs** | 1.1 |
| 13 | Alias/view syntax | Use `alias:name` syntax | - |

---

## Appendix A: Current Command Matrix

### Commands with `--yes` and `--interactive`
- `modify`, `finish`, `close`, `delete`, `annotate`, `sessions modify`

### Commands with `--json`
- `list`, `status`, `projects list`, `clock list`, `sessions list`

### Commands with `--task` flag
- `sessions list` (legacy), `sessions show`, `clock in` (as positional, not flag)

### Commands accepting filters
- `list`, `show`, `modify`, `finish`, `close`, `delete`, `annotate`, `sessions list`

### Commands with implicit defaults
- `finish` → clock[0]
- `annotate` → live task
- `clock in` → clock[0]
- `clock out` → current session

---

## Appendix B: Planned Command Matrix (Post-Implementation)

### Top-Level Commands

| Command | Description | Target | Flags | Notes |
|---------|-------------|--------|-------|-------|
| `add` | Create task | N/A | `-y`, `--on`, `--enqueue` | `--on` starts timer |
| `list` | List tasks | filter | `--json`, `--relative` | Supports `alias:name` |
| `show` | Show task details | id/range/filter | - | `tatl 10` implicit |
| `modify` | Modify task(s) | id/filter | `-y`, `-n`, `--on` | |
| `annotate` | Add annotation | id/filter (opt) | `-y`, `-n` | Defaults to live task |
| `finish` | Complete task | id/filter (opt) | `-y`, `-n`, `--next` | Defaults to clock[0] |
| `close` | Close task | id/filter (opt) | `-y`, `-n` | **NEW: defaults to clock[0]** |
| `delete` | Delete task | id/filter | `-y`, `-n` | Requires target |
| `on` | Start timer | task_id (opt), time (opt) | - | **NEW: replaces `clock in`** |
| `off` | Stop timer | time (opt) | - | **NEW: replaces `clock out`** |
| `enqueue` | Add to queue | id/range/list | - | Top-level only |
| `dequeue` | Remove from queue | task_id (opt) | - | **NEW: defaults to queue[0]** |
| `status` | Show dashboard | N/A | `--json` | |
| `projects` | Project management | subcommand | - | |
| `sessions` | Session management | subcommand | - | Historical records |
| `recur` | Recurrence management | subcommand | - | |
| `report` | Generate reports | subcommand | - | **NEW** |

### Project Subcommands (`tatl projects ...`)

| Subcommand | Description | Args | Flags |
|------------|-------------|------|-------|
| `add` | Create project | name | - |
| `list` | List projects | - | `--archived`, `--json` |
| `rename` | Rename project | old new | `--force` |
| `archive` | Archive project | name | - |
| `unarchive` | Unarchive project | name | - |
| `delete` | Delete project | name | `-y` | **NEW** |

### Session Subcommands (`tatl sessions ...`)

| Subcommand | Description | Args | Flags |
|------------|-------------|------|-------|
| `list` | List sessions | filter (opt) | `--json` |
| `show` | Show session details | - | `--task` |
| `modify` | Modify session | session_id interval | `-y`, `--force` |
| `delete` | Delete session | session_id | `-y` |
| `add` | Add manual session | task_id interval | - |

### Report Subcommands (`tatl report ...`) **NEW**

| Subcommand | Description | Args | Flags |
|------------|-------------|------|-------|
| `time` | Time report | start..end (opt) | `--json` |
| `tasks` | Task summary | filter (opt) | `--json` |
| `projects` | Project summary | filter (opt) | `--json` |

### Recur Subcommands (`tatl recur ...`)

| Subcommand | Description | Args | Flags |
|------------|-------------|------|-------|
| `run` | Generate instances | - | `--until` |

---

## Appendix C: Removed Commands (Immediate, No Deprecation)

| Removed | Replacement | Reason |
|---------|-------------|--------|
| `clock in` | `on` | Simpler, faster to type |
| `clock out` | `off` | Simpler, faster to type |
| `clock list` | Kanban queue column | Redundant visualization |
| `clock pick <index>` | `on <task_id>` | No index-based UX |
| `clock next` | `on <task_id>` | Reduced command surface |
| `clock drop <index>` | `dequeue [<task_id>]` | No index-based UX |
| `clock clear` | (removed) | Use `dequeue` iteratively |
| `clock enqueue` | `enqueue` | Top-level only |
| `sessions report` | `report time` | Unified reporting |
| `--at` flag | positional time arg | Streamlined syntax |
| `--clock-in` flag | `--on` | Consistent naming |

---

## Appendix D: Interval Syntax Reference

| Context | Syntax | Example |
|---------|--------|---------|
| Full interval | `<start>..<end>` | `09:00..17:00` |
| Open start | `..<end>` | `..17:00` (modify end only) |
| Open end | `<start>..` | `09:00..` (modify start only) |
| Date range filter | `field:<start>..<end>` | `due:2026-01-01..2026-01-31` |
| Single time | `<time>` | `09:00` (for `on`/`off`) |

### Time Expression Formats
- Time only: `09:00`, `17:30`
- Date only: `2026-01-15`, `tomorrow`, `+2d`
- DateTime: `2026-01-15T09:00`
- Relative: `+1h`, `-30m`, `eod`, `eow`

---

## Appendix E: Target Syntax Reference

All task-targeting commands accept these formats:

| Format | Example | Description |
|--------|---------|-------------|
| Single ID | `10` | One task |
| ID list | `1,3,5` | Multiple specific tasks |
| ID range | `10-15` | Contiguous range |
| Mixed | `1,3-5,10` | Combined list and range |
| Filter | `project:work +urgent` | Filter expression |
| Implicit | (omitted) | Command-specific default |

### Default Targets by Command

| Command | Default Target |
|---------|----------------|
| `finish` | queue[0] (live task) |
| `close` | queue[0] (live task) |
| `annotate` | Live task (if clocked on) |
| `off` | Current session |
| `dequeue` | queue[0] |
| `on` | queue[0] |

---

## Appendix F: Consistency Check

### Potential Conflicts to Resolve

| # | Conflict | Resolution Needed |
|---|----------|-------------------|
| 1 | `--on` flag vs `on` command | Both exist: `tatl add --on` and `tatl on`. `--on` is a convenience flag that implies adding then starting timer. Clear distinction. ✅ |
| 2 | `finish` vs `off` | `finish` = complete task + stop timer. `off` = just stop timer. Different semantics. ✅ |
| 3 | `dequeue` default vs `on` default | Both default to queue[0]. `dequeue` removes from queue, `on` starts timer. Consistent. ✅ |
| 4 | Interval syntax for `sessions modify` | Currently uses `start:` and `end:` prefixes. Decision changes to `..` syntax. Need to ensure parser handles context. ⚠️ |
| 5 | Filter range syntax | Adding `due:2026-01-01..2026-01-31`. Need to handle in filter parser, not conflict with existing `:` parsing. ⚠️ |
| 6 | `report time` vs `sessions list` | `report time` = aggregated summary. `sessions list` = raw records. Different purposes, both needed. ✅ |

### Syntax Consistency Verification

| Syntax Element | Commands Using It | Consistent? |
|----------------|-------------------|-------------|
| `field:value` | All filter/modify | ✅ |
| `+tag` / `-tag` | All filter/modify | ✅ |
| `<start>..<end>` | `on`, `sessions modify/add`, filters | ✅ (after changes) |
| `-y` / `-n` | All with prompts | ✅ (after changes) |
| `--json` | `list`, `status`, `sessions list`, `report *` | ✅ |
| `alias:name` | `list`, `sessions list` | ✅ |

### Flag Consistency

| Flag | Standard Meaning | Exceptions |
|------|------------------|------------|
| `-y` | Auto-confirm (yes) | None |
| `-n` | Auto-decline (no) | None |
| `--on` | Start timer after | `add`, `modify` only |
| `--json` | JSON output | Read-only commands only |
| `--force` | Override safety checks | `projects rename`, `sessions modify` |
| `--next` | Continue to next task | `finish` only |

### Breaking Changes Summary

All changes are immediate. No deprecation period (single user, move fast).

| Change | Impact | New Syntax |
|--------|--------|------------|
| `clock in` → `on` | High | `tatl on [<task_id>] [<time>]` |
| `clock out` → `off` | High | `tatl off [<time>]` |
| `clock pick/next/drop/clear/list` | Medium | Removed entirely |
| `clock enqueue` → `enqueue` | Low | `tatl enqueue <id>` (top-level only) |
| `sessions modify` syntax | Medium | Use interval `..` syntax |
| `--at` flag | Low | Use positional time arg |
| `--yes`/`--auto-create-project` | Low | Standardize to `-y`/`-n` |

---

## Implementation Status

### ✅ Phase 1: Bug Fixes & Quick Wins (COMPLETED)

| # | Item | Status | Notes |
|---|------|--------|-------|
| 1.1 | Fix `summary` → `show` in abbrev.rs | ✅ Done | Changed in TASK_SUBCOMMANDS |
| 1.2 | Extend implicit show to ID lists/ranges | ✅ Done | `tatl 1,2,3` and `tatl 1-5` now work |
| 1.3 | Standardize `-y` flags | ✅ Done | All `--yes` flags now have `-y` short form |

### ✅ Phase 2: Timer Command Overhaul (COMPLETED)

| # | Item | Status | Notes |
|---|------|--------|-------|
| 2.1 | Add `on` command | ✅ Done | Replaces `clock in` |
| 2.2 | Add `off` command | ✅ Done | Replaces `clock out` |
| 2.3 | Add `dequeue` command | ✅ Done | Replaces `clock drop`, defaults to queue[0] |
| 2.4 | Add `--on` flag to `add` | ✅ Done | Replaces `--clock-in` |
| 2.5 | Add `--on` flag to `modify` | ✅ Done | Start timing after modify |
| 2.6 | Remove all `clock` subcommands | ✅ Done | `in`, `out`, `pick`, `next`, `drop`, `clear`, `list` removed |

### Remaining Phases

#### Phase 3: Interval Syntax Unification
| # | Item | Status | Notes |
|---|------|--------|-------|
| 3.1 | Implement one-sided intervals `..17:00`, `09:00..` | ☐ Pending | For session modify |
| 3.2 | Support range filters `due:2026-01-01..2026-01-31` | ☐ Pending | Filter parser change |
| 3.3 | Remove `--at` flag from `finish` | ☐ Pending | Use positional time arg |
| 3.4 | Update `sessions modify` to use interval syntax | ☐ Pending | Replace `start:`/`end:` |
| 3.5 | Update `sessions add` to require interval | ☐ Pending | Consistency |

#### Phase 4: Default & Target Improvements
| # | Item | Status | Notes |
|---|------|--------|-------|
| 4.1 | `close` defaults to queue[0] | ☐ Pending | Match `finish` behavior |
| 4.2 | Standardize target patterns across commands | ☐ Pending | Consolidate parsing code |
| 4.3 | Add `show` to TASK_SUBCOMMANDS for ID-first | ✅ Done | `tatl 10 show` works |

#### Phase 5: New Features
| # | Item | Status | Notes |
|---|------|--------|-------|
| 5.1 | Add `projects delete` | ☐ Pending | No cascade to nested |
| 5.2 | Add `report` command with subcommands | ☐ Pending | `time`, `tasks`, `projects` |
| 5.3 | Merge kanban `working` → `queue` | ☐ Pending | Simplify kanban model |

#### Phase 6: Documentation
| # | Item | Status | Notes |
|---|------|--------|-------|
| 6.1 | Finalize single-letter aliases | ☐ Pending | After syntax stabilizes |
| 6.2 | Update COMMAND_REFERENCE.md | ☐ Pending | Reflect all changes |
| 6.3 | Update README.md | ☐ Pending | Quick start examples |

---

## Next Steps

1. ☐ Resolve open question: sessions/time/clock terminology
2. ☐ Begin Phase 3 implementation (interval syntax)
3. ☐ Update documentation for completed changes

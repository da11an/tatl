# Plan 39c: CLI Syntax with Pipe Operator and Equality

## Problem Statement

Tatl's current syntax mixes conventional CLI patterns with TaskWarrior-style conventions, creating several issues:

1. **Action flags create special cases** - `--on`, `--onoff`, `--enqueue` are not really flags but actions on tasks
2. **Unconventional separator** - `:` colon is TaskWarrior-specific, while `=` is universal
3. **Limited composability** - Can't easily chain multiple actions on the same task
4. **Flag inconsistencies** - `--on` requires equals syntax while `--onoff` accepts space syntax
5. **No comparison operators** - Current syntax can't express `due > tomorrow`
6. **High syntax surface area** - Many special flags for different action combinations

This plan proposes two transformative changes:

1. **Pipe operator (`then`)** - Chain commands on the same task, eliminating action flags
2. **Equality operators (`=`, `>`, `<`, etc.)** - Use standard operators for fields and comparisons

These changes create a **dramatically simpler, more composable, and more conventional** CLI syntax while maintaining all capabilities.

---

## Core Change 1: Pipe Operator

### The Concept

Instead of special action flags, use a **pipe operator** to chain commands that operate on the same task:

```bash
# Current (special flags)
tatl add "Fix bug" project:work --on
tatl add "Historical work" --onoff 09:00..12:00 --finish
tatl modify 10 project:home --on

# Proposed (pipe operator)
tatl add "Fix bug" project=work then on
tatl add "Historical work" then onoff 09:00..12:00 then finish
tatl modify 10 project=home then on
```

### Why `then`?

**Candidates considered:**
- `|` - Requires shell escaping `\|` every time (deal-breaker)
- `->` - Good alternative, but less readable
- `,` - Conflicts with value lists (`status=pending,completed`)
- `;` - Requires shell escaping `\;`
- `then` - **No escaping, reads naturally, no conflicts** ✓

**The winner:** `then` (keyword)

```bash
tatl add "Staff meeting" project=admin alloc=30m due=today then on 09:30
```

Reads as: "Add task THEN start timing at 9:30"

### Pipe Semantics

The pipe operator creates a sequential flow:

1. First command executes → produces task ID
2. Task ID implicitly passed to next command
3. Next command executes on that task
4. Repeat for each `then`

**Example:**
```bash
tatl add "Client call" project=sales then on 14:00 then annotate "Discussed Q1"
```

**Execution:**
1. `add "Client call" project=sales` → creates task #123
2. `on 14:00` (task #123) → starts timing task 123 at 14:00
3. `annotate "Discussed Q1"` (task #123) → adds annotation to task 123

### What Gets Eliminated

**These action flags disappear entirely:**
- `--on` → `then on`
- `--onoff <interval>` → `then onoff <interval>`
- `--enqueue` → `then enqueue`
- `--finish` → `then finish`
- `--close` → `then close`
- `--next` → `finish then on` (finish current, start next)

**Syntax becomes:**
- Cleaner (no special action flags)
- More composable (chain any commands)
- More consistent (one pattern for all actions)
- More conventional (resembles Unix pipes, dplyr %>%, method chaining)

---

## Core Change 2: Equality Operators

### The Concept

Replace `:` with `=` and enable comparison operators:

```bash
# Current (colon separator)
tatl add "Fix bug" project:work due:tomorrow
tatl list project:work status:pending

# Proposed (equality operators)
tatl add "Fix bug" project=work due=tomorrow
tatl list project=work status=pending
tatl list "due>tomorrow" "allocation>=2h"    # NEW: Comparisons
tatl list status=pending,completed           # IN operator
tatl list project!=work                      # Negation
```

### Why `=`?

**Universal convention:**
- SQL: `WHERE project='work' AND due > '2026-01-15'`
- Docker: `--env KEY=VALUE`
- Kubernetes: `-l app=nginx,env=prod`
- Git: `git config user.name=value`
- Shell: `export VAR=value`

Everyone knows `key=value`. Zero learning curve.

### Operator Set

```bash
=           # Set value or test equality
=val1,val2  # IN operator (OR logic)
>           # Greater than (dates/durations)
<           # Less than
>=          # Greater than or equal
<=          # Less than or equal
!=          # Not equal
<>          # Not equal (SQL-style)
```

---

## Combined Example

**Before (current syntax):**
```bash
tatl add "Staff meeting" project:admin allocation:30m due:today --on=09:30
tatl add "Yesterday's work" project:dev --onoff 09:00..17:00 --finish
tatl list project:work +urgent
tatl modify 10 project:home --on
tatl finish 10 --next
```

**After (pipe + equality):**
```bash
tatl add "Staff meeting" project=admin allocation=30m due=today then on 09:30
tatl add "Yesterday's work" project=dev then onoff 09:00..17:00 then finish
tatl list project=work +urgent
tatl modify 10 project=home then on
tatl finish 10 then on
```

**Impact:**
- Action flags eliminated
- `:` replaced with `=`
- Dramatically more readable
- More composable (chain any actions)
- More conventional (standard operators)

---

## Detailed Changes

### 1. Replace `:` with `=` for Field Tokens

**Setting fields:**
```bash
tatl add "task" project=work due=tomorrow allocation=2h
tatl modify 10 project=home scheduled=+1w
```

**Equality filtering:**
```bash
tatl list project=work
tatl list status=pending,completed    # IN operator (OR)
```

**Comparison filtering (NEW):**
```bash
tatl list "due>now"                   # Overdue
tatl list "due<=eod"                  # Due today or earlier
tatl list "allocation>=2h"            # Large tasks
tatl list project!=work               # Everything except work
```

**Clearing fields:**
```bash
tatl modify 10 project=none           # Explicit
tatl modify 10 project=               # Empty (also clears)
```

---

### 2. Introduce `then` Pipe Operator

**Basic piping:**
```bash
tatl add "Fix bug" project=work then on
tatl add "Backlog item" project=work then enqueue
```

**Multi-stage piping:**
```bash
tatl add "Historical task" then onoff 09:00..17:00 then finish
tatl add "Client call" then on 14:00 then annotate "Discussed renewal"
```

**Pipe from any command:**
```bash
tatl modify 10 project=home then on              # Modify, then start
tatl finish 10 then on                           # Finish, then start next
tatl enqueue 5 then on                           # Enqueue, then start
```

**Pipe-able commands:**

After `add` (creates task):
- `on [time]` - Start timing
- `onoff <interval>` - Add historical session
- `enqueue` - Add to queue
- `finish` - Create completed (triggers respawn)
- `close` - Create closed
- `annotate <text>` - Add note
- `send <recipient>` - Send external

After `modify` (updates task):
- Same as above

After `finish` (completes task):
- `on [time]` - Start next task

---

### 3. Remove Field Abbreviations

**Only full field names allowed:**
```bash
tatl list status=pending              # ✓
tatl list st=pending                  # ✗ Error with suggestion
```

Commands can still abbreviate (`mod`, `fin`), but field names cannot.

---

### 4. Simplify Respawn Syntax

**Remove redundant keywords:**
```bash
# Simple frequencies
respawn=daily
respawn=weekly

# Intervals (remove "every:")
respawn=2d              # Every 2 days
respawn=3w              # Every 3 weeks

# Weekdays (remove "weekdays:")
respawn=mon,wed,fri

# Monthdays (remove "monthdays:")
respawn=1,15

# Nth weekday (remove "nth:", use hyphen)
respawn=2nd-tue         # 2nd Tuesday
respawn=last-fri        # Last Friday
```

---

### 5. Standardize Clear-Field Syntax

**Both forms accepted:**
```bash
project=none            # Explicit
project=                # Empty (also clears)
```

With `=`, the empty value looks intentional (like `unset VAR=` in shell).

---

### 6. Improve Discoverability

**Add `tatl fields` command:**
```bash
$ tatl fields
Built-in Fields:
  project      - Project assignment
  due          - Due date/time
  scheduled    - Scheduled date/time
  allocation   - Time allocation
  respawn      - Respawn pattern

Operators:
  =            - Set or test equality
  >            - Greater than
  <            - Less than
  >=, <=       - Greater/less than or equal
  !=, <>       - Not equal

Pipe Operator:
  then         - Chain commands on same task
                 Example: tatl add "task" then on
```

**Enhanced help text:**
```bash
$ tatl add --help
Create a new task

USAGE:
  tatl add [description] [field=value]... [+tag]... [then <command>]...

EXAMPLES:
  tatl add "Fix bug" project=work due=tomorrow +urgent
  tatl add "Staff meeting" project=admin then on 09:30
  tatl add "Historical" then onoff 09:00..12:00 then finish

FIELD SYNTAX:
  field=value    Set field (project=work, due=tomorrow)
  +tag           Add tag
  -tag           Remove tag

PIPE SYNTAX:
  then <command> Chain additional commands
                 tatl add "task" then on
                 tatl add "task" then onoff 09:00..12:00 then finish
```

---

## Implementation Plan

### Phase 1: Core Syntax Changes (v1.0)

1. **Replace `:` with `=`**
   - Support both during transition with deprecation warnings
   - Add comparison operator parsing (`>`, `<`, `>=`, `<=`, `!=`, `<>`)

2. **Add `then` pipe operator**
   - Parse `then` as command separator
   - Pass task ID context through pipe chain
   - Remove action flags (`--on`, `--onoff`, etc.)

3. **Simplify respawn syntax**
   - Remove `every:`, `weekdays:`, `monthdays:`, `nth:` prefixes
   - Support old syntax with deprecation warnings

4. **Remove field abbreviations**

### Phase 2: Discoverability (v1.1)

5. Add `tatl fields` command
6. Enhance help text with pipe examples
7. Add shell completion

### Phase 3: Advanced Features (v1.2)

8. Add `--confirm` flag for explicit multi-target confirmation
9. Improve error messages with operator suggestions

---

## Migration Guide

### Field Tokens (`:` → `=`)
```bash
# Before
tatl add "task" project:work due:tomorrow
tatl list project:work status:pending

# After
tatl add "task" project=work due=tomorrow
tatl list project=work status=pending
```

### Action Flags → Pipe Operator
```bash
# Before
tatl add "task" project:work --on
tatl add "task" --onoff 09:00..12:00 --finish
tatl add "task" --enqueue
tatl finish 10 --next

# After
tatl add "task" project=work then on
tatl add "task" then onoff 09:00..12:00 then finish
tatl add "task" then enqueue
tatl finish 10 then on
```

### Respawn Syntax
```bash
# Before
respawn:every:2d
respawn:weekdays:mon,wed
respawn:nth:2nd:tue

# After
respawn=2d
respawn=mon,wed
respawn=2nd-tue
```

### New Capabilities (Comparisons)
```bash
# Overdue tasks
tatl list "due<now"

# Large tasks due soon
tatl list "allocation>=2h" "due<=+7d"

# Everything except work
tatl list project!=work

# Complex piping
tatl add "Client call" then on 14:00 then annotate "Setup meeting"
```

---

## Appendix A: Complete Syntax Reference

### A.1 Command Structure

```
tatl <command> [args] [field=value]... [+tag]... [then <command> [args]...]*
```

### A.2 Field Token Syntax

**Format:** `fieldname=value`

**Built-in fields:**
```
project=<name>
due=<datetime>
scheduled=<datetime>
wait=<datetime>
allocation=<duration>
respawn=<pattern>
description=<text>
template=<name>
```

**User-defined:** `uda.<key>=<value>`

**Clearing:** `field=none` or `field=`

### A.3 Operator Syntax

**Assignment/Equality:**
```
=              Set or test equality
=val1,val2     IN operator (OR logic)
=none          Set to null / test for null
=              Empty (equivalent to =none)
```

**Comparison (filters only):**
```
>              Greater than
<              Less than
>=             Greater than or equal
<=             Less than or equal
```

**Negation (filters only):**
```
!=             Not equal
<>             Not equal (SQL-style)
```

**Quoting for shell:**
- `=` - No quoting needed
- `>`, `<`, `>=`, `<=` - Must quote: `"due>tomorrow"`
- `!=` - Quote if shell expands history: `"project!=work"`

### A.4 Tag Syntax

```
+<tag>         Add tag
-<tag>         Remove tag
```

Tag names: `[A-Za-z0-9_\-\.]`

### A.5 Pipe Operator Syntax

**Format:** `then <command> [args]...`

**Examples:**
```bash
tatl add "task" then on
tatl add "task" then on 09:30
tatl add "task" then onoff 09:00..12:00
tatl add "task" then onoff 09:00..12:00 then finish
tatl modify 10 project=home then on
tatl finish 10 then on
```

**Pipe-able commands:**
- `on [time]`
- `onoff <interval>`
- `enqueue`
- `finish`
- `close`
- `annotate <text>`
- `send <recipient>`

**Pipe semantics:**
1. First command executes → produces task ID
2. Task ID passed to next command
3. Next command executes on that task
4. Repeat

### A.6 Date/Time Expressions

**Absolute:**
```
2026-01-15
2026-01-15T14:30
14:30
```

**Relative:**
```
+2d, -1w, +3m         Forward/backward offset
today, tomorrow       Named dates
eod, eow, eom         End of period
now                   Current time
```

**Intervals:**
```
2024-01-01..2024-01-31
-7d..now
09:00..12:00
```

### A.7 Duration Syntax

**Format:** Descending units, each once max

```
1h
2h30m
1d2h15m
45s
```

Units: `d` (days), `h` (hours), `m` (minutes), `s` (seconds)

### A.8 Respawn Patterns

**Simple:**
```
respawn=daily
respawn=weekly
respawn=monthly
respawn=yearly
```

**Intervals:**
```
respawn=2d              Every 2 days
respawn=3w              Every 3 weeks
```

**Weekdays:**
```
respawn=mon,wed,fri     Specific days
```

**Monthdays:**
```
respawn=1,15            1st and 15th
```

**Nth weekday:**
```
respawn=2nd-tue         2nd Tuesday
respawn=last-fri        Last Friday
```

### A.9 Filter Expressions

**Field comparisons:**
```
project=work
status=pending,completed          # IN (OR)
"due>tomorrow"                    # Greater than
"allocation>=2h"                  # Greater than or equal
project!=work                     # Not equal
"due!=none"                       # Has value (NOT NULL)
```

**Tag filters:**
```
+urgent                           # Has tag
-waiting                          # Lacks tag
```

**Boolean operators:**
```
project=work +urgent              # Implicit AND
+urgent or +important             # OR
not +waiting                      # NOT
```

**Precedence:**
1. Field comparisons, `not`
2. Implicit `and`
3. `or`

### A.10 Complete Examples

**Task creation:**
```bash
# Basic
tatl add "Fix bug" project=work due=tomorrow +urgent

# With piping
tatl add "Fix bug" project=work then on
tatl add "Staff meeting" project=admin due=today then on 09:30

# Multi-stage
tatl add "Historical task" then onoff 09:00..17:00 then finish

# Already completed
tatl add "Yesterday's work" then finish

# Enqueue for later
tatl add "Backlog item" then enqueue

# Complex workflow
tatl add "Client call" project=sales then on 14:00 then annotate "Renewal discussion"
```

**Task modification:**
```bash
# Basic
tatl modify 10 +urgent due=+2d

# With piping
tatl modify 10 project=home then on
tatl modify project=work allocation=2h then enqueue
```

**Filtering:**
```bash
# Equality
tatl list project=work status=pending

# Comparisons
tatl list "due>now"                          # Overdue
tatl list "due<=eod"                         # Due today
tatl list "allocation>=2h"                   # Large tasks

# Complex
tatl list project=work "due>now" "due<=+7d" +urgent
# Work tasks, overdue or due within week, urgent
```

**Time tracking:**
```bash
# Traditional
tatl on 10
tatl off
tatl onoff 09:00..12:00 10

# With piping
tatl add "Meeting" then on 14:00
tatl modify 5 project=urgent then on
tatl finish 10 then on                        # Finish, start next
```

**Sessions:**
```bash
tatl sessions list -7d project=work
tatl sessions modify 1 09:00..17:00 --yes
tatl sessions report 2024-01-01..2024-01-31
```

**Projects:**
```bash
tatl projects add work.email.inbox
tatl projects list
tatl projects report
```

---

## Appendix B: Design Rationale

### Why Pipe Operator?

**Problem:** Action flags (`--on`, `--onoff`) are not really flags but commands

**Solution:** Treat them as commands, chain with pipe operator

**Benefits:**
- **Eliminates special cases** - All actions are primary commands
- **Composable** - Chain any sequence: `then on then annotate then send`
- **Consistent** - One pattern for all operations
- **Familiar** - Unix pipes, dplyr %>%, method chaining

**Why `then` not `|`?**
- `|` requires escaping `\|` every time (shell pipe)
- `then` works naturally, no escaping
- `then` reads like English: "add task THEN start timing"

### Why `=` Over `:`?

**Universal convention:** SQL, Docker, Kubernetes, Git, shell all use `key=value`

**Benefits:**
- Natural extension to `>`, `<`, `>=`, `<=`, `!=`
- No special cases (`:` appears in times and URLs)
- Clearer semantics (`=` means equals/assign, `:` just separates)

**Trade-off:** Shell escaping for `<`, `>` (~5% of operations)

**Verdict:** Benefits far outweigh minor quoting for advanced queries

### Why Keep `+`/`-` for Tags?

**Multi-valued semantics:** Tags are a set (can have many)

**Add/remove clarity:** `+` = add, `-` = remove

**No `=` alternative:** `tag=urgent` ambiguous (replace all? add one?)

### Why Remove Field Abbreviations?

**Open set:** UDAs make fields unbounded

**Collision risk:** Adding fields breaks abbreviations

**Commands vs fields:** Commands (closed set) can abbreviate, fields (open set) cannot

---

## Appendix C: Syntax Comparison

### Before (Current)

```bash
tatl add "Fix bug" project:work due:tomorrow +urgent --on
tatl add "Historical" --onoff 09:00..12:00 --finish
tatl list project:work status:pending
tatl modify 10 project:home --on
tatl finish 10 --next
```

**Issues:**
- `:` separator is TaskWarrior-specific
- Action flags are special cases
- No comparisons (`due > tomorrow`)
- Limited composability

### After (Plan 39c)

```bash
tatl add "Fix bug" project=work due=tomorrow +urgent then on
tatl add "Historical" then onoff 09:00..12:00 then finish
tatl list project=work status=pending
tatl modify 10 project=home then on
tatl finish 10 then on
```

**Benefits:**
- `=` is universal convention
- No action flags (pure commands)
- Comparisons: `"due>tomorrow"`
- Infinitely composable via `then`

### Character Count

**Action flags vs pipe:**
- `--on` (4 chars) vs `then on` (7 chars): +3
- But: eliminates flag ambiguity, increases composability, reduces special cases

**Net:** Slightly more typing for dramatically clearer, more consistent syntax

---

## Appendix D: Future Considerations

### Shell Completion

```bash
# Field names
tatl add pro<TAB>  → project=

# Operators after field
tatl list due<TAB> → due=  due>  due<  due>=  due<=  due!=

# Project names
tatl add project=w<TAB> → work  workspace

# Pipe commands
tatl add "task" then <TAB> → on  onoff  enqueue  finish  annotate  send
```

### Advanced Piping

**Conditional piping (future):**
```bash
tatl finish 10 then if queued on
# If queue not empty, start next; otherwise do nothing
```

**Named pipes (future):**
```bash
tatl add "task" then save as task1
tatl add "dependency" then depends on task1
```

All extensions must maintain composability and consistency.

---

## Decision

**Approve Phase 1-3 implementation** with:

1. **Pipe operator `then`** as primary simplification
2. **Equality operators `=`** with comparisons as universal syntax
3. Breaking changes gated behind v1.0 with deprecation path

This creates a **dramatically simpler, more conventional, and more powerful** CLI that aligns with universal patterns (SQL, Unix pipes, modern DSLs) while eliminating special cases and maximizing composability.

The combination of `then` pipes and `=` operators makes tatl both easier to learn (familiar patterns) and more expressive (chain any operations, query with SQL-like syntax).

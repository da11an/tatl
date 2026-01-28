# Plan 39d: CLI Syntax with Pipe Operator and Equality

## Problem Statement

Tatl's current syntax has grown organically, creating inconsistencies and unnecessary complexity:

1. **Action flags are actually commands** - `--on`, `--onoff`, `--enqueue` aren't flags but actions on tasks
2. **Unconventional separator** - `:` colon is TaskWarrior-specific, while `=` is universal
3. **Limited composability** - Can't easily chain multiple actions on the same task
4. **No comparison operators** - Can't express `due > tomorrow` or `allocation >= 2h`
5. **High syntax surface area** - Many special flags for different action combinations
6. **Poor scannability** - Text blends together, structure isn't visually obvious

This plan proposes two transformative changes that create a **dramatically simpler, more scannable, and more conventional** CLI syntax:

1. **Pipe operator (` : `)** - Chain commands on the same task using space-colon-space
2. **Equality operators (`=`, `>`, `<`, etc.)** - Use standard operators for fields and comparisons

---

## Core Change 1: Pipe Operator (` : `)

### The Concept

Use **space-colon-space** (` : `) to chain commands that operate on the same task:

```bash
# Current (special flags)
tatl add "Fix bug" project:work --on
tatl add "Historical work" --onoff 09:00..12:00 --finish

# Proposed (pipe operator)
tatl add "Fix bug" project=work : on
tatl add "Historical work" : onoff 09:00..12:00 : finish
```

### Why ` : ` (Space-Colon-Space)?

**Visual scannability:**
```bash
tatl add "Staff meeting" project=admin due=today : on 09:30
#                                                 ^
#                                                 Instantly visible pipe
```

The ` : ` creates a **visual break** - like `|` in Unix pipes - making command structure immediately apparent. Compare to `then`:

```bash
tatl add "Staff meeting" project=admin due=today then on 09:30
#                                                 ^^^^
#                                                 Blends with text
```

With `then`, you have to **read** to find the pipe. With ` : `, you **see** it instantly.

**Other advantages:**
- ✅ No shell escaping needed
- ✅ Brief (3 chars with spaces)
- ✅ Resembles Unix `|` (single vertical character)
- ✅ No conflicts with existing syntax

### Parsing Rules

**What is ` : `?**
- Only **space-colon-space** (exactly) is the pipe operator
- Shell tokenizes this as three tokens: `["previous_token", ":", "next_token"]`

**What is NOT ` : `?**
- `09:30` - Time (no spaces around `:`)
- `https://` - URL (no spaces)
- `Note:` - End of word (no space before)
- `:tag` - Start of word (no space after)

**Edge case: Descriptions with ` : `**
```bash
tatl add "Formula: x : y = z" : on
#                    ^^^      ^
#                    Ambiguous Pipe

# Solution: Quote description (standard practice)
tatl add "Formula: x : y = z" : on
```

This is **extremely rare** in practice. Most descriptions don't contain space-colon-space.

### Pipe Semantics

The pipe operator creates a sequential flow:

1. First command executes → produces task ID
2. Task ID implicitly passed to next command
3. Next command executes on that task
4. Repeat for each ` : `

**Example:**
```bash
tatl add "Client call" project=sales : on 14:00 : annotate "Discussed renewal"
```

**Execution:**
1. `add "Client call" project=sales` → creates task #123
2. `on 14:00` (task #123) → starts timing task 123 at 14:00
3. `annotate "Discussed renewal"` (task #123) → adds annotation to task 123

### What Gets Eliminated

**These action flags disappear entirely:**
- `--on` → ` : on`
- `--onoff <interval>` → ` : onoff <interval>`
- `--enqueue` → ` : enqueue`
- `--finish` → ` : finish`
- `--close` → ` : close`
- `--next` → `finish : on` (finish current, start next)

**Result:**
- Smaller syntax surface area
- More composable (chain any commands)
- More consistent (one pattern for all actions)
- More scannable (visual structure)

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

**Everyone knows `key=value`.** Zero learning curve.

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

### Shell Escaping

**No escaping needed:**
- `=` operator: `project=work` ✓
- IN operator: `status=pending,completed` ✓

**Escaping required:**
- `>`, `<` operators: Must quote to prevent shell redirection
  ```bash
  tatl list "due>tomorrow"        # Quoted
  tatl list due\>tomorrow         # Escaped
  ```
- `>=`, `<=` operators: Also quote
  ```bash
  tatl list "allocation>=2h"
  ```

**Impact:** ~5% of operations (comparison filters) need quoting. 95% (setting fields, equality filters) work naturally.

---

## Combined Syntax

### Before (Current)

```bash
tatl add "Staff meeting" project:admin allocation:30m due:today --on=09:30
tatl add "Yesterday's work" project:dev --onoff 09:00..17:00 --finish
tatl list project:work +urgent
tatl modify 10 project:home --on
tatl finish 10 --next
```

**Issues:**
- `:` separator is unconventional
- Action flags are special cases
- No comparisons possible
- Poor scannability (text blends)

### After (Plan 39d)

```bash
tatl add "Staff meeting" project=admin allocation=30m due=today : on 09:30
tatl add "Yesterday's work" project=dev : onoff 09:00..17:00 : finish
tatl list project=work +urgent
tatl modify 10 project=home : on
tatl finish 10 : on
```

**Benefits:**
- `=` is universal convention
- ` : ` pipe operator (no action flags)
- Comparisons: `"due>tomorrow"`
- Excellent scannability (` : ` pops visually)
- Infinitely composable

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
tatl list "due!=none"                 # Has due date (not null)
```

**Clearing fields:**
```bash
tatl modify 10 project=none           # Explicit
tatl modify 10 project=               # Empty (also clears)
```

---

### 2. Introduce ` : ` Pipe Operator

**Basic piping:**
```bash
tatl add "Fix bug" project=work : on
tatl add "Backlog item" project=work : enqueue
```

**Multi-stage piping:**
```bash
tatl add "Historical task" : onoff 09:00..17:00 : finish
tatl add "Client call" : on 14:00 : annotate "Discussed renewal"
```

**Pipe from any command:**
```bash
tatl modify 10 project=home : on              # Modify, then start
tatl finish 10 : on                           # Finish, then start next
tatl enqueue 5 : on                           # Enqueue, then start
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

**Invalid pipes:**
- `off` - No task context (just stops timing)
- `list` - Doesn't produce single task
- `delete` - Task no longer exists

---

### 3. Remove Field Abbreviations

**Only full field names allowed:**
```bash
tatl list status=pending              # ✓
tatl list st=pending                  # ✗ Error with suggestion
```

**Rationale:**
- Commands (closed set) can abbreviate: `mod` → `modify`
- Fields (open set, UDAs) cannot abbreviate (prevents collisions)

---

### 4. Simplify Respawn Syntax

**Remove redundant keywords:**
```bash
# Simple frequencies
respawn=daily
respawn=weekly
respawn=monthly

# Intervals (remove "every:")
respawn=2d              # Every 2 days
respawn=3w              # Every 3 weeks

# Weekdays (remove "weekdays:")
respawn=mon,wed,fri

# Monthdays (remove "monthdays:")
respawn=1,15            # 1st and 15th

# Nth weekday (remove "nth:", use hyphen)
respawn=2nd-tue         # 2nd Tuesday
respawn=last-fri        # Last Friday
```

**Pattern recognition rules:**
- Contains duration unit (`2d`, `3w`) → interval
- Contains day names (`mon,wed`) → weekdays
- Contains numbers (`1,15`) → monthdays
- Contains ordinal-hyphen-day (`2nd-tue`) → nth weekday

---

### 5. Standardize Clear-Field Syntax

**Both forms accepted:**
```bash
project=none            # Explicit keyword
project=                # Empty value (also clears)
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
  wait         - Wait until date
  allocation   - Time allocation
  respawn      - Respawn pattern

Operators:
  =            - Set or test equality
  >            - Greater than
  <            - Less than
  >=, <=       - Greater/less than or equal
  !=, <>       - Not equal

Pipe Operator:
  :            - Chain commands (must have spaces: " : ")
                 Example: tatl add "task" : on
```

**Enhanced help text:**
```bash
$ tatl add --help
Create a new task

USAGE:
  tatl add [description] [field=value]... [+tag]... [ : <command>]...

EXAMPLES:
  tatl add "Fix bug" project=work due=tomorrow +urgent
  tatl add "Staff meeting" project=admin : on 09:30
  tatl add "Historical" : onoff 09:00..12:00 : finish

FIELD SYNTAX:
  field=value    Set field (project=work, due=tomorrow)
  +tag           Add tag
  -tag           Remove tag
  field=none     Clear field
  field=         Clear field (empty)

PIPE SYNTAX:
  :              Chain commands (space-colon-space)
                 tatl add "task" : on
                 tatl add "task" : onoff 09:00..12:00 : finish
```

---

## Implementation Plan

### Phase 1: Core Syntax Changes (v0.x → v1.0)

1. **Replace `:` with `=`**
   - Support both during transition with deprecation warnings
   - Add comparison operator parsing (`>`, `<`, `>=`, `<=`, `!=`, `<>`)

2. **Add ` : ` pipe operator**
   - Parse space-colon-space as command separator
   - Pass task ID context through pipe chain
   - Remove action flags (`--on`, `--onoff`, etc.)

3. **Simplify respawn syntax**
   - Remove redundant prefixes
   - Support old syntax with deprecation warnings

4. **Remove field abbreviations**

### Phase 2: Discoverability (v1.1)

5. Add `tatl fields` command
6. Enhance help text with pipe examples
7. Add shell completion

### Phase 3: Polish (v1.2)

8. Add `--confirm` flag for explicit confirmation
9. Improve error messages with suggestions

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
tatl finish 10 --next

# After
tatl add "task" project=work : on
tatl add "task" : onoff 09:00..12:00 : finish
tatl finish 10 : on
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

### New Capabilities
```bash
# Comparisons
tatl list "due>now"                          # Overdue
tatl list "allocation>=2h"                   # Large tasks
tatl list project!=work                      # Everything except work

# Complex piping
tatl add "Client call" : on 14:00 : annotate "Renewal discussion"
tatl add "Historical" : onoff 09:00..17:00 : finish
```

---

## Appendix A: Complete Syntax Reference

### A.1 Command Structure

```
tatl <command> [args] [field=value]... [+tag]... [ : <command> [args]...]*
```

**Key elements:**
- `field=value` - Field assignment/filtering
- `+tag` / `-tag` - Tag operations
- ` : ` - Pipe operator (space-colon-space)

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

**Comparison (filters only, must quote for shell):**
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

**Shell escaping:**
- `=` - No quoting needed
- `>`, `<`, `>=`, `<=` - Must quote: `"due>tomorrow"`
- `!=` - Quote if bash history expansion enabled

### A.4 Tag Syntax

```
+<tag>         Add tag
-<tag>         Remove tag
```

Tag names: `[A-Za-z0-9_\-\.]`

### A.5 Pipe Operator Syntax

**Format:** ` : <command> [args]...`

**Critical:** Must have **spaces** around colon: ` : `

**Examples:**
```bash
tatl add "task" : on
tatl add "task" : on 09:30
tatl add "task" : onoff 09:00..12:00
tatl add "task" : onoff 09:00..12:00 : finish
tatl modify 10 project=home : on
tatl finish 10 : on
```

**Not pipes (no spaces):**
```
09:30          Time
https://       URL
Note:          Description text
```

**Pipe-able commands:**
- `on [time]` - Start timing
- `onoff <interval>` - Add historical session
- `enqueue` - Add to queue
- `finish` - Complete task
- `close` - Close task
- `annotate <text>` - Add annotation
- `send <recipient>` - Send external

**Pipe semantics:**
1. First command executes → task ID
2. Task ID passed to next command
3. Next command executes on that task
4. Repeat

### A.6 Date/Time Expressions

**Absolute:**
```
2026-01-15              Date only
2026-01-15T14:30        ISO 8601 datetime
14:30                   Time only
```

**Relative:**
```
+2d, -1w, +3m           Offset
today, tomorrow         Named
eod, eow, eom           End of period
now                     Current
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

Units: `d`, `h`, `m`, `s`

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
"allocation>=2h"                  # Greater/equal
project!=work                     # Not equal
"due!=none"                       # Has value
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

# With pipe
tatl add "Fix bug" project=work : on
tatl add "Staff meeting" project=admin due=today : on 09:30

# Multi-stage
tatl add "Historical task" : onoff 09:00..17:00 : finish

# Complex workflow
tatl add "Client call" project=sales : on 14:00 : annotate "Renewal discussion"
```

**Task modification:**
```bash
# Basic
tatl modify 10 +urgent due=+2d

# With pipe
tatl modify 10 project=home : on
tatl modify project=work allocation=2h : enqueue
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
```

**Time tracking:**
```bash
# Traditional
tatl on 10
tatl off
tatl onoff 09:00..12:00 10

# With pipe
tatl add "Meeting" : on 14:00
tatl modify 5 project=urgent : on
tatl finish 10 : on                          # Finish, start next
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

### Why ` : ` (Space-Colon-Space)?

**Scannability is critical for CLI tools.**

Compare:
```bash
# With "then" (blends into text)
tatl add "Fix bug" project=work then on then annotate "Note"
#                               ^^^^    ^^^^
#                               Have to READ to find pipes

# With " : " (visually pops)
tatl add "Fix bug" project=work : on : annotate "Note"
#                                ^     ^
#                                Instantly VISIBLE
```

The ` : ` creates a **visual break** - just like `|` in Unix pipes. You don't have to read it, you just see the structure.

**Why not other operators?**
- `|` - Requires shell escaping `\|` every time (deal-breaker)
- `then` - Too readable, blends with text (not scannable)
- `->` - Less visual, doesn't pop like single character
- `::` - Semantic mismatch (namespace operator, not sequence)
- `,` - Conflicts with value lists (`status=pending,completed`)
- `;` - Requires shell escaping

**Why ` : ` works:**
- ✅ Visual (single vertical character like `|`)
- ✅ Scannable (instantly visible)
- ✅ Brief (3 chars with spaces)
- ✅ No escaping needed
- ✅ No conflicts (times and URLs don't have spaces around `:`)

### Why `=` Over `:`?

**Universal convention:** `key=value` appears in:
- SQL: `WHERE project='work'`
- Docker: `--env KEY=VALUE`
- Kubernetes: `-l app=nginx`
- Git: `git config user.name=value`
- Shell: `export VAR=value`

**Natural operator extension:**
- `=` extends to `>`, `<`, `>=`, `<=`, `!=` naturally
- `:` has no natural comparison operators

**No special cases:**
- With `:`, times (`09:30`) and URLs (`https://`) need exceptions
- With `=`, they don't contain `=`, so no ambiguity

**Verdict:** `=` is more conventional, more powerful, cleaner to parse

### Why Keep `+`/`-` for Tags?

**Multi-valued semantics:** Tags are a set, can have many simultaneously

**Add/remove clarity:**
- `+` explicitly means "add to set"
- `-` explicitly means "remove from set"

**No `=` alternative:**
- `tag=urgent` ambiguous: replace all tags? add one tag?
- `+urgent` is unambiguous: add this tag

### Why Remove Field Abbreviations?

**Open set:** UDAs make fields unbounded, abbreviations cause collisions

**Commands vs fields:**
- Commands (closed set, frequent) → can abbreviate
- Fields (open set, UDAs) → cannot abbreviate

**Example collision:**
- `p:work` → `project:work` (works)
- User adds `uda.priority:high`
- Now `p:` is ambiguous (project or priority?)

**Verdict:** Remove abbreviations to prevent future breakage

---

## Appendix C: Scannability Analysis

### Visual Structure Comparison

**Current syntax (poor scannability):**
```bash
tatl add "Fix bug in authentication handler" project:work allocation:2h due:tomorrow +urgent --on
```

Everything blends together. Hard to see structure at a glance.

**With `=` but no pipe (better):**
```bash
tatl add "Fix bug in authentication handler" project=work allocation=2h due=tomorrow +urgent --on
```

Better, but `--on` still looks like a flag, not an action.

**With ` : ` pipe (excellent scannability):**
```bash
tatl add "Fix bug in authentication handler" project=work allocation=2h due=tomorrow +urgent : on
#                                                                                             ^
#                                                                                         POPS VISUALLY
```

The ` : ` instantly shows: "description and fields" | "action"

### Multi-Stage Pipeline Scannability

**Without visual pipe:**
```bash
tatl add "Client call discussion about Q1 renewal" project=sales then on 14:00 then annotate "Discussed pricing"
```

Have to **read** the entire line to understand structure.

**With ` : ` visual pipe:**
```bash
tatl add "Client call discussion about Q1 renewal" project=sales : on 14:00 : annotate "Discussed pricing"
#                                                                  ^         ^
#                                                                  STAGE 1   STAGE 2
```

**Instantly scannable** - see three stages at a glance.

### Real-World Usage Pattern

Power users scan their shell history constantly. Visual structure matters:

```bash
# Shell history
tatl add "Meeting" project=admin : on 09:30
tatl add "Code review" project=dev : on : annotate "Found 3 issues"
tatl add "Email triage" project=admin : onoff 10:00..10:30 : finish
tatl modify 15 project=urgent : on
tatl finish 10 : on
tatl list "due<=eod" status=pending
```

The ` : ` pipes **jump out** - you immediately see which commands are creating/modifying tasks vs. filtering.

---

## Appendix D: Edge Cases and Solutions

### Edge Case 1: Description Contains ` : `

**Problem:**
```bash
tatl add "Formula: x : y = z" : on
#                    ^^^      ^
#                    Ambiguous?
```

**Solution:** Quote description (standard practice)
```bash
tatl add "Formula: x : y = z" : on
```

Shell sees: `["tatl", "add", "Formula: x : y = z", ":", "on"]`

The `:` within the quoted string is protected, only the standalone `:` is parsed as pipe.

**How common is this?**
- Very rare - most people don't write ` : ` in task descriptions
- More common: `Note:` (no space before), `09:30` (no spaces)
- Easy fix: quote description

### Edge Case 2: Time Expressions

**Not a problem:**
```bash
tatl add "Meeting" due=09:30 : on 14:00
#                      ^^^^      ^ ^^^^
#                      Time      Pipe Time
```

Times have no spaces around `:`, so no ambiguity.

### Edge Case 3: URLs

**Not a problem:**
```bash
tatl add "Check https://example.com/path" : on
#                     ^^                  ^
#                     URL                 Pipe
```

URLs have no spaces around `:`, so no ambiguity.

### Edge Case 4: Multiple Pipes in Quoted Description

**Problem:**
```bash
tatl add "Steps: download : process : upload" : on
#                ^^^^^^^^^^^^^^^^^^^^^^^^^^^  ^
#                Quoted description            Pipe
```

**Solution:** Shell quoting protects interior `:` characters
```bash
# Shell sees:
["tatl", "add", "Steps: download : process : upload", ":", "on"]
#                ^--- Single token, colons protected  ^--- Pipe operator
```

**No ambiguity** - shell handles quoting before tatl sees arguments.

---

## Appendix E: Future Considerations

### Shell Completion

Implement completion for:
- Field names: `pro<TAB>` → `project=`
- Operators: `due<TAB>` → `due=`, `due>`, `due<`, etc.
- Pipe commands: ` : <TAB>` → `on`, `onoff`, `enqueue`, `finish`, etc.
- Project names: `project=w<TAB>` → `work`, `workspace`

### Advanced Piping

**Conditional piping (future):**
```bash
tatl finish 10 : if queued : on
# If queue not empty, start next; otherwise do nothing
```

**Named task references (future):**
```bash
tatl add "Task A" : save as taskA
tatl add "Task B depends on A" : depends on taskA
```

All extensions must maintain scannability and consistency.

---

## Decision

**Approve Phase 1-3 implementation** with:

1. **Pipe operator ` : `** (space-colon-space) for maximum scannability
2. **Equality operators `=`** with comparisons (`>`, `<`, `>=`, `<=`, `!=`) for universal convention
3. Breaking changes gated behind v1.0 with deprecation warnings in v0.x

This creates a **dramatically simpler, more scannable, and more conventional** CLI that:
- Eliminates action flags (replaced by composable pipes)
- Uses universal operators (`=` like SQL/Docker/Git)
- Provides instant visual structure (` : ` pops like Unix `|`)
- Enables powerful queries (`"due>now"`, `"allocation>=2h"`)
- Maintains all capabilities while reducing syntax surface area

The combination of **scannable pipes** and **universal operators** makes tatl both easier to learn (familiar patterns) and faster to use (visual structure recognition).

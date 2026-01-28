# Plan 39b: CLI Syntax with Equality and Comparison Operators

## Problem Statement

Tatl has a hybrid syntax that mixes conventional CLI patterns (subcommands, flags) with TaskWarrior-style field tokens (`project:work`, `+tag`). While this syntax is functional, it has several issues:

1. **Unconventional separator** - The `:` colon is specific to TaskWarrior, while `=` is universal (SQL, Docker, Kubernetes, Git, environment variables)
2. **No comparison operators** - Current syntax has no natural way to express `due > tomorrow` or `allocation >= 2h`
3. **Flag inconsistencies** - `--on` requires equals syntax while `--onoff` accepts space syntax
4. **Field abbreviation brittleness** - `st:` works but `s:` fails with ambiguity errors
5. **Respawn syntax redundancy** - Mix of standalone values (`daily`) and verbose structured syntax (`every:2d`, `weekdays:mon`)
6. **Clear-field syntax ambiguity** - Both `field:none` and `field:` (empty) work
7. **Discoverability gaps** - Field token syntax is hidden from standard help output

This plan proposes switching to **equality and comparison operators** (`=`, `>`, `<`, `>=`, `<=`, `!=`, `<>`) for field operations, making tatl dramatically more conventional while enabling powerful new filtering capabilities.

---

## Guiding Principles

### What We're Keeping (Strategic Departures from Pure CLI Convention)

1. **Field tokens over flags** - `project=work` not `--project work` (brevity for daily use)
2. **Tag sigils** - `+tag` / `-tag` not `--tag` / `--untag` (visual clarity, conciseness)
3. **Implicit description** - `add "task" +urgent` not `add --description "task"` (reduce ceremony)
4. **Rich time expressions** - `due=tomorrow`, `scheduled=+2d`, `wait=eod` (natural language)

### One Right Way

For each operation, there should be **one canonical syntax** (not multiple equivalent forms). When alternatives exist, deprecate all but one.

### Convention Alignment

**NEW:** Use operators that developers already know from SQL, shell scripting, and configuration files:
- `=` for assignment and equality testing
- `>`, `<`, `>=`, `<=` for comparisons
- `!=`, `<>` for negation

---

## Core Change: Field Token Syntax

### Current (TaskWarrior-style)
```bash
tatl add "Fix bug" project:work due:tomorrow +urgent
tatl list project:work status:pending
tatl modify 10 project:none        # Clear field
```

### Proposed (Equality operators)
```bash
tatl add "Fix bug" project=work due=tomorrow +urgent
tatl list project=work status=pending
tatl list "due>tomorrow" "allocation>=2h"    # NEW: Comparison operators
tatl list status=pending,completed           # IN operator (OR logic)
tatl list project!=work                      # Negation
tatl modify 10 project=none                  # Clear field (explicit)
tatl modify 10 project=                      # Clear field (empty = clear)
```

### Why This Change?

**Universal familiarity:**
```bash
# Docker
docker run --env KEY=VALUE

# Kubernetes
kubectl get pods -l app=nginx,env=prod

# SQL
SELECT * FROM tasks WHERE project='work' AND due > '2026-01-15'

# Git config
git config user.name=value

# Shell
export VAR=value

# Proposed tatl (same pattern)
tatl add project=work due=tomorrow
tatl list project=work "due>2026-01-15"
```

Developers encounter `key=value` syntax dozens of times per day. It's the de facto standard for key-value pairs.

---

## Proposed Changes

### 1. Replace `:` with `=` for Field Tokens (Breaking Change)

**Solution:** Use `=` for all field operations

**Setting fields:**
```bash
tatl add "task" project=work due=tomorrow allocation=2h
tatl modify 10 project=home scheduled=+1w
```

**Equality filtering:**
```bash
tatl list project=work
tatl list status=pending
tatl list status=pending,completed    # Multiple values = OR (IN operator)
```

**Comparison filtering (NEW capability):**
```bash
# Date/time comparisons
tatl list "due>now"                   # Overdue tasks
tatl list "due<=eod"                  # Due today or earlier
tatl list "scheduled>=tomorrow"       # Future scheduled

# Duration comparisons
tatl list "allocation>=2h"            # Large tasks

# Negation
tatl list project!=work               # Everything except work
tatl list "due!=none"                 # Has due date (not null)
tatl list "due<>none"                 # Alternative syntax for !=
```

**Clearing fields:**
```bash
tatl modify 10 project=none           # Explicit keyword
tatl modify 10 project=               # Empty value (clear)
```

**Trade-offs:**

1. **Shell escaping required for `>` and `<`** (affects ~5% of operations)
   ```bash
   tatl list "due>tomorrow"    # Must quote
   tatl list due\>tomorrow     # Or escape
   ```

2. **Potential ambiguity with descriptions containing `=`**
   ```bash
   tatl add Check if status=ready project=work
   # Risk: "status=ready" might be parsed as field if "status" is a known field
   ```

   **Mitigations:**
   - Strict field validation (only known field names match)
   - User education (quote descriptions with `=`)
   - Positional convention (fields before description)
   - In practice: `word=word` only ambiguous if first word is a field name

**Benefits outweigh costs:**
- 80% of operations (setting fields) need no escaping
- 15% (equality filters) need no escaping
- 5% (comparisons) need quoting - but users doing complex queries already quote
- Ambiguity is rare and easily mitigated
- Comparison operators enable powerful queries without inventing new syntax

---

### 2. Fix `--on` Flag Consistency (High Priority)

**Problem:**
```bash
tatl add --on=09:00 "task"           # Works (requires equals)
tatl add --on 09:00 "task"           # Fails
tatl add --onoff 09:00..10:00 "task" # Works (accepts space)
```

**Solution:** Implement Plan 38 Option A

- Accept space syntax in trailing position: `add "task" --on 09:00`
- Keep `require_equals` in clap for leading position (prevents ambiguity)
- Update `modify --on` to accept optional time value (change from `bool` to `Option<String>`)

**Outcome:**
```bash
# All of these work
tatl add "task" --on
tatl add "task" --on 09:00
tatl add "task" --on=09:00
tatl add "task" --onoff 09:00..10:00
tatl add "task" --onoff=09:00..10:00
```

---

### 3. Remove Field Name Abbreviations in Filters (High Priority)

**Problem:**
```bash
tatl list st=pending    # Works (unambiguous)
tatl list sc=tomorrow   # Works (unambiguous)
tatl list s=pending     # Error: ambiguous (status/scheduled)
```

**Solution:** Remove field abbreviation support

- Commands can still abbreviate: `mod` → `modify`, `fin` → `finish`
- Field names must be spelled out: `status=`, `scheduled=`, `project=`

**Outcome:**
```bash
# Only full names work
tatl list status=pending
tatl list scheduled=tomorrow
tatl list project=work

# Abbreviations produce helpful error
tatl list st=pending
# Error: Unknown field 'st'. Did you mean 'status'?
```

---

### 4. Simplify Respawn Syntax (High Priority)

**Problem:** Mix of concise and verbose patterns
```bash
respawn:daily              # Concise
respawn:every:2d           # "every:" is redundant
respawn:weekdays:mon,wed   # "weekdays:" is redundant
```

**Solution:** Use `=` and remove redundant keywords

```bash
# Simple frequencies
respawn=daily
respawn=weekly
respawn=monthly
respawn=yearly

# Interval frequencies (remove "every:")
respawn=2d              # Every 2 days
respawn=3w              # Every 3 weeks

# Specific weekdays (remove "weekdays:")
respawn=mon,wed,fri     # Comma-separated days

# Specific days of month (remove "monthdays:")
respawn=1,15            # 1st and 15th

# Nth weekday of month (remove "nth:", use hyphen)
respawn=2nd-tue         # 2nd Tuesday
respawn=1st-mon         # 1st Monday
```

---

### 5. Standardize Clear-Field Syntax (Medium Priority)

**Solution:** Support both `field=none` and `field=` (empty)

```bash
# Both work, both are clear with = operator
tatl modify 10 project=none       # Explicit keyword
tatl modify 10 project=           # Empty value (intuitive with =)
tatl modify 10 due=               # Clear due date
```

With `=`, the empty value looks intentional (like `unset VAR=` in shell), less ambiguous than `field:`.

---

### 6. Normalize `externals` Command (Medium Priority)

**Solution:** Make it a filter on `list` (Recommended)

```bash
tatl list external=any              # All external tasks
tatl list external=colleague        # Sent to colleague
tatl list external=any status=pending
```

Fewer commands to remember, integrates with existing filter system.

---

### 7. Improve Discoverability (Medium Priority)

**Solution A:** Add `tatl fields` command
```bash
$ tatl fields
Built-in Fields:
  project      - Project assignment (use dot notation: work.email)
  due          - Due date/time
  scheduled    - Scheduled date/time
  wait         - Wait until date (task hidden until then)
  allocation   - Time allocation
  respawn      - Respawn pattern

Operators:
  =            - Set value or test equality
  >            - Greater than (dates/durations)
  <            - Less than (dates/durations)
  >=           - Greater than or equal
  <=           - Less than or equal
  !=, <>       - Not equal

Special Syntax:
  +tag         - Add tag
  -tag         - Remove tag
```

**Solution B:** Enhance command help text
```bash
$ tatl add --help
...
FIELD SYNTAX:
  field=value    Set attribute (project=work, due=tomorrow)
  +tag           Add tag
  -tag           Remove tag
  field=none     Clear attribute
  field=         Clear attribute (empty)

FILTER SYNTAX:
  field=value    Equality (status=pending)
  field>value    Greater than (due>tomorrow)
  field<value    Less than
  field>=value   Greater than or equal
  field<=value   Less than or equal
  field!=value   Not equal (project!=work)

Run 'tatl fields' for list of all available fields.
```

**Solution C:** Add shell completions for field names, projects, tags

---

### 8. Add Explicit Confirmation Flag (Low Priority)

**Solution:** Add `--confirm` flag for explicitness
```bash
--yes             # Never prompt (auto-confirm all)
--confirm         # Always prompt (even for single target)
(default)         # Prompt for multi-target, skip for single target
```

---

## Operator Reference

### Assignment / Equality
```
=           Set field or test equality
            add: project=work      → set project to "work"
            list: project=work     → filter where project equals "work"

=val1,val2  IN operator (OR logic)
            list: status=pending,completed → status IN (pending, completed)

=none       Set to null / test for null
            modify: project=none   → clear project
            list: project=none     → tasks without project

=           Empty value (equivalent to =none)
            modify: project=       → clear project
```

### Comparison (Filters Only)
```
>           Greater than (dates, durations)
            list: "due>tomorrow"   → due after tomorrow

<           Less than (dates, durations)
            list: "due<now"        → overdue tasks

>=          Greater than or equal
            list: "allocation>=2h" → tasks with 2h+ allocation

<=          Less than or equal
            list: "due<=eod"       → due today or earlier
```

### Negation (Filters Only)
```
!=          Not equal
            list: project!=work    → all projects except work
            list: "due!=none"      → tasks with due date

<>          Not equal (SQL-style alternative to !=)
            list: "due<>none"      → tasks with due date
```

### Tags (Unchanged)
```
+tag        Add tag
-tag        Remove tag
```

---

## Implementation Plan

### Phase 1: Breaking Changes (v0.x → v1.0)

1. **Replace `:` with `=` for field tokens**
   - Support both syntaxes with deprecation warnings during transition
   - Update parser to recognize `=` as field token delimiter
   - Add comparison operator parsing (`>`, `<`, `>=`, `<=`, `!=`, `<>`)

2. **Remove field abbreviations** in filters

3. **Simplify respawn syntax** (with deprecation warnings)

4. **Standardize clear-field** - accept both `field=none` and `field=`

### Phase 2: Consistency Fixes (v1.0)

5. Fix `--on` flag syntax (Plan 38 Option A)

6. Normalize `externals` to use filter syntax

### Phase 3: Discoverability (v1.1)

7. Add `tatl fields` command

8. Enhance help text for operator syntax

9. Add shell completion support

### Phase 4: Polish (v1.2)

10. Add `--confirm` flag

11. Improve error messages with operator suggestions

---

## Migration Guide

### For Field Tokens (`:` → `=`)
```bash
# Before
tatl add "task" project:work due:tomorrow
tatl list project:work status:pending
tatl modify 10 project:none

# After
tatl add "task" project=work due=tomorrow
tatl list project=work status=pending
tatl modify 10 project=none
# Or
tatl modify 10 project=
```

### For Field Abbreviations
```bash
# Before
tatl list st:pending sc:tomorrow

# After
tatl list status=pending scheduled=tomorrow
```

### For Respawn Syntax
```bash
# Before
respawn:every:2d
respawn:weekdays:mon,wed,fri
respawn:monthdays:1,15
respawn:nth:2:tue

# After
respawn=2d
respawn=mon,wed,fri
respawn=1,15
respawn=2nd-tue
```

### New Capabilities (Comparison Operators)
```bash
# Overdue tasks
tatl list "due<now"

# Large tasks
tatl list "allocation>=2h"

# Due soon
tatl list "due<=eod"

# Everything except work
tatl list project!=work

# Tasks with due dates
tatl list "due!=none"

# Complex queries
tatl list project=work "due>now" "due<=+7d" status=pending
# → Work tasks, overdue or due within 7 days, still pending
```

---

## Appendix A: Comprehensive Syntax Rules

### A.1 Syntax System Hierarchy

```
Command Structure:
  tatl <command> [subcommand] [targets] [fields] [filters] [flags]

Examples:
  tatl add [fields] [flags]
  tatl list [filters] [flags]
  tatl modify [targets] [fields] [flags]
  tatl sessions list [filters] [flags]
  tatl projects add <name>
```

---

### A.2 Command Rules

#### CR-1: Command Resolution
- Exact matches take precedence over prefix matches
- `on` matches `on` not `onoff`
- Ambiguous prefixes show all matches and error

#### CR-2: Command Abbreviation
- Commands support prefix abbreviation: `mod` → `modify`, `fin` → `finish`
- Only works for main commands, not subcommands
- Rationale: Commands are a closed set, frequently typed

---

### A.3 Target Specification Rules

#### TR-1: Target Types (Mutually Exclusive)
- **Single ID**: `10`
- **ID range**: `1-5` (inclusive, numeric only)
- **ID list**: `1,3,5` (comma-separated, no spaces)
- **Filter expression**: `project=work +urgent` (any valid filter)

#### TR-2: Target Resolution
- Numeric patterns (`10`, `1-5`, `1,3,5`) always resolve to IDs
- Non-numeric patterns resolve to filters
- Cannot mix ID patterns with filter expressions

---

### A.4 Field Token Rules

#### FR-1: Field Token Format
- Syntax: `fieldname=value`
- Field names: lowercase, alphanumeric, no abbreviations
- Equals sign is required (distinguishes from description text)
- Field name must match exactly (no prefix matching)

#### FR-2: Built-in Fields
```
project=<name>           - Project assignment (supports dot notation)
due=<datetime>           - Due date/time
scheduled=<datetime>     - Scheduled date/time
wait=<datetime>          - Wait until date
allocation=<duration>    - Time allocation
respawn=<pattern>        - Respawn pattern
description=<text>       - Task description (usually implicit)
template=<name>          - Template to apply
```

#### FR-3: User-Defined Attributes (UDAs)
- Syntax: `uda.<key>=<value>`
- Key must be alphanumeric with underscore/hyphen
- Namespaced to prevent collision with built-in fields

#### FR-4: Clear Field Values
- Syntax: `fieldname=none` (explicit) OR `fieldname=` (empty)
- Both forms canonically clear the field
- Rationale: `=` makes empty value look intentional (like `unset VAR=`)

#### FR-5: Field Token Validation
- Unknown field names produce error with suggestions
- Exception: Time expressions (`09:00`) don't contain `=`, no ambiguity
- Exception: URLs (`https://example.com`) don't contain `=`, no ambiguity
- Read-only fields (`status`, `created`, `modified`, `id`) produce error with hint

#### FR-6: Field Token Ambiguity Mitigation
- Only tokens matching known field names are treated as field tokens
- `status=ready` parsed as field ONLY if `status` is a known field
- Unknown `word=value` patterns treated as description
- User education: Quote descriptions containing `=` for clarity

---

### A.5 Operator Rules

#### OP-1: Assignment Operator (`=`)
- **Context: Setting fields** (add, modify)
  - Assigns value to field
  - Example: `project=work` sets project to "work"

- **Context: Filtering** (list, show, etc.)
  - Tests equality
  - Example: `project=work` matches tasks where project equals "work"

#### OP-2: IN Operator (`=` with comma-separated values)
- Syntax: `field=val1,val2,val3`
- Tests if field equals ANY of the values (OR logic)
- Example: `status=pending,completed` matches pending OR completed
- Only valid in filter context, not setting

#### OP-3: Comparison Operators (`>`, `<`, `>=`, `<=`)
- **Only valid in filter context** (list, show, sessions, etc.)
- Apply to ordered types: dates, times, durations
- Must be quoted or escaped to prevent shell interpretation
  ```bash
  tatl list "due>tomorrow"      # Quoted
  tatl list due\>tomorrow       # Escaped
  ```
- Error on non-ordered types:
  ```bash
  tatl list "project>work"      # Error: Cannot compare projects
  ```

#### OP-4: Negation Operators (`!=`, `<>`)
- **Only valid in filter context**
- Both are equivalent (SQL uses `<>`, C-style uses `!=`)
- Example: `project!=work` excludes work project
- Special case: `field!=none` means "has value" (NOT NULL)

#### OP-5: Null Testing
- `field=none` - Field is null/empty
- `field!=none` or `field<>none` - Field has any value
- Equivalent to SQL `IS NULL` / `IS NOT NULL`

#### OP-6: Operator Precedence in Filters
1. Field comparisons (highest binding)
2. `not` operator
3. Implicit `and` (adjacent terms)
4. `or` operator (lowest binding)

Examples:
```bash
# due>tomorrow AND project=work (implicit AND)
tatl list "due>tomorrow" project=work

# NOT urgent OR important
tatl list not +urgent or +important
# Parsed as: (NOT urgent) OR important

# Work tasks OR home tasks, both must be pending
tatl list project=work or project=home status=pending
# Parsed as: (project=work OR project=home) AND status=pending
```

#### OP-7: Shell Escaping Requirements

**No escaping needed:**
- `=` operator: `project=work` ✓
- Comma in values: `status=pending,completed` ✓
- Most filter operations: `tatl list project=work status=pending` ✓

**Escaping required:**
- `>`, `<` operators (shell redirects): `"due>tomorrow"` or `due\>tomorrow`
- `>=`, `<=` operators: `"allocation>=2h"`
- `!=` operator if shell expands history: `"project!=work"` (bash with histexpand)

**Best practice:** Quote all comparison expressions
```bash
tatl list "due>tomorrow" "allocation>=2h" project=work
```

---

### A.6 Tag Rules

#### TGR-1: Tag Syntax
- Add tag: `+<tagname>`
- Remove tag: `-<tagname>`
- Tag names: alphanumeric, underscore, hyphen, dot: `[A-Za-z0-9_\-\.]`

#### TGR-2: Tag Validation
- Empty tags rejected: `+` → error
- Invalid characters rejected: `+urgent!` → error
- Tags are case-sensitive

#### TGR-3: Tag vs Field Operators
- Tags use `+`/`-` (multi-valued, add/remove semantics)
- Fields use `=` (single-valued, assignment semantics)
- Keeps syntax distinct for different data types

---

### A.7 Description Rules

#### DR-1: Implicit Description
- Any token not matching field, tag, or flag patterns becomes description
- Description fragments are joined with spaces
- Fields, tags, and description can appear in any order

#### DR-2: Explicit Description
- Use `description=<text>` for edge cases
- Or quote entire command line

#### DR-3: Ambiguity Handling
```bash
# Ambiguous: is "status=ready" a field or description?
tatl add Check if status=ready project=work

# Unambiguous: quote the description
tatl add "Check if status=ready" project=work

# Or use description= explicitly
tatl add description="Check if status=ready" project=work
```

---

### A.8 Date/Time Expression Rules

#### DT-1: Absolute Formats
```
2026-01-15              - Date only (midnight)
2026-01-15T14:30        - ISO 8601 datetime
2026-01-15 14:30        - Space-separated datetime
14:30                   - Time only (today or tomorrow via 24h window rule)
```

#### DT-2: Relative Expressions
```
+<n>d / +<n>w / +<n>m / +<n>y    - Forward offset
-<n>d / -<n>w / -<n>m / -<n>y    - Backward offset
<n>d / <n>w / <n>m / <n>y        - Forward offset (sign optional)
```

#### DT-3: Named Dates
```
today / tomorrow / yesterday
eod / eow / eom                   - End of day/week/month
now                               - Current timestamp
```

#### DT-4: Date Intervals
- Syntax: `<start>..<end>`
- Both sides can be any datetime expression
- Examples: `2024-01-01..2024-01-31`, `-7d..now`, `09:00..12:00`

#### DT-5: 24-Hour Window Rule (Time-Only)
- Time-only expressions resolve to nearest occurrence within:
  - 8 hours past
  - 16 hours future

#### DT-6: Special Filter Values
- `field=any` - Matches tasks where field has any value (deprecated, use `field!=none`)
- `field=none` - Matches tasks where field is null/empty

---

### A.9 Duration Rules

#### DUR-1: Duration Format
- Syntax: Largest to smallest units, each unit max once
- Units: `d` (days), `h` (hours), `m` (minutes), `s` (seconds)
- Examples: `1h`, `2h30m`, `1d2h`, `45s`

#### DUR-2: Duration Validation
- Units must be in descending order: `1h30m` valid, `30m1h` invalid
- Each unit appears at most once: `2h3h` invalid
- No spaces between units

#### DUR-3: Duration Comparisons
```bash
# Allocations
tatl list "allocation>=2h"        # 2 hours or more
tatl list "allocation<30m"        # Under 30 minutes
```

---

### A.10 Respawn Pattern Rules

#### RP-1: Simple Frequencies
```
respawn=daily               - Every day at same time
respawn=weekly              - Every week on same weekday
respawn=monthly             - Every month on same day
respawn=yearly              - Every year on same date
```

#### RP-2: Interval Frequencies
```
respawn=<n>d                - Every N days
respawn=<n>w                - Every N weeks
respawn=<n>m                - Every N months
respawn=<n>y                - Every N years
```

#### RP-3: Weekday Patterns
```
respawn=<day>,<day>,...     - Specific weekdays (mon,wed,fri)
```
- Days: `mon`, `tue`, `wed`, `thu`, `fri`, `sat`, `sun`
- Detection: Contains comma-separated day names

#### RP-4: Monthday Patterns
```
respawn=<n>,<n>,...         - Specific days of month (1,15,30)
```
- Numbers 1-31
- Detection: Contains comma-separated numbers

#### RP-5: Nth Weekday Patterns
```
respawn=<ordinal>-<day>     - Nth weekday of month (2nd-tue, last-fri)
```
- Ordinals: `1st`, `2nd`, `3rd`, `4th`, `5th`, `last`
- Detection: Contains hyphen between ordinal and day name

---

### A.11 Filter Expression Rules

#### FE-1: Filter Operators
- **(implicit AND)** - Adjacent terms are ANDed
- **or** - OR operator (lowest precedence)
- **not** - NOT operator (highest precedence)

#### FE-2: Operator Precedence
1. Field comparisons and `not` (highest)
2. Implicit `and` (between adjacent terms)
3. `or` (lowest)

#### FE-3: Field Filters
```
id=<n> or <n>                   - Match by ID
status=<value>[,<value>,...]    - Match status (comma = OR)
project=<name>                  - Match project (prefix match for nested)
due=<datetime>                  - Exact match
"due><datetime>"                - Greater than (after)
"due<=<datetime>"               - Less than or equal (before/at)
"allocation>=<duration>"        - Greater than or equal
project!=<name>                 - Not equal
"due!=none"                     - Has due date (not null)
```

#### FE-4: Tag Filters
- `+<tag>` - Tasks with tag
- `-<tag>` - Tasks without tag

#### FE-5: Derived Filters
- `waiting` - Tasks with future wait date (wait_ts > now)

---

### A.12 Flag Rules

#### FL-1: Action Flags (Trigger Side Effects)
```
--on[=<time>]           - Start timing (optionally at time)
--onoff <interval>      - Add historical session
--enqueue               - Add to queue
--finish                - Complete task
--close                 - Close/cancel task
--next                  - Start next task after finish
```

#### FL-2: Modifier Flags
```
--yes / -y              - Auto-confirm all prompts
--confirm               - Force prompt even for single target
--interactive           - One-by-one confirmation
--force                 - Allow conflicts/overrides
```

#### FL-3: Output Flags
```
--json                  - JSON output format
--relative              - Show relative timestamps
--full                  - Ignore terminal width limits
```

#### FL-4: Flag Value Syntax
- Boolean flags: `--flag` (no value)
- Optional value: `--flag` or `--flag=<value>`
- Required value: `--flag <value>` or `--flag=<value>`
- Space syntax preferred for trailing flags
- Equals syntax required for leading flags when ambiguous

---

### A.13 Command-Specific Syntax

#### CS-1: `add` Command
```
tatl add [description] [fields] [flags]
```
- Description is implicit (any non-field/tag text)
- Fields and tags in any order
- Action flags: `--on`, `--onoff`, `--enqueue`, `--finish`, `--close`

Examples:
```bash
tatl add "Fix bug" project=work due=tomorrow +urgent
tatl add project=work "Fix bug" +urgent due=tomorrow    # Order doesn't matter
tatl add --on "Start task" project=work                 # Start timing
tatl add "Historical" --onoff 09:00..12:00 project=work
tatl add respawn=daily due=09:00 "Daily standup"
```

#### CS-2: `list` Command
```
tatl list [filter_expression] [flags]
```

Examples:
```bash
tatl list project=work +urgent
tatl list "due>tomorrow" status=pending
tatl list "allocation>=2h" "due<=eod"
tatl list project!=work status=pending,completed
tatl list +urgent or +important
tatl list not +waiting
```

#### CS-3: `modify` Command
```
tatl modify <target> [fields] [flags]
```

Examples:
```bash
tatl modify 10 +urgent due=+2d
tatl modify project=work description="Updated task" --yes
tatl modify 1-5 project=work --yes
tatl modify 5 respawn=daily due=09:00
tatl modify 10 project= allocation=      # Clear fields
```

#### CS-4: `on` / `off` Commands
```
tatl on [target] [time]
tatl off [time]
tatl offon <time>               - Break capture
tatl onoff <interval> [target]  - Historical session
```

Examples:
```bash
tatl on                         # Start queue[0]
tatl on 10                      # Start task 10
tatl on 14:00                   # Start from 14:00
tatl off                        # Stop now
tatl off 17:00                  # Stop at 17:00
tatl offon 14:30                # Break from 14:30 to now
tatl offon 14:30..15:00         # Break from 14:30 to 15:00
tatl onoff 09:00..12:00         # Add 3h session to queue[0]
```

#### CS-5: Session Commands
```
tatl sessions list [filter] [flags]
tatl sessions modify <id> <interval> [flags]
tatl sessions delete <id> [flags]
tatl sessions report [filter] [flags]
```

Examples:
```bash
tatl sessions list -7d
tatl sessions list project=work +urgent
tatl sessions modify 1 09:00..17:00 --yes
tatl sessions report 2024-01-01..2024-01-31 project=work
```

#### CS-6: Project Commands
```
tatl projects add <name>
tatl projects list [flags]
tatl projects rename <old> <new>
tatl projects archive <name>
tatl projects unarchive <name>
tatl projects report [flags]
```

---

### A.14 Parsing Rules

#### PR-1: Token Classification Order
1. Check for flag patterns (`--flag`, `-f`)
2. Check for tag patterns (`+tag`, `-tag`)
3. Check for field patterns (`field=value`, `"field>value"`)
4. Check for target patterns (numeric IDs, ranges, lists)
5. Remaining tokens are description

#### PR-2: Field Token Recognition
- Pattern: `<word>=<value>` or `<word><op><value>`
- Operators: `=`, `>`, `<`, `>=`, `<=`, `!=`, `<>` (quoted for shell)
- Only matches if `<word>` is a known field name (strict validation)
- Unknown `word=value` treated as description

#### PR-3: Whitespace Handling
- Tokens are space-separated
- No spaces allowed within field tokens (`project=work` not `project= work`)
- No spaces within operators (`due>=tomorrow` not `due> =tomorrow`)
- Comparison expressions should be quoted as single token: `"due>tomorrow"`

#### PR-4: Quoting
- Shell quoting applies (handled by shell before tatl)
- Use quotes for descriptions with `=`: `"Check if x=y"`
- Use quotes for comparison operators: `"due>tomorrow"`
- Use quotes for descriptions with spaces: `"fix urgent bug"`

---

### A.15 Validation Rules

#### VR-1: Unknown Fields
- Produce error with "Did you mean?" suggestion
- Use fuzzy matching against known fields
- Example: `projct=work` → suggests `project=work`

#### VR-2: Read-Only Fields
- Fields like `status`, `created`, `modified`, `id` cannot be modified
- Error includes hint: "Use 'finish' command to mark as completed"

#### VR-3: Invalid Values
- Empty field values: Both `project=none` and `project=` clear the field
- Invalid date formats produce parse error with examples
- Invalid duration formats show expected pattern

#### VR-4: Invalid Operators
- Comparison operators on non-ordered fields:
  ```bash
  tatl list "project>work"
  # Error: Cannot use > with project field (not ordered)
  # Valid operators for project: =, !=
  ```

#### VR-5: Operator Context Validation
- Setting context (add, modify): Only `=` allowed
  ```bash
  tatl modify 10 "due>tomorrow"
  # Error: Cannot use > when setting fields. Use due=tomorrow
  ```

---

### A.16 Consistency Rules

#### CN-1: One Canonical Form
- Clear field: Both `field=none` and `field=` accepted (equivalent)
- Respawn interval: `2d` (not `every:2d`)
- Weekday respawn: `mon,wed` (not `weekdays:mon,wed`)
- Negation: Both `!=` and `<>` accepted (equivalent)

#### CN-2: Semantic Operator Choice
- Field tokens use `=` (assignment/equality)
- Tags use `+`/`-` (add/remove)
- Comparisons use `>`, `<`, `>=`, `<=` (ordering)
- Negation uses `!=` or `<>` (inequality)
- These are distinct concerns with distinct operators

#### CN-3: Order Independence (Where Possible)
- Field tokens can appear in any order
- Tags can appear in any order
- Description fragments joined regardless of position
- Exception: Flags may have position constraints

#### CN-4: Explicit Over Implicit
- Comparison operators explicit: `due>tomorrow` not `due:after:tomorrow`
- Confirmation: `--yes` or `--confirm` (not silent)
- Status changes: Use commands (`finish`, `close`) not `status=completed`

---

## Appendix B: Design Rationale

### Why `=` Over `:`?

**Decision:** Use `project=work` not `project:work`

**Rationale:**
1. **Universal convention** - `key=value` is standard in SQL, Docker, K8s, Git, shell, config files
2. **Natural operator extension** - `=` extends to `>`, `<`, `>=`, `<=`, `!=` naturally; `:` does not
3. **No special cases** - `09:00` and URLs don't need exceptions (no `=` in them)
4. **Clearer semantics** - `=` explicitly means "equals" or "assign"; `:` is just a separator
5. **Intuitive clearing** - `project=` looks intentional; `project:` looks incomplete

**Trade-off:**
- Shell escaping for `<`, `>` (~5% of operations)
- Slight ambiguity if description contains `word=value` (mitigated by field validation)

**Verdict:** The benefits (convention, comparisons, clarity) far outweigh minimal costs

---

### Why Keep `+`/`-` for Tags?

**Decision:** Keep `+tag` / `-tag`, don't use `tag=value`

**Rationale:**
1. **Multi-valued semantics** - Tags are a set, can have many simultaneously
2. **Add/remove clarity** - `+` explicitly means "add to set", `-` means "remove from set"
3. **Visual distinction** - Tags look different from fields, reflecting different data types
4. **Ambiguity avoidance** - `tag=urgent` unclear: replace all tags? add one tag?

**Alternative considered:** `tag+=urgent`, `tag-=waiting` (too verbose)

---

### Why Comparison Operators?

**Decision:** Enable `due>tomorrow`, `allocation>=2h` in filters

**Rationale:**
1. **No alternative syntax** - Current system has no way to express "after tomorrow"
2. **SQL familiarity** - Developers already know `WHERE due > '2026-01-15'`
3. **Composability** - Combine with other filters: `"due>now" "due<=+7d" project=work`
4. **No new invention** - Uses standard comparison operators, not custom syntax

**Alternative considered:** Named filters like `due:after:tomorrow` (verbose, unclear for complex comparisons)

---

### Why Allow Both `=none` and `=` (Empty)?

**Decision:** Accept both `project=none` and `project=` for clearing

**Rationale:**
1. **User expectations** - Some users prefer explicit (`=none`), others prefer terse (`=`)
2. **Looks intentional** - With `=`, empty value looks like "set to nothing" (familiar from shell)
3. **No ambiguity** - Both forms unambiguous with `=` operator

**Comparison:** With `:`, the `project:` looks incomplete/broken. With `=`, the `project=` looks intentional.

---

## Appendix C: Comparison with TaskWarrior

| Feature | TaskWarrior | Tatl (Plan 39b) |
|---------|-------------|-----------------|
| **Field separator** | `:` | `=` |
| **Comparisons** | `due.after:tomorrow` | `"due>tomorrow"` |
| **Tag add** | `+tag` | `+tag` ✓ Same |
| **Tag remove** | `-tag` | `-tag` ✓ Same |
| **Boolean ops** | `and`, `or`, `not` | `and` (implicit), `or`, `not` ✓ Same |
| **Clear field** | `project:` | `project=` or `project=none` |
| **IN operator** | `status:pending,completed` | `status=pending,completed` |
| **Negation** | `status.not:completed` | `status!=completed` |
| **Convention** | TaskWarrior-specific | SQL/universal ✓ Better |

---

## Appendix D: Future Considerations

### Shell Completion

Implement completion for:
- Field names: Type `pro` + TAB → suggests `project=`
- Operators after field: Type `due` + TAB → suggests `due=`, `due>`, `due<`, etc.
- Project names after `project=`: Type `project=w` + TAB → suggests workspace, work
- Date expressions: Type `due=` + TAB → suggests `tomorrow`, `eod`, `+2d`

### Advanced Query Syntax

Potential future additions following established patterns:
- **BETWEEN**: `"due>=tomorrow" "due<=+7d"` (already expressible)
- **LIKE/pattern matching**: `project=work.*` or `project~work` (glob/regex)
- **NULL-safe comparisons**: Already supported via `field=none` / `field!=none`

### Multiple Respawn Patterns
```bash
respawn=mon,wed,fri|1st-mon    # Weekdays OR 1st Monday
```

All extensions must follow rule: **Use standard operators where possible**

---

## Decision

**Approve Phase 1-4 implementation** with `=` operator as the foundational syntax change.

This is a breaking change gated behind v1.0 release with:
- Deprecation warnings in v0.x supporting both `:` and `=`
- Clear migration guide
- `tatl migrate-syntax` command to update config files/aliases

The switch to `=` and comparison operators makes tatl dramatically more conventional while unlocking powerful filtering capabilities. This is the right foundation for long-term growth.

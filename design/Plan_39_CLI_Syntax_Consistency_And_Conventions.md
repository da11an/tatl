# Plan 39: CLI Syntax Consistency and Conventions

## Problem Statement

Tatl has a hybrid syntax that mixes conventional CLI patterns (subcommands, flags) with TaskWarrior-style field tokens (`project:work`, `+tag`). While the field tokens are efficient and expressive, there are **consistency issues** that create friction and violate the "one right way" principle:

1. **Flag inconsistencies**: `--on` requires equals syntax while `--onoff` accepts space syntax
2. **Field abbreviation brittleness**: `st:` works but `s:` fails with ambiguity errors
3. **Respawn syntax redundancy**: Mix of standalone values (`daily`) and verbose structured syntax (`every:2d`, `weekdays:mon`)
4. **Clear-field syntax ambiguity**: Both `field:none` and `field:` (empty) work
5. **Discoverability gaps**: Field token syntax is hidden from standard help output

This plan proposes targeted fixes to achieve consistency while preserving tatl's efficient, low-ceremony design.

---

## Guiding Principles

### What We're Keeping (Strategic Departures from Pure CLI Convention)

1. **Field tokens over flags** - `project:work` not `--project work` (brevity for daily use)
2. **Tag sigils** - `+tag` / `-tag` not `--tag` / `--untag` (visual clarity, conciseness)
3. **Implicit description** - `add "task" +urgent` not `add --description "task"` (reduce ceremony)
4. **Rich time expressions** - `due:tomorrow`, `scheduled:+2d`, `wait:eod` (natural language)

### One Right Way

For each operation, there should be **one canonical syntax** (not multiple equivalent forms). When alternatives exist, deprecate all but one.

---

## Proposed Changes

### 1. Fix `--on` Flag Consistency (High Priority)

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
- Make `--onoff` behavior consistent with `--on`

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

### 2. Remove Field Name Abbreviations in Filters (High Priority)

**Problem:**
```bash
tatl list st:pending    # Works (unambiguous)
tatl list sc:tomorrow   # Works (unambiguous)
tatl list s:pending     # Error: ambiguous (status/scheduled)
tatl list p:work        # Works now, but breaks if "priority:" field added
```

Abbreviations create **invisible landmines** - adding new fields can break existing queries.

**Solution:** Remove field abbreviation support

- Commands can still abbreviate: `mod` → `modify`, `fin` → `finish` (closed set, rarely changes)
- Field names must be spelled out: `status:`, `scheduled:`, `project:` (open set, UDAs extend it)
- Better error messages suggest full field name if abbreviation attempted

**Outcome:**
```bash
# Only full names work
tatl list status:pending
tatl list scheduled:tomorrow
tatl list project:work

# Abbreviations produce helpful error
tatl list st:pending
# Error: Unknown field 'st'. Did you mean 'status'?
```

**Rationale:** Commands are a **closed set** typed frequently (abbreviations have high ROI). Fields are **open-ended** due to UDAs (abbreviations create conflicts).

---

### 3. Simplify Respawn Syntax (High Priority)

**Problem:** Mix of concise and verbose patterns
```bash
respawn:daily              # Concise
respawn:every:2d           # "every:" is redundant
respawn:weekdays:mon,wed   # "weekdays:" is redundant
respawn:monthdays:1,15     # "monthdays:" is redundant
respawn:nth:2:tue          # "nth:" is redundant
```

**Solution:** Remove redundant keywords, use pattern recognition

```bash
# Simple frequencies (unchanged)
respawn:daily
respawn:weekly
respawn:monthly
respawn:yearly

# Interval frequencies (remove "every:")
respawn:2d              # Every 2 days
respawn:3w              # Every 3 weeks
respawn:2m              # Every 2 months

# Specific weekdays (remove "weekdays:")
respawn:mon,wed,fri     # Comma-separated days
respawn:monday,friday   # Full names supported

# Specific days of month (remove "monthdays:")
respawn:1,15            # 1st and 15th (numeric pattern detection)
respawn:1               # First of month

# Nth weekday of month (remove "nth:", use hyphen)
respawn:2nd-tue         # 2nd Tuesday
respawn:1st-mon         # 1st Monday
respawn:last-fri        # Last Friday
```

**Pattern Recognition Rules:**
- Contains duration unit (`2d`, `3w`) → interval frequency
- Contains comma-separated day names → weekdays
- Contains comma-separated numbers → monthdays
- Contains hyphen with ordinal → nth weekday (`2nd-tue`)

**Breaking Change:** Yes, but syntax is more intuitive and consistent

**Migration:** Show warning for old syntax, suggest new form:
```
Warning: 'respawn:every:2d' is deprecated, use 'respawn:2d'
```

---

### 4. Standardize Clear-Field Syntax (Medium Priority)

**Problem:**
```bash
project:none     # Clears project
project:         # Also clears project (empty value)
due:none         # Clears due
due:             # Also clears due
```

**Solution:** Canonicalize on `field:none` only

- Remove support for empty `field:` (ambiguous - typo vs. intentional?)
- `field:none` is explicit and clear
- Consistent with filter syntax (`project:none` means "no project")

**Outcome:**
```bash
# Only way to clear
tatl modify 10 project:none
tatl modify 10 due:none
tatl modify 10 allocation:none

# Empty value produces error
tatl modify 10 project:
# Error: Empty value for 'project:'. Use 'project:none' to clear.
```

---

### 5. Normalize `externals` Command (Medium Priority)

**Problem:** Inconsistent with other subcommand patterns
```bash
tatl projects list       # Subcommand pattern
tatl sessions list       # Subcommand pattern
tatl externals           # No subcommand?
tatl externals colleague # Filter as positional arg
```

**Solution:** Make `externals` a subcommand or integrate into `list`

**Option A:** Make it a subcommand
```bash
tatl externals list
tatl externals list colleague
tatl externals list --json
```

**Option B:** Make it a filter on `list` (Recommended)
```bash
tatl list external:any              # All external tasks
tatl list external:colleague        # Sent to colleague
tatl list external:any status:pending
```

**Recommendation:** Option B - fewer commands to remember, integrates with existing filter system

---

### 6. Improve Discoverability (Medium Priority)

**Problem:** Field token syntax is powerful but hidden - not discoverable via `--help`

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
  description  - Task description

User-Defined Attributes (UDAs):
  uda.priority - Custom priority field
  uda.estimate - Time estimate

Special Syntax:
  +tag         - Add tag
  -tag         - Remove tag
```

**Solution B:** Enhance command help text
```bash
$ tatl add --help
...
FIELD SYNTAX:
  field:value    Set attribute (project:work, due:tomorrow)
  +tag           Add tag
  -tag           Remove tag
  field:none     Clear attribute

Run 'tatl fields' for list of all available fields.
```

**Solution C:** Add shell completions
Generate completions for bash/zsh/fish that suggest:
- Field names after typing `:`
- Project names after `project:`
- Common date expressions after `due:` / `scheduled:`

---

### 7. Add Explicit Confirmation Flag (Low Priority)

**Problem:** Implicit behavior for multi-target operations can be surprising

**Current:**
```bash
tatl modify 10 +urgent              # No prompt (single target)
tatl modify project:work +urgent    # Prompts (multi-target)
tatl modify project:work +urgent -y # Skip prompt
```

**Solution:** Add `--confirm` flag for explicitness
```bash
--yes             # Never prompt (auto-confirm all)
--confirm         # Always prompt (even for single target)
(default)         # Prompt for multi-target, skip for single target
```

**Outcome:**
```bash
tatl modify 10 +urgent --confirm    # Force confirmation even for single target
tatl finish project:work --yes      # Skip all confirmations
tatl delete 1-5                     # Default: prompts because multi-target
```

---

## Implementation Plan

### Phase 1: Breaking Changes (v0.x → v1.0)
1. Remove field abbreviations in filters
2. Simplify respawn syntax (with deprecation warnings)
3. Standardize clear-field syntax (reject `field:`)

### Phase 2: Consistency Fixes (v1.0)
4. Fix `--on` flag syntax (Plan 38 Option A)
5. Normalize `externals` to use filter syntax

### Phase 3: Discoverability (v1.1)
6. Add `tatl fields` command
7. Enhance help text for field syntax
8. Add shell completion support

### Phase 4: Polish (v1.2)
9. Add `--confirm` flag
10. Improve error messages with suggestions

---

## Migration Guide

### For Field Abbreviations
```bash
# Before
tatl list st:pending sc:tomorrow

# After
tatl list status:pending scheduled:tomorrow
```

### For Respawn Syntax
```bash
# Before
respawn:every:2d
respawn:weekdays:mon,wed,fri
respawn:monthdays:1,15
respawn:nth:2:tue

# After
respawn:2d
respawn:mon,wed,fri
respawn:1,15
respawn:2nd-tue
```

### For Clear-Field Syntax
```bash
# Before
project:          # Empty value
due:              # Empty value

# After
project:none      # Explicit
due:none          # Explicit
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
- **Filter expression**: `project:work +urgent` (any valid filter)

#### TR-2: Target Resolution
- Numeric patterns (`10`, `1-5`, `1,3,5`) always resolve to IDs
- Non-numeric patterns resolve to filters
- Cannot mix ID patterns with filter expressions

---

### A.4 Field Token Rules

#### FR-1: Field Token Format
- Syntax: `fieldname:value`
- Field names: lowercase, alphanumeric, no abbreviations
- Colon is required (distinguishes from description text)
- Field name must match exactly (no prefix matching)

#### FR-2: Built-in Fields
```
project:<name>           - Project assignment (supports dot notation)
due:<datetime>           - Due date/time
scheduled:<datetime>     - Scheduled date/time
wait:<datetime>          - Wait until date
allocation:<duration>    - Time allocation
respawn:<pattern>        - Respawn pattern
description:<text>       - Task description (usually implicit)
template:<name>          - Template to apply
```

#### FR-3: User-Defined Attributes (UDAs)
- Syntax: `uda.<key>:<value>`
- Key must be alphanumeric with underscore/hyphen
- Namespaced to prevent collision with built-in fields

#### FR-4: Clear Field Values
- Syntax: `fieldname:none`
- Only canonical form for clearing a field
- Empty values (`field:`) are rejected with error
- Rationale: Explicit is better than implicit; prevents accidental clears from typos

#### FR-5: Field Token Validation
- Unknown field names produce error with suggestions
- Exception: Time expressions (`09:00`) and URLs are not treated as field tokens
- Read-only fields (`status`, `created`, `modified`, `id`) produce error with hint to use appropriate command

---

### A.5 Tag Rules

#### TGR-1: Tag Syntax
- Add tag: `+<tagname>`
- Remove tag: `-<tagname>`
- Tag names: alphanumeric, underscore, hyphen, dot: `[A-Za-z0-9_\-\.]`

#### TGR-2: Tag Validation
- Empty tags rejected: `+` → error
- Invalid characters rejected: `+urgent!` → error
- Tags are case-sensitive

---

### A.6 Description Rules

#### DR-1: Implicit Description
- Any token not matching field, tag, or flag patterns becomes description
- Description fragments are joined with spaces
- Fields, tags, and description can appear in any order

#### DR-2: Explicit Description
- Use `description:<text>` for edge cases containing colons
- Or quote entire command line to treat as literal

---

### A.7 Date/Time Expression Rules

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
in <n> days / weeks / months     - Natural language forward
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
- Example: At 10:00 AM, `09:00` resolves to yesterday 9 AM (within 8h past)

#### DT-6: Special Filter Values
- `field:any` - Matches tasks where field has any value
- `field:none` - Matches tasks where field is null/empty

---

### A.8 Duration Rules

#### DUR-1: Duration Format
- Syntax: Largest to smallest units, each unit max once
- Units: `d` (days), `h` (hours), `m` (minutes), `s` (seconds)
- Examples: `1h`, `2h30m`, `1d2h`, `45s`

#### DUR-2: Duration Validation
- Units must be in descending order: `1h30m` valid, `30m1h` invalid
- Each unit appears at most once: `2h3h` invalid
- No spaces between units

---

### A.9 Respawn Pattern Rules

#### RP-1: Simple Frequencies
```
daily               - Every day at same time
weekly              - Every week on same weekday
monthly             - Every month on same day
yearly              - Every year on same date
```

#### RP-2: Interval Frequencies
```
<n>d                - Every N days
<n>w                - Every N weeks
<n>m                - Every N months
<n>y                - Every N years
```

#### RP-3: Weekday Patterns
```
<day>,<day>,...     - Specific weekdays (mon,wed,fri)
```
- Days: `mon`, `tue`, `wed`, `thu`, `fri`, `sat`, `sun` (full names also supported)
- Detection: Contains comma-separated day names

#### RP-4: Monthday Patterns
```
<n>,<n>,...         - Specific days of month (1,15,30)
```
- Numbers 1-31
- Detection: Contains comma-separated numbers

#### RP-5: Nth Weekday Patterns
```
<ordinal>-<day>     - Nth weekday of month (2nd-tue, last-fri)
```
- Ordinals: `1st`, `2nd`, `3rd`, `4th`, `5th`, `last`
- Detection: Contains hyphen between ordinal and day name

---

### A.10 Filter Expression Rules

#### FE-1: Filter Operators
- **(implicit AND)** - Adjacent terms are ANDed
- **or** - OR operator (lowest precedence)
- **not** - NOT operator (highest precedence)

#### FE-2: Operator Precedence
1. `not` (highest)
2. Implicit `and` (between adjacent terms)
3. `or` (lowest)

#### FE-3: Field Filters
```
id:<n> or <n>                   - Match by ID
status:<value>[,<value>,...]    - Match status (OR for multiple)
project:<name>                  - Match project (prefix match for nested)
due:<datetime>                  - Match due date
scheduled:<datetime>            - Match scheduled date
wait:<datetime>                 - Match wait date
kanban:<status>[,<status>,...]  - Match kanban status
desc:<pattern>                  - Description substring search
external:<recipient>            - Match external recipient
```

#### FE-4: Tag Filters
- `+<tag>` - Tasks with tag
- `-<tag>` - Tasks without tag

#### FE-5: Derived Filters
- `waiting` - Tasks with future wait date (wait_ts > now)

---

### A.11 Flag Rules

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
- Space syntax preferred for trailing flags (more conventional)
- Equals syntax required for leading flags when ambiguous

---

### A.12 Command-Specific Syntax

#### CS-1: `add` Command
```
tatl add [description] [fields] [flags]
```
- Description is implicit (any non-field/tag text)
- Fields and tags in any order
- Action flags: `--on`, `--onoff`, `--enqueue`, `--finish`, `--close`

#### CS-2: `list` Command
```
tatl list [filter_expression] [flags]
```
- Filter expression is free-form (fields, tags, operators)
- No explicit target required

#### CS-3: `modify` Command
```
tatl modify <target> [fields] [flags]
```
- Target is required (ID, range, list, or filter)
- Fields specify changes to apply
- Prompts for confirmation on multi-target

#### CS-4: `on` / `off` Commands
```
tatl on [target] [time]
tatl off [time]
tatl offon <time>               - Break capture, start = time, end = now
tatl offon <interval>           - Break capture, explicit interval
tatl onoff <interval> [target]  - Historical session
```
- Target optional (defaults to queue[0])
- Time expressions parsed as datetime

#### CS-5: Session Commands
```
tatl sessions list [filter] [flags]
tatl sessions modify <id> <interval> [flags]
tatl sessions delete <id> [flags]
tatl sessions report [filter] [flags]
```
- Interval syntax: `<start>..<end>`, `<start>..`, `..<end>`

#### CS-6: Project Commands
```
tatl projects add <name>
tatl projects list [flags]
tatl projects rename <old> <new>
tatl projects archive <name>
tatl projects unarchive <name>
tatl projects report [flags]
```
- Project names support dot notation for nesting

---

### A.13 Parsing Rules

#### PR-1: Token Classification Order
1. Check for flag patterns (`--flag`, `-f`)
2. Check for tag patterns (`+tag`, `-tag`)
3. Check for field patterns (`field:value`)
4. Check for target patterns (numeric IDs, ranges, lists)
5. Remaining tokens are description

#### PR-2: Field Token Exceptions
- Time expressions (`09:00`, `14:30`) are NOT field tokens
- URLs (`https://example.com`) are NOT field tokens
- These patterns explicitly excluded from field validation

#### PR-3: Whitespace Handling
- Tokens are space-separated
- No spaces allowed within field tokens (`project:work` not `project: work`)
- No spaces within duration values (`2h30m` not `2h 30m`)
- No spaces within tag names (`+code-review` not `+code review`)

#### PR-4: Quoting
- Shell quoting applies (handled by shell before tatl sees args)
- Use quotes to include spaces in description: `"fix urgent bug"`
- Use quotes to escape special characters: `"description: with colon"`

---

### A.14 Validation Rules

#### VR-1: Unknown Fields
- Produce error with "Did you mean?" suggestion
- Use fuzzy matching against known fields
- Helps catch typos: `projct:` → suggests `project:`

#### VR-2: Read-Only Fields
- Fields like `status`, `created`, `modified`, `id` cannot be modified
- Error message includes hint: "Use 'finish' command to mark as completed"

#### VR-3: Invalid Values
- Empty field values rejected: `project:` → error (use `project:none`)
- Invalid date formats produce parse error with examples
- Invalid duration formats show expected pattern

#### VR-4: Ambiguous Abbreviations
- When multiple fields match prefix, error lists all matches
- Example: `s:pending` → "Ambiguous field 's', could be: status, scheduled"

---

### A.15 Consistency Rules

#### CN-1: One Canonical Form
- For each operation, only one syntax is correct
- Clear field: `field:none` (not `field:`)
- Respawn interval: `2d` (not `every:2d`)
- Weekday respawn: `mon,wed` (not `weekdays:mon,wed`)

#### CN-2: No Semantic Overlap
- Field tokens set task attributes (data)
- Action flags trigger side effects (behavior)
- These are distinct concerns with distinct syntax

#### CN-3: Order Independence (Where Possible)
- Field tokens can appear in any order
- Tags can appear in any order
- Description fragments joined regardless of position
- Exception: Flags may have position constraints (leading vs trailing)

#### CN-4: Explicit Over Implicit
- Clearing fields: `field:none` (not `field:`)
- Confirmation: `--yes` or `--confirm` (not silent default)
- Status changes: Use explicit commands (`finish`, `close`) not `status:completed`

---

## Appendix B: Design Rationale

### Why Field Tokens Over Flags?

**Decision:** Use `project:work` not `--project work`

**Rationale:**
1. **Brevity** - Typed dozens of times per day, every character matters
2. **UDA support** - Arbitrary user fields (`uda.priority:high`) awkward as flags
3. **Visual scanning** - Easier to scan `+urgent due:tomorrow` than `--tag urgent --due tomorrow`
4. **Precedent** - TaskWarrior established pattern, tatl users expect it

**Trade-off:** Lower discoverability (addressed via `tatl fields` command)

---

### Why Implicit Description?

**Decision:** `tatl add "Fix bug" +urgent` not `tatl add --description "Fix bug" --tag urgent`

**Rationale:**
1. **Description always required** - Making it positional is natural
2. **Reduced ceremony** - Focus on the task, not the tool
3. **Precedent** - `git commit -m "message"` uses short flag but same concept

**Trade-off:** Edge cases with colons require explicit `description:` field

---

### Why Remove Field Abbreviations?

**Decision:** Require full field names in filters

**Rationale:**
1. **Open set** - UDAs can add arbitrary fields, abbreviations create conflicts
2. **Hidden breakage** - Adding `priority:` field breaks existing `p:work` (was `project:`)
3. **Maintenance burden** - Collision detection and error messages for ambiguous cases
4. **Marginal benefit** - Fields typed less frequently than commands

**Trade-off:** Slightly more typing (addressed via shell completion)

---

### Why Simplify Respawn Syntax?

**Decision:** `respawn:2d` not `respawn:every:2d`

**Rationale:**
1. **Redundancy** - `respawn:` already implies repetition, `every:` is noise
2. **Pattern recognition** - Parsers can detect `2d` vs `mon,wed` vs `1,15` vs `2nd-tue`
3. **Consistency** - All patterns use same delimiter style (no mix of `:` and `,`)
4. **Precedent** - Duration syntax already uses `2d` for "2 days"

**Trade-off:** Breaking change (mitigated by deprecation warnings)

---

## Appendix C: Future Considerations

### Shell Completion

Implement completion for:
- Field names after typing `:` → suggest `project:`, `due:`, etc.
- Project names after `project:` → suggest from database
- Tag names after `+` → suggest from database
- Date expressions after `due:` → suggest `tomorrow`, `+2d`, etc.

### Syntax Extensions

Potential future additions that follow established patterns:
- **Date ranges in respawn**: `respawn:2026-01-01..2026-12-31:weekly` (only during date range)
- **Multiple respawn patterns**: `respawn:mon,wed,fri|1st-mon` (weekdays OR 1st Monday)
- **Conditional respawn**: `respawn:daily:if:+urgent` (only if condition met)

All extensions must follow rule: **One syntax pattern per semantic concept**

---

## Decision

**Approve Phase 1-4 implementation** with prioritization as outlined.

Breaking changes (field abbreviations, respawn syntax, clear-field) gated behind v1.0 release with clear migration guide and deprecation warnings in v0.x series.

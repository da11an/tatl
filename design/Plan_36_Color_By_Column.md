# Plan 36: Color By Column

## Overview

Add the ability to colorize task list output based on column values, similar to how ggplot in R allows `aes(color = column)`. This would make visual patterns in data immediately apparent - e.g., seeing all "work" tasks in blue, all "home" tasks in green, or priority gradients from green (low) to red (high).

## Inspiration

**R ggplot2:**
```r
ggplot(data, aes(x = due, y = priority, color = project)) + geom_point()
```

**Proposed tatl syntax:**
```bash
tatl list color:project              # Color rows by project
tatl list color:priority             # Color by priority (gradient)
tatl list fill:kanban                # Background fill by kanban stage
tatl list color:project fill:status  # Multiple color mappings
```

---

## Design Options

### Option 1: Row-Based Coloring

Color the entire row based on a column's value.

**Syntax:**
```bash
tatl list color:project        # Font color by project
tatl list fill:project         # Background color by project
```

**Implementation:**
- Map unique column values to a color palette
- Apply ANSI color codes to entire row
- Works well for categorical columns (project, status, kanban)

**Pros:**
- Simple mental model
- Easy to implement
- Strong visual grouping

**Cons:**
- May reduce readability if too many colors
- Doesn't work well with numeric columns

---

### Option 2: Column-Specific Coloring

Color only a specific column's cell based on its value.

**Syntax:**
```bash
tatl list color:priority@priority    # Color priority column by its value
tatl list color:project@project      # Color project column by project
tatl list color:kanban@q             # Color Q column by kanban stage
```

**Implementation:**
- Apply color only to the specified column's cells
- Other columns remain default color

**Pros:**
- Less visual noise
- Targeted highlighting
- Works well alongside other formatting

**Cons:**
- More complex syntax
- May be less impactful visually

---

### Option 3: Indicator Column Coloring (Q Column Enhancement)

Enhance the Q column with color based on kanban stage or other values.

**Current Q Column:**
```
Q
────
▶     (active - running session)
1     (queued position 1)
2     (queued position 2)
?     (proposed)
!     (stalled)
@     (external)
✓     (completed)
x     (closed)
```

**With Color:**
```
Q     (colored)
────
▶     (green - active)
1     (blue - queued)
2     (blue - queued)
?     (gray - proposed)
!     (yellow - stalled)
@     (magenta - external)
✓     (green - completed)
x     (red - closed)
```

**Pros:**
- Non-intrusive
- High information density
- Works well with existing indicators

**Cons:**
- Limited to Q column
- Single dimension of color

---

### Option 4: Separator/Gap Coloring

Color the gaps between columns based on a value.

**Example:**
```
Q │ ID │ Description        │ Project │ Status
──│────│────────────────────│─────────│────────
1 │ 10 │ Fix auth bug       │ work    │ pending     ← blue gap (work)
2 │ 11 │ Review PR          │ work    │ pending     ← blue gap (work)
3 │ 12 │ Grocery shopping   │ home    │ pending     ← green gap (home)
```

**Implementation:**
- Replace column separator with colored character
- Or use colored vertical bars

**Pros:**
- Subtle but effective
- Doesn't interfere with text readability
- Works well for grouping visualization

**Cons:**
- Less prominent than full row coloring
- May not work in all terminals

---

### Option 5: Gradient Coloring for Numeric Columns

Apply color gradients based on numeric values.

**Examples:**
- Priority: red (high) → yellow → green (low)
- Due date: red (overdue) → yellow (soon) → green (far away)
- Alloc: red (high) → blue (low)
- Clock: gradient by time spent

**Syntax:**
```bash
tatl list color:priority:gradient    # Gradient by priority
tatl list color:due:heat             # Heat map by due date
```

**Pros:**
- Excellent for numeric data
- Immediately shows patterns
- Natural mental model

**Cons:**
- Requires gradient palette implementation
- May not work for all numeric columns

---

## Color Palette Strategies

### Strategy A: Fixed Palette

Assign colors from a fixed palette based on value order.

```
Palette: blue, green, yellow, orange, red, magenta, cyan, white
```

Values are assigned colors in order of first appearance or alphabetically.

### Strategy B: Semantic Palette

Use semantically meaningful colors.

| Value Type | Color |
|------------|-------|
| work | blue |
| home | green |
| urgent | red |
| pending | white/default |
| completed | green |
| closed | gray |
| overdue | red |
| stalled | yellow |
| external | magenta |

### Strategy C: User-Defined Palette

Allow users to define color mappings in config.

```ini
# ~/.tatl/rc
color.project.work = blue
color.project.home = green
color.status.completed = green
color.status.closed = gray
color.priority.high = red
color.priority.low = green
```

### Strategy D: Hash-Based Palette

Generate consistent colors from value hashes.

```rust
fn color_from_value(value: &str) -> Color {
    let hash = hash(value);
    PALETTE[hash % PALETTE.len()]
}
```

Ensures same value always gets same color across sessions.

---

## Syntax Proposals

### Minimal Syntax
```bash
tatl list color:project
tatl list fill:status
```

### Explicit Target Syntax
```bash
tatl list color:project@row          # Color whole row
tatl list color:project@project      # Color project column only
tatl list color:kanban@q             # Color Q column
```

### Multiple Mappings
```bash
tatl list color:project fill:status
tatl list color:priority@priority color:kanban@q
```

### Gradient Syntax
```bash
tatl list color:priority:gradient
tatl list color:due:heat
```

---

## Implementation Considerations

### ANSI Color Support

Standard 16-color ANSI:
```
Black, Red, Green, Yellow, Blue, Magenta, Cyan, White
+ bright variants
```

256-color support:
```
0-7: standard colors
8-15: bright colors
16-231: 6x6x6 color cube
232-255: grayscale
```

True color (24-bit):
```
\x1b[38;2;R;G;Bm  (foreground)
\x1b[48;2;R;G;Bm  (background)
```

### Terminal Compatibility

- Check `$TERM` and `$COLORTERM` for capability detection
- Fallback to 16-color or no color for limited terminals
- Honor `NO_COLOR` environment variable

### Performance

- Color assignment should be O(1) per row
- Pre-compute palette mappings before rendering
- Avoid repeated hash calculations

---

## Open Questions

### 1. Default Behavior

Should any coloring be on by default?

**Options:**
- [x] No default coloring (current behavior)
- [ ] Q column colored by kanban stage by default
- [ ] Configurable default in rc file

### 2. Color Scheme

Light vs dark terminal considerations:

**Options:**
- [ ] Single palette (optimized for dark terminals)
- [x] Detect terminal background and adjust
- [ ] User-configurable theme (light/dark)

### 3. Interaction with Grouping

When grouped by a column, should that column also be colored?

```bash
tatl list group:project color:project
```

**Options:**
- [ ] Color group headers only
- [ ] Color all rows
- [x] Color group separators

### 4. JSON Output

Should `--json` output include color information?

**Options:**
- [x] No - JSON is for data, not presentation
- [ ] Yes - include color hints as fields
- [ ] Optional flag: `--json --with-colors`

### 5. Persistence

Should color preferences be saved?

**Options:**
- [ ] Per-command only (no persistence)
- [ ] Save with view aliases
- [ ] Global config option

---

## Recommended Approach

### Phase 1: Q Column Coloring (Low Effort, High Impact)

Add automatic coloring to the Q column based on kanban stage.

```
▶  green (active)
1  blue (queued)
?  gray (proposed)
!  yellow (stalled)
@  magenta (external)
✓  green (done)
x  gray (closed)
```

**Implementation:** ~50 lines in `output.rs`

### Phase 2: Row Coloring by Column

Add `color:column` syntax for row-based coloring.

```bash
tatl list color:project
tatl list color:status
```

**Implementation:** ~100 lines + palette system

### Phase 3: Gradient Support

Add gradient coloring for numeric columns.

```bash
tatl list color:priority:gradient
tatl list color:due:heat
```

**Implementation:** ~150 lines + gradient calculation

### Phase 4: User Configuration

Allow custom color mappings in config.

```ini
color.project.work = blue
color.kanban.stalled = yellow
```

**Implementation:** ~200 lines + config parsing

---

## Example Outputs

### Before (Current)
```
Q    ID   Description              Project    Status
──── ──── ──────────────────────── ────────── ─────────
▶    10   Fix authentication bug   work       pending
1    11   Review PR                work       pending
2    12   Write documentation      docs       pending
?    13   Research new library     research   pending
!    14   Waiting for API          work       pending
```

### After (Phase 1 - Q Column Colored)
```
Q    ID   Description              Project    Status
──── ──── ──────────────────────── ────────── ─────────
▶    10   Fix authentication bug   work       pending     (▶ in green)
1    11   Review PR                work       pending     (1 in blue)
2    12   Write documentation      docs       pending     (2 in blue)
?    13   Research new library     research   pending     (? in gray)
!    14   Waiting for API          work       pending     (! in yellow)
```

### After (Phase 2 - Row Colored by Project)
```bash
tatl list color:project
```
```
Q    ID   Description              Project    Status
──── ──── ──────────────────────── ────────── ─────────
▶    10   Fix authentication bug   work       pending     (entire row in blue)
1    11   Review PR                work       pending     (entire row in blue)
2    12   Write documentation      docs       pending     (entire row in green)
?    13   Research new library     research   pending     (entire row in yellow)
!    14   Waiting for API          work       pending     (entire row in blue)
```

---

## Success Criteria

1. ✅ Q column shows semantic colors for kanban stages
2. ✅ `color:column` syntax colorizes rows by column value
3. ✅ Colors are consistent across sessions (same project = same color)
4. ✅ Respects `NO_COLOR` environment variable
5. ✅ Graceful fallback on terminals without color support
6. ✅ Documentation updated with color options
7. ✅ Tests verify color output (ANSI code presence)

---

## Related Work

- **taskwarrior:** Uses colors for urgency, due dates, projects
- **ls --color:** File type coloring
- **git diff:** Addition/deletion coloring
- **htop:** CPU/memory gradient coloring
- **ggplot2:** Aesthetic mapping (color, fill, size, shape)

---

## Appendix: ANSI Color Codes

### 16-Color Palette (Standard)
```
Foreground: 30-37 (normal), 90-97 (bright)
Background: 40-47 (normal), 100-107 (bright)

30/40  Black     90/100  Bright Black (Gray)
31/41  Red       91/101  Bright Red
32/42  Green     92/102  Bright Green
33/43  Yellow    93/103  Bright Yellow
34/44  Blue      94/104  Bright Blue
35/45  Magenta   95/105  Bright Magenta
36/46  Cyan      96/106  Bright Cyan
37/47  White     97/107  Bright White
```

### Example ANSI Sequences
```
\x1b[31m   Red foreground
\x1b[42m   Green background
\x1b[1;31m Bold red
\x1b[0m    Reset
```

# Decisions

**Omit Phase 1.** No automatic coloring by default.

**Phase 2 Implementation:**

1. **Support both categorical and numeric data coloring.**
   - Categorical columns (project, status, kanban, tags) → hash-based fixed palette
   - Numeric columns (priority, alloc, clock) → gradient
   - Date columns (due, scheduled) → heat map (red=overdue → green=far away)
   - Program auto-detects appropriate mapping per column

2. **Support both `color:` and `fill:` options.**
   - `color:<column>` → foreground/text color
   - `fill:<column>` → background color

3. **Color only group separators, not task rows.**
   - When using `group:project color:project`, only the group header/separator gets colored
   - Task rows remain uncolored for readability and minimalism

4. **Use program defaults for color schemes.**
   - No user-selectable color scheme at command line
   - Config file customization deferred to future work

5. **Simple syntax:**
   ```bash
   tatl list color:project           # Text color by project
   tatl list fill:kanban             # Background by kanban stage
   tatl list color:priority          # Gradient by priority
   tatl list group:project color:project  # Colored group separators
   ```

6. **No special persistence.**
   - Works with existing alias system: `tatl list color:project save:myview`
   - No additional persistence mechanism needed
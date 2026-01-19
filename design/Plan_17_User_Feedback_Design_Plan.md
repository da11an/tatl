# Plan 17 User Feedback Design Plan

## Goals
- Translate Plan 17 feedback into a concrete, testable design plan.
- Challenge dubious choices and identify implementation barriers.
- Suggest improvements where the feedback is ambiguous or risky.

## Non-Goals
- No implementation in this document.
- No behavior changes outside the listed feedback items.

## Assumptions
- CLI uses `clap` with subcommands for task/sessions/clock.
- Output formatting lives in `src/cli/output.rs`.
- Filters are parsed in `src/filter/*`.
- Kanban stages are computed in `src/cli/output.rs` via `calculate_kanban_status()`.
- Current kanban ordinal order: proposed(0), queued(1), paused(2), working(3), next(4), live(5), done(6), quit(7).

## Open Questions
- For item 2: Should "live id" be a special token like `live` or `@live`, or should it be a command option like `--live`? Add a @live and @next token to call out these single id categories.
- For item 3: Should color support be terminal-dependent (ANSI codes) or should we use a library like `colored` or `termcolor`? Yes -- whatever is going to not cause issues for people.
- For item 4: How should description search interact with existing filter tokens? Should it be a fallback when no tokens match? What does taskwarrior do? I'm just saying if it's not a recognized token, let it pass through to the next level, which will usually be description.
- For item 5: Should nested project display show only direct children, or full hierarchy? Should archived projects be included? Archived projects should be shown in parenthesis, a less distinct font or something like that. Annd we should show the full nesting heirarchy, like and outline.

## Design Summary
1. Reorder kanban ordinal values to prioritize active work: live(0), next(1), working(2), queued(3), paused(4), proposed(5), done(6), quit(7).
2. Add allocation view/list command and programmatic access to live task ID.
3. Add color column option for row colorization based on column values.
4. Add description search fallback for non-token text in task list command.
5. Enhance projects list to show nested structure with task counts by status.

---

## 1) Reorder Kanban: live, next, queued <-> paused, proposed

**Current Order:**
- proposed(0), queued(1), paused(2), working(3), next(4), live(5), done(6), quit(7)

**Proposed Order:**
- live(0), next(1), queued(2), working(3), paused(4), proposed(5), done(6), quit(7)

**Design**
- Update `kanban_sort_order()` function in `src/cli/output.rs` to reflect new ordinal values.
- This affects sorting when `sort:kanban` is used (ascending will now show live first).
- Grouping order will also change when `group:kanban` is used.

**Challenge**
- This is a breaking change for users who rely on current kanban sort order. However, it makes semantic sense: active work (live, next) should appear before inactive work (paused, proposed).

**Implementation Barrier**
- None. Simple function update.

**Tests**
- Verify kanban sorting shows live tasks first, then next, then queued, etc.
- Verify grouping respects new order.
- Verify negation (`sort:-kanban`) reverses correctly.

**Decision**
- Proceed with reordering. The new order better reflects workflow priority.

---

## 2) New view to show allocations + programmatic live task ID access

**Design**

### 2a) Allocation View/List
- Add new command: `task allocations [filter]` or extend `task list` with `alloc:` filter/view.
- Display tasks with their `alloc_secs` values in a dedicated view.
- Show: ID, Description, Project, Allocation (formatted duration), Status.
- Optionally show: Allocated vs. Logged time comparison.

**Challenge**
- Should this be a separate command or a filter/view option? Separate command is clearer.
- Should we show only tasks with allocations, or all tasks (with "none" for unallocated)?

**Implementation Barrier**
- Need to decide on command structure. Suggestion: `task allocations [filter]` as a new subcommand.

### 2b) Programmatic Live Task ID Access
- Add `task live` command that outputs only the task ID of the currently clocked-in task.
- Exit code 0 if live task exists, non-zero if no live task.
- Optionally support `--json` for structured output: `{"task_id": 42}` or `null`.

**Alternative Approaches:**
- `task show live` - but this might conflict with description search (item 4).
- `task clock live` - but this is more about clock state than task ID.
- Environment variable: `TASK_LIVE_ID` - but this requires shell integration.

**Challenge**
- Need to ensure "live" doesn't conflict with description search when used in `task list live`.

**Implementation Barrier**
- Need to parse "live" as a special token in command parsing, not as a filter/description.

**Tests**
- `task live` outputs task ID when clocked in.
- `task live` exits with error when not clocked in.
- `task live --json` outputs JSON format.
- `task allocations` shows tasks with allocations.
- `task allocations` handles filters correctly.

**Decision**
- Use `task live` for live task ID access (simple, clear).
- Use `task allocations [filter]` for allocation view (separate command for clarity).

---

## 3) Color column option for row colorization

**Design**
- Add `color:column` token to list command (e.g., `task list color:priority`).
- Colorize rows based on column values:
  - **Priority**: Red (high), Yellow (medium), Green (low), White (none).
  - **Due date**: Red (overdue), Yellow (due soon), Green (not due), White (no due date).
  - **Allocation**: Color based on allocation vs. logged time ratio.
  - **Status**: Color based on status (pending=white, completed=green, closed=gray).
  - **Kanban**: Color based on stage (live=cyan, next=yellow, etc.).

**Challenge**
- Terminal color support detection (should we auto-detect or require flag?).
- Color scheme definition (what colors for what values?).
- Performance: colorizing many rows might be slow.

**Implementation Barrier**
- Need to add color library dependency (e.g., `colored` or `termcolor`).
- Need to detect if output is a TTY (don't colorize when piping to file).
- Need to define color mapping for each column type.

**Tests**
- Verify colors appear correctly in TTY output.
- Verify no colors in non-TTY output (piping).
- Verify color mapping for each supported column.
- Verify `--no-color` flag disables colors.

**Decision**
- Use `colored` crate for simplicity.
- Auto-detect TTY, but allow `--color=always|never|auto` flag.
- Define color scheme in a configurable way (hardcoded initially, configurable later).

---

## 4) Catch non-token text after task list as description search

**Design**
- When `task list` receives arguments that don't match any known filter token, treat them as description search.
- Example: `task list fix bug` → searches for tasks with "fix" and "bug" in description.
- Should work with existing filters: `task list project:work fix bug` → filters by project AND searches description.

**Challenge**
- How to distinguish between:
  - Filter tokens: `project:work`, `status:pending`, `+urgent`
  - Description search: `fix bug`, `meeting notes`
- Taskwarrior uses a heuristic: if it looks like a filter token (has `:`, `+`, `-`, etc.), treat as filter; otherwise, treat as description search.

**Implementation Barrier**
- Need to update argument parsing in `handle_task_list()` to:
  1. Parse known filter tokens first.
  2. Collect remaining arguments as description search terms.
  3. Add description search to filter evaluation.

**Tests**
- `task list fix bug` searches descriptions.
- `task list project:work meeting` filters by project AND searches descriptions.
- `task list +urgent fix` filters by tag AND searches descriptions.
- Edge cases: empty search, special characters, etc.

**Decision**
- Use Taskwarrior-style heuristic: tokens with `:`, `+`, `-`, `@` are filters; everything else is description search.
- Description search should be case-insensitive substring match (or word-boundary match?).

---

## 5) Task projects list to reflect nesting properly

**Design**
- Enhance `task projects list` to show:
  - Nested project hierarchy with indentation (e.g., `admin` → `  admin.email`).
  - Task count per project (direct tasks only, no rollup).
  - Task counts broken down by status: pending, completed, closed (in columns).

**Output Format:**
```
ID    Name              Tasks    Pending    Completed    Closed
---------------------------------------------------------------
1     work              15       10         4            1
2       work.backend    8        6          2            0
3         work.backend.api  3     2          1            0
4       work.frontend   7        4          2            1
5     personal          5        3          2            0
```

**Challenge**
- How to handle projects with no direct tasks but with nested projects that have tasks?
- Should archived projects be shown? (probably only with `--archived` flag).
- Column width management for long project names.

**Implementation Barrier**
- Need to:
  1. Build project hierarchy tree.
  2. Query task counts per project (grouped by status).
  3. Format with proper indentation.
  4. Handle edge cases (orphaned nested projects, etc.).

**Tests**
- Flat projects display correctly.
- Nested projects show proper indentation.
- Task counts are accurate (direct tasks only).
- Status breakdown is correct.
- Archived projects excluded by default.
- `--archived` includes archived projects.

**Decision**
- Show only direct task counts (no rollup) as specified.
- Use 2-space indentation for nested levels.
- Include archived projects only with `--archived` flag.
- Use fixed-width columns for alignment.

---

## Implementation Order

Suggested order based on dependencies and complexity:

1. **Item 1 (Kanban reorder)** - Simplest, no dependencies.
2. **Item 2b (Live task ID)** - Simple, useful for scripting.
3. **Item 2a (Allocations view)** - Moderate complexity, builds on existing list infrastructure.
4. **Item 4 (Description search)** - Moderate complexity, requires filter parser updates.
5. **Item 5 (Projects list nesting)** - Moderate complexity, requires hierarchy building.
6. **Item 3 (Color column)** - Most complex, requires new dependency and TTY detection.

---

## Risks and Mitigations

### Risk 1: Kanban reorder breaks user workflows
- **Mitigation**: Document the change clearly. Users can use `sort:-kanban` to reverse if needed.

### Risk 2: Description search conflicts with filter parsing
- **Mitigation**: Use strict token detection (require `:`, `+`, `-`, `@` for filters). Test edge cases thoroughly.

### Risk 3: Color support adds complexity and dependency
- **Mitigation**: Make color optional, auto-detect TTY, provide `--no-color` flag. Use lightweight color library.

### Risk 4: Projects list becomes too wide for terminals
- **Mitigation**: Use reasonable column widths, truncate long names with ellipsis, consider `--wide` flag.

---

## Documentation Updates

- Update `docs/COMMAND_REFERENCE.md` with:
  - New `task live` command.
  - New `task allocations` command.
  - `color:` filter documentation.
  - Description search behavior.
  - Enhanced `task projects list` output format.
- Update kanban stage documentation to reflect new sort order.
- Add examples for each new feature.

---

## Testing Strategy

For each item:
1. Unit tests for core logic (sorting, parsing, formatting).
2. Integration tests for command-line interface.
3. Acceptance tests for end-to-end workflows.
4. Edge case testing (empty results, invalid input, etc.).

---

## Future Enhancements (Out of Scope)

- Configurable color schemes (item 3).
- Full-text search with indexing (item 4).
- Project rollup totals option (item 5).
- Allocation vs. logged time visualization (item 2a).

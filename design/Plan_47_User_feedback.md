# Plan 47: User Feedback — Mostly Bugs

---

## Item 1: Status Changes Should Quote Task Description

### Current State
Inconsistent output across status-changing commands:
- `close` prints: `Closed task 5` — no description
- `cancel` prints: `Cancelled task 5` — no description
- `reopen` prints: `Reopened task 5: Fix login bug` — includes description
- `delete` prints: `Deleted task 5: Fix login bug` — includes description

### Proposal
Update `close` and `cancel` to include the task description, matching the pattern already used by `reopen` and `delete`:

```
Closed task 5: Fix login bug
Cancelled task 5: Fix login bug
```

The task object is already fetched in both handlers. The println calls just need the description appended.

### Files to Modify
- `src/cli/commands.rs` — 4 println statements:
  - Line ~4343: `handle_task_close` non-interactive path
  - Line ~4420: `handle_close_interactive` path
  - Line ~4560: `handle_task_cancel` non-interactive path
  - Line ~4632: `handle_cancel_interactive` path

### Decision Required
- **None** — bug fix for consistency.

---

## Item 2: Timer Column Name Mismatch

### Current State
The column enum variant is `TaskListColumn::Clock`, the display label is `"Timer"` (changed in a previous plan), but `parse_task_column()` only accepts `"clock"`:

```rust
TaskListColumn::Clock => "Timer",   // display label
"clock" => Some(TaskListColumn::Clock), // parse input
```

Users see a column labeled "Timer" but must use `hide:clock`, `sort:clock`, `color:clock`. Using `hide:timer` silently fails (unrecognized column name, ignored).

### Proposal
Add `"timer"` as an accepted alias in `parse_task_column()`, keeping `"clock"` for backward compatibility:

```rust
"clock" | "timer" => Some(TaskListColumn::Clock),
```

### Files to Modify
- `src/cli/output.rs` — line ~826, add `"timer"` to the match arm

### Decision Required
- **Should we also rename the enum variant from `Clock` to `Timer`?** This is internal-only and doesn't affect behavior, but would improve code clarity. Recommend deferring — it's a rename-only refactor with no user impact.
- Decision: do not defer. Code clarity is of top concern. Also rename the enum variant from `Clock` to `Timer`.

---

## Item 3: Add Modified Column to Task List

### Current State
- `Task.modified_ts` exists in the model and is updated on field changes and status changes.
- `show` displays it, but `TaskListColumn` enum has no `Modified` variant.
- No way to see when tasks were last touched in list view.

The user also notes this should reflect "activities indicating work/focus (task status, annotations, sessions, ..)". Currently `modified_ts` is only updated for task field edits and status changes — annotations and sessions do NOT update it.

### Proposal

**Part A — Add Modified column (simple):**
Add a `Modified` variant to `TaskListColumn` with parsing aliases `"modified"` and `"mod"`. Format it the same way as the `Created` column (relative age like "2d", "1w"). Not shown by default — available via `sort:modified` or list views.

**Part B — Broaden modified_ts semantics (decision required):**
Expand what updates `modified_ts` to include:
- Adding/deleting annotations
- Starting/stopping sessions (on/off)
- Sending/collecting externals

This makes `modified_ts` reflect "last activity" rather than "last field edit".

### Files to Modify
- `src/cli/output.rs` — add `Modified` to `TaskListColumn` enum, `parse_task_column()`, `column_label()`, default column widths, and value population in `format_task_list_table()`
- Part B only: `src/repo/annotation.rs`, `src/repo/session.rs`, `src/repo/external.rs` — touch `modified_ts` after writes
  - Or: `src/repo/task.rs` — add a `touch_modified(conn, task_id)` helper called from relevant operations

### Decision Required
- **Part B — Broaden semantics?** Options:
  - **(a)** Keep `modified_ts` as field-edit-only, add column as-is. Simple. Users see when fields changed.
  - **(b)** Broaden `modified_ts` to include all activity (annotations, sessions, externals). More useful for "when did I last work on this?" but changes existing semantics.
  - **(c)** Add a separate `activity_ts` column (migration v10) that tracks all activity, keep `modified_ts` for field edits. Most precise, but adds schema complexity.
  - Recommend **(b)** — `modified_ts` should mean "last meaningful interaction."
- Decision: (c) add activity_ts and modified_ts columns. This provides the best data for later analytics. Visible by default. Let modified be the first column hidden due with width contraints, and activity to be hidden just after created column is omitted.

---

## Item 4: Annotate Pipe Compatibility

### Current State
Investigation shows annotate **is** already pipe-compatible:
- `execute_piped_command` handles `"annotate"` (line ~801)
- The first-command pipe dispatch handles `Commands::Annotate` (line ~1013)
- Abbreviation `ann` expands correctly

However, the user reports this isn't working. Possible causes to verify:
1. The abbreviation `ann` may not be recognized in pipe context (abbreviations are expanded before pipe splitting, so this should work)
2. The `--delete` flag is explicitly rejected in pipe context
3. There may be an edge case with the argument parsing when `annotate` is abbreviated in a pipe

### Proposal
Run the manual test script to reproduce. If the issue is confirmed:
- Trace the exact failing command and fix
- Add a test case

If the issue cannot be reproduced, mark as already-working and add regression test coverage.

### Files to Modify
- TBD after reproduction

### Decision Required
- **None** — needs investigation to confirm.
- Defer for now

---

## Item 5: Consistent ID Fallback to Active Session Task

### Current State
Commands behave inconsistently when no task ID is provided:

| Command | No ID behavior |
|---------|---------------|
| `on` | Uses queue[0] |
| `off` | Uses active session (only one possible) |
| `close` | Uses queue[0] (requires active session) |
| `annotate` | Falls back to active session task |
| `show` | **Requires** explicit ID (fails) |
| `modify` | **Requires** explicit ID (fails) |

The user expectation: if I'm clocked in to task 5, `tatl show` should show task 5 and `tatl modify +tag` should modify task 5.

### Proposal
For commands where it makes sense, fall back to the active session's task ID when no explicit target is provided:

- **`show`**: Change `target` from required `String` to `Option<String>`. If None, check for active session. If no session, error with suggestion.
- **`modify`**: Same pattern — `target` becomes `Option<String>`.

Do NOT change `close` — it already has its own queue[0] logic which is correct (close what you're working on).

Error message when no session and no ID:
```
No task ID provided and no session is running. Usage: tatl show <id>
```

### Files to Modify
- `src/cli/commands.rs` — change `Show.target` and `Modify.target` from `String` to `Option<String>`, update handlers to check active session

### Decision Required
- **Which commands should support this?** Proposal covers `show` and `modify`. Should `send`, `enqueue`, `clone` also fall back? Recommend limiting to `show` and `modify` — the others have explicit intent that benefits from requiring an ID.
- Just do show and modify for now.

---

## Item 6: Enqueue and Annotate as Pipe-In/Out Commands

### Current State
Investigation shows both **are** already pipe-compatible:
- `enqueue` is in `execute_piped_command` (line ~789) and in the first-command pipe dispatch (line ~989)
- `annotate` is similarly supported (see Item 4)
- Abbreviations `enq` and `ann` expand before pipe processing

Same situation as Item 4 — need reproduction of the reported failure.

### Proposal
Same as Item 4: run the manual test script, reproduce the exact failing command, and fix or confirm working. Add regression tests for abbreviated pipe usage.

If the issue is that abbreviations don't work *within* pipe segments (e.g., `tatl add "Task" : enq`), the fix is in pipe segment parsing — currently abbreviation expansion only runs on the first segment.

### Investigation Required
Check if `expand_command_abbreviations()` is called on pipe segments in `execute_piped_command()`. The pipe segment parser receives raw strings — if abbreviations aren't expanded for subsequent segments, that's the bug.

### Files to Modify
- Likely `src/cli/commands.rs` — add abbreviation expansion in `execute_piped_command()` for the command name
- Or: expand abbreviations for all pipe segments during the initial pipe-splitting phase

### Decision Required
- **None** — bug investigation and fix.
- Defer for now

---

## Item 7: Task Nesting / Subtasks

### Current State
No task nesting exists. The `Task` model has no `parent_id` or subtask fields. Project nesting (via dot notation) exists and works well, but there is no equivalent for tasks.

No prior plan has scoped task nesting — the user may be recalling project nesting discussions.

### Proposal
This is a significant feature that warrants its own design plan. Key design questions:

1. **Representation**: `parent_id` column on tasks (tree) vs. dot notation in description vs. linking table
2. **Completion semantics**: Does closing a parent close children? Does closing all children auto-close the parent? Does a parent's "progress" derive from children?
3. **Queue interaction**: Can parent tasks be enqueued, or only leaves? Do children inherit queue position?
4. **Time tracking**: Does time on a child roll up to the parent? Are both independently trackable?
5. **Display**: Indented tree in `list`? Separate `subtasks` command?
6. **Respawn/clone**: Does cloning a parent clone the tree?

### Recommendation
Defer to **Plan 48: Task Nesting** as a standalone design. The scope is too large for a bug-fix batch. Capture the user's stated use cases:
- Breaking a big task into components
- Grouping small tasks under a parent
- Parent accounts for children's completion

### Decision Required
- **Defer to separate plan?** Recommend yes — this is an architectural feature, not a bug fix.
- Defer to a separate plan for now.

---

## Implementation Priority

| Item | Effort | Type | Suggested Order |
|------|--------|------|-----------------|
| 1. Description in close/cancel | Tiny | Bug | 1st — 4 println fixes |
| 2. Timer column alias | Tiny | Bug | 2nd — 1 line |
| 4/6. Pipe abbreviation investigation | Small | Bug | 3rd — investigate + fix |
| 5. ID fallback to active session | Medium | Enhancement | 4th — show + modify |
| 3. Modified column | Medium | Enhancement | 5th — new column + semantics |
| 7. Task nesting | Large | New feature | Defer to Plan 48 |

Items 1–2 can be done immediately. Items 4/6 need reproduction first. Item 5 is a targeted change to two commands. Item 3 depends on the semantics decision.

---

## Decisions Summary

1. **Timer column — rename enum variant?** Recommend defer (internal-only, no user impact). -- yes, rename
2. **Modified column semantics**: (a) field-edit-only, (b) broaden to all activity, or (c) separate `activity_ts`? -- separate activity_ts and show modified_ts
3. **ID fallback scope**: `show` and `modify` only, or broader? only show and modify for now
4. **Task nesting**: Defer to Plan 48? Yes

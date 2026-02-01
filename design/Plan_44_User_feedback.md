# Plan 44: User Feedback

## Item 1: `tatl clone <id>` — Task Duplication

### Current State
No clone/duplicate command exists. Tasks are created via `TaskRepo::create_full()` which accepts all task fields (description, project_id, due_ts, scheduled_ts, wait_ts, alloc_secs, template, respawn, udas, tags).

### Proposal
Add a `Clone` variant to `Commands` enum:

```
tatl clone <id> [field overrides...]
tatl clone 10 project=other due=2026-01-01
```

Behavior:
- Fetch source task by ID, read its fields + tags + UDAs
- Apply any override arguments (same parser as `modify`)
- Call `TaskRepo::create_full()` with merged attributes
- Always set status to `open` regardless of source status
- Generate new UUID (automatic — `create_full` does this)
- Do NOT copy sessions, annotations, externals, or queue position
- Print: `Cloned task {source_id} → new task {new_id}: {description}`

Support in pipe operator: `tatl close 10 : clone` would clone the just-closed task as a new open instance. This overlaps with respawn semantically but is a manual one-shot operation.

### Files to Modify
- `src/cli/commands.rs` — add `Clone` variant, `handle_clone()`, pipe support in `execute_piped_command()`
- No schema changes needed

### Decision Required
- **Should clone copy the `respawn` field?** Cloning a respawning task creates two respawn sources. Options: (a) always copy it, (b) always clear it, (c) copy it but warn the user.
Decision: B -- always clear it. This is one shot.

---

## Item 2: Respawn Indicator in Listings

### Current State
The `respawn` field is only visible in `tatl show <id>` output. The `TaskListColumn` enum has no respawn column. The task list already has many columns that compete for terminal width.

### Proposal
Append a symbol to the description column when `task.respawn.is_some()`. This avoids adding a full column.

Suggested symbol: `↻` (Unicode recycling/loop arrow) appended to description.

Implementation in `format_task_list_table()` at the point where description is inserted into `values`:
```rust
let desc_display = if task.respawn.is_some() {
    format!("{} ↻", task.description)
} else {
    task.description.clone()
};
values.insert(TaskListColumn::Description, desc_display);
```

The symbol is purely visual — filter/sort still operates on the raw `description` field.

### Files to Modify
- `src/cli/output.rs` — append symbol in `format_task_list_table()` (~line 891)

### Decision Required
- **Symbol choice**: `↻` (loop), `⟳` (clockwise), `♻` (recycle), or a plain ASCII marker like `[R]`? Terminal font support varies for some Unicode symbols.
Decision: use the Unicode recyling/loop arrow, same as is used elsewhere with respawn. 

---

## Item 3: Task Details / Long-form Notes

### Current State
- `Task` has only `description` (single-line string) — no long-form text field
- `Annotation` model stores timestamped notes (linked to task and optionally to a session)
- Annotations are displayed in `tatl show` output but not printed during `on`
- No editor integration exists anywhere in the codebase

### Analysis
This is the most complex item and involves a design tension:

**Option A: New `details` column on tasks table**
- Add `details TEXT NULL` to tasks (migration v10)
- `tatl modify <id> details="..."` or `tatl details <id>` opens `$EDITOR`
- Print details on `tatl on` (start timing)
- Pro: Clean separation — description is the "what", details is the "how"
- Con: Schema change, another field to maintain, potential for stale details

**Option B: Promote annotations as the details mechanism**
- Print the most recent annotation (or all annotations) when `tatl on` starts
- Add `tatl annotate <id> --edit` to open `$EDITOR` for longer notes
- Pro: No schema change, reuses existing infrastructure
- Con: Annotations are timestamped append-only log entries — editing the "current details" would mean finding the latest one, which is awkward semantically

**Option C: Hybrid — details field + annotations append to it**
- Add `details TEXT NULL` to tasks
- `tatl annotate` appends a timestamped entry to `details` (like a running log)
- `tatl details <id>` opens `$EDITOR` on the full text
- Print details on `on`
- Pro: Single place for all task notes, with timestamps
- Con: Mixing structured annotations with free-form text is messy

### Editor Integration (applies to any option)
```rust
fn open_editor(initial_content: &str) -> Result<String> {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());
    // Write initial_content to temp file
    // Spawn editor on temp file
    // Read back result
    // Return content
}
```

Flag: `-m "inline text"` for quick entry, no flag opens editor.

### Printing on `on`
In `handle_on()` / `handle_task_on()`, after printing "Started timing task N: desc", print the details/recent annotations if they exist. Keep it brief — maybe last 3 annotations or truncated details.

### Files to Modify (Option A)
- `src/db/migrations.rs` — migration v10 adding `details` column
- `src/models/task.rs` — add `details` field to Task struct
- `src/repo/task.rs` — include `details` in all queries
- `src/cli/commands.rs` — add `Details` command or `--edit` flag on modify, print on `on`
- `src/cli/output.rs` — include in `show` output

### Decision Required
- **Which option (A, B, or C)?** This determines scope. Option B is smallest change, Option A is cleanest design.
- **Is this in scope for tatl's philosophy of "doing work, not managing work"?** The `on` printout of execution details directly serves someone sitting down to do the work. Annotations-as-log serves retrospective analysis. Both seem aligned, but the editor integration adds complexity.
- **Should this be a separate plan?** Given the design decisions needed, this could be deferred to Plan 45 once the approach is chosen.

---

## Item 4: Print Timestamps in On/Off Messages

### Current State
- `on` prints: `Started timing task {id}: {description}` — no timestamp
- `off` prints: `Stopped timing task {id}: {description}` — no timestamp
- `offon` already prints timestamps: `Stopped timing task {id} at {time}`

### Proposal
Include the effective start/end timestamp in all timer messages, consistent with `offon`:

```
Started timing task 10: Fix bug (14:32)
Stopped timing task 10: Fix bug (15:07, 35m)
```

The parenthetical shows local time and, for `off`, the session duration.

Implementation: In `handle_on`/`handle_task_on` and `handle_off`, format the effective timestamp using `chrono::Local` and append it to the println. For `off`, also compute duration from `session.start_ts`.

### Files to Modify
- `src/cli/commands.rs` — update println calls in `handle_on`, `handle_task_on`, `handle_off`, and the piped `"off"` handler in `execute_piped_command()`

### Decision Required
- **Format**: `(14:32)` vs `(2026-01-31 14:32)` vs `(14:32:00)`. Short time-only seems best for typical same-day use. Could include date only when start/end cross a day boundary.

---

## Item 5: `send` Stops Active Timer

### Current State
`handle_send()` removes the task from the queue and creates an external record, but does NOT check for or close an active session. If you send task 10 while its timer is running, the timer keeps running on a task that's now "external" — an inconsistent state.

### Proposal
In `handle_send()`, after the current queue removal logic, check for an active session on the task and close it:

```rust
let open_session = SessionRepo::get_open(&conn)?;
if let Some(session) = open_session {
    if session.task_id == task_id {
        let end_ts = chrono::Utc::now().timestamp();
        let end_ts = std::cmp::max(end_ts, session.start_ts + 1);
        SessionRepo::close_open(&conn, end_ts)?;
        let duration = end_ts - session.start_ts;
        println!("Stopped timing task {}: {} ({})",
            task_id, task.description, format_duration(duration));
    }
}
```

Also update `execute_piped_command` for the `"send"` case — though `send` in a pipe goes through `handle_send` directly, so the fix propagates.

### Files to Modify
- `src/cli/commands.rs` — update `handle_send()` to close active session

### Decision Required
None — this is a clear bug fix for state consistency.

---

## Item 6: Close Echoes Timer Stop

### Current State
`handle_task_close()` does stop the timer if the closed task has an active session (calls `SessionRepo::close_open()`), but only prints `Closed task {id}` — no mention of the timer being stopped. The user gets no feedback that their timer was ended.

### Proposal
After closing the session in `handle_task_close()` and `handle_close_interactive()`, print the timer stop message with duration:

```
Stopped timing task 10: Fix bug (2h15m)
Closed task 10
```

This matches the standard `off` output and makes the side effect visible.

Implementation: In the session-closing block (~line 4062-4071 and ~line 4142-4150), add a println after `SessionRepo::close_open()` succeeds.

### Files to Modify
- `src/cli/commands.rs` — add println in `handle_task_close()` and `handle_close_interactive()` after session close

### Decision Required
None — straightforward UX improvement.

---

## Implementation Priority

| Item | Effort | Risk | Suggested Order |
|------|--------|------|-----------------|
| 6. Close echoes timer stop | Tiny | None | 1st — trivial fix |
| 5. Send stops timer | Small | None | 2nd — bug fix |
| 4. Timestamps in on/off | Small | None | 3rd — quick UX win |
| 2. Respawn indicator | Small | None | 4th — display tweak |
| 1. Clone command | Medium | Low | 5th — new command |
| 3. Details/notes field | Large | Medium | Defer — needs design decisions |

Items 4, 5, 6 can be done in a single pass through `commands.rs`. Item 2 is isolated to `output.rs`. Item 1 is a standalone new command. Item 3 should be split into its own plan once the approach is decided.

## Decisions Summary

1. **Clone + respawn**: Should clone copy the respawn rule? (clear)
2. **Respawn symbol**: Which character? (`↻`)
3. **Details field approach**: defer and create a new Plan capturing it independently for analysis.
4. **Timestamp format**: Time-only `(14:32)` or full `(2026-01-31 14:32)`? Whatever is most consistent.

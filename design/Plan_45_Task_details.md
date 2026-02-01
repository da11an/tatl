# Plan 45: Task Details / Long-form Notes

Extracted from Plan 44 Item 3 for independent analysis.

## Problem Statement

Users want a way to attach execution details to a task — the "how" alongside the description's "what". These details should print when a timer starts (`tatl on`) so the user is reminded of context. Entry should support an editor (`$EDITOR`) for long-form text, with an inline `-m` flag for quick notes.

This overlaps with annotations, which are timestamped append-only notes linked to tasks and sessions. The question is whether to extend annotations, add a new field, or combine both.

## Current State

- `Task` has only `description` (single-line string) — no long-form text field
- `Annotation` model stores timestamped notes (linked to task and optionally to a session)
- Annotations are displayed in `tatl show` output but not printed during `on`
- No editor integration exists anywhere in the codebase

## Options

### Option A: New `details` column on tasks table

- Add `details TEXT NULL` to tasks (migration v10)
- `tatl modify <id> details="..."` or `tatl details <id>` opens `$EDITOR`
- Print details on `tatl on` (start timing)
- Pro: Clean separation — description is the "what", details is the "how"
- Con: Schema change, another field to maintain, potential for stale details

### Option B: Promote annotations as the details mechanism

- Print the most recent annotation (or all annotations) when `tatl on` starts
- Add `tatl annotate <id> --edit` to open `$EDITOR` for longer notes
- Pro: No schema change, reuses existing infrastructure
- Con: Annotations are timestamped append-only log entries — editing the "current details" would mean finding the latest one, which is awkward semantically

### Option C: Hybrid — details field + annotations append to it

- Add `details TEXT NULL` to tasks
- `tatl annotate` appends a timestamped entry to `details` (like a running log)
- `tatl details <id>` opens `$EDITOR` on the full text
- Print details on `on`
- Pro: Single place for all task notes, with timestamps
- Con: Mixing structured annotations with free-form text is messy

## Editor Integration (applies to any option)

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

## Printing on `on`

In `handle_on()` / `handle_task_on()`, after printing "Started timing task N: desc", print the details/recent annotations if they exist. Keep it brief — maybe last 3 annotations or truncated details.

## Files to Modify (Option A)

- `src/db/migrations.rs` — migration v10 adding `details` column
- `src/models/task.rs` — add `details` field to Task struct
- `src/repo/task.rs` — include `details` in all queries
- `src/cli/commands.rs` — add `Details` command or `--edit` flag on modify, print on `on`
- `src/cli/output.rs` — include in `show` output

## Design Questions

- **Which option (A, B, or C)?** This determines scope. Option B is smallest change, Option A is cleanest design.
- **Is this in scope for tatl's philosophy of "doing work, not managing work"?** The `on` printout of execution details directly serves someone sitting down to do the work. Annotations-as-log serves retrospective analysis. Both seem aligned, but the editor integration adds complexity.
- **Project-level rollup**: The original feedback mentioned rolling up notes to project level for trajectory. How would this work? Concatenate all task details for a project? This seems like a reporting concern rather than a data model concern.

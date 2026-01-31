# Plan 42b: Database Overhaul Implementation

## Goal

Implement Plan 41's orthogonal state model: replace the current status/kanban duality with lifecycle + derived classification, enforce invariants at the DB and repo layer, and present a single "Stage" classification to the user.

## Status Mapping

| Current `tasks.status` | New `tasks.status` | Plan 41 concept |
|---|---|---|
| `pending` | `open` | Lifecycle: alive |
| `completed` | `closed` | Lifecycle: terminal (finished) |
| `closed` | `cancelled` | Lifecycle: terminal (abandoned) |
| `deleted` | `deleted` | Ledger correction (unchanged) |

The `completed` vs `closed` distinction maps to "finished work" vs "abandoned/cancelled". Both are terminal in Plan 41. The user-facing commands remain `finish` (sets `closed`) and `close` (sets `cancelled`).

## Derived Stage Classification

Replaces the current `kanban` column. Computed from orthogonal facts:

### Precedence (highest wins)

| Priority | Condition | Stage |
|---|---|---|
| 1 | `status = closed` | **Completed** |
| 2 | `status = cancelled` | **Cancelled** |
| 3 | open session exists for task | **Active** |
| 4 | active external exists | **External** |
| 5 | in queue + has sessions | **In Progress** |
| 6 | in queue + no sessions | **Planned** |
| 7 | not in queue + has sessions | **Suspended** |
| 8 | not in queue + no sessions | **Proposed** |

### Current Kanban → Stage Mapping

| Old Kanban | New Stage | Notes |
|---|---|---|
| `done` | `completed` or `cancelled` | Split by lifecycle |
| `external` | `external` | Same concept |
| `queued` (active session) | `active` | New: distinguished from queued |
| `queued` (no active session) | `in progress` or `planned` | Split by work history |
| `stalled` | `suspended` | Renamed |
| `proposed` | `proposed` | Unchanged |

The user should have the ability to configure the stage mapping to match the current kanban if desired.

## Invariants to Enforce

1. **Single active session** - Already enforced by `ux_sessions_single_open` unique index. No change needed.

2. **Active work implies queued** - If a task has an open session, it must be in `stack_items`. Enforced in repo layer (SessionRepo.create already calls StackRepo.push_to_top).

3. **External waiting does not keep queue position** - When timer is off and task has active externals, remove from queue. Enforced:
   - `send` command removes from queue (already implemented)
   - `off` command removes external task from queue (NEW)
   - `enqueue` rejects external-waiting tasks. (NEW)
   - External-waiting tasks may be `tatl on <task_id of external task>` which temporarily puts first in queue and turns on timer. It is removed from queue when timer stops.

4. **Terminal lifecycle cleanup** - When status becomes `closed` or `cancelled`:
   - Remove from queue (already implemented in finish/close)
   - Clear active externals (NEW - finish/close must mark_all_returned)

5. **Reopen restores to open** - `reopen` sets status back to `open`. Does not re-queue or re-externalize.

---

## Implementation Phases

### Phase 1: Schema Migration (DB version 8)

**File: `src/db/migrations.rs`**

Add migration v8:

```sql
-- 1. Rename status values
UPDATE tasks SET status = 'open' WHERE status = 'pending';
UPDATE tasks SET status = 'cancelled' WHERE status = 'closed';
UPDATE tasks SET status = 'closed' WHERE status = 'completed';

-- 2. Update CHECK constraint
-- SQLite doesn't support ALTER CHECK, so we need to recreate.
-- Use the standard SQLite migration pattern:
--   a) Create new table with correct constraint
--   b) Copy data
--   c) Drop old table
--   d) Rename new table

CREATE TABLE tasks_new (
    id INTEGER PRIMARY KEY,
    uuid TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('open','closed','cancelled','deleted')),
    project_id INTEGER NULL REFERENCES projects(id),
    due_ts INTEGER NULL,
    scheduled_ts INTEGER NULL,
    wait_ts INTEGER NULL,
    alloc_secs INTEGER NULL,
    template TEXT NULL,
    respawn TEXT NULL,
    udas_json TEXT NULL,
    created_ts INTEGER NOT NULL,
    modified_ts INTEGER NOT NULL
);

INSERT INTO tasks_new SELECT * FROM tasks;
DROP TABLE tasks;
ALTER TABLE tasks_new RENAME TO tasks;

-- 3. Recreate indexes
CREATE INDEX idx_tasks_project_id ON tasks(project_id);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_due_ts ON tasks(due_ts);
CREATE INDEX idx_tasks_scheduled_ts ON tasks(scheduled_ts);
CREATE INDEX idx_tasks_wait_ts ON tasks(wait_ts);

-- 4. Data cleanup: remove queue entries for terminal tasks
DELETE FROM stack_items WHERE task_id IN (
    SELECT id FROM tasks WHERE status IN ('closed', 'cancelled', 'deleted')
);

-- 5. Data cleanup: clear externals for terminal tasks
UPDATE externals SET returned_ts = CAST(strftime('%s', 'now') AS INTEGER)
WHERE returned_ts IS NULL AND task_id IN (
    SELECT id FROM tasks WHERE status IN ('closed', 'cancelled', 'deleted')
);

-- 6. Data cleanup: remove queue entries for external-waiting tasks
--    (only those without an open session)
DELETE FROM stack_items WHERE task_id IN (
    SELECT e.task_id FROM externals e
    WHERE e.returned_ts IS NULL
    AND e.task_id NOT IN (
        SELECT s.task_id FROM sessions s WHERE s.end_ts IS NULL
    )
);
```

**Note on table recreation**: Dropping the `tasks` table will cascade-delete rows in `task_tags`, `task_annotations`, `sessions`, `task_events`, `stack_items`, and `externals` due to `ON DELETE CASCADE` foreign keys. The migration must:
1. Disable foreign keys: `PRAGMA foreign_keys = OFF`
2. Wrap in transaction
3. Copy all dependent data or recreate without CASCADE

**Safer approach**: Since SQLite doesn't enforce CHECK constraints retroactively and we're just renaming values, we can:
1. Update the status values in-place (the 3 UPDATE statements)
2. Leave the old CHECK constraint in the schema (SQLite doesn't validate existing data against CHECK)
3. Enforce the new values in application code only
4. Do the table recreation in a future major version if needed

**Decision needed**: Full table recreation with proper CHECK constraint vs. in-place UPDATE with app-only enforcement?

### Phase 2: Model + Repo Layer

**File: `src/models/task.rs`**

Update `TaskStatus` enum:

```rust
pub enum TaskStatus {
    Open,       // was Pending
    Closed,     // was Completed
    Cancelled,  // was Closed
    Deleted,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Cancelled => "cancelled",
            Self::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(Self::Open),
            "closed" => Some(Self::Closed),
            "cancelled" => Some(Self::Cancelled),
            "deleted" => Some(Self::Deleted),
            // Legacy compatibility
            "pending" => Some(Self::Open),
            "completed" => Some(Self::Closed),
            _ => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Closed | Self::Cancelled | Self::Deleted)
    }
}
```

**File: `src/repo/task.rs`**

- `TaskRepo::complete()` → sets status to `Closed`
- `TaskRepo::close()` → sets status to `Cancelled`
- `TaskRepo::reopen()` → sets status to `Open`
- `TaskRepo::set_status()` → update string values
- Add `TaskRepo::is_terminal()` helper

**File: `src/repo/stack.rs`**

Add invariant checks to `StackRepo::enqueue()`:

```rust
// Before adding to queue via enqueue, validate:
// 1. Task status must be Open
// 2. Task must not have active externals (enqueue always rejects; use 'on' for temporary work)
fn validate_enqueue_eligibility(conn: &Connection, task_id: i64) -> Result<()> {
    let task = TaskRepo::get_by_id(conn, task_id)?
        .ok_or_else(|| anyhow!("Task {} not found", task_id))?;

    if task.status.is_terminal() {
        user_error(&format!("Cannot queue task {}: status is {}", task_id, task.status.as_str()));
    }

    if ExternalRepo::has_active_externals(conn, task_id)? {
        user_error(&format!(
            "Cannot queue task {}: waiting on external party. Use 'collect' first, or 'on {}' to work on it temporarily.",
            task_id, task_id
        ));
    }

    Ok(())
}
```

`push_to_top()` does NOT call this validation — it is used by the `on` command, which is the explicit path for temporarily working on external tasks. The `on` handler starts a session first, then pushes to queue top. When `off` is called, the external task is auto-removed from queue (invariant 3).

**File: `src/repo/external.rs`**

No structural changes needed. The `send`/`collect` logic moves to command handlers (Phase 3).

**File: `src/cli/commands.rs`**

Update `handle_task_on()` (already pushes to top - just needs to allow external tasks):
- Current code validates `status == Pending` → change to `status == Open`
- Allow external-waiting tasks to be temporarily worked on

Update `handle_off()` / session close path:
- After closing session, check if task has active externals
- If yes, remove task from queue (enforce invariant 3)

```rust
// In handle_off, after SessionRepo::close_open():
if ExternalRepo::has_active_externals(&conn, task_id)? {
    let stack = StackRepo::get_or_create_default(&conn)?;
    StackRepo::remove_task(&conn, stack.id.unwrap(), task_id)?;
}
```

Update `handle_task_finish()` and `handle_task_close()`:
- After setting terminal status, clear active externals:
```rust
ExternalRepo::mark_all_returned_for_task(&conn, task_id)?;
```
- Queue removal already happens (verified in exploration)

Update `handle_send()`:
- Already removes from queue (verified). No change needed.

Update `handle_collect()`:
- After marking externals returned, re-enqueue at bottom:
```rust
ExternalRepo::mark_all_returned_for_task(&conn, task_id)?;
let stack = StackRepo::get_or_create_default(&conn)?;
StackRepo::enqueue(&conn, stack.id.unwrap(), task_id)?;
println!("Collected task {} and added to queue", task_id);
```

**Decision needed**: Should `collect` auto-enqueue at bottom (Plan 41 says "default: bottom unless specified") or just clear external status and leave user to enqueue manually (current behavior)?

### Phase 3: Derived Stage + Filters

**File: `src/filter/evaluator.rs`**

Replace `calculate_task_kanban()` with `calculate_task_stage()`:

```rust
pub fn calculate_task_stage(task: &Task, conn: &Connection) -> Result<String> {
    // Priority 1-2: Terminal lifecycle
    match task.status {
        TaskStatus::Closed => return Ok("completed".to_string()),
        TaskStatus::Cancelled => return Ok("cancelled".to_string()),
        TaskStatus::Deleted => return Ok("deleted".to_string()),
        TaskStatus::Open => {}
    }

    // Priority 3: Active (has open session)
    if let Some(session) = SessionRepo::get_open(conn)? {
        if session.task_id == task.id.unwrap() {
            return Ok("active".to_string());
        }
    }

    // Priority 4: External waiting
    if ExternalRepo::has_active_externals(conn, task.id.unwrap())? {
        return Ok("external".to_string());
    }

    // Priority 5-8: Internal states
    let stack = StackRepo::get_or_create_default(conn)?;
    let items = StackRepo::get_items(conn, stack.id.unwrap())?;
    let in_queue = items.iter().any(|i| i.task_id == task.id.unwrap());

    let has_sessions = SessionRepo::get_for_task(conn, task.id.unwrap())?
        .len() > 0;

    match (in_queue, has_sessions) {
        (true, true) => Ok("in progress".to_string()),
        (true, false) => Ok("planned".to_string()),
        (false, true) => Ok("suspended".to_string()),
        (false, false) => Ok("proposed".to_string()),
    }
}
```

**File: `src/filter/parser.rs`**

Add `Stage` filter term alongside existing `Kanban`:

```rust
pub enum FilterTerm {
    // ...existing...
    Stage(ComparisonOp, Vec<String>),  // stage=active, stage=proposed, etc.
    // Keep Kanban as alias
}
```

In `parse_filter_term()`, map both `stage=X` and `kanban=X` to the same filter logic. Provide a mapping for old kanban values:

| Old filter | Maps to |
|---|---|
| `kanban=proposed` | `stage=proposed` |
| `kanban=stalled` | `stage=suspended` |
| `kanban=queued` | `stage=planned,in progress,active` |
| `kanban=external` | `stage=external` |
| `kanban=done` | `stage=completed,cancelled` |
| `status=pending` | `status=open` (with legacy compat) |
| `status=completed` | `status=closed` |
| `status=closed` | `status=cancelled` |

**File: `src/filter/evaluator.rs`**

Update `FilterTerm::matches()` to handle Stage filter using `calculate_task_stage()`.

### Phase 4: Output + CLI

**File: `src/cli/output.rs`**

- Rename "Kanban" column header to "Stage"
- Update column value display to use stage names
- Update color mapping for stage values

**File: `src/cli/commands.rs`**

- Update help text: replace "kanban" references with "stage"
- Update `looks_like_filter()` to recognize `stage=`
- Update `show` command output to display "Stage:" instead of "Kanban:"
- Update `projects report` column headers to new stage names
- Update `report` dashboard to use stage terminology

**File: `src/cli/commands.rs` - Status references**

Throughout the file, update status checks:
- `TaskStatus::Pending` → `TaskStatus::Open`
- `TaskStatus::Completed` → `TaskStatus::Closed`
- `TaskStatus::Closed` → `TaskStatus::Cancelled`
- String comparisons: `"pending"` → `"open"`, etc.

### Phase 5: Tests + Docs

**Test files to update** (all `tests/*.rs`):
- `status=pending` → `status=open`
- `status=completed` → `status=closed`
- `status=closed` → `status=cancelled`
- `kanban=queued` → `stage=planned` or `stage=in progress` or `stage=active`
- `kanban=proposed` → `stage=proposed`
- `kanban=stalled` → `stage=suspended`
- `kanban=external` → `stage=external`
- `kanban=done` → `stage=completed`
- All "Kanban" header assertions → "Stage"
- Add new tests for invariant enforcement:
  - Enqueue rejects terminal tasks
  - Enqueue rejects external-waiting tasks
  - Off removes external task from queue
  - Finish/close clears externals
  - Collect re-enqueues (if decided)

**Documentation:**
- `README.md` - Update status/stage references
- `docs/COMMAND_REFERENCE.md` - Update filter syntax, stage values
- `CLAUDE.md` - Update architecture notes

---

## Open Decisions

1. **Schema migration strategy**: Full table recreation (proper CHECK constraint) vs. in-place UPDATE (simpler, app-enforced)?
   - Recommendation: In-place UPDATE. SQLite doesn't validate existing rows against CHECK. Enforce in app. Avoids CASCADE delete risk.
   - Accept recommendation.

2. **Collect auto-enqueue**: Should `collect` automatically add the task back to the queue?
   - Plan 41 says: "Insert task into queue (default: bottom unless specified)"
   - Current behavior: Does NOT re-enqueue
   - Recommendation: Yes, auto-enqueue at bottom. Matches Plan 41.
   - Accept recommmendation.

3. **Legacy filter compatibility**: Keep `kanban=` as permanent alias or deprecate?
   - Recommendation: Keep as permanent alias. Low maintenance cost, avoids breaking muscle memory.
   - Decision: Drop kanban. This is new enough to make drastic changes.

4. **Legacy status compatibility**: Keep `status=pending` working or require `status=open`?
   - Recommendation: Keep legacy values in `from_str()` parsing. Accept both in filters.
   - Decision: Drop legacy. Force flip to new terminology. We're building for the future user, not the current user.

5. **User-facing terminology for terminal states**: `finish` → Completed, `close` → Cancelled. These map to `status=closed` and `status=cancelled` in the DB. Is "closed" confusing vs the command name "close"?
   - Option A: DB values `closed`/`cancelled`, user sees "Completed"/"Cancelled" (Plan 41)
   - Option B: DB values `finished`/`cancelled`, user sees "Finished"/"Cancelled"
   - Recommendation: Option A (matches Plan 41). The `status` column stores DB values; the Stage classification shows user-friendly labels.
   - Decision: update commands to match DB values: close, cancel. I like using close because it is less loaded. Intent fulfilled = closed. Intent shifted / not fulfilled = cancelled.

---

## File Change Summary

### Core files (in order):

| # | File | Changes |
|---|---|---|
| 1 | `src/db/migrations.rs` | Add v8 migration: rename status values, data cleanup |
| 2 | `src/models/task.rs` | Rename TaskStatus variants, add `is_terminal()`, legacy compat |
| 3 | `src/repo/task.rs` | Update status strings, complete→Closed, close→Cancelled |
| 4 | `src/repo/stack.rs` | Add `validate_queue_eligibility()` to enqueue/push_to_top |
| 5 | `src/cli/commands.rs` | Update status checks, off→dequeue-if-external, finish/close→clear-externals, collect→re-enqueue, help text |
| 6 | `src/filter/evaluator.rs` | Replace `calculate_task_kanban()` with `calculate_task_stage()` |
| 7 | `src/filter/parser.rs` | Add `Stage` filter term, keep `Kanban` as alias, legacy status mapping |
| 8 | `src/cli/output.rs` | Rename Kanban→Stage column, update colors |
| 9 | `src/cli/commands_sessions.rs` | Update any status string references |

### Test files (~30 files):

All test files with `status=`, `kanban=`, or "Kanban" assertions need updating.

### Doc files:

| File | Changes |
|---|---|
| `README.md` | Status/stage terminology |
| `docs/COMMAND_REFERENCE.md` | Filter syntax, stage values, command semantics |
| `CLAUDE.md` | Architecture overview |

## Verification

After each phase:
1. `cargo build` - compilation
2. `cargo test` - full test suite
3. `cargo clippy` - warnings

Smoke tests after all phases:
```bash
tatl add "Test" project=work         # stage=proposed
tatl enqueue 1                       # stage=planned
tatl on                              # stage=active
tatl off                             # stage=in progress
tatl send 1 bob "Review this"        # stage=external, removed from queue
tatl on 1                            # stage=active (temporary, stays external)
tatl off                             # stage=external, auto-removed from queue
tatl collect 1                       # stage=in progress (auto-enqueued)
tatl finish                          # stage=completed
tatl list stage=completed            # shows finished task
tatl list kanban=done                # legacy alias works
tatl list status=open                # new terminology
tatl list status=pending             # legacy alias works
```

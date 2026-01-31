# Plan 42: Migration Assessment for Task Management State Model

## Source Model (Plan 41 recap)
- **Orthogonal facts**: lifecycle (open/closed/cancelled -- deleting is not a lifecycle state, it is to fix mistakes like duplicates, not tracking), queue membership (queue_position), external waiting (externals table), work history (sessions exist), timer (open session), all stored independently.
- **Derived classification**: Proposed/Planned/In progress/Suspended/External/Active/Completed/Cancelled via precedence.
- **Invariants**: active ⇒ queued; external waiting ⇒ not queued when timer off; terminal lifecycle ⇒ not queued/external; single open session.
- **Command semantics**: `send` moves to external and dequeues; `collect` clears external and (re)queues; `on` ensures queue head; `off` dequeues if external waiting.

## Current Tatl Model (today)
- **Stored status**: pending/completed/closed/deleted (`tasks.status`).
- **Kanban**: derived in code (queued/stalled/proposed/external/done) based on status, sessions, externals, stack.
- **Queue**: `stack_items` table; allowed even for external tasks.
- **External**: `externals` table with sent/collected; no enforced exclusivity with queue.
- **Timer**: single open session enforced in code, not schema.
- **Classification**: mix of status + derived kanban; inconsistencies exist (e.g., external tasks can remain queued).

## Delta Analysis

### Database / Schema
- Introduce **lifecycle** column (open/closed/cancelled) or map existing `status` (`pending` => open, `completed/closed` => closed, `deleted` => cancelled/deleted).
  - Decision: Map to existing status `pending` => `open`, 'completed` => `closed`, `closed` => `cancelled`, `deleted` is not a lifecycle state, it's for fixing mistakes (duplicates, etc.), not state tracking.
- Enforce **queue integrity**: trigger on `stack_items` to reject tasks that have `external waiting` and no open session; reject queued terminal tasks.
- Enforce **single open session** at DB level (unique partial index on sessions where `end_ts IS NULL`).
- External exclusivity: add partial constraint to prevent `stack_items` rows when `externals.status='waiting'` and no open session.
- Optional: add **task_classification view** to compute derived stage (replacing kanban/status duality).

### Repository Layer
- Update `TaskRepo` to expose lifecycle vs derived classification; deprecate direct status mutations in favor of lifecycle changes plus queue/external/session side effects.
- StackRepo enqueue/dequeue/push should check external/lifecycle invariants before executing.
- SessionRepo create/close should bump lifecycle-derived classification (active) and adjust queue membership per invariant.
- ExternalRepo send/collect should adjust queue membership and reject illegal states.

### CLI / Command Handlers
- `send/collect`: enforce dequeue on send; enqueue on collect; disallow send on closed/cancelled tasks.
- `enqueue`/queue ops: reject if external waiting (unless timer on) or lifecycle not open.
- `on/off/onoff/offon`: ensure active ⇒ queued, and off on external ⇒ dequeue.
- `finish/close/reopen/delete`: map to lifecycle; ensure queue/externals cleaned.
- `list`/filters: expose `stage` (derived) and keep `status`/`kanban` as aliases to computed values.
- `show/status`: display derived stage instead of mixing status/kanban.

### Filters / Derived Fields
- Add `stage` filter term; map existing `status`/`kanban` filters to derived classification.
- Ensure `external` filter reflects external waiting fact, not just derived stage.

### Output / Columns
- Replace/augment Kanban column with derived Stage column; keep legacy labels as aliases initially.
- Queue column must hide external tasks unless active; clock/active markers align with timer state.

### Migration Steps
1) **Schema prep**: add lifecycle column (or reuse status mapping), add partial indexes/triggers:
   - unique open session
   - forbid queue for terminal lifecycle
   - forbid queue for external waiting when timer off
2) **Backfill**: set lifecycle from status; clear stack_items for closed/deleted; drop stack entries for external waiting tasks without open session.
3) **Views**: add `task_stage_view` computing derived classification.
4) **Code rollout**: update repos/CLI to honor invariants; keep legacy status/kanban interfaces mapped to new facts.
5) **Deprecation**: remove direct writes to status/kanban; rely on lifecycle + facts; migrate tests to stage terminology.

### Impact / Disruption
- **DB changes**: moderate; new constraints/triggers + backfill + possible new lifecycle column. Risk: existing data with illegal combos will be rejected and must be cleaned during migration.
- **Code changes**: high-touch across queue ops, external send/collect, session start/stop, finish/close/reopen, filters, list output, status/kanban derivation.
- **Compatibility**: need aliasing for existing filters (`status`, `kanban`) and columns; update docs/help.
- **Testing**: broad regression of CLI commands, queue/external interactions, filters, and list/status output; add migration tests for constraint enforcement.

### Recommended Approach
- Start with **DB constraints and views** to enforce invariants and surface derived stage, then adapt repositories/CLI to consume the view.
- Ship with **legacy compatibility layer** (status/kanban derived) and transitional errors that point users to the new model.
- Provide **data cleaning script** to drop illegal queue entries and external+queue conflicts before enabling triggers.

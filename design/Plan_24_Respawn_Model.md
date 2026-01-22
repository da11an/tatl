# Plan 24: Respawn Model for Tasks

## Executive Summary

This plan proposes replacing the current **recurrence model** with a **respawn model** better suited for task management (as opposed to calendar appointments). The key insight: for tasks like "do dishes daily," there's no value in creating multiple queued instances. Instead, the next instance should spawn only when the current one is completed.

## Problem Statement

### Current Recurrence Model Issues

The existing implementation (`src/recur/`) uses a **time-based generation** approach:

1. **Seed tasks** have a `recur` field (e.g., "daily", "weekly")
2. Running `tatl recur run --until <date>` creates all instances from now until that date
3. A `recur_occurrences` table tracks generated (seed_task_id, occurrence_ts) pairs for idempotency

**Problems:**

| Issue | Example | Impact |
|-------|---------|--------|
| **Task churn** | Daily task → 14 instances immediately | Cluttered task list |
| **Missed tasks pile up** | Miss 3 days → 3 separate "do dishes" tasks | Meaningless work |
| **No completion awareness** | Generates regardless of previous completion | Zombie tasks accumulate |
| **Appointment-centric** | Designed for "meeting at 10am" | Poor fit for task semantics |

### Recurrence vs. Respawn Mental Models

| Aspect | Recurrence (Appointments) | Respawn (Tasks) |
|--------|---------------------------|-----------------|
| **Trigger** | Time-based (clock strikes X) | Completion-based (previous done) |
| **Instances** | Many pre-created | One active at a time |
| **Missed events** | Each is distinct (missed meeting ≠ rescheduled) | Single obligation persists |
| **Due date** | Fixed in time | Relative to completion |

## Proposed Solution: Respawn Model

### Core Concept

A **respawn rule** defines when the next instance of a task should be created, triggered by **completion** of the current instance.

**Flow:**
1. Create a task with a respawn rule
2. Work on and complete the task
3. On completion → calculate next spawn date → create new instance with updated due date
4. Repeat

### Respawn Rule Syntax

```
respawn:<pattern>
```

**Patterns:**

| Pattern | Description | Example |
|---------|-------------|---------|
| `daily` | Every day | Next day after completion |
| `weekly` | Every week | Same weekday, next week |
| `monthly` | Every month | Same day of month |
| `every:Nd` | Every N days | `every:3d` = every 3 days |
| `every:Nw` | Every N weeks | `every:2w` = every 2 weeks |
| `every:Nm` | Every N months | `every:6m` = every 6 months |
| `weekdays:mon,wed,fri` | Specific weekdays | Next matching weekday |
| `monthdays:1,15` | Specific days of month | Next matching day |
| `nth:2:tue` | Nth weekday of month | 2nd Tuesday |

### Respawn Behavior

#### Scenario 1: Task Completed On Time

```
Task: "Submit timesheet" due:15th respawn:monthdays:14,30

Timeline:
- Jan 15: Complete task
- Respawn: Create new instance, due Jan 30
- Jan 30: Complete task
- Respawn: Create new instance, due Feb 14
```

#### Scenario 2: Task Completed Late

```
Task: "Submit timesheet" due:Jan 14 respawn:monthdays:14,30

Timeline:
- Jan 14: Due date passes (not completed)
- Jan 31: Finally complete task
- Respawn calculation:
  - Pattern: 14th and 30th of each month
  - Jan 30 already passed
  - Next valid: Feb 14
- Create new instance, due Feb 14
```

**Key insight**: The respawn looks for the **next future occurrence** from the completion date, not from the original due date.

#### Scenario 3: Daily Task with Catch-up

```
Task: "Do dishes" due:noon respawn:daily

Timeline:
- Monday noon: Due, not done
- Tuesday noon: Due, not done (same task!)
- Wednesday 8am: Complete task
- Respawn: Create new instance, due Wednesday noon (today)
  - If completion is after noon: due Thursday noon
```

### Key Design Decisions

#### Decision 1: Respawn Trigger Point

**Options:**
- A. On `finish` command (completed successfully)
- B. On `close` command (completed or abandoned)
- C. On either `finish` or `close`

**Recommendation:** Option C - Both `finish` and `close` trigger respawn

**Rationale:** 
- `finish` = success → respawn makes sense
- `close` = abandoned/cancelled → still want the obligation to respawn
- If truly cancelling a recurring obligation, use `tatl delete` or remove the respawn rule

#### Decision 2: Where Respawn Rules Live

**Options:**
- A. On the task itself (like current `recur` field)
- B. Separate "respawn template" entity
- C. Both (template provides defaults, task can override)

**Recommendation:** Option A (initially), expandable to C

**Decision:** Go with Option A. Details like allocation, description, project, tags, are copied into the new instance. Sessions obviously are not.

**Rationale:** 
- Keep it simple for MVP
- The current `recur` field can be repurposed as `respawn`
- Templates already support attribute inheritance

#### Decision 3: Handling "Orphaned" Tasks

What if a task with a respawn rule is deleted?

**Recommendation:** Respawn chain ends - no action needed

**Rationale:**
- Delete is explicit intent to remove obligation
- To truly remove a recurring obligation, delete is the right action
- No need for additional confirmation

**Decision:** Recommendation is perfect. We already have a delete command. If you intended to keep the task respawning going without completing the task, you could close instead of deleting the task.

#### Decision 4: Due Date Calculation

**Options:**
- A. Next occurrence from completion date
- B. Next occurrence from original due date
- C. Next occurrence from max(completion date, original due date)

**Recommendation:** Option A - From completion date

**Rationale:**
- Most intuitive for tasks
- Avoids creating already-overdue instances
- Matches mental model of "finished this, when's next?"

**Decision:** Option A

#### Decision 5: Carrying Forward Attributes

**Recommendation:** New instance inherits all attributes except:
- `status` → reset to Pending
- `due_ts` → calculated from respawn rule
- `id` / `uuid` → new identity
- Sessions → not carried (fresh task)

**Decision:** Yes

### Respawn Rule Parsing

#### Grammar

```
respawn_rule = frequency | specific_days | nth_weekday

frequency    = "daily" | "weekly" | "monthly" | "yearly"
             | "every:" number unit

specific_days = "weekdays:" weekday_list
              | "monthdays:" number_list

nth_weekday  = "nth:" number ":" weekday

weekday_list = weekday ("," weekday)*
weekday      = "mon" | "tue" | "wed" | "thu" | "fri" | "sat" | "sun"

number_list  = number ("," number)*
number       = [1-9][0-9]*
unit         = "d" | "w" | "m" | "y"
```

#### Examples

| Input | Interpretation |
|-------|----------------|
| `respawn:daily` | Every day at same time |
| `respawn:every:3d` | Every 3 days |
| `respawn:weekly` | Every 7 days, same weekday |
| `respawn:weekdays:mon,fri` | Every Monday and Friday |
| `respawn:monthdays:1,15` | 1st and 15th of each month |
| `respawn:nth:1:mon` | First Monday of each month |
| `respawn:monthly` | Same day each month |

## Implementation Plan

### Phase 1: Rename and Refactor (Foundation)

**Goal:** Rename `recur` to `respawn` throughout codebase

1. **Database Migration**
   - Rename `recur` column to `respawn` in `tasks` table
   - Drop `recur_occurrences` table (no longer needed)
   - Add migration handling for existing data

2. **Model Updates**
   - `Task.recur` → `Task.respawn`
   - Update all references in repos and handlers

3. **Parser Updates**
   - `src/recur/` → `src/respawn/`
   - Update `RespawnRule` struct with new patterns

4. **CLI Updates**
   - Field name: `recur:` → `respawn:`
   - Remove `tatl recur run` command (obsolete)

### Phase 2: Respawn Logic (Core)

**Goal:** Implement respawn-on-completion behavior

1. **Calculate Next Occurrence**
   ```rust
   fn next_occurrence(rule: &RespawnRule, from_ts: i64) -> Option<i64>
   ```
   - Handle each pattern type
   - Return `None` if rule is invalid or no future occurrence

2. **Respawn on Finish/Close**
   ```rust
   fn respawn_task(conn: &Connection, task_id: i64) -> Result<Option<i64>>
   ```
   - Check if task has respawn rule
   - If so, calculate next due date
   - Create new task instance with updated due
   - Return new task ID (or None if no respawn)

3. **Integrate with Handlers**
   - `handle_task_finish` → call `respawn_task`
   - `handle_task_close` → call `respawn_task`
   - Print respawn notification to user

### Phase 3: Advanced Patterns

**Goal:** Support complex respawn patterns

1. **Weekday Patterns**
   - `weekdays:mon,wed,fri`
   - Find next matching weekday from completion date

2. **Monthday Patterns**
   - `monthdays:1,15`
   - Handle month boundaries (e.g., no 31st in February)

3. **Nth Weekday**
   - `nth:2:tue` = 2nd Tuesday
   - Calculate within month context

4. **Time Preservation**
   - Carry forward time-of-day from original due date
   - If no due date, respawn with no time constraint

### Phase 4: Edge Cases and Polish

1. **No Due Date Tasks**
   - Respawn creates instance due "N days from completion"
   - Or: no due date on respawn (just creates next task)

2. **Template Integration**
   - `template:` field on respawned task
   - Re-evaluate template on respawn? (probably not - keep it simple)

3. **Testing**
   - Unit tests for all patterns
   - Integration tests for finish/close → respawn
   - Edge case tests (month boundaries, leap years, etc.)

4. **Documentation**
   - Update command reference
   - Add respawn examples to README

## Database Changes

### Migration

```sql
-- Drop old recurrence tracking (no longer needed)
DROP TABLE IF EXISTS recur_occurrences;

-- Rename recur field to respawn
-- (handled via column rename or recreate table)
ALTER TABLE tasks RENAME COLUMN recur TO respawn;
```

### New Schema

```sql
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY,
    uuid TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    project_id INTEGER REFERENCES projects(id),
    due_ts INTEGER,
    scheduled_ts INTEGER,
    wait_ts INTEGER,
    alloc_secs INTEGER,
    template TEXT,
    respawn TEXT,  -- formerly 'recur'
    udas_json TEXT,
    created_ts INTEGER NOT NULL,
    modified_ts INTEGER NOT NULL
);
```

## API Changes

### CLI

**Removed:**
- `tatl recur run [--until <date>]` - obsolete

**Changed:**
- `recur:<pattern>` → `respawn:<pattern>` (field name)

**Behavior Change:**
- `tatl finish` / `tatl close` on respawn-enabled task → creates next instance

### Output

```
$ tatl finish
Task 42 completed: "Submit timesheet"
↻ Respawned as task 43, due: 2026-01-30

$ tatl show 43
Task 43: Submit timesheet
  Status: pending
  Due: 2026-01-30 17:00
  Respawn: monthdays:14,30
```

## Testing Scenarios

### Basic Respawn

```
Given: Task "Daily standup" with respawn:daily, due:2026-01-21 09:00
When: User runs "tatl finish"
Then: Task marked complete
And: New task created with due:2026-01-22 09:00
And: New task has respawn:daily
```

### Late Completion

```
Given: Task "Timesheet" with respawn:monthdays:14,30, due:2026-01-14
When: Completed on 2026-01-31
Then: New task created with due:2026-02-14 (next valid date)
```

### Weekday Pattern

```
Given: Task "Team sync" with respawn:weekdays:mon,wed,fri, due:2026-01-20 (Mon)
When: Completed on 2026-01-20
Then: New task created with due:2026-01-22 (Wed)
```

### Close Also Respawns

```
Given: Task with respawn:daily, due:2026-01-21
When: User runs "tatl close" (abandons without completing)
Then: New task created (obligation persists)
```

### Delete Ends Chain

```
Given: Task with respawn:daily
When: User runs "tatl delete"
Then: Task deleted
And: No respawn occurs (chain ended)
```

## Migration Path

### For Existing Users

1. `recur` field renamed to `respawn`
2. Old values remain valid (same syntax)
3. `recur_occurrences` table dropped
4. Behavior change: respawn on completion, not time-based generation

### Backward Compatibility

- Field parser accepts both `recur:` and `respawn:` initially
- Deprecation warning for `recur:` usage
- Remove `recur:` support in future release

## Open Questions

### Question 1: What about appointment-style recurrence?

If someone truly needs "create 10 instances of this meeting for the next 10 weeks," the respawn model doesn't serve this.

**Options:**
- A. Don't support - use external calendar
- B. Add separate `schedule:` field for pre-generating instances
- C. Provide bulk-add command: `tatl add "Meeting" --repeat 10 --every week`

**Initial recommendation:** A - Focus on task semantics, point to calendar for appointments

### Question 2: Should respawn happen immediately on finish?

Or should user confirm?

**Options:**
- A. Automatic (current plan)
- B. Prompt: "Create next instance due X? [Y/n]"
- C. Flag: `tatl finish --no-respawn`

**Initial recommendation:** A with C fallback - Automatic by default, `--no-respawn` to suppress

### Question 3: Respawn with or without enqueue?

When respawn creates new task, should it:
- A. Just create (no queue manipulation)
- B. Enqueue to bottom of stack
- C. Match original's queue position

**Initial recommendation:** A - Just create, user enqueues if wanted

## Success Criteria

1. ✅ `recur` field renamed to `respawn` throughout
2. ✅ `recur_occurrences` table removed
3. ✅ `tatl recur run` command removed
4. ✅ Respawn triggers on `finish`
5. ✅ Respawn triggers on `close`
6. ✅ Basic patterns work: daily, weekly, monthly, yearly, every:Nd/Nw/Nm/Ny
7. ✅ Weekday patterns work: weekdays:mon,wed,fri
8. ✅ Monthday patterns work: monthdays:1,15
9. ✅ Nth weekday patterns work: nth:1:mon
10. ✅ Late completion finds next future date correctly
11. ✅ Time-of-day preserved in due date
12. ✅ All attributes except status/due/id carried forward
13. ✅ Tests cover all patterns and edge cases
14. ✅ Documentation updated

## Appendix: Current vs. Proposed Comparison

| Aspect | Current (Recur) | Proposed (Respawn) |
|--------|-----------------|-------------------|
| Trigger | `tatl recur run` command | `tatl finish` or `tatl close` |
| Generation | Time-based, batch | Event-based, single |
| Active instances | Many (pre-generated) | One at a time |
| Missed deadlines | Pile up as separate tasks | Single task persists |
| Table | `recur_occurrences` | None needed |
| Mental model | Calendar appointments | Task obligations |

## Next Steps

~~1. [ ] Review and approve this plan~~
~~2. [ ] Decide on open questions~~
~~3. [ ] Begin Phase 1 implementation~~
~~4. [ ] Iterate through phases with testing~~

## Implementation Complete

All phases have been implemented:

### Phase 1: Rename and Refactor ✅
- Database migration v5 added: renames `recur` column to `respawn`, drops `recur_occurrences` table
- Model updated: `Task.recur` → `Task.respawn`
- Module renamed: `src/recur/` → `src/respawn/`
- CLI field: `recur:` → `respawn:`
- Removed `tatl recur run` command

### Phase 2: Respawn Logic ✅
- Implemented `next_occurrence()` for all patterns
- Implemented `respawn_task()` function
- Integrated with `finish` and `close` handlers

### Phase 3: Advanced Patterns ✅
- Daily, weekly, monthly, yearly patterns
- Interval patterns: `every:Nd`, `every:Nw`, `every:Nm`, `every:Ny`
- Weekday patterns: `weekdays:mon,wed,fri`
- Monthday patterns: `monthdays:1,15`
- Nth weekday patterns: `nth:2:tue`

### Phase 4: Testing ✅
- Updated all tests to use new `respawn:` field
- Created new respawn tests for on-completion behavior
- Removed old recurrence batch tests

### Version
Version bumped to 0.3.0

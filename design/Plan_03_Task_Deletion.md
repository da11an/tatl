## Task Deletion Feature - Planning Document

This document specifies the addition of task deletion functionality to Task Ninja.

---

### Current Behavior

- Tasks can be marked as completed using `task done`
- Completed tasks remain in the database with status = "completed"
- No mechanism exists to permanently delete tasks
- Stack operations can remove tasks from the stack, but don't delete the task itself

---

### Proposed Behavior

Add a `task delete` command that permanently removes tasks from the database.

**Command Syntax:**
```
task [<id|filter>] delete [--yes] [--interactive]
```

**Behavior:**
1. If `<id|filter>` provided: delete the specified task(s)
2. If `<id|filter>` omitted: error (deletion requires explicit specification)
3. Support filters for bulk deletion (e.g., `task project:test delete`)
4. Remove task and all related data:
   - Task record
   - Task tags
   - Task annotations
   - Task sessions
   - Stack items referencing the task
   - Task events
   - Recurrence occurrences
5. Safety features:
   - Require confirmation by default (unless `--yes` flag)
   - `--interactive` flag for one-by-one confirmation
   - Show task description in confirmation prompts

**Options:**
- `--yes` - Delete all matching tasks without confirmation
- `--interactive` - Confirm each task one by one

---

### Decisions Made

1. **Command name:** `delete` (clear and unambiguous)

2. **Safety:**
   - Default behavior: require confirmation
   - `--yes` flag: skip confirmation (for scripts/automation)
   - `--interactive` flag: confirm each task individually (for bulk operations)

3. **Filter support:**
   - Support both `<id>` and `<filter>` patterns
   - Follow same pattern as `task done` and `task modify`
   - Support `task <filter> delete` and `task delete <filter>` (backward compatibility)

4. **Data cleanup:**
   - Delete all related data in a transaction (atomic operation)
   - Remove from stack if present
   - Delete all sessions for the task
   - Delete all annotations for the task
   - Delete all events for the task
   - Delete all recurrence occurrences for the task
   - Delete all tags for the task

5. **Error handling:**
   - If task not found: show error and continue with other tasks (for filters)
   - If deletion fails: rollback transaction and show error

6. **Output:**
   - Show confirmation prompt with task ID and description
   - After deletion: "Deleted task <id>: <description>"
   - For bulk operations: show count of deleted tasks

---

### Implementation Considerations

1. **Repository layer:**
   - Add `TaskRepo::delete(conn, task_id)` method
   - Use transaction to ensure atomicity
   - Delete in correct order (respecting foreign key constraints):
     1. Recurrence occurrences
     2. Task events
     3. Task annotations
     4. Task sessions
     5. Stack items
     6. Task tags
     7. Task record

2. **CLI layer:**
   - Add `Delete` variant to `Commands` enum
   - Add handler `handle_task_delete()`
   - Support both `task <id|filter> delete` and `task delete <id|filter>` patterns
   - Implement confirmation logic (default, --yes, --interactive)

3. **Filter support:**
   - Use existing filter parsing infrastructure
   - Support multi-task deletion with confirmation

4. **Testing:**
   - Test single task deletion
   - Test bulk deletion with filters
   - Test confirmation prompts
   - Test transaction rollback on error
   - Test related data cleanup

---

### Examples

```bash
# Delete single task (with confirmation)
task 5 delete

# Delete single task (without confirmation)
task 5 delete --yes

# Delete multiple tasks (with confirmation)
task project:test delete

# Delete multiple tasks (interactive confirmation)
task project:test delete --interactive

# Delete multiple tasks (no confirmation)
task project:test delete --yes

# Filter-before-command pattern
task 5 delete
task project:test delete
```

---

### Implementation Checklist

- [x] Add `TaskRepo::delete()` method with transaction support
- [x] Add deletion of related data (sessions, annotations, events, etc.) - handled by CASCADE
- [x] Add `Delete` command variant to CLI
- [x] Implement `handle_task_delete()` function
- [x] Add support for `task <id|filter> delete` pattern
- [x] Add support for `task delete <id|filter>` pattern (backward compatibility) - Works via clap
- [x] Tested: Both patterns work correctly
- [x] Implement confirmation prompts
- [x] Add `--yes` flag support
- [x] Add `--interactive` flag support
- [ ] Write tests for single task deletion
- [ ] Write tests for bulk deletion
- [ ] Write tests for confirmation logic
- [ ] Write tests for related data cleanup
- [ ] Update command reference documentation
- [x] Update abbreviation support (add "delete" to command list)

---

### Database Schema Considerations

The deletion must handle foreign key relationships:
- `recur_occurrences.task_id` → `tasks.id`
- `task_events.task_id` → `tasks.id`
- `task_annotations.task_id` → `tasks.id`
- `sessions.task_id` → `tasks.id`
- `stack_items.task_id` → `tasks.id`
- `task_tags.task_id` → `tasks.id`

All these should be deleted before the task record itself.

---

## Implementation Notes

- **Safety First:** Default to requiring confirmation
- **Atomicity:** Use transactions to ensure all-or-nothing deletion
- **Consistency:** Follow patterns established by `task done` and `task modify`
- **Testing:** Comprehensive tests for all deletion scenarios


### 1. Something like `task sessions add task:<task id> start:<time or datetime> end:<time or datetime> note:<note>` command for adding sessions that were not recorded. Allow positional arguments or with labels, note (annotate) being optional 

### 2. Migrate `task clock enqueue` to `task enqueue`. This makes more sense as a task operation than a clock operation, just like `task clock in` is a top level command.

### 3. Update clock command `task clock` help information. It's too verbose and note very helpful. Task clock already explains its subcommands. So we just need something like:

clock    start and stop timing or manage clock timing queue

### 4. Add Clock (or similarly titled) column to task list showing the amount of time elapsed on that task

### 5. In task list show Due as relative time

### 6. Fix --clock-in flag so that it works in the following cases:

- Clock running but adding new task
- Clock not running yet but adding new task

In either case, adding the flag should move the task to the top of the clock stack and toggle the clock in if not already

### 7. Build in derived statuses called "kanban" to task list views and for filtering

Proposed mapping:
| Kanban    | Status    | Clock stack      | Sessions list                  | Clock status |
| --------- | --------- | ---------------- | ------------------------------ | ------------ |
| proposed  | pending   | Not in stack     | Task id not in sessions list   | N/A          |
| paused    | pending   | Not in stack     | Task id in sessions list       | N/A          |
| queued    | pending   | Position > 0     | Task id not in sessions list   | N/A          |
| working   | pending   | Position > 0     | Task id in sessions list       | N/A          |
| NEXT      | pending   | Position = 0     | N/A                            | Out          |
| LIVE      | pending   | Position = 0     | (Task id in sessions list)     | In           |
| done      | completed | (ineligible)     | N/A                            | N/A          |

Status has two primative states: pending, completed. Now we will incorporate a "kanban" column that will be shown by default in the task list and will ideally be available for filtering.

### 8. Display priority column, and shrink allocation label to alloc (just in the table)
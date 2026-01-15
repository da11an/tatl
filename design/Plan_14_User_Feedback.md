

### 1. `task annotate <id> <note>` already implemented. Make <id> optional when clocked in, defaulting to the LIVE session.

### 3. Make yes default for task session modify:

$ task sessions modify 10 start:10:15
Modify session 10?
  Start: 2026-01-13 11:42:06 -> 2026-01-13 10:15:00
Are you sure? ([y]/n):
My design principle is to trust the user to be right, by default.

### 4. Add task info e.g. Modify session 9 (task 7: Group meeting)?

Current state:
Modify session 9?
  Start: 2026-01-13 11:15:00 -> 2026-01-13 09:09:00
  End: 2026-01-13 11:40:00 -> 2026-01-13 10:15:00
 
### 5a. `task list ...` Add support for sorting, filters, and grouping.

`task list sort:colA,colB,... group:Kanban,tag,...`

- Sort columns become like multi-indexes in pandas. Sorted by first first, second second, etc.
- Sort columns put first in the table.
- ID, Description is always after sort columns
- Group by columns directly follow Description and overlap the description field as follows, if ColA and ColB are sort fields and ColC and ColD are group fields:

ColA ColB ID Description ColC ColD ...
---------------------------------- ...
-------------------------Val1:Val1 ...
Val1 Val1 XX Description can split over labeling grouping divider line
Val2 Val2 XX Description for the next item
-----------------------------:Val2 ...
Val3 Val4 XX Description of the next item
-------------------------Val2:Val1 ...
...

### 5b. Aliases for custom list views

Support saving task list sorting, filters, and grouping under and alias:

syntax suggestions for alias:
- `task list <sorting, filters, grouping> --add-alias myview
- `task list <sorting, filters, grouping> alias:myview
usage
- `task list myview

### 5c. Extend same behavior to task sessions list

### 6. token abbreviation in filtering: allow unambiguous abbreviations (if it only matches a single token)

Current behavior:
(base) [princdr@brtuxwrkst03 ~]$ task list st:pending
Error: Filter parse error: Invalid filter token: st:pending
(base) [princdr@brtuxwrkst03 ~]$ task list stat:pending
Error: Filter parse error: Invalid filter token: stat:pending
(base) [princdr@brtuxwrkst03 ~]$ task list status:pending
[worked]

### 7. Parse/allow relative dates when specifying due dates:

Currrently only absolute dates are allowed, e.g.:
$ task add project:admin.networking Weekly catchup connection alloc:30m due:1week
Internal error: Failed to parse due date

### 8. Make done a subset of modify that also sets the status to completed

- Also allow status to be set in modify using status:blah if a valid status option -- all tokens should be settable here.
- Unrecognized tokens (limit syntax to no space token:value):
    - If not in quotes, and token fitting strict no-space syntax isn't a recognized token, give user dialog to cancel, or include token as (part of) description.

### 9. Allow new project entry in modify like in add with user dialog

### 10. alloc -> Alloc in task list table column title.

### 11. Priority -> Prior and before Alloc column in task list

### 12. `task status` include task description in Clock Status sections, not just id.

### 13. Modify Kanban to apply NEXT stage to task that is in position 1 if task pos 0 is LIVE. Updated logic:

Proposed mapping:
| Kanban    | Status    | Clock stack      | Sessions list                  | Clock status |
| --------- | --------- | ---------------- | ------------------------------ | ------------ |
| proposed  | pending   | Not in stack     | Task id not in sessions list   | N/A          |
| paused    | pending   | Not in stack     | Task id in sessions list       | N/A          |
| queued    | pending   | Position > 0     | Task id not in sessions list   | N/A          |
| working   | pending   | Position > 0     | Task id in sessions list       | N/A          |
| NEXT      | pending   | Position = 0     | N/A                            | Out          |
| NEXT      | pending   | Position = 1     | N/A                            | In           |
| LIVE      | pending   | Position = 0     | (Task id in sessions list)     | In           |
| done      | completed | (ineligible)     | N/A                            | N/A          |

### 14. Change task clock roll to task clock next syntax to match with Kanban labeling.

### 15. Close vs Complete task completion statuses.

Enable `task close <id>` -- this is to assign a new `closed` status instead of `completed`
Update syntax of `task done <id>` to `task finish <id>` to make it a verb, status still becomes `completed`. Drop done syntax.



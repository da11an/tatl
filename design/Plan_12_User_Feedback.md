

### 1. Implement tab completion rather than allowing abbreviations

Rationale: less mental work, more natural commandline expectations
Applies to: commands, subcommands, project names, filters, ...

### 2. `task clock list`

Drop `task clock show` alias. We want to have a tight syntax with one right way to do things (pythonic)
`task clock list` should include the same columns as the regular task list, but leading with the clock stack position column and sorted accordingly. It is too hard to remember what the items are without details.

### 3. `task list` should include more columns, like allocation


### 4. Apply filtering to `task sessions list <arguments>` similar to `task list <arguments>`


### 5. Allow project creation during task creation with confirmation

- Consider the same for tags
- This is a new [project|tag] <similar to ...>. Add new [project|tag]? y=Add <name> [project|tag] and apply, n=No but create task without, [c]=No and cancel task creation (cancel is default)

### 6. `task done <id>` if not running throws error. Need to be able to check off task even if not clocked in

### 7. `task clock in --task <id>` is verbose. Can we just have `task clock in <id>` Implications about ambiguity with clock stack ids?

### 8. Need syntax to clock in task on adding it in the same one liner command

### 9. Drop status lines from individual commands and provide a single dashboard or status command that provides all the most actionable information

- Clock status
- Top up to 3 tasks from clock stack
- Top priority up to 3 tasks from task list NOT on clock stack
- Session summary for day
- Number of tasks overdue, if none, when tasks will become overdue based on ...
- Etc.

### 10. `task list` provide multiple views as arguments, group by and sort by options

- Group by Project view (complex group by representing nestings)
- Group by Kanban-like stage, and allow marking stage, like reviewed, etc, if done is not binary
- Group by timeliness status (overdue, future due date, timely completion threatened based on allocated vs session duration vs daily moving average of sum of sessions (recency biased)
- Priority score organized view (develop priority score)
- Sort flags (choose a column), sorts within groups if grouped

### 11. Add a plot or show option for every list option

### 12. Support more statuses, and list by status

---

## Addendum: Version Management

### Version Command

Added `task --version` (or `task -V`) command to display the current version of the application.

**Implementation:**
- Version is read from `Cargo.toml` using `env!("CARGO_PKG_VERSION")`
- Automatically displayed via clap's built-in version handling
- Current version: **0.2.0**

**Version History:**
- **0.2.0** - User feedback improvements (Plan 12):
  - Enhanced `task clock list` with full task details
  - Added allocation column to `task list`
  - Applied filtering to `task sessions list`
  - Allowed `task done` without clock requirement
  - Simplified `task clock in` syntax (removed `--task` flag)
  - Added `--clock-in` flag to `task add`
  - Added interactive project creation during task creation
  - Added `--version` command
- **0.1.0** - Initial release 

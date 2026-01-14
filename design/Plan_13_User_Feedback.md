
### 1. Something like `task sessions add task:<task id> start:<time or datetime> end:<time or datetime> note:<note>` command for adding sessions that were not recorded. Allow positional arguments or with labels, note (annotate) being optional 

### 2. Migrate `task clock enqueue` to `task enqueue`. This makes more sense as a task operation than a clock operation, just like `task clock in` is a top level command.

### 3. Update clock command `task clock` help information. It's too verbose and note very helpful. Task clock already explains its subcommands. So we just need something like:

clock    start and stop timing or manage clock timing queue

### 4. Add Clock (or similarly titled) column to task list showing the amount of time elapsed on that task

### 5. In task list show Due as relative time
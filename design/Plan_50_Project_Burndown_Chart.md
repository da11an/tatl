# Plan 50: Project Burndown Chart

## Overview

Replace `tatl projects report` with a graphical burndown chart rendered in the terminal. The chart shows work remaining (below a baseline) and work completed (above the baseline) over time, using stacked bar characters with ANSI colors.

## Data Model

No schema changes needed. All data is derived from existing fields:

- **Task closure time**: `modified_ts` for tasks with status `closed` (the timestamp when the status change occurred). We use `task_events` table with `event_type = 'status_changed'` and `new_status` in payload for accurate timestamps.
- **Task creation time**: `created_ts`
- **Task cancellation time**: Same as closure — `task_events` with `status_changed` to `cancelled`. Cancelled tasks disappear from the chart at cancellation.
- **Session time**: `sessions.start_ts` and `sessions.end_ts` for actual time worked per bin.
- **Allocation**: `tasks.alloc_secs` for planned effort of open tasks.

## Chart Model

A horizontal baseline divides each vertical bar:

- **Above the baseline**: Completed work (tasks closed in that bin, or session time logged in that bin)
- **Below the baseline**: Remaining work (tasks open during that bin, or allocated time for open tasks)

Cancelled tasks vanish from both areas at the point of cancellation — they reduce the "below" count but don't add to "above."

### Reconstructing Historical State

For each time bin, determine which tasks were open:
- A task is **open in a bin** if `created_ts <= bin_end` AND (status is still `open` OR the task's closure/cancellation event timestamp > bin_start)
- A task is **completed in a bin** if its closure event falls within the bin (status_changed to `closed`)
- A task is **cancelled in a bin** if its cancellation event falls within the bin — it leaves both areas

For time metric:
- **Above**: Sum of session durations that overlap with the bin
- **Below**: Sum of `alloc_secs` for tasks open in that bin (tasks without allocation contribute 0)

## Command Interface

```
tatl projects report [PROJECT] [DATE_RANGE] [--bin day|week|month] [--metric tasks|time]
```

### Arguments

- `PROJECT` (optional): Filter to a specific project or subproject. If omitted, cumulative across all projects.
- `DATE_RANGE` (optional): Same interval syntax as `tatl sessions report` (e.g., `-30d`, `-30d..now`, `2024-01-01..2024-06-30`). Default: all time.
- `--bin`: Time bin size. `day`, `week` (default), or `month`.
- `--metric`: What to measure. `tasks` (default) = count of tasks. `time` = hours (sessions for completed, allocation for remaining).

### Examples

```
tatl projects report                        # all projects, weekly bins, task counts
tatl projects report work                   # project "work" only
tatl projects report -90d --bin month       # last 90 days, monthly bins
tatl projects report --metric time          # hours instead of counts
tatl projects report work -30d --bin day    # daily bars for "work" project, last 30 days
```

## Terminal Rendering

### Bar Characters

Use Unicode block elements for half-cell resolution:

| Character | Meaning |
|-----------|---------|
| `█` (U+2588) | Full block |
| `▄` (U+2584) | Lower half block |
| `▀` (U+2580) | Upper half block |
| ` ` | Empty |

### Colors (ANSI)

- **Above baseline (completed)**: Green (`\x1b[32m`)
- **Below baseline (remaining)**: Blue (`\x1b[34m`)
- **Baseline**: Rendered as `─` characters

### Layout

```
     4 │          ██
     3 │       █  ██  █
     2 │    █  █  ██  ██  █
     1 │ █  █  █  ██  ██  ██
     ──┼──────────────────────
     1 │ ██ ██ ██ ██ █  █
     2 │ ██ ██ ██ ██ █
     3 │ ██ ██ ██ █
     4 │ ██ ██ ██
     5 │ ██ ██
       └──────────────────────
        W1  W2  W3  W4  W5  W6
```

- Y-axis: count (tasks) or hours (time metric)
- X-axis: time bins with short labels
- Bar width: 2 characters per bin, 1 space gap between bars
- Chart height: adapts to data, capped at ~20 rows (10 above + 10 below baseline)
- Terminal width determines max number of bins displayed

### Scaling

If max value exceeds available rows (half the chart height), scale values so each row represents multiple units. Show scale on Y-axis.

### X-Axis Labels

- Day bins: `Mon`, `Tue`, ... or `1/5`, `1/6`, ...
- Week bins: `W01`, `W02`, ... or `Jan1`, `Jan8`, ...
- Month bins: `Jan`, `Feb`, ... or `J25`, `F25`, ...

### Summary Line

Below the chart, print a one-line summary:
```
Completed: 12 tasks (45h 30m)  |  Remaining: 8 tasks (32h 0m)  |  Period: 2024-01-01 to 2024-06-30
```

## Implementation

### Files to Modify

1. **`src/cli/commands.rs`**
   - Replace `handle_projects_report()` with new burndown implementation
   - Update `ProjectCommands::Report` to accept optional args (project, date range, bin, metric)

2. **`src/cli/output.rs`** (optional)
   - Add `get_terminal_height()` utility if needed for chart sizing

### Data Pipeline

1. Parse arguments (project filter, date range, bin size, metric)
2. Query all tasks (filtered by project if specified)
3. Query closure/cancellation events from `task_events` for those tasks
4. Query sessions for time metric
5. Build time bins across the date range
6. For each bin, compute above/below values
7. Scale values to fit terminal height
8. Render chart

### Bin Computation

For task counts:
```
for each bin [bin_start, bin_end]:
    above = count of tasks whose closure event falls in [bin_start, bin_end]
    below = count of tasks where created_ts <= bin_end AND
            (status == open OR closure/cancel event > bin_start)
            excluding cancelled tasks that were cancelled before bin_start
```

For time:
```
for each bin [bin_start, bin_end]:
    above = sum of session durations overlapping [bin_start, bin_end]
    below = sum of alloc_secs for tasks open in this bin
```

## Non-TTY Fallback

When output is piped (not a TTY), fall back to a simple text table showing bin labels with above/below counts — no ANSI colors or block characters.

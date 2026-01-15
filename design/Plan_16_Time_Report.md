# Plan 16: Time Report for Billing

## Overview

Implement a summary time report suitable for billing purposes. The report aggregates clocked time by project, supporting hierarchical project structures (dot notation), and displays both absolute time and percentages.

## Command Syntax

```bash
task sessions report [<start>] [<end>]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `start`  | No       | Start datetime for the reporting period. Supports date expressions (e.g., `2024-01-01`, `monday`, `1week-ago`). Defaults to earliest session. |
| `end`    | No       | End datetime for the reporting period. Defaults to now. |

### Examples

```bash
# All time
task sessions report

# This month
task sessions report 2024-01-01 2024-01-31

# Last week
task sessions report 1week-ago today

# Since a specific date
task sessions report 2024-01-15
```

## Report Structure

### Hierarchical Project Aggregation

Projects using dot notation (e.g., `client.projectA.frontend`) are treated hierarchically:

- `client` - top-level project (sum of all nested)
- `client.projectA` - sub-project (sum of its children)
- `client.projectA.frontend` - leaf project (direct sessions)

### Output Format

```
Time Report: 2024-01-01 to 2024-01-31
================================================================================

Project                              Time          %
--------------------------------------------------------------------------------
client                             45h 30m    75.0%
  projectA                         30h 00m    49.5%
    frontend                       18h 00m    29.7%
    backend                        12h 00m    19.8%
  projectB                         15h 30m    25.6%
internal                           12h 00m    19.8%
  meetings                          8h 00m    13.2%
  admin                             4h 00m     6.6%
(no project)                        3h 10m     5.2%
--------------------------------------------------------------------------------
TOTAL                              60h 40m   100.0%

Sessions: 127 | Period: 31 days
```

### Design Decisions

1. **Indentation**: 2 spaces per nesting level for visual hierarchy
2. **Time Format**: `Xh Ym` for readability (e.g., `45h 30m`)
3. **Percentage**: One decimal place, right-aligned
4. **No-project tasks**: Grouped under `(no project)` at the end
5. **Column alignment**: Project left-aligned, Time and % right-aligned
6. **Header/Footer**: Clear period indication and summary stats

### Edge Cases

| Case | Handling |
|------|----------|
| No sessions in period | Display "No sessions found for this period" |
| Sessions spanning period boundary | Include only the portion within the period |
| Running session | Include time up to `end` or now |
| Orphan sub-projects | If `a.b` exists but not `a`, create virtual parent `a` |

## Implementation Plan

### Phase 1: Core Report Logic

**File**: `src/cli/commands_sessions.rs`

1. Add `Report` variant to `SessionsCommands` enum:
   ```rust
   Report {
       #[arg(value_name = "START")]
       start: Option<String>,
       #[arg(value_name = "END")]
       end: Option<String>,
   }
   ```

2. Implement `handle_sessions_report(start: Option<String>, end: Option<String>)`:
   - Parse start/end datetime expressions
   - Query sessions within the period
   - Build project hierarchy tree
   - Calculate durations and percentages
   - Format and print report

### Phase 2: Project Hierarchy Tree

**Data Structure**:
```rust
struct ProjectNode {
    name: String,           // Just this segment (e.g., "frontend")
    full_path: String,      // Full path (e.g., "client.projectA.frontend")
    direct_time: Duration,  // Time from sessions directly on this project
    total_time: Duration,   // Direct + all children
    children: BTreeMap<String, ProjectNode>,
}
```

**Algorithm**:
1. Group sessions by project
2. For each project path, split by `.` and insert into tree
3. Bottom-up aggregation: sum child times into parent totals
4. Sort children alphabetically at each level

### Phase 3: Duration Calculation

**Session time within period**:
```rust
fn session_duration_in_period(session: &Session, start: i64, end: i64) -> Duration {
    let session_start = session.start_ts.max(start);
    let session_end = session.end_ts.unwrap_or(now()).min(end);
    
    if session_start >= session_end {
        Duration::zero()
    } else {
        Duration::seconds(session_end - session_start)
    }
}
```

### Phase 4: Output Formatting

**Helper functions**:
```rust
fn format_duration_hm(duration: Duration) -> String {
    let hours = duration.num_hours();
    let minutes = duration.num_minutes() % 60;
    format!("{}h {:02}m", hours, minutes)
}

fn format_percentage(part: Duration, total: Duration) -> String {
    if total.is_zero() {
        "0.0%".to_string()
    } else {
        let pct = (part.num_seconds() as f64 / total.num_seconds() as f64) * 100.0;
        format!("{:.1}%", pct)
    }
}
```

**Table formatting**:
- Project column: 40 chars, left-aligned with indentation
- Time column: 12 chars, right-aligned
- Percent column: 8 chars, right-aligned

## Test Cases

### Unit Tests

1. `test_duration_in_period_fully_inside` - Session fully within period
2. `test_duration_in_period_spans_start` - Session starts before period
3. `test_duration_in_period_spans_end` - Session ends after period
4. `test_duration_in_period_spans_both` - Session spans entire period
5. `test_duration_in_period_outside` - Session outside period (returns 0)
6. `test_running_session_capped_at_end` - Running session capped at end time

### Integration Tests

1. `test_report_all_time` - Report with no date arguments
2. `test_report_date_range` - Report with start and end
3. `test_report_nested_projects` - Verify hierarchy aggregation
4. `test_report_no_project_tasks` - Tasks without projects grouped
5. `test_report_empty_period` - No sessions message
6. `test_report_percentages_sum_to_100` - Verify math

### Acceptance Tests

```bash
# Setup: Create tasks with projects and log time
task add "Frontend work" project:client.projectA.frontend
task clock in 1
# ... log some time ...

# Verify report output
task sessions report 2024-01-01 2024-01-31
# Should show hierarchical breakdown
```

## Future Work

1. **Tag filtering**: `task sessions report --tag billable`
2. **Project filtering**: `task sessions report --project client`
3. **Export formats**: `--format csv`, `--format json`
4. **Weekly breakdown**: `--weekly` to show per-week subtotals
5. **Comparison reports**: Compare two periods side-by-side
6. **Rate calculation**: `--rate 150` to show billable amounts

## Estimates

| Task | Estimate |
|------|----------|
| Command parsing & routing | 30 min |
| Session querying with date filter | 30 min |
| Project hierarchy tree building | 1 hour |
| Duration calculations | 30 min |
| Report formatting | 1 hour |
| Tests | 1 hour |
| **Total** | **4.5 hours** |

## Files to Modify

- `src/cli/commands_sessions.rs` - Add Report command and handler
- `src/cli/abbrev.rs` - Add "report" to SESSION_COMMANDS
- `tests/sessions_tests.rs` - Add report tests

## Dependencies

- Existing date parsing: `utils/date.rs` - `parse_date_expr()`
- Existing session repo: `repo/session.rs` - May need new query method
- Existing project repo: `repo/project.rs` - For project name lookup

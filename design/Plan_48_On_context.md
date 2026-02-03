# Plan 48: Task Context on `tatl on`

## Overview

When a user starts timing a task (`tatl on`), show a brief context block so they can orient themselves. Currently the output is just:

```
Started timing task 5: Fix bug (14:30)
```

After this change:

```
Started timing task 5: Fix the authentication timeout bug (14:30)
  - Check if the token refresh is being called before expiry
  - Tried extending TTL to 30min, still fails intermittently
  - Look at the retry logic in auth_middleware.rs
  Timer: 2h 15m / 4h 0m [===========-------] 56%
```

## Design

### Output Format

After the existing "Started timing..." line, print:

1. **Annotations** (if any) -- bulleted list, no timestamps, most recent last
2. **Progress bar** (if allocation exists) -- `Timer: <logged> / <alloc> [====----] NN%`
   - If no allocation: `Timer: <logged>` (no bar, just total logged)
   - If no logged time and no allocation: omit entirely

### Progress Bar

Fixed 20-char bar using `=` for filled and `-` for remaining:

```
Timer: 1h 30m / 4h 0m [=======-------------] 37%
Timer: 4h 30m / 4h 0m [====================] 112%
```

When over 100%, show the percentage but cap the bar fill at 20 chars.

## Files to Modify

### 1. `src/cli/output.rs`

Add `format_on_context()` function.

### 2. `src/cli/commands.rs`

Call `format_on_context()` from `handle_task_on()` and `handle_on_queue_top()` after "Started timing" output. Not for interval recordings.

## Verification

1. `cargo build` and `cargo test`
2. Manual: `tatl on` with various annotation/allocation combinations

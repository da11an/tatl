# Plan 38: `--on` Flag Syntax Consistency

## Problem Statement

The `--on` flag has inconsistent behavior across commands and inconsistent syntax compared to other flags.

A user would reasonably expect `tatl add --on 09:00 "Fix bug"` to work, but it fails:

```
$ tatl add --on 09:00 "Fix bug"
Error: Unrecognized field name '09'
  Did you mean 'due'?
```

The `09:00` is not consumed by `--on` and instead falls through to field parsing, where `09:` is misinterpreted as a field name.

---

## Current Behavior (Test Results)

### `tatl add` with `--on`

| Syntax | Result | Notes |
|--------|--------|-------|
| `add --on "task"` | OK | Bare flag, starts at now |
| `add --on=09:00 "task"` | OK | Equals syntax works |
| `add --on 09:00 "task"` | FAIL | `09:00` parsed as field token |
| `add "task" --on` | OK | Trailing bare flag (manual parsing) |
| `add "task" --on=09:00` | OK | Trailing equals (manual parsing) |
| `add "task" --on 09:00` | FAIL | `09:00` parsed as field token |

### `tatl add` with `--onoff`

| Syntax | Result | Notes |
|--------|--------|-------|
| `add "task" --onoff 09:00..10:00` | OK | Space syntax works |
| `add "task" --onoff=09:00..10:00` | OK | Equals syntax works |

### `tatl on` (standalone command)

| Syntax | Result | Notes |
|--------|--------|-------|
| `on 1 09:00` | OK | Positional args, space syntax |
| `on 09:00` | OK | Time only, uses queue[0] |

### `tatl modify` with `--on`

| Syntax | Result | Notes |
|--------|--------|-------|
| `modify 1 +urgent --on` | OK | Boolean flag only |
| `modify 1 --on=09:00` | FAIL | `unexpected value '09:00' for '--on'` |

---

## Root Causes

### 1. `require_equals = true` on `add --on`

The clap definition for `--on` in the `Add` command uses `require_equals = true`:

```rust
#[arg(long = "on", num_args = 0..=1, require_equals = true, default_missing_value = "")]
start_timing: Option<String>,
```

This forces `--on=<time>` (equals) and rejects `--on <time>` (space). This was done to disambiguate `--on "Fix bug"` (bare flag + description) from `--on 09:00` (flag + time).

### 2. `--on` in `modify` is boolean, not optional-value

```rust
// In Add:  Option<String> with optional value
start_timing: Option<String>,

// In Modify:  bool with no value
start_timing: bool,
```

Users cannot specify a start time with `modify --on`. They can only start timing at the current time.

### 3. Manual parsing inconsistency

The manual extraction code (for trailing flags after `trailing_var_arg`) handles `--onoff` with space syntax but NOT `--on`:

```rust
// --on: only bare and equals
if args[i] == "--on" {
    start_timing = Some(String::new());
} else if args[i].starts_with("--on=") {
    start_timing = Some(args[i][eq_pos + 1..].to_string());
}

// --onoff: space AND equals
if args[i] == "--onoff" {
    onoff_interval = Some(args[i + 1].clone());  // <-- consumes next arg
} else if args[i].starts_with("--onoff=") {
    onoff_interval = Some(args[i][8..].to_string());
}
```

### 4. `tatl on` uses positional args, `add --on` uses optional flag value

The standalone `on` command takes time as a positional argument, so `tatl on 09:00` works naturally. But `add --on` tries to encode the same concept as an optional flag value, creating the ambiguity.

---

## Proposed Fix

### Option A: Add space syntax support to manual parsing (Recommended)

Keep `require_equals = true` in the clap definition (needed for the ambiguity when `--on` precedes the description), but add space syntax support in the manual extraction code for trailing position.

**Changes:**

1. **Update manual parsing in `handle_task_add`** to handle `--on <time>` when `--on` appears in trailing args:

```rust
if args[i] == "--on" || args[i] == "--clock-in" {
    // Check if next arg looks like a time expression
    if i + 1 < args.len() && looks_like_time_expr(&args[i + 1]) {
        start_timing = Some(args[i + 1].clone());
        i += 1; // consume time arg
    } else {
        start_timing = Some(String::new());
    }
}
```

2. **Add `looks_like_time_expr` helper** that checks whether a string matches common time patterns (HH:MM, YYYY-MM-DD, relative expressions like +3d, etc.) without fully parsing. This disambiguates `--on "Fix bug"` (not a time) from `--on 09:00` (a time).

3. **Update `modify --on`** to accept optional time value (change from `bool` to `Option<String>`) for consistency with `add --on`.

4. **Update help text** to document both syntaxes: `--on` (start now) and `--on <time>` or `--on=<time>` (start at time).

**Pros:**
- Space syntax works in trailing position (most common usage)
- No breaking changes to existing equals syntax
- Consistent with `--onoff` behavior
- The `require_equals` in clap still protects the leading position from ambiguity

**Cons:**
- `--on 09:00` still won't work in leading position (before description), only `--on=09:00`
- Adds a time-expression heuristic function

### Option B: Split into `--on` and `--at`

Remove the optional time value from `--on` entirely. Use a separate `--at <time>` flag for specifying start time.

```
tatl add --on "Fix bug"              # Start timing now
tatl add --on --at 09:00 "Fix bug"   # Start timing at 09:00
tatl add --at 09:00 "Fix bug"        # Implies --on, start at 09:00
```

**Pros:**
- No ambiguity, no heuristics needed
- Clear separation of concerns
- Both leading and trailing positions work consistently

**Cons:**
- Breaking change for `--on=<time>` users
- More flags to remember
- `--at` alone implying `--on` might be surprising

### Option C: Documentation only

Keep current behavior, update documentation to clearly explain:
- `--on` starts timing at current time
- `--on=<time>` starts timing at specified time (equals sign required)
- `--on <time>` does NOT work (explain why)

**Pros:**
- No code changes
- No risk of regressions

**Cons:**
- Doesn't fix the underlying UX issue
- Users will keep trying `--on <time>` and getting confused

---

## Decision

<!-- Choose A, B, or C -->

---

## Additional Notes

- The pre-existing test `test_add_with_on_equals_time()` in `tests/add_clock_in_tests.rs` tests `--on=14:00` syntax and should continue to pass with any option.
- The `--clock-in` alias should follow the same pattern as `--on`.
- The `tatl on <time>` standalone command is already consistent (uses positional args) and doesn't need changes.

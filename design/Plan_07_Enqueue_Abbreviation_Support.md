## Enqueue Command Abbreviation Support - Planning Document

This document explores subcommand abbreviation support and outlines a plan to implement abbreviation support for the `enqueue` command and similar task subcommands.

---

### Current Behavior

**Subcommand Abbreviations ARE Supported:**
- Top-level commands with Clap subcommands support abbreviations:
  - `task proj ad test` → `task projects add test` ✓
  - `task st sh` → `task stack show` ✓
  - `task cl in` → `task clock in` ✓

**Task Subcommand Abbreviations ARE NOT Supported:**
- Commands using the `task <id> <subcommand>` pattern do NOT support abbreviations:
  - `task 1 enq` → Error: unrecognized subcommand ✗
  - `task 1 mod +urgent` → Error: unrecognized subcommand ✗
  - `task 1 don` → Error: unrecognized subcommand ✗
  - `task 1 del` → Error: unrecognized subcommand ✗

**Why This Happens:**
1. The abbreviation system (`src/cli/abbrev.rs`) only expands commands that are in `TOP_LEVEL_COMMANDS` or their registered subcommands.
2. Commands like `enqueue`, `modify`, `done`, `delete`, `annotate`, `summary` when used as `task <id> <subcommand>` are handled via **pre-clap parsing** in `src/cli/commands.rs`.
3. The pre-clap parsing checks for **exact string matches** (e.g., `a == "enqueue"`), not abbreviations.
4. Abbreviation expansion happens **before** pre-clap parsing, but it only expands top-level commands, not task subcommands.

---

### Proposed Behavior

Add abbreviation support for task subcommands that follow the `task <id> <subcommand>` pattern:

**Supported Abbreviations:**
- `task 1 enq` → `task 1 enqueue` ✓
- `task 1 enque` → `task 1 enqueue` ✓
- `task 1 mod` → `task 1 modify` ✓
- `task 1 modi` → `task 1 modify` ✓
- `task 1 don` → `task 1 done` ✓
- `task 1 del` → `task 1 delete` ✓
- `task 1 del` → `task 1 delete` ✓
- `task 1 ann` → `task 1 annotate` ✓
- `task 1 sum` → `task 1 summary` ✓

**Ambiguous Abbreviations:**
- `task 1 d` → Error: Ambiguous (matches "done", "delete")
- `task 1 de` → Error: Ambiguous (matches "done", "delete")
- `task 1 del` → `task 1 delete` (unambiguous)

**Consistency:**
- Same abbreviation rules as top-level commands: unambiguous prefix match is accepted, ambiguous match shows error with suggestions.

---

### Analysis: Current Abbreviation System

**How It Works:**
1. `expand_command_abbreviations()` is called in `run()` before any parsing.
2. It checks if `args[0]` is a top-level command (not a flag, not a number).
3. If it matches a top-level command, it expands it and checks for subcommands.
4. For commands with registered subcommands (via `get_subcommands()`), it expands the next arg as a subcommand.

**Limitations:**
1. Only expands `args[0]` as a top-level command.
2. Only expands subcommands for registered top-level commands (projects, stack, clock, recur, sessions).
3. Does not handle the pattern `task <id> <subcommand>` where the first arg is a number.

**Task Subcommands Pattern:**
- `task <id> enqueue` - handled via pre-clap parsing
- `task <id> modify` - handled via pre-clap parsing
- `task <id> done` - handled via pre-clap parsing
- `task <id> delete` - handled via pre-clap parsing
- `task <id> annotate` - handled via pre-clap parsing
- `task <id> summary` - handled via pre-clap parsing
- `task <id> clock in` - handled via pre-clap parsing
- `task <id> sessions list` - handled via pre-clap parsing

---

### Implementation Plan

#### Step 1: Define Task Subcommands List

Add a new constant in `src/cli/abbrev.rs`:

```rust
/// Task subcommands (used with task <id> <subcommand> pattern)
pub const TASK_SUBCOMMANDS: &[&str] = &[
    "enqueue", "modify", "done", "delete", "annotate", "summary"
];
```

**Note:** `clock` and `sessions` are already top-level commands, so they're handled differently.

#### Step 2: Enhance Abbreviation Expansion

Modify `expand_command_abbreviations()` in `src/cli/abbrev.rs` to handle task subcommands:

**New Logic:**
1. After expanding top-level commands, check if `args[0]` is a number (task ID).
2. If `args[0]` is a number and `args.len() >= 2`, check if `args[1]` matches a task subcommand.
3. Use `find_unique_command()` to expand the subcommand abbreviation.
4. If ambiguous, return an error with suggestions.
5. If no match, pass through (might be a filter or other pattern).

**Implementation:**
```rust
// After top-level command expansion, check for task subcommands
if i == 0 && args[i].parse::<i64>().is_ok() {
    // First arg is a number (task ID)
    if i + 1 < args.len() {
        let next_arg = &args[i + 1];
        // Check if next arg is a task subcommand (not a flag)
        if !next_arg.starts_with('-') {
            match find_unique_command(next_arg, TASK_SUBCOMMANDS) {
                Ok(full_subcmd) => {
                    expanded.push(args[i].clone()); // Keep task ID
                    expanded.push(full_subcmd.to_string());
                    i += 2;
                    continue;
                }
                Err(matches) => {
                    if matches.is_empty() {
                        // No match - might be a filter or other pattern, pass through
                        expanded.push(args[i].clone());
                        i += 1;
                        continue;
                    } else {
                        // Ambiguous subcommand
                        let match_list = matches.join(", ");
                        return Err(format!(
                            "Ambiguous task subcommand '{}'. Did you mean one of: {}?",
                            next_arg, match_list
                        ));
                    }
                }
            }
        }
    }
}
```

#### Step 3: Update Pre-Clap Parsing

The pre-clap parsing in `src/cli/commands.rs` already checks for exact string matches. Since abbreviation expansion happens **before** pre-clap parsing, the expanded command names will be used automatically.

**No changes needed** - the existing code will work because:
- `expand_command_abbreviations()` runs first (line 258)
- Pre-clap parsing checks for "enqueue", "modify", etc. (lines 347, 406, etc.)
- If "enq" is expanded to "enqueue", the pre-clap parsing will find it

#### Step 4: Handle Special Cases

**Case 1: `task <id> clock in`**
- This is already handled via pre-clap parsing (line 331-343).
- Abbreviation expansion should handle "cl" → "clock" for the top-level command.
- The "in" subcommand is handled by Clap's ClockCommands enum.

**Case 2: `task <id> sessions list`**
- Similar to clock, handled via pre-clap parsing.
- Abbreviation expansion should handle "ses" → "sessions" for the top-level command.

**Case 3: Ambiguous Abbreviations**
- `task 1 d` → Should error: "Ambiguous task subcommand 'd'. Did you mean one of: done, delete?"
- `task 1 de` → Should error: "Ambiguous task subcommand 'de'. Did you mean one of: done, delete?"
- `task 1 del` → Should expand to "delete" (unambiguous)

#### Step 5: Testing

Create comprehensive tests in `tests/enqueue_tests.rs` or a new `tests/abbreviation_tests.rs`:

```rust
#[test]
fn test_enqueue_abbreviation() {
    // Test various abbreviations
    assert_eq!(expand_task_subcommand("enq"), Ok("enqueue"));
    assert_eq!(expand_task_subcommand("enque"), Ok("enqueue"));
    assert_eq!(expand_task_subcommand("enqueue"), Ok("enqueue"));
}

#[test]
fn test_ambiguous_task_subcommand() {
    // Test ambiguous abbreviations
    let result = expand_task_subcommand("d");
    assert!(result.is_err());
    // Should suggest "done, delete"
}

#[test]
fn test_task_enqueue_abbreviation_integration() {
    // Test full command with abbreviation
    let _temp_dir = setup_test_env();
    let mut cmd = get_task_cmd();
    cmd.args(&["add", "Test task"]).assert().success();
    
    // Test abbreviation
    let mut cmd = get_task_cmd();
    cmd.args(&["1", "enq"]).assert().success();
    
    // Verify it worked
    let mut cmd = get_task_cmd();
    cmd.args(&["stack", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
}
```

---

### Implementation Checklist

- [x] Add `TASK_SUBCOMMANDS` constant to `src/cli/abbrev.rs`
- [x] Modify `expand_command_abbreviations()` to handle task subcommands when first arg is a number
- [x] Test abbreviation expansion for all task subcommands:
  - [x] `enqueue` (enq, enqu, enque, enqueu, enqueue)
  - [x] `modify` (mod, modi, modif, modify)
  - [x] `done` (don, done)
  - [x] `delete` (del, dele, delet, delete)
  - [x] `annotate` (ann, anno, annot, annota, annotat, annotate)
  - [x] `summary` (sum, summ, summa, summar, summary)
- [x] Test ambiguous abbreviations (d, de → done, delete)
- [x] Test integration with pre-clap parsing (verify expanded commands work)
- [x] Write unit tests for abbreviation expansion
- [ ] Write integration tests for `task <id> <abbrev>` commands
- [ ] Update documentation if needed
- [x] Test edge cases (empty string, very short abbreviations, etc.)

---

### Design Decisions

1. **Abbreviation Rules:**
   - Same as top-level commands: unambiguous prefix match is accepted.
   - Ambiguous prefix match shows error with suggestions.
   - Case-insensitive matching.

2. **Task Subcommands List:**
   - Only includes commands that use the `task <id> <subcommand>` pattern.
   - Excludes `clock` and `sessions` (they're top-level commands with their own subcommands).

3. **Integration Point:**
   - Abbreviation expansion happens in `expand_command_abbreviations()` before pre-clap parsing.
   - Pre-clap parsing continues to work with exact string matches (now expanded).

4. **Error Messages:**
   - Ambiguous abbreviations show: "Ambiguous task subcommand 'X'. Did you mean one of: Y, Z?"
   - Consistent with existing abbreviation error messages.

---

### Examples

**Before (No Abbreviation Support):**
```bash
$ task 1 enq
error: unrecognized subcommand '1'

$ task 1 enqueue
Enqueued task 1
```

**After (With Abbreviation Support):**
```bash
$ task 1 enq
Enqueued task 1

$ task 1 enque
Enqueued task 1

$ task 1 d
Error: Ambiguous task subcommand 'd'. Did you mean one of: done, delete?

$ task 1 del
Deleted task 1
```

---

### Implementation Notes

- **Backward Compatibility:** Full command names continue to work exactly as before.
- **Consistency:** Task subcommand abbreviations follow the same rules as top-level command abbreviations.
- **Performance:** Minimal overhead - abbreviation expansion is a simple prefix match.
- **Maintainability:** Task subcommands are defined in one place (`TASK_SUBCOMMANDS` constant).

---

### Future Considerations

1. **Filter-Before-Command Pattern:**
   - Currently, `task <filter> list` works, but `task <filter> enq` would need special handling.
   - This is out of scope for this plan but could be added later.

2. **ID Ranges and Lists:**
   - `task 1-3 enq` should work after abbreviation expansion.
   - The existing `parse_task_id_spec()` should handle this.

3. **Configuration:**
   - Could add a config option to disable abbreviations (deferred from Plan 02).

---

## Summary

Subcommand abbreviations ARE supported for Clap-structured commands (projects, stack, clock, etc.), but NOT for task subcommands that use the `task <id> <subcommand>` pattern. This plan adds abbreviation support by:

1. Defining a `TASK_SUBCOMMANDS` list
2. Enhancing `expand_command_abbreviations()` to detect task ID patterns and expand subcommands
3. Leveraging existing pre-clap parsing (no changes needed)
4. Maintaining consistency with existing abbreviation rules

The implementation is straightforward and maintains backward compatibility while adding the requested feature.

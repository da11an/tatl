## Task ID Ranges and Lists - Planning Document

This document specifies the addition of task ID ranges and comma-separated lists to Task Ninja.

---

### Current Behavior

- Single task IDs: `task 5 delete`, `task 2 modify`
- Filter expressions: `task project:work delete`, `task +urgent list`
- OR syntax: `task id:2 or id:3 or id:4 list` (works, but verbose)

**Not Supported:**
- Ranges: `task 2-4 delete` → Error: Invalid filter token
- Lists: `task 2,3,4 delete` → Error: Invalid filter token

---

### Proposed Behavior

Add support for task ID ranges and comma-separated lists in commands that accept task IDs.

**Syntax:**
- **Ranges:** `2-4` (inclusive, expands to 2, 3, 4)
- **Lists:** `2,3,4` (comma-separated IDs)
- **Combined:** `2,5-7,10` (mix of lists and ranges)

**Commands to Support:**
- `task <id|range|list> delete`
- `task <id|range|list> modify`
- `task <id|range|list> done`
- `task <id|range|list> list` (for listing specific tasks)
- `task <id|range|list> annotate`
- `task <id|range|list> sessions`
- Any other command that accepts task IDs

**Examples:**
```bash
# Delete tasks 2, 3, 4
task 2-4 delete --yes

# Delete tasks 2, 5, 6, 7, 10
task 2,5-7,10 delete --yes

# Modify tasks 1, 3, 5
task 1,3,5 modify +urgent

# List tasks 2-4
task 2-4 list

# Complete tasks 1-3
task 1-3 done --yes
```

---

### Decisions Made

1. **Range syntax:**
   - `2-4` means inclusive range (2, 3, 4)
   - Single number is valid (`5-5` = `5`)
   - Reverse ranges (`4-2`) should be handled (expand to 2, 3, 4)

2. **List syntax:**
   - Comma-separated: `2,3,4`
   - Can mix with ranges: `2,5-7,10`
   - Spaces around commas optional: `2, 3, 4` or `2,3,4`

3. **Parsing priority:**
   - Try to parse as ID range/list first
   - If that fails, fall back to existing behavior (single ID or filter)
   - This maintains backward compatibility

4. **Validation:**
   - All IDs in range/list must be positive integers
   - Invalid IDs in list should show clear error
   - Empty ranges/lists should error

5. **Integration:**
   - Add parsing function: `parse_task_id_spec()` → `Vec<i64>`
   - Update `validate_task_id()` or create new function
   - Integrate into handlers that accept task IDs

---

### Implementation Considerations

1. **Parsing function:**
   - Create `parse_task_id_spec(spec: &str) -> Result<Vec<i64>, String>`
   - Handle ranges: `2-4` → `[2, 3, 4]`
   - Handle lists: `2,3,4` → `[2, 3, 4]`
   - Handle combined: `2,5-7,10` → `[2, 5, 6, 7, 10]`
   - Handle reverse ranges: `4-2` → `[2, 3, 4]` (sorted)

2. **Integration points:**
   - Update handlers that use `validate_task_id()`:
     - `handle_task_delete()`
     - `handle_task_modify()`
     - `handle_task_done()`
     - `handle_task_list()` (for bare numeric IDs)
     - `handle_annotation_add()` (with task ID)
     - `handle_task_sessions_*()` (with task ID)
   - Try parsing as ID spec first, then fall back to filter

3. **Error handling:**
   - Clear error messages for invalid ranges/lists
   - Example: "Invalid task ID range: '2-x'. Range must be two numbers separated by '-'."
   - Example: "Invalid task ID in list: 'abc'. Task IDs must be numbers."

4. **Edge cases:**
   - Empty string
   - Single number (should work as before)
   - Reverse ranges (4-2)
   - Overlapping ranges/lists (deduplicate)
   - Very large ranges (performance consideration)

5. **Testing:**
   - Single ID (backward compatibility)
   - Simple range: `2-4`
   - Simple list: `2,3,4`
   - Combined: `2,5-7,10`
   - Reverse range: `4-2`
   - Invalid syntax
   - Empty ranges/lists

---

### Implementation Checklist

- [x] Create `parse_task_id_spec()` function
- [x] Handle range parsing (`2-4`)
- [x] Handle list parsing (`2,3,4`)
- [x] Handle combined parsing (`2,5-7,10`)
- [x] Handle reverse ranges (`4-2` → sorted)
- [x] Add deduplication for overlapping IDs
- [x] Update `handle_task_delete()` to use ID spec parsing
- [x] Update `handle_task_modify()` to use ID spec parsing
- [x] Update `handle_task_done()` to use ID spec parsing
- [x] Update `handle_task_list()` to use ID spec parsing (for bare numeric IDs)
- [ ] Update `handle_annotation_add()` to use ID spec parsing
- [ ] Update `handle_task_sessions_*()` to use ID spec parsing
- [x] Write tests for range parsing
- [x] Write tests for list parsing
- [x] Write tests for combined parsing
- [x] Write tests for edge cases
- [ ] Write integration tests for commands
- [ ] Update command reference documentation

---

### Examples

```bash
# Delete range
task 2-4 delete --yes

# Delete list
task 2,3,4 delete --yes

# Delete combined
task 1,5-7,10 delete --yes

# Modify range
task 2-4 modify +urgent

# List range
task 2-4 list

# Complete list
task 1,3,5 done --yes
```

---

## Implementation Notes

- **Backward Compatibility:** Single IDs must continue to work exactly as before
- **Performance:** Large ranges (e.g., `1-10000`) should be handled efficiently
- **User Experience:** Clear error messages for invalid syntax
- **Consistency:** Same syntax works across all commands that accept task IDs

---

## Outstanding Items

See `Outstanding_Items.md` for remaining tasks:
- Update `handle_annotation_add()` to use ID spec parsing
- Update `handle_task_sessions_*()` to use ID spec parsing
- Write integration tests for commands
- Update command reference documentation

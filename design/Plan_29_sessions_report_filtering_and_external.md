# Plan 29: Sessions Report Filtering and External Kanban Order

## Issues

1. **Sessions report project filtering**: `tatl sessions report project:pro1,pro2` returns no sessions. Comma-separated project values should be treated as OR (match sessions for tasks in pro1 OR pro2), not as a single project name.

2. **Kanban stage order**: External should come immediately after stalled. Current order: `proposed → stalled → queued → external → done`. Desired: `proposed → stalled → external → queued → done`.

---

## Analysis

### Issue 1: Project Filter Multi-Value Support

**Current State:**
- `FilterTerm::Project` is a single `String`
- `project:pro1,pro2` is parsed as project name "pro1,pro2" (literal comma)
- This matches no projects, resulting in no sessions

**Root Cause:**
- Project filter doesn't support comma-separated values like `kanban:` and `status:` do
- Sessions report uses the same filter parser as task list, so it inherits this limitation

**Solution:**
- Update `FilterTerm::Project` to support `Vec<String>` (like `Status` and `Kanban`)
- Parse comma-separated project names: `project:pro1,pro2` → `["pro1", "pro2"]`
- Update evaluator to match if task's project matches ANY of the provided project names (OR logic)
- This should work for both task list and sessions report

**Implementation:**
1. Change `FilterTerm::Project(String)` to `FilterTerm::Project(Vec<String>)`
2. Update parser to split on commas: `project:pro1,pro2` → `vec!["pro1", "pro2"]`
3. Update evaluator to check if task's project matches any value in the vector
4. Update all references to `FilterTerm::Project` in tests

### Issue 2: Kanban Stage Order

**Current State:**
```rust
fn kanban_sort_order(kanban: &str) -> i64 {
    match kanban.to_lowercase().as_str() {
        "proposed" => 0,
        "stalled" => 1,
        "queued" => 2,
        "external" => 3,
        "done" => 4,
        ...
    }
}
```

**Desired State:**
```rust
fn kanban_sort_order(kanban: &str) -> i64 {
    match kanban.to_lowercase().as_str() {
        "proposed" => 0,
        "stalled" => 1,
        "external" => 2,  // Moved before queued
        "queued" => 3,
        "done" => 4,
        ...
    }
}
```

**Rationale:**
- External tasks are "out of your hands" - similar to stalled (needs attention but not actively working)
- Queued tasks are "ready to work" - should come after external/stalled
- Logical flow: proposed (new) → stalled (needs attention) → external (waiting on others) → queued (ready) → done

**Implementation:**
- Update `kanban_sort_order()` in `src/cli/output.rs`
- Update ordinal values: external = 2, queued = 3
- No other changes needed (kanban calculation logic is already correct)

---

## Implementation Plan

### Phase 1: Fix Project Filter Multi-Value Support

1. **Update FilterTerm enum** (`src/filter/parser.rs`):
   - Change `Project(String)` to `Project(Vec<String>)`
   - Update comment to indicate multi-value support

2. **Update filter parser** (`src/filter/parser.rs`):
   - In `parse_filter_term()`, when parsing `project:` key:
     - Split value on commas: `value.split(',')`
     - Trim and collect into `Vec<String>`
     - Create `FilterTerm::Project(values)`

3. **Update filter evaluator** (`src/filter/evaluator.rs`):
   - In `evaluate_term()`, update `FilterTerm::Project` match arm:
     - Iterate over project names in vector
     - For each project name, check if task's project matches (exact or prefix)
     - Return true if ANY project matches (OR logic)

4. **Update tests**:
   - Find tests that use `FilterTerm::Project`
   - Update to use `Vec<String>` instead of `String`
   - Add test for multi-value project filtering

### Phase 2: Fix Kanban Stage Order

1. **Update kanban_sort_order** (`src/cli/output.rs`):
   - Swap ordinal values: external = 2, queued = 3
   - Update comment to reflect new order

2. **Verify no other changes needed**:
   - Kanban calculation logic is already correct
   - Sort order is the only thing that needs updating

---

## Testing

### Test 1: Multi-Value Project Filter
```bash
# Create tasks in different projects
tatl add "Task 1" project:pro1
tatl add "Task 2" project:pro2
tatl add "Task 3" project:pro3

# Create sessions for these tasks
tatl on 1
tatl off
tatl on 2
tatl off
tatl on 3
tatl off

# Test multi-value filter
tatl sessions report project:pro1,pro2
# Should show sessions for tasks 1 and 2, not task 3

tatl list project:pro1,pro2
# Should show tasks 1 and 2, not task 3
```

### Test 2: Kanban Sort Order
```bash
# Create tasks in different kanban states
tatl add "Proposed task"
tatl add "Stalled task" && tatl on <id> && tatl dequeue <id>
tatl add "External task" && tatl send <id> colleague
tatl add "Queued task" && tatl enqueue <id>

# Sort by kanban
tatl list sort:kanban
# Should show: proposed, stalled, external, queued (in that order)
```

---

## Files to Modify

1. `src/filter/parser.rs`:
   - Change `FilterTerm::Project` to `Vec<String>`
   - Update parser to split on commas

2. `src/filter/evaluator.rs`:
   - Update `FilterTerm::Project` evaluation to handle vector

3. `src/cli/output.rs`:
   - Update `kanban_sort_order()` to swap external and queued ordinals

4. Tests (if any reference `FilterTerm::Project`):
   - Update to use `Vec<String>`

---

## Success Criteria

1. ✅ `tatl sessions report project:pro1,pro2` shows sessions for tasks in pro1 OR pro2
2. ✅ `tatl list project:pro1,pro2` shows tasks in pro1 OR pro2
3. ✅ Kanban sort order is: proposed → stalled → external → queued → done
4. ✅ All existing tests pass
5. ✅ No breaking changes to single-value project filters (`project:pro1` still works)

---

## Notes

- This is a backward-compatible change: single project names (`project:pro1`) will still work (vector with one element)
- The OR logic for multi-value is consistent with `kanban:` and `status:` filters
- Kanban order change is purely cosmetic (sorting only), doesn't affect kanban calculation logic

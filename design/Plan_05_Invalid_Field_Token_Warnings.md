## Invalid Field Token Warnings - Planning Document

This document specifies:
1. Field name abbreviation support (like command abbreviations)
2. Detection and error reporting for invalid field tokens
3. Empty field value handling (treat as `none`/null)

---

### Current Behavior

When a user provides an invalid field token (e.g., `projects:testing` instead of `project:testing`), the parser:
1. Doesn't recognize it as a field token
2. Treats it as part of the description
3. Silently ignores the intended field assignment
4. No feedback is provided to the user

**Examples of problematic syntax:**
- `projects:testing` (plural) → treated as description text
- `proj:testing` (partial abbreviation) → treated as description text (should work if unambiguous)
- `duee:tomorrow` (typo) → treated as description text
- `schedule:next-week` (wrong field name) → treated as description text
- `tag:urgent` (wrong syntax, should be `+urgent`) → treated as description text

**Additional Issues:**
- Field name abbreviations not supported (e.g., `proj:` should work like `project:`)
- Empty field values (e.g., `project:`) not handled as clearing the field

**Problem:**
Users may think they've assigned a project, due date, etc., but the task is created without those attributes. This leads to confusion and data integrity issues.

---

### Proposed Behavior

**Part 1: Field Name Abbreviation (Like Commands)**

Apply the same abbreviation rules as commands to field names:
- If the field name prefix is **unambiguous** (uniquely matches the start of a single field name), accept it automatically
- If the field name prefix is **ambiguous** (matches multiple field names), show an error with suggestions
- If the field name doesn't match any field name start, use fuzzy matching to suggest the closest match

**Examples:**
- `proj:` → matches only `project` (unambiguous) → accepted as `project:`
- `proje:` → matches only `project` (unambiguous) → accepted as `project:`
- `d:` → matches `due` (unambiguous) → accepted as `due:`
- `du:` → matches `due` (unambiguous) → accepted as `due:`
- `sc:` → matches only `scheduled` (unambiguous) → accepted as `scheduled:`
- `sch:` → matches only `scheduled` (unambiguous) → accepted as `scheduled:`
- `t:` → matches `template` (unambiguous) → accepted as `template:`
- `te:` → matches only `template` (unambiguous) → accepted as `template:`

**Part 2: Empty Field Values**

If a field value is left blank (e.g., `project:`), interpret it as setting the field to `none`/null:
- `project:` → clear project assignment
- `due:` → clear due date
- `scheduled:` → clear scheduled date
- Particularly useful with `task modify` command

**Part 3: Invalid Field Token Detection**

For tokens that don't match any field name (even with abbreviation):
- Use fuzzy matching (Levenshtein distance) to find the closest match
- Show error with suggestion
- Don't silently treat as description text

**User Experience:**

**Abbreviation Matching (Automatic):**
```
$ task add "Fix bug" sc:tomorrow +urgent
Created task 1: Fix bug (scheduled: 2025-01-15, tags: +urgent)
```

**Ambiguous Abbreviation (Error):**
```
$ task add "Fix bug" s:tomorrow +urgent

Error: Ambiguous field name 's'
  Matches: 'scheduled', 'scheduled'
  Use a longer prefix to disambiguate, or use the full field name.
```

**Invalid Field Name (Error with Suggestion):**
```
$ task add "Fix bug" schedule:tomorrow +urgent

Error: Unrecognized field name 'schedule'
  Did you mean 'scheduled'?
```

**Empty Field Value (Clear Field):**
```
$ task modify 1 scheduled:
Modified task 1 (cleared scheduled date)
```

---

### Decisions Made

1. **Abbreviation Strategy (Like Commands):**
   - Use prefix matching to find unambiguous field names
   - If prefix matches exactly one field name, accept it automatically
   - If prefix matches multiple field names, show error with all matches
   - Reuse logic from `abbrev.rs` for consistency

2. **Field Name List (with abbreviations):**
   - `project` (can be abbreviated: `proj`, `proje`, etc.)
   - `due` (can be abbreviated: `d`, `du`, etc.)
   - `scheduled` (can be abbreviated: `sc`, `sch`, etc.)
   - `wait` (can be abbreviated: `w`, `wa`, etc.)
   - `alloc` (can be abbreviated: `a`, `al`, etc.)
   - `template` (can be abbreviated: `t`, `te`, `tem`, etc.)
   - `recur` (can be abbreviated: `r`, `re`, `rec`, etc.)
   - `uda.*` (UDAs have prefix, can't be abbreviated)

3. **Empty Field Value Handling:**
   - `field:` (empty value) → interpret as `field:none`
   - Clear the field assignment (set to null)
   - Works for all fields: `project:`, `due:`, `scheduled:`, etc.
   - Particularly useful for `task modify` to clear fields

4. **Invalid Field Name Detection:**
   - If field name doesn't match any field (even with abbreviation), use fuzzy matching
   - Find the single most similar field name (Levenshtein distance)
   - Show error with suggestion
   - Don't silently treat as description text

5. **Integration Points:**
   - `task add` command
   - `task modify` command
   - Both should support abbreviations and empty values

6. **Error vs Warning:**
   - Ambiguous abbreviations: **Error** (must be resolved)
   - Invalid field names: **Error** with suggestion (don't proceed)
   - Empty field values: **Accepted** (clear the field)

---

### Implementation Considerations

1. **Abbreviation Matching:**
   - Create `expand_field_name_abbreviation(field: &str) -> Result<String, Vec<String>>`
   - Similar to `find_unique_command()` in `abbrev.rs`
   - Return `Ok(field_name)` if unambiguous, `Err(matches)` if ambiguous
   - Field name list: `["project", "due", "scheduled", "wait", "alloc", "template", "recur"]`

2. **Fuzzy Matching (for invalid field names):**
   - Reuse `levenshtein_distance` from `src/utils/fuzzy.rs`
   - Create `find_similar_field_name(field: &str) -> Option<String>`
   - Find the single most similar field name
   - Only used when abbreviation matching fails

3. **Parser Changes:**
   - Modify `parse_field_token()` to handle empty values (`field:` → `field:none`)
   - Modify `parse_task_args()` to:
     - Try abbreviation expansion first
     - If ambiguous, return error with matches
     - If no match, try fuzzy matching and return error with suggestion
     - Handle empty values as `none`
   - Return type: `Result<ParsedTaskArgs, FieldParseError>`

4. **Error Structure:**
   ```rust
   pub enum FieldParseError {
       AmbiguousAbbreviation {
           field: String,
           matches: Vec<String>,
       },
       InvalidFieldName {
           field: String,
           suggestion: String,
       },
   }
   ```

5. **Command Handler Changes:**
   - `handle_task_add()`: Handle `FieldParseError`, display error, exit
   - `handle_task_modify()`: Same error handling
   - Empty field values automatically handled as `none`

6. **Field Name Resolution:**
   - Try exact match first
   - Try abbreviation expansion (unambiguous prefix match)
   - If ambiguous, return error with all matches
   - If no match, use fuzzy matching to find closest
   - Return error with single suggestion

7. **Empty Value Handling:**
   - Detect empty value: `field:` (value is empty string after colon)
   - Convert to `field:none` internally
   - Apply same logic as explicit `field:none` in modify command

8. **Edge Cases:**
   - `proj:` → unambiguous (matches only `project`) → accepted
   - `sc:` → unambiguous (matches only `scheduled`) → accepted
   - `s:` → ambiguous (if multiple fields start with 's') → error
   - `scheduled:` → empty value → treated as `scheduled:none`
   - `xyz:value` → no match → fuzzy match to find closest → error with suggestion
   - UDAs: `uda.xyz:value` → handled separately, no abbreviation needed

---

### Examples

**Example 1: Valid Abbreviation (Unambiguous)**
```bash
$ task add "Fix bug" sc:tomorrow +urgent

Created task 1: Fix bug (scheduled: 2025-01-15, tags: +urgent)
```

**Example 2: Ambiguous Abbreviation (Error)**
```bash
$ task add "Fix bug" s:tomorrow +urgent

Error: Ambiguous field name 's'
  Matches: 'scheduled', 'scheduled'
  Use a longer prefix to disambiguate, or use the full field name.
```

**Example 3: Invalid Field Name (Error with Suggestion)**
```bash
$ task add "Fix bug" schedule:tomorrow +urgent

Error: Unrecognized field name 'schedule'
  Did you mean 'scheduled'?
```

**Example 4: Empty Field Value (Clear Field)**
```bash
$ task modify 1 scheduled:

Modified task 1 (cleared scheduled date)
```

**Example 5: Multiple Empty Values**
```bash
$ task modify 1 due: scheduled: wait:

Modified task 1 (cleared due date, scheduled date, and wait date)
```

**Example 6: Abbreviation with Empty Value**
```bash
$ task modify 1 sc:

Modified task 1 (cleared scheduled date)
```

**Example 7: Invalid Field Name (Fuzzy Match)**
```bash
$ task add "Fix bug" schedule:tomorrow

Error: Unrecognized field name 'schedule'
  Did you mean 'scheduled'?
```

**Example 8: Multiple Errors**
```bash
$ task add "Fix bug" schedule:tomorrow duee:tomorrow

Error: Unrecognized field name 'schedule'
  Did you mean 'scheduled'?

Error: Unrecognized field name 'duee'
  Did you mean 'due'?
```

---

### Implementation Checklist

- [ ] Create `expand_field_name_abbreviation()` function (similar to command abbreviation)
- [ ] Create `find_similar_field_name()` using fuzzy matching
- [ ] Create `FieldParseError` enum for error handling
- [ ] Update `parse_field_token()` to handle empty values (`field:` → `field:none`)
- [ ] Update `parse_task_args()` to:
  - [ ] Try abbreviation expansion for field names
  - [ ] Return error if ambiguous abbreviation
  - [ ] Use fuzzy matching if no abbreviation match
  - [ ] Return error with suggestion if invalid field name
  - [ ] Handle empty field values as `none`
- [ ] Update `handle_task_add()` to handle `FieldParseError`
- [ ] Update `handle_task_modify()` to handle `FieldParseError`
- [ ] Update field value handling to treat empty string as `none`
- [ ] Write tests for field name abbreviation (unambiguous cases)
- [ ] Write tests for ambiguous abbreviation errors
- [ ] Write tests for invalid field name errors with suggestions
- [ ] Write tests for empty field value handling
- [ ] Write integration tests for `task add` with abbreviations
- [ ] Write integration tests for `task modify` with empty values
- [ ] Update command reference documentation

---

### Field Name Reference

**Valid Field Names (with abbreviation rules):**
- `project` - Project assignment
  - Can be abbreviated: `proj`, `proje`, `projec`, etc.
- `due` - Due date
  - Can be abbreviated: `d`, `du`, `due`
- `scheduled` - Scheduled date
  - Can be abbreviated: `sc`, `sch`, `sche`, etc.
- `wait` - Wait until date
  - Can be abbreviated: `w`, `wa`, `wai`, `wait`
- `alloc` - Time allocation
  - Can be abbreviated: `a`, `al`, `all`, etc.
- `template` - Template name
  - Can be abbreviated: `t`, `te`, `tem`, etc.
- `recur` - Recurrence rule
  - Can be abbreviated: `r`, `re`, `rec`, etc.
- `uda.<key>` - User-defined attribute
  - No abbreviation support (prefix required)

**Abbreviation Examples:**
- `proj:` → unambiguous → accepted as `project:`
- `d:` → unambiguous → accepted as `due:`
- `sc:` → unambiguous → accepted as `scheduled:`
- `sch:` → unambiguous → accepted as `scheduled:`
- `t:` → unambiguous → accepted as `template:`
- `s:` → ambiguous (if multiple fields start with 's') → error

**Empty Value Examples:**
- `project:` → treated as `project:none` → clear project
- `due:` → treated as `due:none` → clear due date
- `scheduled:` → treated as `scheduled:none` → clear scheduled date
- `sc:` → treated as `scheduled:none` → clear scheduled date (after abbreviation expansion)

---

## Implementation Notes

- **Consistency with Commands:** Use the same abbreviation logic as command abbreviations for consistency
- **Error vs Warning:** Invalid field names are errors (don't proceed), not warnings
- **Empty Values:** Treating empty values as `none` is particularly useful for `modify` command
- **Performance:** Abbreviation matching is fast (simple prefix comparison), fuzzy matching only when needed
- **User Experience:** Clear error messages with actionable suggestions
- **Extensibility:** Easy to add new field names to the abbreviation list

---

## Design Decisions

1. **Abbreviation Rules:**
   - Same as command abbreviations: unambiguous prefix match is accepted
   - Ambiguous prefix match is an error (must be resolved)
   - No fuzzy matching for abbreviations (only for completely invalid field names)

2. **Empty Value Handling:**
   - `field:` (empty value) is treated as `field:none`
   - This is consistent with explicit `field:none` syntax
   - Particularly useful for clearing fields in `modify` command

3. **Error Handling:**
   - Invalid field names are errors (don't create/modify task)
   - Show clear error message with suggestion
   - User must correct the error to proceed

4. **Field Name List:**
   - Only full field names are in the list (no explicit aliases)
   - Abbreviations are handled dynamically via prefix matching
   - Example: `proj:` matches `project` unambiguously, so it's accepted

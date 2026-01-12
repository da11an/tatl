## Missing Help Documentation - Analysis

This document identifies all commands and command patterns that are not accessible through the CLI help documentation.

---

### Commands Missing from Help

#### 1. **`enqueue`** ✗
- **Status:** NOT in Clap help
- **Patterns:**
  - `task <id> enqueue` (syntactic sugar, handled via pre-clap parsing)
  - `task stack enqueue <id>` (should be canonical form, but NOT implemented yet)
- **Location:** Handled via pre-clap parsing (line 345-354)
- **Why Missing:** Not a Clap command variant
- **Impact:** Users can't discover this command via help

#### 2. **`task <id>` (default summary)** ✗
- **Status:** NOT documented in help
- **Pattern:** `task <id>` (no subcommand) → shows task summary
- **Location:** Handled via pre-clap parsing (line 367-381)
- **Why Missing:** Implicit behavior, not a command
- **Impact:** Users might not know `task 1` shows summary

#### 3. **Filter-Before-Command Patterns** ⚠️
- **Status:** Partially documented (commands exist, but patterns not explained)
- **Patterns:**
  - `task <filter> list` (e.g., `task project:work list`)
  - `task <id|filter> modify` (e.g., `task 1 modify` or `task +urgent modify`)
  - `task <id|filter> done` (e.g., `task 1 done` or `task project:work done`)
  - `task <id|filter> delete` (e.g., `task 1 delete` or `task +old delete`)
  - `task <id|filter> annotate` (e.g., `task 1 annotate "note"`)
  - `task <id|filter> summary` (e.g., `task 1 summary` or `task +urgent summary`)
  - `task <id|filter> sessions list/show` (e.g., `task 1 sessions list`)
- **Location:** Handled via pre-clap parsing
- **Why Missing:** Commands exist in help, but the filter-before-command syntax isn't explained
- **Impact:** Users might not know they can use filters before commands

#### 4. **Alternative Stack Syntax** ⚠️
- **Status:** Partially documented
- **Patterns:**
  - `task stack <index> pick` (alternative to `task stack pick <index>`)
  - `task stack <index> drop` (alternative to `task stack drop <index>`)
- **Location:** Handled via pre-clap parsing (line 421-430)
- **Why Missing:** Clap shows `task stack pick <index>`, but alternative syntax exists
- **Impact:** Users might not know about the alternative syntax

#### 5. **Task-Specific Clock In** ⚠️
- **Status:** Partially documented
- **Pattern:** `task <id> clock in` (pushes task to top and starts timing)
- **Location:** Handled via pre-clap parsing (line 331-343)
- **Why Missing:** `task clock in` is documented, but task-specific form isn't mentioned
- **Impact:** Users might not know they can specify a task ID

#### 6. **`templates` Command** ❓
- **Status:** Mentioned in code but NOT a command
- **Pattern:** `task templates` - doesn't exist
- **Location:** Referenced in global subcommand checks (line 372, 388, 440)
- **Why Missing:** Reserved word, but no actual command exists
- **Impact:** Confusing - users might try `task templates` and get an error

---

### Commands in Help vs. Actual Patterns

| Command in Help | Actual Patterns | Missing from Help? |
|-----------------|-----------------|---------------------|
| `task modify` | `task modify <id>`<br>`task <id> modify`<br>`task <filter> modify` | Filter-before-command pattern |
| `task done` | `task done`<br>`task <id> done`<br>`task <filter> done` | Filter-before-command pattern |
| `task delete` | `task delete <id>`<br>`task <id> delete`<br>`task <filter> delete` | Filter-before-command pattern |
| `task annotate` | `task annotate <note>`<br>`task <id> annotate <note>`<br>`task <filter> annotate <note>` | Filter-before-command pattern |
| `task summary` | `task summary <id>`<br>`task <id> summary`<br>`task <id>` (implicit) | Filter-before-command pattern, implicit form |
| `task list` | `task list`<br>`task <filter> list` | Filter-before-command pattern |
| `task clock in` | `task clock in`<br>`task <id> clock in` | Task-specific form |
| `task stack pick` | `task stack pick <index>`<br>`task stack <index> pick` | Alternative syntax |
| `task stack drop` | `task stack drop <index>`<br>`task stack <index> drop` | Alternative syntax |
| `task sessions list` | `task sessions list`<br>`task <id> sessions list`<br>`task <filter> sessions list` | Filter-before-command pattern |
| `task sessions show` | `task sessions show`<br>`task <id> sessions show`<br>`task <filter> sessions show` | Filter-before-command pattern |
| (none) | `task <id> enqueue`<br>`task stack enqueue <id>` (should exist) | **Completely missing** |

---

### Recommendations

#### High Priority

1. **Add `enqueue` to Stack Commands** (Option B from Plan 08)
   - Add `Enqueue` variant to `StackCommands` enum
   - Makes `task stack enqueue <id>` valid and visible in help
   - Document `task <id> enqueue` as syntactic sugar

2. **Add Task Subcommands Section** (Option D from Plan 08)
   - Add to main `task --help` output
   - Lists all `task <id> <subcommand>` patterns
   - Helps users discover these patterns

3. **Document `task <id>` Default Behavior**
   - Add to Summary command help
   - Mention that `task <id>` (no subcommand) shows summary
   - Could be: "If no subcommand is provided, shows task summary"

#### Medium Priority

4. **Document Filter-Before-Command Patterns**
   - Add note to relevant command help text
   - Example: "You can also use: `task <filter> <command>` (e.g., `task project:work list`)"
   - Or create a "Command Patterns" section in help

5. **Document Alternative Stack Syntax**
   - Add to Stack command help
   - Mention: "You can also use: `task stack <index> pick` (alternative syntax)"

6. **Document Task-Specific Clock In**
   - Add to Clock command help
   - Mention: "You can also use: `task <id> clock in` to push task to top and start timing"

#### Low Priority

7. **Clarify `templates` Status**
   - Remove from global subcommand checks if not a command
   - Or implement `task templates` command if it should exist
   - Currently confusing - referenced but doesn't exist

---

### Implementation Priority

**Phase 1 (Critical - Blocks Discovery):**
- [ ] Add `enqueue` to Stack Commands (Option B)
- [ ] Add Task Subcommands section to main help (Option D)
- [ ] Document `task <id>` default behavior

**Phase 2 (Important - Improves Usability):**
- [ ] Document filter-before-command patterns
- [ ] Document alternative stack syntax
- [ ] Document task-specific clock in

**Phase 3 (Polish):**
- [ ] Resolve `templates` status (remove or implement)
- [ ] Add cross-references in help text
- [ ] Update all command help to mention alternative forms

---

### Summary

**Completely Missing from Help:**
1. `enqueue` command (not in any help)
2. `task <id>` default behavior (implicit summary)

**Partially Documented (Patterns Not Explained):**
1. Filter-before-command patterns (`task <filter> <command>`)
2. Alternative stack syntax (`task stack <index> pick`)
3. Task-specific clock in (`task <id> clock in`)

**Confusing:**
1. `templates` - referenced in code but not a command

**Key Insight:** The help system should document not just commands, but also **command patterns** and **alternative syntaxes** to help users discover all ways to use the CLI.

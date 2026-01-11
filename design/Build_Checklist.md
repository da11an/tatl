# Build Checklist

**Purpose:** Ordered checklist for incremental development and testing of Task Ninja.

**Principle:** Build incrementally, test thoroughly at each step. Each item should be fully implemented and tested before moving to the next.

---

## Phase 1: Foundation (Database & Core Infrastructure)

### 1.1 Database Schema & Migrations
- [x] Create database migration system
- [x] Implement initial schema (all tables from Section 9)
- [x] Add migration versioning/tracking
- [x] Test: migrations apply cleanly to empty database
- [x] Test: migrations are idempotent
- [x] Test: foreign key constraints work correctly

### 1.2 Database Connection & Configuration
- [x] Implement configuration file parsing (`~/.taskninja/rc`)
- [x] Implement database location resolution (default + override)
- [x] Implement database connection management
- [x] Test: default location creates database at `~/.taskninja/tasks.db`
- [x] Test: configuration override works
- [x] Test: directory structure auto-creation

### 1.3 Core Data Models
- [x] Define Task model/struct
- [x] Define Project model/struct
- [x] Define Tag model/struct (tags are stored in task_tags table, no separate model needed)
- [x] Define Session model/struct
- [x] Define Stack model/struct
- [x] Define Annotation model/struct
- [x] Test: models serialize/deserialize correctly
- [x] Test: models validate constraints

---

## Phase 2: Basic CRUD Operations

### 2.1 Project CRUD
- [x] Implement `task projects add <name>`
- [x] Implement `task projects list [--archived]`
- [x] Implement `task projects rename <old> <new> [--force]`
- [x] Implement `task projects archive <name>`
- [x] Implement `task projects unarchive <name>`
- [x] Test: Project creation with unique names
- [x] Test: Nested project support (dot notation)
- [x] Test: Project merge with `--force`
- [x] Test: Archive/unarchive behavior
- [ ] Acceptance: All project scenarios from Section 11.8 (deferred - will add when acceptance test framework is ready)

### 2.2 Task CRUD (Basic)
- [x] Implement `task add` command
- [x] Implement description parsing (no `--` delimiter)
- [x] Implement field token parsing (`project:`, `due:`, etc.)
- [x] Implement tag parsing (`+tag`, `-tag`)
- [x] Implement `task list` command
- [x] Test: Task creation with all field types (basic support - full date/duration parsing in Phase 9)
- [x] Test: Description parsing handles mixed tokens
- [x] Test: Tag add/remove
- [x] Test: UDA storage format (JSON, keys without prefix)
- [ ] Acceptance: Basic task add/list scenarios (will add when more features complete)

### 2.3 Task Modification
- [x] Implement `task <id|filter> modify` command (ID support only for now, filter support in Phase 3)
- [x] Implement multi-task confirmation (yes/no/interactive) - structure in place, full support when filters added
- [x] Implement `--yes` and `--interactive` flags (structure in place)
- [x] Test: Single task modification
- [x] Test: Filter-based modification with confirmation
- [x] Test: Description replacement
- [x] Test: Field clearing (`field:none`)
- [ ] Acceptance: Modify scenarios (will add when more features complete)

---

## Phase 3: Filtering & Querying

### 3.1 Filter Parser
- [x] Implement filter token parsing
- [x] Implement AND/OR/NOT logic
- [x] Implement precedence (not > and > or)
- [x] Test: Simple filters (`project:work`, `+urgent`)
- [x] Test: AND combinations
- [x] Test: OR combinations
- [x] Test: NOT combinations
- [x] Test: Complex expressions
- [x] Integration: Filter support in `task list` command
- [ ] Acceptance: Filter scenarios from Section 11.5 (will add when more features complete)

### 3.2 Filter Terms Implementation
- [x] Implement `id:<n>` and bare numeric ID
- [x] Implement `status:` filter
- [x] Implement `project:` filter (with nested project prefix matching)
- [x] Implement `+tag` / `-tag` filters
- [x] Implement `due:`, `scheduled:`, `wait:` filters (any/none/date expressions)
- [x] Implement `waiting` derived filter
- [x] Test: Each filter term independently
- [x] Test: Combined filter terms
- [x] Test: Nested project prefix matching

---

## Phase 4: Stack Foundation

### 4.1 Stack Initialization
- [x] Implement auto-creation of default stack on first operation
- [x] Test: Stack created on first `stack show`
- [x] Test: Stack created on first stack operation
- [ ] Acceptance: Stack auto-initialization scenario (will add when acceptance test framework ready)

### 4.2 Basic Stack Operations
- [x] Implement `task stack show`
- [x] Implement `task <id> enqueue` (add to end)
- [x] Implement `task stack <index> pick`
- [x] Implement `task stack roll [n]` (default n=1)
- [x] Implement `task stack <index> drop`
- [x] Implement `task stack clear`
- [x] Test: Stack operations with empty stack
- [x] Test: Index clamping (0, -1, out-of-range)
- [x] Test: Roll default behavior (n=1)
- [ ] Acceptance: Stack basics scenarios from Section 11.1 (will add when acceptance test framework ready)

---

## Phase 5: Clock & Sessions

### 5.1 Session Model & Storage
- [x] Implement session creation/retrieval
- [x] Implement single open session constraint
- [x] Test: Only one open session allowed
- [x] Test: Session timestamps (UTC storage)

### 5.2 Basic Clock Commands
- [x] Implement `task clock in` (requires stack non-empty)
- [x] Implement `task clock out`
- [x] Implement default "now" behavior
- [x] Test: Clock in errors on empty stack
- [x] Test: Clock in errors if already running
- [x] Test: Clock out closes session
- [ ] Acceptance: Clock scenarios from Section 11.2 (will add when acceptance test framework ready)

### 5.3 Clock with Task ID
- [x] Implement `task <id> clock in` (push to top and start)
- [x] Implement session closing when switching tasks
- [x] Test: Task pushed to stack[0] on clock in
- [x] Test: Previous session closed when new one starts
- [x] Test: Timestamp handling (same timestamp for close/start)
- [ ] Acceptance: Clock with task scenarios (will add when acceptance test framework ready)

### 5.4 Clock Interval Syntax
- [x] Implement interval parsing (`start..end`)
- [x] Implement closed session creation
- [x] Implement overlap prevention (amend end time)
- [x] Test: Interval creates closed session
- [x] Test: Overlap prevention amends end time
- [ ] Acceptance: Interval scenarios from Section 11.2 (will add when acceptance test framework ready)

### 5.5 Stack Operations with Clock
- [x] Implement `--clock in` and `--clock out` flags
- [x] Implement stack operations affecting running sessions
- [x] Test: Stack roll while clock running switches live task
- [x] Test: Stack pick while stopped doesn't create sessions
- [ ] Acceptance: Stack and clock coupling scenarios (will add when acceptance test framework ready)

---

## Phase 6: Annotations

### 6.1 Annotation CRUD
- [x] Implement `task [<id>] annotate <note...>`
- [x] Implement annotation without ID (when clocked in)
- [x] Implement session linking (session_id in annotations)
- [x] Implement `task <id> annotate --delete <annotation_id>`
- [x] Test: Annotation creation with task ID
- [x] Test: Annotation creation without ID (when clocked in)
- [x] Test: Session linking when created during session
- [x] Test: Annotation deletion
- [ ] Acceptance: Annotation scenarios (will add when acceptance test framework ready)

---

## Phase 7: Done Command

### 7.1 Done with Single Task
- [x] Implement `task done` (shorthand for stack[0])
- [x] Implement `task <id> done`
- [x] Implement session closing on done
- [x] Implement task completion (status change)
- [x] Implement stack removal on done
- [x] Test: Done errors if stack empty
- [x] Test: Done errors if no session running
- [x] Test: Done completes task and removes from stack
- [x] Test: Done with --next starts next task
- [ ] Acceptance: Done semantics scenarios from Section 11.3 (will add when acceptance test framework ready)

### 7.2 Done with Filter & Confirmation
- [x] Implement `task [<id|filter>] done` with filtering
- [x] Implement multi-task confirmation (yes/no/interactive)
- [x] Implement `--yes` and `--interactive` flags
- [x] Implement `--next` flag (start next task)
- [x] Test: Filter-based done with confirmation
- [x] Test: `--next` starts next task in stack
- [ ] Acceptance: Done with filter scenarios (will add when acceptance test framework ready)

---

## Phase 8: Micro-Session Policy

### 8.1 Micro-Session Detection
- [x] Implement MICRO constant (30 seconds)
- [x] Implement micro-session detection (duration < MICRO)
- [x] Test: Micro-session identification

### 8.2 Merge/Purge Logic
- [x] Implement merge rule (same task, within MICRO of end)
- [x] Implement purge rule (different task, within MICRO of end)
- [x] Implement merge/purge application logic
- [x] Implement warning messages
- [x] Test: Merge on bounce back to same task
- [x] Test: Purge on rapid switch to different task
- [x] Test: Micro-session preserved if no rule triggers
- [ ] Acceptance: Micro-session scenarios from Section 11.4 (will add when acceptance test framework ready)

---

## Phase 9: Date & Time Handling

### 9.1 Date Expression Parser
- [x] Implement absolute date parsing (`2026-01-10`, `2026-01-10T14:30`)
- [x] Implement relative date parsing (`today`, `tomorrow`, `+2d`, etc.)
- [x] Implement time-only parsing with 24-hour window rule
- [x] Test: All date expression forms
- [x] Test: Time-only resolution (8h past, 16h future window)
- [x] Test: "Twice as close" rule for time-only

### 9.2 Timezone & DST Handling
- [x] Implement UTC storage (epoch seconds) - Already implemented via chrono
- [x] Implement local timezone parsing - Already implemented via chrono Local
- [x] Implement local timezone display - Already implemented via chrono Local
- [x] Implement DST fall back handling (first occurrence) - Implemented in parse_local_datetime
- [x] Implement DST spring forward handling (error on invalid) - Implemented in parse_local_datetime
- [x] Test: UTC storage consistency - Verified via existing tests
- [x] Test: DST transition edge cases (basic tests implemented, full transition testing deferred due to complexity)
- [x] Test: Timezone conversion accuracy - Verified via existing tests

### 9.3 Duration Parser
- [x] Implement duration format parsing (`30s`, `1h30m`, etc.)
- [x] Implement unit ordering validation (largest to smallest)
- [x] Test: Valid duration formats
- [x] Test: Invalid duration formats (wrong order, spaces, etc.)

---

## Phase 10: Task Events (Audit Log)

### 10.1 Event Recording
- [x] Implement event creation for all task changes
- [x] Implement event types (created, modified, status_changed, etc.)
- [x] Implement event payload JSON serialization
- [x] Test: Events recorded for all state changes
- [x] Test: Event immutability (never modified/deleted)
- [x] Test: Event payload structure

### 10.2 Event Queries (Future)
- [x] Note: Event querying deferred to future (analysis features)

---

## Phase 11: Recurrence

### 11.1 Recurrence Rule Parser
- [x] Implement grammar parser for recurrence rules
- [x] Implement simple frequencies (`daily`, `weekly`, `monthly`, `yearly`)
- [x] Implement interval frequencies (`every:Nd`, `every:Nw`, etc.)
- [x] Implement weekday modifier (`byweekday:`)
- [x] Implement day-of-month modifier (`bymonthday:`)
- [x] Test: All recurrence rule formats
- [x] Test: Modifier validation (compatibility with frequency)

### 11.2 Recurrence Generation
- [x] Implement `task recur run [--until <date_expr>]`
- [x] Implement occurrence generation logic
- [x] Implement idempotency (recur_occurrences table)
- [x] Implement attribute precedence (template → seed → computed dates)
- [x] Test: Idempotent generation (no duplicates)
- [x] Test: Attribute precedence
- [ ] Test: Date computation relative to occurrence - Basic implementation (dates copied as-is, relative date evaluation deferred)
- [ ] Acceptance: Recurrence scenarios from Section 11.7 - Basic scenarios tested

---

## Phase 12: Templates

### 12.1 Template CRUD
- [x] Implement template storage
- [x] Implement template retrieval
- [x] Test: Template creation and retrieval
- [x] Note: Template management commands deferred (use via `template:<name>` field token)
- [x] Integrate templates into task creation (auto-create on use)
- [x] Integrate templates into recurrence generation (attribute precedence)

---

## Phase 13: Sessions Commands

### 13.1 Sessions List & Show
- [x] Implement `task [<id>] sessions list [--json]`
- [x] Implement `task [<id>] sessions show`
- [x] Test: List all sessions
- [x] Test: List sessions for specific task
- [x] Test: Show current running session
- [x] Test: Show most recent session for task
- [x] Test: JSON output format

---

## Phase 14: Output & Formatting

### 14.1 Human-Readable Output
- [x] Implement table formatting for `list` commands
- [x] Implement stack display formatting
- [x] Implement clock transition messages
- [x] Test: Output formatting consistency
- [x] Test: Column alignment and readability

### 14.2 JSON Output
- [x] Implement `--json` flag support
- [x] Implement JSON schema for tasks
- [x] Implement JSON schema for projects
- [x] Implement JSON schema for stack
- [x] Implement JSON schema for sessions
- [x] Test: JSON output validity
- [x] Test: JSON schema consistency

---

## Phase 15: Error Handling & Validation

### 15.1 Error Messages
- [x] Implement error message format ("Error: " prefix)
- [x] Implement internal error format ("Internal error: " prefix)
- [x] Implement stderr output for errors
- [x] Test: All error messages follow standard format
- [x] Test: Exit codes match specification

### 15.2 Input Validation
- [x] Implement validation for all command inputs
- [x] Implement helpful error messages for invalid input
- [x] Test: Invalid input handling
- [x] Test: Error message clarity

---

## Phase 16: Integration & Acceptance Testing

### 16.1 Acceptance Test Framework
- [x] Set up acceptance test infrastructure
- [x] Implement test database setup/teardown
- [x] Implement Given/When/Then test runner
- [x] Test: Test framework works correctly

### 16.2 Acceptance Test Implementation
- [ ] Implement all acceptance tests from Section 11
- [ ] Test: Stack basics (Section 11.1)
- [ ] Test: Clock and stack coupling (Section 11.2)
- [ ] Test: Done semantics (Section 11.3)
- [ ] Test: Micro-session behavior (Section 11.4)
- [ ] Test: Tags and filters (Section 11.5)
- [ ] Test: Scheduling and waiting (Section 11.6)
- [ ] Test: Recurrence (Section 11.7)
- [ ] Test: Projects (Section 11.8)

### 16.3 End-to-End Testing
- [ ] Test: Complete workflows (add → clock in → annotate → done)
- [ ] Test: Complex filter scenarios
- [ ] Test: Recurrence generation workflows
- [ ] Test: Project management workflows

---

## Phase 17: Transaction & Atomicity

### 17.1 Transaction Implementation
- [ ] Ensure all state-mutating commands run in transactions
- [ ] Implement rollback on errors
- [ ] Test: Atomic operations (stack + clock, done --next, etc.)
- [ ] Test: Rollback on failure
- [ ] Test: No partial state changes

---

## Phase 18: Performance & Optimization

### 18.1 Database Indexes
- [ ] Verify all indexes from DDL are created
- [ ] Test: Query performance with indexes
- [ ] Test: Index usage in common queries

### 18.2 Query Optimization
- [ ] Optimize common queries (list, filter, etc.)
- [ ] Test: Performance with large datasets
- [ ] Profile and optimize bottlenecks

---

## Phase 19: Documentation & Polish

### 19.1 Code Documentation
- [ ] Add inline documentation for all public APIs
- [ ] Document complex algorithms (micro-session, recurrence, etc.)
- [ ] Add examples in code comments

### 19.2 User Documentation
- [ ] Create man pages or help system
- [ ] Document all commands with examples
- [ ] Create troubleshooting guide

---

## Testing Strategy

### Unit Tests
- Each module/function should have unit tests
- Test edge cases and error conditions
- Aim for high code coverage

### Integration Tests
- Test database operations end-to-end
- Test command parsing and execution
- Test transaction behavior

### Acceptance Tests
- All scenarios from Section 11 must pass
- Tests run against temporary databases
- Tests are deterministic and repeatable

### Test Database Setup
- Use in-memory SQLite for fast tests
- Use temporary file databases for integration tests
- Clean up after each test

---

## Build Order Rationale

1. **Foundation first**: Database and models must exist before anything else
2. **Simple before complex**: Basic CRUD before advanced features
3. **Dependencies respected**: Stack before clock (clock needs stack), sessions before annotations (annotations link to sessions)
4. **Test as you go**: Each phase should be fully tested before moving on
5. **Incremental value**: Each phase delivers working functionality

---

## Notes

- Build one phase at a time
- Complete all tests for a phase before moving to next
- Update acceptance tests as you implement features
- Keep design documents in sync with implementation
- Document any deviations from design in `Design_Decisions.md`

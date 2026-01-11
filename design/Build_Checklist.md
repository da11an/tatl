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
- [ ] Implement `task <id|filter> modify` command
- [ ] Implement multi-task confirmation (yes/no/interactive)
- [ ] Implement `--yes` and `--interactive` flags
- [ ] Test: Single task modification
- [ ] Test: Filter-based modification with confirmation
- [ ] Test: Description replacement
- [ ] Test: Field clearing (`field:none`)
- [ ] Acceptance: Modify scenarios

---

## Phase 3: Filtering & Querying

### 3.1 Filter Parser
- [ ] Implement filter token parsing
- [ ] Implement AND/OR/NOT logic
- [ ] Implement precedence (not > and > or)
- [ ] Test: Simple filters (`project:work`, `+urgent`)
- [ ] Test: AND combinations
- [ ] Test: OR combinations
- [ ] Test: NOT combinations
- [ ] Test: Complex expressions
- [ ] Acceptance: Filter scenarios from Section 11.5

### 3.2 Filter Terms Implementation
- [ ] Implement `id:<n>` and bare numeric ID
- [ ] Implement `status:` filter
- [ ] Implement `project:` filter (with nested project prefix matching)
- [ ] Implement `+tag` / `-tag` filters
- [ ] Implement `due:`, `scheduled:`, `wait:` filters
- [ ] Implement `waiting` derived filter
- [ ] Test: Each filter term independently
- [ ] Test: Combined filter terms
- [ ] Test: Nested project prefix matching

---

## Phase 4: Stack Foundation

### 4.1 Stack Initialization
- [ ] Implement auto-creation of default stack on first operation
- [ ] Test: Stack created on first `stack show`
- [ ] Test: Stack created on first stack operation
- [ ] Acceptance: Stack auto-initialization scenario

### 4.2 Basic Stack Operations
- [ ] Implement `task stack show`
- [ ] Implement `task <id> enqueue` (add to end)
- [ ] Implement `task stack <index> pick`
- [ ] Implement `task stack roll [n]` (default n=1)
- [ ] Implement `task stack <index> drop`
- [ ] Implement `task stack clear`
- [ ] Test: Stack operations with empty stack
- [ ] Test: Index clamping (0, -1, out-of-range)
- [ ] Test: Roll default behavior (n=1)
- [ ] Acceptance: Stack basics scenarios from Section 11.1

---

## Phase 5: Clock & Sessions

### 5.1 Session Model & Storage
- [ ] Implement session creation/retrieval
- [ ] Implement single open session constraint
- [ ] Test: Only one open session allowed
- [ ] Test: Session timestamps (UTC storage)

### 5.2 Basic Clock Commands
- [ ] Implement `task clock in` (requires stack non-empty)
- [ ] Implement `task clock out`
- [ ] Implement default "now" behavior
- [ ] Test: Clock in errors on empty stack
- [ ] Test: Clock in errors if already running
- [ ] Test: Clock out closes session
- [ ] Acceptance: Clock scenarios from Section 11.2

### 5.3 Clock with Task ID
- [ ] Implement `task <id> clock in` (push to top and start)
- [ ] Implement session closing when switching tasks
- [ ] Test: Task pushed to stack[0] on clock in
- [ ] Test: Previous session closed when new one starts
- [ ] Test: Timestamp handling (same timestamp for close/start)
- [ ] Acceptance: Clock with task scenarios

### 5.4 Clock Interval Syntax
- [ ] Implement interval parsing (`start..end`)
- [ ] Implement closed session creation
- [ ] Implement overlap prevention (amend end time)
- [ ] Test: Interval creates closed session
- [ ] Test: Overlap prevention amends end time
- [ ] Acceptance: Interval scenarios from Section 11.2

### 5.5 Stack Operations with Clock
- [ ] Implement `--clock in` and `--clock out` flags
- [ ] Implement stack operations affecting running sessions
- [ ] Test: Stack roll while clock running switches live task
- [ ] Test: Stack pick while stopped doesn't create sessions
- [ ] Acceptance: Stack and clock coupling scenarios

---

## Phase 6: Annotations

### 6.1 Annotation CRUD
- [ ] Implement `task [<id>] annotate <note...>`
- [ ] Implement annotation without ID (when clocked in)
- [ ] Implement session linking (session_id in annotations)
- [ ] Implement `task <id> annotate --delete <annotation_id>`
- [ ] Test: Annotation creation with task ID
- [ ] Test: Annotation creation without ID (when clocked in)
- [ ] Test: Session linking when created during session
- [ ] Test: Annotation deletion
- [ ] Acceptance: Annotation scenarios

---

## Phase 7: Done Command

### 7.1 Done with Single Task
- [ ] Implement `task done` (shorthand for stack[0])
- [ ] Implement `task <id> done`
- [ ] Implement session closing on done
- [ ] Implement task completion (status change)
- [ ] Implement stack removal on done
- [ ] Test: Done errors if stack empty
- [ ] Test: Done errors if no session running
- [ ] Test: Done completes task and removes from stack
- [ ] Acceptance: Done semantics scenarios from Section 11.3

### 7.2 Done with Filter & Confirmation
- [ ] Implement `task [<id|filter>] done` with filtering
- [ ] Implement multi-task confirmation (yes/no/interactive)
- [ ] Implement `--yes` and `--interactive` flags
- [ ] Implement `--next` flag (start next task)
- [ ] Test: Filter-based done with confirmation
- [ ] Test: `--next` starts next task in stack
- [ ] Acceptance: Done with filter scenarios

---

## Phase 8: Micro-Session Policy

### 8.1 Micro-Session Detection
- [ ] Implement MICRO constant (30 seconds)
- [ ] Implement micro-session detection (duration < MICRO)
- [ ] Test: Micro-session identification

### 8.2 Merge/Purge Logic
- [ ] Implement merge rule (same task, within MICRO of end)
- [ ] Implement purge rule (different task, within MICRO of end)
- [ ] Implement merge/purge application logic
- [ ] Implement warning messages
- [ ] Test: Merge on bounce back to same task
- [ ] Test: Purge on rapid switch to different task
- [ ] Test: Micro-session preserved if no rule triggers
- [ ] Acceptance: Micro-session scenarios from Section 11.4

---

## Phase 9: Date & Time Handling

### 9.1 Date Expression Parser
- [ ] Implement absolute date parsing (`2026-01-10`, `2026-01-10T14:30`)
- [ ] Implement relative date parsing (`today`, `tomorrow`, `+2d`, etc.)
- [ ] Implement time-only parsing with 24-hour window rule
- [ ] Test: All date expression forms
- [ ] Test: Time-only resolution (8h past, 16h future window)
- [ ] Test: "Twice as close" rule for time-only

### 9.2 Timezone & DST Handling
- [ ] Implement UTC storage (epoch seconds)
- [ ] Implement local timezone parsing
- [ ] Implement local timezone display
- [ ] Implement DST fall back handling (first occurrence)
- [ ] Implement DST spring forward handling (error on invalid)
- [ ] Test: UTC storage consistency
- [ ] Test: DST transition edge cases
- [ ] Test: Timezone conversion accuracy

### 9.3 Duration Parser
- [ ] Implement duration format parsing (`30s`, `1h30m`, etc.)
- [ ] Implement unit ordering validation (largest to smallest)
- [ ] Test: Valid duration formats
- [ ] Test: Invalid duration formats (wrong order, spaces, etc.)

---

## Phase 10: Task Events (Audit Log)

### 10.1 Event Recording
- [ ] Implement event creation for all task changes
- [ ] Implement event types (created, modified, status_changed, etc.)
- [ ] Implement event payload JSON serialization
- [ ] Test: Events recorded for all state changes
- [ ] Test: Event immutability (never modified/deleted)
- [ ] Test: Event payload structure

### 10.2 Event Queries (Future)
- [ ] Note: Event querying deferred to future (analysis features)

---

## Phase 11: Recurrence

### 11.1 Recurrence Rule Parser
- [ ] Implement grammar parser for recurrence rules
- [ ] Implement simple frequencies (`daily`, `weekly`, `monthly`, `yearly`)
- [ ] Implement interval frequencies (`every:Nd`, `every:Nw`, etc.)
- [ ] Implement weekday modifier (`byweekday:`)
- [ ] Implement day-of-month modifier (`bymonthday:`)
- [ ] Test: All recurrence rule formats
- [ ] Test: Modifier validation (compatibility with frequency)

### 11.2 Recurrence Generation
- [ ] Implement `task recur run [--until <date_expr>]`
- [ ] Implement occurrence generation logic
- [ ] Implement idempotency (recur_occurrences table)
- [ ] Implement attribute precedence (template → seed → computed dates)
- [ ] Test: Idempotent generation (no duplicates)
- [ ] Test: Attribute precedence
- [ ] Test: Date computation relative to occurrence
- [ ] Acceptance: Recurrence scenarios from Section 11.7

---

## Phase 12: Templates

### 12.1 Template CRUD
- [ ] Implement template storage
- [ ] Implement template retrieval
- [ ] Test: Template creation and retrieval
- [ ] Note: Template management commands deferred (use via `--template` flag)

---

## Phase 13: Sessions Commands

### 13.1 Sessions List & Show
- [ ] Implement `task [<id>] sessions list [--json]`
- [ ] Implement `task [<id>] sessions show`
- [ ] Test: List all sessions
- [ ] Test: List sessions for specific task
- [ ] Test: Show current running session
- [ ] Test: Show most recent session for task
- [ ] Test: JSON output format

---

## Phase 14: Output & Formatting

### 14.1 Human-Readable Output
- [ ] Implement table formatting for `list` commands
- [ ] Implement stack display formatting
- [ ] Implement clock transition messages
- [ ] Test: Output formatting consistency
- [ ] Test: Column alignment and readability

### 14.2 JSON Output
- [ ] Implement `--json` flag support
- [ ] Implement JSON schema for tasks
- [ ] Implement JSON schema for projects
- [ ] Implement JSON schema for stack
- [ ] Implement JSON schema for sessions
- [ ] Test: JSON output validity
- [ ] Test: JSON schema consistency

---

## Phase 15: Error Handling & Validation

### 15.1 Error Messages
- [ ] Implement error message format ("Error: " prefix)
- [ ] Implement internal error format ("Internal error: " prefix)
- [ ] Implement stderr output for errors
- [ ] Test: All error messages follow standard format
- [ ] Test: Exit codes match specification

### 15.2 Input Validation
- [ ] Implement validation for all command inputs
- [ ] Implement helpful error messages for invalid input
- [ ] Test: Invalid input handling
- [ ] Test: Error message clarity

---

## Phase 16: Integration & Acceptance Testing

### 16.1 Acceptance Test Framework
- [ ] Set up acceptance test infrastructure
- [ ] Implement test database setup/teardown
- [ ] Implement Given/When/Then test runner
- [ ] Test: Test framework works correctly

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

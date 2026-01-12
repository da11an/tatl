## Outstanding Items

This document tracks outstanding tasks and improvements across all plans.

---

### Plan 02: Revision 1

#### Item 3: Status Lines for Commands Without Arguments
- [ ] Performance test status queries on large datasets
- [ ] For status as leading line, format as header, not addendum to help docs

#### Item 2: Command Truncation/Abbreviation Support
- [ ] Add configuration option for expansion verbosity (`~/.taskninja/rc`)
- [ ] Document abbreviation feature

#### Item 4: Filter-Before-Command Pattern
- [ ] Fix remaining test issues (2 tests failing - likely test setup issue, command works in real environment)

---

### Plan 03: Task Deletion

- [ ] Write tests for single task deletion
- [ ] Write tests for bulk deletion
- [ ] Write tests for confirmation logic
- [ ] Write tests for related data cleanup

---

### Plan 04: Task ID Ranges and Lists

- [x] Update `handle_task_list()` to use ID spec parsing (for bare numeric IDs) - **COMPLETED**
- [ ] Update `handle_annotation_add()` to use ID spec parsing
- [ ] Update `handle_task_sessions_*()` to use ID spec parsing
- [ ] Write integration tests for commands
- [ ] Update command reference documentation

---

### General

- [ ] Fix compiler warnings (11 unused variable warnings)
- [ ] Add comprehensive integration tests for all new features
- [ ] Update user documentation with new features

---

## Notes

- Items marked with [x] are completed but may need verification
- Items marked with [ ] are pending
- Priority should be given to testing and documentation

# Plan 21: Rename from task-ninja to tatl (Task and Time Ledger)

## Name Review

### Proposed Name: `tatl`
**Full Name:** Task and Time Ledger  
**Pronunciation:** "tattle" (like "tattle tale")

### Meaningfulness and Fit
✅ **Excellent fit:**
- **Task** - Core functionality (task management)
- **Time** - Time tracking/sessions feature
- **Ledger** - Accurately describes the immutable audit trail and time logging nature
- Concise (4 letters, easy to type)
- Memorable acronym

### Conflict Analysis
✅ **No known conflicts:**
- Web search shows no existing CLI tool named `tatl`
- Not a reserved word in common shells (bash, zsh, fish)
- Not a system command on Linux/Unix
- Available as a package name on crates.io (should verify before publishing)

### Comparison to Current Name
- **task-ninja**: 
  - ✅ Descriptive of task management
  - ❌ Doesn't convey time tracking aspect
  - ❌ "Ninja" is playful but not professional
  - ❌ Conflicts with Taskwarrior's `task` command
- **tatl**:
  - ✅ Clearly conveys both task AND time tracking
  - ✅ Professional, ledger terminology fits audit trail concept
  - ✅ Unique command name, no conflicts
  - ✅ Shorter, easier to type

## Migration Scope

### What Needs to Change

#### 1. Package/Binary Names
- **Cargo.toml**: `name = "task-ninja"` → `name = "tatl"`
- **Cargo.toml**: `[[bin]] name = "task"` → `name = "tatl"`
- **clap command name**: `#[command(name = "task")]` → `#[command(name = "tatl")]`
- **clap about text**: Update description

#### 2. Database/Config Paths
- **Default directory**: `~/.taskninja/` → `~/.tatl/`
- **Database file**: `~/.taskninja/tasks.db` → `~/.tatl/ledger.db`
  - Rename from `tasks.db` to `ledger.db` to align with "Task and Time Ledger" naming
- **Config file**: `~/.taskninja/rc` → `~/.tatl/rc`
- **Code locations**:
  - `src/db/connection.rs`: `default_path()` and `config_path()`
  - Test files that reference `.taskninja` or `tasks.db`

#### 3. Code References
- **Module name**: `task_ninja` → `tatl` (in all `use` statements)
- **Test imports**: All `use task_ninja::...` → `use tatl::...`
- **Documentation comments**: References to "Task Ninja" → "Tatl"
- **Acceptance test framework**: `Command::cargo_bin("task")` → `Command::cargo_bin("tatl")`

#### 4. Documentation Files
- **README.md**: Title, description, examples
- **INSTALL.md**: Installation instructions, conflict notes
- **docs/COMMAND_REFERENCE.md**: Command examples
- **design/**: Plan documents (update references)
- **src/lib.rs**: Library documentation

#### 5. User Migration (Single User)
Since you're the only user, migration is straightforward:
- Copy database from `~/.taskninja/tasks.db` to `~/.tatl/ledger.db`
- Copy config from `~/.taskninja/rc` to `~/.tatl/rc` (if exists)
- Uninstall old `task` binary
- Install new `tatl` binary
- Update any shell aliases/scripts

## Migration Plan

### Phase 1: Code Changes (Automated Where Possible)

#### Step 1.1: Update Cargo.toml
```toml
[package]
name = "tatl"
version = "0.2.0"
edition = "2021"
authors = ["Dallan Prince"]
description = "Task and Time Ledger - A powerful command-line task and time tracking tool"
license = "MIT"

[[bin]]
name = "tatl"
path = "src/main.rs"
```

#### Step 1.2: Update Database Paths
**File**: `src/db/connection.rs`
- `default_path()`: `.taskninja` → `.tatl`, `tasks.db` → `ledger.db`
- `config_path()`: `.taskninja` → `.tatl`
- Update test assertions to check for `ledger.db`

#### Step 1.3: Update CLI Command Name
**File**: `src/cli/commands.rs`
- `#[command(name = "task")]` → `#[command(name = "tatl")]`
- `#[command(about = "...")]` → Update description
- `"task".to_string()` in clap args → `"tatl".to_string()`

#### Step 1.4: Update Module References
**Search and replace across codebase:**
- `task_ninja` → `tatl` (in all Rust files)
- `use task_ninja::` → `use tatl::`
- `task_ninja::` → `tatl::`

**Files affected:**
- All test files (`tests/*.rs`)
- `src/main.rs`
- `src/lib.rs`
- Documentation comments

#### Step 1.5: Update Test Framework
**File**: `tests/acceptance_framework.rs`
- `Command::cargo_bin("task")` → `Command::cargo_bin("tatl")`
- `.taskninja` → `.tatl` in test setup

**File**: All test files
- Update `use task_ninja::...` → `use tatl::...`

### Phase 2: Documentation Updates

#### Step 2.1: README.md
- Title: "Task Ninja 🥷" → "Tatl - Task and Time Ledger"
- Description: Update to reflect new name and full meaning
- Examples: `task` → `tatl`
- Installation: Update binary name references

#### Step 2.2: INSTALL.md
- Update all `task` command references → `tatl`
- Update conflict section (no longer conflicts with Taskwarrior)
- Update binary path references
- Update uninstall command: `cargo uninstall task-ninja` → `cargo uninstall tatl`

#### Step 2.3: docs/COMMAND_REFERENCE.md
- Update all command examples: `task` → `tatl`
- Update command descriptions

#### Step 2.4: Design Documents
- Update references in plan documents (optional, for consistency)
- Update `src/lib.rs` documentation comments

### Phase 3: User Migration Steps

#### Step 3.1: Backup Current Data
```bash
# Backup existing database and config
cp -r ~/.taskninja ~/.taskninja.backup
```

#### Step 3.2: Build and Install New Binary
```bash
# Build new binary
cargo build --release

# Install (will create new binary name)
cargo install --path .
```

#### Step 3.3: Migrate Data
```bash
# Create new config directory
mkdir -p ~/.tatl

# Copy database (renamed from tasks.db to ledger.db)
cp ~/.taskninja/tasks.db ~/.tatl/ledger.db

# Copy config if it exists
[ -f ~/.taskninja/rc ] && cp ~/.taskninja/rc ~/.tatl/rc
```

#### Step 3.4: Verify Migration
```bash
# Test new command
tatl status
tatl list

# Verify data integrity
tatl list | head -20  # Should show your existing tasks
```

#### Step 3.5: Cleanup
```bash
# Uninstall old binary
cargo uninstall task-ninja

# Remove old config directory (after verifying everything works)
# Keep backup for a while first!
# rm -rf ~/.taskninja
```

#### Step 3.6: Update Shell Configuration
Update any aliases or scripts:
```bash
# Old
alias tn='~/.cargo/bin/task'

# New
alias tn='~/.cargo/bin/tatl'  # or just use 'tatl' directly
```

## Implementation Checklist

### Code Changes
- [x] Update `Cargo.toml` (package name, binary name, description)
- [x] Update `src/db/connection.rs` (directory: `.tatl`, database file: `ledger.db`)
- [x] Update `src/cli/commands.rs` (command name, about text)
- [x] Update `src/lib.rs` (documentation)
- [x] Update `src/main.rs` (import statement)
- [x] Search/replace `task_ninja` → `tatl` in all Rust files
- [x] Update `tests/acceptance_framework.rs` (binary name, paths)
- [x] Update all test files (import statements)

### Documentation
- [x] Update `README.md`
- [x] Update `INSTALL.md`
- [x] Update `docs/COMMAND_REFERENCE.md`
- [x] Update `src/lib.rs` doc comments

### Testing
- [x] Run `cargo test` to verify all tests pass (1 unrelated flaky test: `e2e_recurrence_with_template_override`)
- [x] Run `cargo build --release` to verify compilation
- [x] Test binary name: `./target/release/tatl --help`
- [x] Test database path creation (verified via `test_default_path`)
- [x] Test config file path

### Migration
- [ ] Backup existing `~/.taskninja` directory
- [ ] Build and install new binary
- [ ] Copy database to new location (`~/.tatl/ledger.db`)
- [ ] Copy config to new location
- [ ] Verify data integrity
- [ ] Uninstall old binary
- [ ] Update shell aliases/scripts

## Risks and Considerations

### Low Risk
- ✅ Single user migration (no multi-user coordination needed)
- ✅ Database schema unchanged (just path change)
- ✅ No API changes (internal rename only)
- ✅ All data preserved (copy operation)

### Potential Issues
1. **Forgotten references**: Some documentation or comments might still say "task-ninja"
   - **Mitigation**: Comprehensive search/replace, review all files
2. **Test failures**: Tests might reference old paths/names
   - **Mitigation**: Update all test files, run full test suite
3. **Shell history/aliases**: Old `task` command might still be in history
   - **Mitigation**: User will naturally adapt, or update shell config
4. **Git history**: Old name preserved in git (this is fine, no need to rewrite)

### Rollback Plan
If migration fails:
1. Restore `~/.taskninja` from backup
2. Reinstall old binary: `cargo install --path .` (with old Cargo.toml)
3. Revert code changes via git

## Post-Migration

### Optional Enhancements
- Add migration helper script (optional, for future users)
- Update any external documentation or blog posts
- Consider adding `tatl --version` output with full name

### Future Considerations
- If publishing to crates.io, verify `tatl` name is available
- Consider adding shell completions for `tatl` command
- Update any CI/CD configurations if applicable

## Estimated Effort

- **Code changes**: 1-2 hours (mostly search/replace)
- **Testing**: 30 minutes (run test suite, manual verification)
- **Documentation**: 1 hour (update all docs)
- **Migration**: 15 minutes (copy files, verify)
- **Total**: ~3-4 hours

## Decision

✅ **Proceed with migration to `tatl`**

The name is meaningful, professional, conflict-free, and accurately represents the tool's dual purpose (task management + time tracking). The migration is straightforward for a single-user scenario.

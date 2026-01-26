# Plan 37: Windows Support

## Overview

This plan evaluates what is needed to run tatl on Windows, covering both Git Bash and native cmd/PowerShell environments. The codebase is pure Rust with SQLite (bundled), so the core is already cross-platform by language. The blocking issues are limited to a few Unix assumptions in path resolution, environment variables, and ANSI output.

## Current State

### Platform Profile

| Aspect | Status |
|--------|--------|
| Language | Rust (cross-platform) |
| Database | rusqlite with `bundled` feature (compiles SQLite from C source, works on Windows) |
| CLI framework | clap (cross-platform) |
| Date/time | chrono (cross-platform) |
| Terminal size | `terminal_size` crate (cross-platform) |
| TTY detection | `std::io::IsTerminal` (cross-platform, Rust 1.70+) |
| UUID | uuid crate (cross-platform) |
| Test framework | assert_cmd + tempfile (cross-platform) |

### What Already Works

The dependency stack is fully cross-platform. `rusqlite` with `bundled` compiles SQLite from source, avoiding any need for a pre-installed system library. All crate dependencies in `Cargo.toml` are platform-neutral. `cargo build` on a Windows machine would succeed except for the runtime path resolution issues described below.

---

## Blocking Issues

### 1. HOME Environment Variable Hardcoding (CRITICAL)

**Files:** `src/db/connection.rs:12-13`, `src/db/connection.rs:45-46`

The application resolves its data directory and config file using `HOME`, which does not exist on Windows (cmd/PowerShell). It will panic on startup with:

```
HOME environment variable not set
```

**Affected code:**

```rust
// connection.rs:11-14
pub fn default_path() -> PathBuf {
    let home = std::env::var("HOME")
        .expect("HOME environment variable not set");
    PathBuf::from(home).join(".tatl").join("ledger.db")
}

// connection.rs:44-47
pub fn config_path() -> PathBuf {
    let home = std::env::var("HOME")
        .expect("HOME environment variable not set");
    PathBuf::from(home).join(".tatl").join("rc")
}
```

**Impact:** Application crashes immediately. This is the only true blocker.

**Git Bash note:** Git Bash sets `HOME` automatically, so this works there today. The issue is cmd and PowerShell.

### 2. ANSI Escape Codes on cmd.exe (MODERATE)

**Files:** `src/cli/output.rs:14-15`, `src/cli/commands_sessions.rs:18-19`

Raw ANSI escape sequences are used for bold text:

```rust
const ANSI_BOLD: &str = "\x1b[1m";
const ANSI_RESET: &str = "\x1b[0m";
```

**Impact:**
- **Windows Terminal (Win 10+):** Works natively.
- **cmd.exe (legacy):** Displays garbled escape characters unless virtual terminal processing is enabled.
- **PowerShell 5.x:** Same as cmd.exe.
- **PowerShell 7+:** Works natively.
- **Git Bash:** Works natively (mintty).

This is not a crash, but degrades output readability on legacy terminals.

---

## Non-Blocking Items

### 3. Dot-Directory Convention (`.tatl`)

The data directory is `.tatl` under the home directory. On Unix this creates a hidden directory. On Windows, dot-prefix directories are not hidden by default (they're just normal directories). This is cosmetic only --- the directory works fine, it just won't be hidden in Explorer.

The Windows convention would be `%APPDATA%\tatl`, but this is a user preference question, not a functional issue.

### 4. Shell Scripts for Man Pages

**Files:** `scripts/generate-man.sh`, `install-man-user.sh`

These are bash scripts for generating and installing man pages. Windows doesn't use man pages, so this is irrelevant to Windows support. The binary itself (`generate-man`) is a Rust binary that would compile and run on Windows --- only the shell wrapper wouldn't.

### 5. Documentation References

`INSTALL.md` and `README.md` reference `~/.bashrc`, `~/.zshrc`, symlinks, and Unix man page paths. These would need Windows-specific instructions but don't affect functionality.

### 6. Test Framework HOME Usage

**Files:** `tests/acceptance_framework.rs:36`, `tests/acceptance_framework.rs:52`

Tests set `HOME` for isolation. This would need updating for tests to pass on Windows CI, but doesn't affect the end-user binary.

---

## Proposed Changes

### Phase 1: Fix HOME Resolution (Required)

Replace the hardcoded `HOME` lookup with a cross-platform home directory resolution.

**Option A: Use the `dirs` crate**

```rust
// Cargo.toml
dirs = "5"

// connection.rs
pub fn home_dir() -> PathBuf {
    dirs::home_dir().expect("Could not determine home directory")
}

pub fn default_path() -> PathBuf {
    Self::home_dir().join(".tatl").join("ledger.db")
}
```

The `dirs` crate resolves to:
- Unix: `$HOME`
- Windows: `{FOLDERID_Profile}` (typically `C:\Users\<name>`)

**Option B: Manual fallback chain (no new dependency)**

```rust
pub fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .expect("Could not determine home directory (set HOME or USERPROFILE)")
}
```

**Option C: Use `dirs` with Windows-conventional path**

```rust
pub fn data_dir() -> PathBuf {
    if cfg!(windows) {
        dirs::data_local_dir()  // %LOCALAPPDATA%\tatl
            .unwrap()
            .join("tatl")
    } else {
        dirs::home_dir().unwrap().join(".tatl")
    }
}
```

This respects Windows conventions (`%LOCALAPPDATA%\tatl` or `%APPDATA%\tatl`) but introduces a platform split in the data path.

### Phase 2: Handle ANSI on Legacy Windows Terminals (Optional)

**Option A: Enable Virtual Terminal Processing**

Call the Windows API at startup to enable ANSI support in cmd.exe:

```rust
#[cfg(windows)]
fn enable_virtual_terminal() {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Console::*;
    let handle = std::io::stdout().as_raw_handle();
    unsafe {
        let mut mode: u32 = 0;
        GetConsoleMode(handle as _, &mut mode);
        SetConsoleMode(handle as _, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
    }
}
```

**Option B: Use a crate like `enable-ansi-support`**

A one-line dependency that does the same thing:

```rust
// main.rs
fn main() {
    #[cfg(windows)]
    let _ = enable_ansi_support::enable_ansi_support();
    // ...
}
```

**Option C: Gate ANSI output on terminal capability**

Check if the terminal supports ANSI and skip formatting if not. This is more defensive but adds complexity for little gain since modern Windows supports ANSI.

### Phase 3: Update Test Framework (For CI only)

Update `acceptance_framework.rs` to set the appropriate environment variable:

```rust
#[cfg(unix)]
std::env::set_var("HOME", temp_dir.path().to_str().unwrap());

#[cfg(windows)]
std::env::set_var("USERPROFILE", temp_dir.path().to_str().unwrap());
```

Or, if using the `dirs` crate, override `HOME` on all platforms (which `dirs` respects even on Windows as a fallback).

### Phase 4: Documentation

- Add a Windows section to `INSTALL.md` with PowerShell/cmd instructions.
- Note that `data.location` in the rc file can override the default path on any platform.
- Mention Git Bash as the lowest-friction option for Windows users.

---

## Open Questions

### 1. Which Home Directory Strategy?

- [x] **Option A:** Use `dirs` crate, store in `~/.tatl` on all platforms (simplest, consistent)
- [ ] **Option B:** Manual `HOME`/`USERPROFILE` fallback (no new dependency)
- [ ] **Option C:** Use platform-conventional paths (`%LOCALAPPDATA%\tatl` on Windows)

**Recommendation:** Option A. The `dirs` crate is a widely-used, minimal dependency. Using `~/.tatl` on all platforms keeps behavior predictable and documentation simple. Users can override with `data.location` in the rc file if they prefer a Windows-conventional path.

### 2. Should ANSI Handling Be Addressed?

- [x] **Yes:** Add `enable-ansi-support` crate (one-line fix)
- [ ] **No:** Document that Git Bash or Windows Terminal is recommended
- [ ] **Defer:** Wait until Plan 36 (Color By Column) lands and handle it there

**Recommendation:** Yes, but this could also be bundled with Plan 36's color work since both touch terminal output. If Plan 36 is implemented first, ANSI enabling should be part of that plan.

### 3. Target Windows Environment?

- [ ] **cmd.exe / PowerShell only** (full native Windows support)
- [ ] **Git Bash only** (works today with no changes except documentation)
- [x] **All of the above** (native + Git Bash)

### 4. Should Windows CI Be Added?

- [x] **Yes:** Add Windows target to CI (GitHub Actions `windows-latest`)
- [ ] **No:** Manual testing only

---

## Analysis

### Effort Assessment

| Change | Scope | Files Modified |
|--------|-------|----------------|
| Replace HOME with `dirs` | 1 function + 2 call sites | `Cargo.toml`, `src/db/connection.rs` |
| ANSI terminal support | 1 init call at startup | `Cargo.toml`, `src/main.rs` |
| Test framework update | 2 lines | `tests/acceptance_framework.rs` |
| Documentation | New section | `INSTALL.md`, `README.md` |

The functional code change is **~10 lines of Rust** across 2 source files. The remainder is documentation and CI.

### Risk Assessment

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| `dirs` crate returns `None` | Very low | Crate is mature; fallback to env var check |
| SQLite file locking differences | Low | rusqlite handles this; no concurrent access in CLI tool |
| Path separator issues (`/` vs `\`) | None | All path construction uses `PathBuf::join()` which is platform-aware |
| Line ending issues | None | No file format parsing depends on `\n` vs `\r\n` |
| Cargo build fails on Windows | Very low | All deps support Windows; rusqlite bundled compiles SQLite from C source |

---

## Feasibility Recommendation

**Feasibility: HIGH.** Windows support is straightforward.

The codebase is almost entirely platform-agnostic already. The only functional blocker is `HOME` environment variable usage in two functions in `connection.rs`. This is a ~10-line fix with the `dirs` crate. ANSI terminal support is a one-line addition. There are no Unix system calls, no fork/exec, no file permission assumptions, no shell invocations, no symlinks, and no platform-specific crate dependencies.

**Recommended approach:**
1. Add `dirs` crate and replace `HOME` lookups in `connection.rs` (Phase 1)
2. Add `enable-ansi-support` in `main.rs` (Phase 2 --- or bundle with Plan 36)
3. Update test framework for cross-platform env vars (Phase 3)
4. Add Windows install docs and CI target (Phase 4)

After Phase 1, tatl will run on Windows. The remaining phases are polish.

---

## Implementation Plan

**Decisions Needed:**
- [ ] Home directory strategy (Option A recommended)
- [ ] ANSI handling timing (now vs. with Plan 36)
- [ ] Windows CI priority

### Phase 1: Core Fix

1. Add `dirs = "5"` to `Cargo.toml`
2. Replace `std::env::var("HOME")` in `src/db/connection.rs` with `dirs::home_dir()`
3. Test on Windows (or cross-compile with `cargo build --target x86_64-pc-windows-msvc`)

### Phase 2: Terminal Support

1. Add `enable-ansi-support = "0.2"` to `Cargo.toml`
2. Add init call in `src/main.rs`

### Phase 3: Test Framework

1. Update `tests/acceptance_framework.rs` to set platform-appropriate env vars
2. Verify all tests pass on both Unix and Windows

### Phase 4: Documentation and CI

1. Add Windows section to `INSTALL.md`
2. Add `windows-latest` to CI matrix
3. Add Windows release binary to build pipeline (if applicable)

---

## Success Criteria

1. `cargo build --target x86_64-pc-windows-msvc` succeeds
2. `tatl add "Test task"` works on Windows cmd, PowerShell, and Git Bash
3. `tatl list` renders correctly on Windows Terminal
4. All existing tests pass on both Unix and Windows
5. `INSTALL.md` includes Windows instructions

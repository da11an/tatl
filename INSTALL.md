# Installation Guide

## Quick Install

```bash
# Build and install
cargo build --release
cargo install --path .
```

This installs the `task` binary to `~/.cargo/bin/task`.

## Handling Taskwarrior Conflicts

If you have Taskwarrior installed (which also uses the `task` command), you have several options:

### Option 1: Use Full Path (Recommended for Testing)

```bash
# Use the full path when you want Task Ninja
~/.cargo/bin/task stack show

# Or create a temporary alias in your current shell
alias tn='~/.cargo/bin/task'
tn stack show
```

### Option 2: Prioritize Task Ninja in PATH

Add `~/.cargo/bin` to the **beginning** of your PATH in `~/.bashrc` or `~/.zshrc`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Then reload your shell:
```bash
source ~/.bashrc  # or source ~/.zshrc
```

**Note:** This will make Task Ninja take precedence over Taskwarrior. To use Taskwarrior, use its full path.

### Option 3: Create an Alias

Add to `~/.bashrc` or `~/.zshrc`:

```bash
# Task Ninja alias
alias task-ninja='~/.cargo/bin/task'
alias tn='~/.cargo/bin/task'
```

Then reload your shell:
```bash
source ~/.bashrc  # or source ~/.zshrc
```

Usage:
```bash
task-ninja stack show
tn stack show
```

### Option 4: Rename the Binary

If you prefer a different name, modify `Cargo.toml`:

```toml
[[bin]]
name = "taskninja"  # or "tn", "task-ninja", etc.
path = "src/main.rs"
```

Then rebuild and install:
```bash
cargo build --release
cargo install --path .
```

### Option 5: Local Development (No Installation)

For testing without installing:

```bash
# Build release version
cargo build --release

# Use directly
./target/release/task stack show

# Or create a symlink in a local bin directory
mkdir -p ~/bin
ln -s $(pwd)/target/release/task ~/bin/task-ninja
export PATH="$HOME/bin:$PATH"
```

## Verify Installation

```bash
# Check if installed
which task
~/.cargo/bin/task stack show

# Check version (if --version is implemented)
~/.cargo/bin/task --help
```

## Uninstall

```bash
cargo uninstall task-ninja
```

## Troubleshooting

**Problem:** `task` command not found after installation

**Solution:** Make sure `~/.cargo/bin` is in your PATH:
```bash
echo $PATH | grep cargo
```

If not, add it to your shell config file.

**Problem:** Wrong `task` command runs (Taskwarrior instead of Task Ninja)

**Solution:** 
- Check which one is first in PATH: `which task`
- Use full path: `~/.cargo/bin/task`
- Or use an alias as described above

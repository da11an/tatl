# Installation Guide

## Quick Install

```bash
# Build and install
cargo build --release
cargo install --path .
```

This installs the `tatl` binary to `~/.cargo/bin/tatl`.

## Note: No Conflicts with Taskwarrior

The `tatl` command does not conflict with Taskwarrior's `task` command, so you can use both tools simultaneously without any special configuration.

## Verify Installation

```bash
# Check if installed
which tatl
~/.cargo/bin/tatl status

# Check version
~/.cargo/bin/tatl --help
```

## Uninstall

```bash
cargo uninstall tatl
```

## Local Development (No Installation)

For testing without installing:

```bash
# Build release version
cargo build --release

# Use directly
./target/release/tatl status

# Or create a symlink in a local bin directory
mkdir -p ~/bin
ln -s $(pwd)/target/release/tatl ~/bin/tatl
export PATH="$HOME/bin:$PATH"
```

## Troubleshooting

**Problem:** `tatl` command not found after installation

**Solution:** Make sure `~/.cargo/bin` is in your PATH:
```bash
echo $PATH | grep cargo
```

If not, add it to your shell config file (`~/.bashrc` or `~/.zshrc`):
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Then reload your shell:
```bash
source ~/.bashrc  # or source ~/.zshrc
```

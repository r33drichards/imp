# Git Hooks

This directory contains git hooks for the project to maintain code quality.

## Installation

To install the git hooks, run:

```bash
./hooks/install.sh
```

## Available Hooks

### pre-commit

The pre-commit hook runs `cargo fmt --check` to ensure all Rust code is properly formatted before allowing a commit.

If the check fails, the commit will be rejected. To fix formatting issues, run:

```bash
cargo fmt
```

Then try your commit again.

## Manual Installation

If you prefer to install hooks manually, copy them from this directory to `.git/hooks/`:

```bash
cp hooks/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

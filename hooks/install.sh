#!/bin/bash
#
# Script to install git hooks for this repository

set -e

HOOKS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GIT_HOOKS_DIR="$(git rev-parse --git-dir)/hooks"

echo "Installing git hooks..."

# Copy pre-commit hook
cp "${HOOKS_DIR}/pre-commit" "${GIT_HOOKS_DIR}/pre-commit"
chmod +x "${GIT_HOOKS_DIR}/pre-commit"

echo "✅ Git hooks installed successfully!"
echo ""
echo "The following hooks have been installed:"
echo "  - pre-commit: Checks Rust code formatting with cargo fmt"
echo ""
echo "To format your code, run: cargo fmt"

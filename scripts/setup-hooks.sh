#!/usr/bin/env bash
# Set up git hooks for the Soul Vault repository.
# Run once after cloning: ./scripts/setup-hooks.sh

set -e

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "Setting up git hooks..."
git config core.hooksPath "$REPO_ROOT/.githooks"
chmod +x "$REPO_ROOT/.githooks/pre-commit"
echo "✓ Git hooks configured (using .githooks/)"

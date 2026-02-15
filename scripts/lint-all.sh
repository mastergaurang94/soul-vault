#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

steps=(
  "scripts/lint-architecture.sh"
  "scripts/lint-file-size.sh"
  "scripts/lint-unwrap.sh"
  "cargo clippy --all-targets -- -D warnings"
  "cargo fmt -- --check"
)

failures=0

run_step() {
  local label="$1"
  local cmd="$2"

  echo
  echo "==> $label"
  if bash -lc "$cmd"; then
    echo "PASS: $label"
  else
    echo "FAIL: $label"
    failures=$((failures + 1))
  fi
}

cd "$ROOT_DIR"

run_step "Architecture lint" "scripts/lint-architecture.sh"
run_step "File-size lint" "scripts/lint-file-size.sh"
run_step "Unwrap/process-exit lint" "scripts/lint-unwrap.sh"
run_step "Clippy" "cargo clippy --all-targets -- -D warnings"
run_step "Fmt check" "cargo fmt -- --check"

echo
if [[ "$failures" -eq 0 ]]; then
  echo "Quality gate passed: all lints and Rust checks succeeded."
  exit 0
fi

echo "Quality gate failed: $failures check(s) failed."
echo "Remediation: run each failing command above, apply fixes, then re-run scripts/lint-all.sh."
exit 1

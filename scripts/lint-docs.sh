#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

failures=0

pass() {
  printf 'PASS: %s\n' "$1"
}

fail() {
  printf 'FAIL: %s\n' "$1"
  failures=$((failures + 1))
}

printf 'Running docs lint checks...\n'

# 1) Every Rust source file has a top-level //! module doc comment.
missing_module_docs=()
while IFS= read -r file; do
  first_nonempty="$(awk 'NF { print; exit }' "$file")"
  if [[ ! "$first_nonempty" =~ ^//! ]]; then
    missing_module_docs+=("$file")
  fi
done < <(find src -type f -name '*.rs' | sort)

if ((${#missing_module_docs[@]} == 0)); then
  pass "All src/*.rs files have top-level //! module docs"
else
  fail "Missing top-level //! module docs in ${#missing_module_docs[@]} file(s)"
  printf '  - %s\n' "${missing_module_docs[@]}"
fi

# 2) Every markdown doc under docs/ has a Last updated marker.
missing_last_updated=()
while IFS= read -r file; do
  if ! rg -q '^Last updated: [0-9]{4}-[0-9]{2}-[0-9]{2}$' "$file"; then
    missing_last_updated+=("$file")
  fi
done < <(find docs -type f -name '*.md' | sort)

if ((${#missing_last_updated[@]} == 0)); then
  pass "All docs markdown files include a Last updated marker"
else
  fail "Missing Last updated marker in ${#missing_last_updated[@]} doc file(s)"
  printf '  - %s\n' "${missing_last_updated[@]}"
fi

# 3) No broken internal markdown links in docs/.
broken_links=()
while IFS= read -r file; do
  while IFS= read -r target; do
    clean_target="${target%%#*}"
    clean_target="${clean_target%%\?*}"

    if [[ -z "$clean_target" ]]; then
      continue
    fi

    if [[ "$clean_target" =~ ^(https?://|mailto:|tel:) ]]; then
      continue
    fi

    if [[ "$clean_target" != *.md ]]; then
      continue
    fi

    if [[ "$clean_target" == /* ]]; then
      candidate="$ROOT_DIR$clean_target"
    else
      candidate="$(cd "$(dirname "$file")" && pwd)/$clean_target"
    fi

    if [[ ! -f "$candidate" ]]; then
      broken_links+=("$file -> $target")
    fi
  done < <(sed -nE 's/.*\[[^]]+\]\(([^)]+)\).*/\1/p' "$file")
done < <(find docs -type f -name '*.md' | sort)

if ((${#broken_links[@]} == 0)); then
  pass "No broken internal markdown links in docs/"
else
  fail "Broken internal markdown links found (${#broken_links[@]})"
  printf '  - %s\n' "${broken_links[@]}"
fi

# 4) AGENTS.md references all stable docs markdown files.
missing_in_agents=()
while IFS= read -r file; do
  rel="${file#./}"

  case "$rel" in
    docs/inbox/*|docs/plans/completed/*)
      continue
      ;;
  esac

  if ! rg -q "${rel//./\\.}" AGENTS.md; then
    missing_in_agents+=("$rel")
  fi
done < <(find ./docs -type f -name '*.md' | sort)

if ((${#missing_in_agents[@]} == 0)); then
  pass "AGENTS.md references all stable docs markdown files"
else
  fail "AGENTS.md is missing references to ${#missing_in_agents[@]} doc file(s)"
  printf '  - %s\n' "${missing_in_agents[@]}"
fi

if ((failures > 0)); then
  printf '\nDoc lint failed with %d issue group(s).\n' "$failures"
  exit 1
fi

printf '\nDoc lint passed.\n'

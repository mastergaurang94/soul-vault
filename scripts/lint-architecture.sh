#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SRC_DIR="$ROOT_DIR/src"
KNOWN_MODULES="adapters auth cli core extractors tui types ui vault main"

if [[ ! -d "$SRC_DIR" ]]; then
  echo "Architecture lint could not find src/ directory at $SRC_DIR."
  echo "Remediation: run this script from the Soul Vault repository root."
  exit 1
fi

module_for_file() {
  local file="$1"
  local rel
  rel="${file#"$SRC_DIR"/}"

  if [[ "$rel" == "main.rs" ]]; then
    echo "main"
    return
  fi

  local top="${rel%%/*}"
  echo "$top"
}

allowed_modules_for() {
  local module="$1"

  case "$module" in
    types)
      echo ""
      ;;
    ui)
      echo ""
      ;;
    vault)
      echo "types"
      ;;
    auth)
      echo "types vault"
      ;;
    extractors)
      echo "types"
      ;;
    adapters)
      echo "types vault"
      ;;
    core)
      echo "types vault ui"
      ;;
    tui)
      echo "core vault ui auth adapters types"
      ;;
    cli)
      echo "core vault extractors adapters auth ui types"
      ;;
    main)
      echo "cli tui ui"
      ;;
    *)
      echo ""
      ;;
  esac
}

in_allowed_list() {
  local target="$1"
  shift
  local item
  for item in "$@"; do
    if [[ "$item" == "$target" ]]; then
      return 0
    fi
  done
  return 1
}

extract_import_targets() {
  local file="$1"
  local modules="$2"

  sed -E 's,//.*$,,' "$file" | awk -v modules="$modules" '
    BEGIN {
      split(modules, m, /[[:space:]]+/)
    }

    {
      while (match($0, /use[[:space:]]+crate::([^;]+);/)) {
        stmt = substr($0, RSTART, RLENGTH)
        body = stmt
        sub(/^use[[:space:]]+crate::/, "", body)
        sub(/;$/, "", body)

        delete seen
        for (i in m) {
          mod = m[i]
          if (mod == "") {
            continue
          }

          # direct path: use crate::mod::...
          direct = "^" mod "(::|$)"
          # grouped import: use crate::{mod, mod::..., other::...}
          grouped = "(^|[,{[:space:]])" mod "([[:space:]]*::|[[:space:]]*[,}])"

          if (body ~ direct || body ~ grouped) {
            seen[mod] = 1
          }
        }

        for (mod in seen) {
          print NR ":" mod
        }

        $0 = substr($0, RSTART + RLENGTH)
      }
    }
  '
}

violations=0
report=""

while IFS= read -r file; do
  module="$(module_for_file "$file")"
  allowed_str="$(allowed_modules_for "$module")"
  # shellcheck disable=SC2206
  allowed=( $allowed_str )

  while IFS= read -r hit; do
    [[ -z "$hit" ]] && continue
    line="${hit%%:*}"
    target="${hit#*:}"

    if [[ "$target" == "$module" ]]; then
      continue
    fi

    if [[ "$module" == "main" ]]; then
      if ! in_allowed_list "$target" "${allowed[@]}"; then
        violations=$((violations + 1))
        rel="${file#"$ROOT_DIR"/}"
        report+="\n[$violations] $rel:$line imports crate::$target, which main.rs must not depend on.\n"
        report+="    Fix: route this call through crate::cli, crate::tui, or crate::ui (per ARCHITECTURE.md).\n"
      fi
      continue
    fi

    if [[ "$target" == "main" ]]; then
      violations=$((violations + 1))
      rel="${file#"$ROOT_DIR"/}"
      report+="\n[$violations] $rel:$line imports crate::main, which is not allowed from library modules.\n"
      report+="    Fix: move shared logic into an allowed module and import that instead.\n"
      continue
    fi

    if ! in_allowed_list "$target" "${allowed[@]}"; then
      violations=$((violations + 1))
      rel="${file#"$ROOT_DIR"/}"
      report+="\n[$violations] $rel:$line imports crate::$target, but module '$module' may only depend on: ${allowed_str:-<none>}.\n"
      report+="    Fix: move code behind an allowed boundary (often types/, vault/, or core/) and import from there.\n"
    fi
  done < <(extract_import_targets "$file" "$KNOWN_MODULES")
done < <(find "$SRC_DIR" -type f -name '*.rs' | sort)

if [[ "$violations" -gt 0 ]]; then
  echo "Architecture lint failed with $violations violation(s)."
  printf '%b' "$report"
  exit 1
fi

echo "Architecture lint passed: dependency direction is clean."

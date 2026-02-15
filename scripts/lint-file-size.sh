#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
LIMIT=250

usage() {
  cat <<USAGE
Usage: scripts/lint-file-size.sh [--limit N]

Checks Rust source files under src/ for line-count limit violations.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --limit)
      if [[ $# -lt 2 ]]; then
        echo "Missing value for --limit."
        echo "Remediation: pass a positive integer, e.g. --limit 220."
        exit 1
      fi
      LIMIT="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1"
      usage
      echo "Remediation: use only --limit N."
      exit 1
      ;;
  esac
done

if ! [[ "$LIMIT" =~ ^[0-9]+$ ]] || [[ "$LIMIT" -le 0 ]]; then
  echo "Invalid limit '$LIMIT'."
  echo "Remediation: --limit must be a positive integer."
  exit 1
fi

violations=0

while IFS= read -r file; do
  lines=$(wc -l < "$file" | tr -d ' ')
  if [[ "$lines" -gt "$LIMIT" ]]; then
    violations=$((violations + 1))
    rel="${file#"$ROOT_DIR"/}"
    echo "[$violations] $rel has $lines lines (limit: $LIMIT)."
    echo "    Fix: split the file by concern (target ~200 lines per docs/DESIGN_PRINCIPLES.md)."
  fi
done < <(find "$ROOT_DIR/src" -type f -name '*.rs' | sort)

if [[ "$violations" -gt 0 ]]; then
  echo "File-size lint failed with $violations violation(s)."
  exit 1
fi

echo "File-size lint passed: all files are <= $LIMIT lines."

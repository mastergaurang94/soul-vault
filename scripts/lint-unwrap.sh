#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SRC_DIR="$ROOT_DIR/src"

violations=0

scan_unwraps() {
  local file="$1"
  awk '
    BEGIN {
      in_test_attr = 0
      in_test_scope = 0
      test_depth = -1
      depth = 0
    }

    {
      line = $0

      if (line ~ /^[[:space:]]*#[[:space:]]*\[[[:space:]]*cfg[[:space:]]*\([[:space:]]*test[[:space:]]*\)[[:space:]]*\]/) {
        in_test_attr = 1
      }

      if (in_test_attr == 1 && line ~ /\{/) {
        in_test_scope = 1
        test_depth = depth
        in_test_attr = 0
      }

      if (in_test_scope == 0 && line ~ /\.unwrap[[:space:]]*\(\)/ && line !~ /^[[:space:]]*\/\//) {
        print NR ":" line
      }

      opens = gsub(/\{/, "{", line)
      closes = gsub(/\}/, "}", line)
      depth += opens - closes

      if (in_test_scope == 1 && depth <= test_depth) {
        in_test_scope = 0
        test_depth = -1
      }
    }
  ' "$file"
}

while IFS= read -r file; do
  while IFS= read -r hit; do
    [[ -z "$hit" ]] && continue
    line_no="${hit%%:*}"
    context="${hit#*:}"
    rel="${file#"$ROOT_DIR"/}"
    violations=$((violations + 1))
    echo "[$violations] $rel:$line_no uses .unwrap() in non-test code."
    echo "    Context: $context"
    echo "    Fix: propagate errors with anyhow::Result, use ?, or provide safe fallback handling."
  done < <(scan_unwraps "$file")
done < <(find "$SRC_DIR" -type f -name '*.rs' | sort)

while IFS= read -r hit; do
  [[ -z "$hit" ]] && continue
  file_path="${hit%%:*}"
  rest="${hit#*:}"
  line_no="${rest%%:*}"
  context="${rest#*:}"

  if [[ "$file_path" == "$SRC_DIR/main.rs" ]]; then
    continue
  fi

  rel="${file_path#"$ROOT_DIR"/}"
  violations=$((violations + 1))
  echo "[$violations] $rel:$line_no calls process::exit outside main.rs."
  echo "    Context: $context"
  echo "    Fix: return an error (anyhow::bail! / Result) and let main.rs decide the exit code."
done < <(rg -n "\b(process::exit|std::process::exit)\s*\(" "$SRC_DIR" --glob '*.rs')

if [[ "$violations" -gt 0 ]]; then
  echo "unwrap/process-exit lint failed with $violations violation(s)."
  exit 1
fi

echo "unwrap/process-exit lint passed: no unsafe unwraps and no process::exit outside main.rs."

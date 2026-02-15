#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/analyze-tui.sh [options]

Capture a series of tmux pane snapshots over time to analyze TUI state changes.

Options:
  --command CMD       Command to run in tmux (default: soul)
  --captures N        Number of captures to take (default: 5)
  --interval SECONDS  Delay between captures (default: 2)
  --keys KEYS         tmux keys sent between captures (example: "2" or "C-n")
  --out-dir DIR       Output directory (default: artifacts/analyze-<timestamp>)
  --help              Show this help message
USAGE
}

error() {
  echo "Error: $*" >&2
  exit 1
}

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    error "Missing dependency '$1'. Install it, then retry."
  fi
}

command_to_run="soul"
captures="5"
interval="2"
keys=""
out_dir=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --command)
      [[ $# -ge 2 ]] || error "--command requires a value."
      command_to_run="$2"
      shift 2
      ;;
    --captures)
      [[ $# -ge 2 ]] || error "--captures requires a value."
      captures="$2"
      shift 2
      ;;
    --interval)
      [[ $# -ge 2 ]] || error "--interval requires a value."
      interval="$2"
      shift 2
      ;;
    --keys)
      [[ $# -ge 2 ]] || error "--keys requires a value."
      keys="$2"
      shift 2
      ;;
    --out-dir)
      [[ $# -ge 2 ]] || error "--out-dir requires a value."
      out_dir="$2"
      shift 2
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      error "Unknown option: $1. Run with --help for usage."
      ;;
  esac
done

if ! [[ "$captures" =~ ^[1-9][0-9]*$ ]]; then
  error "--captures must be a positive integer."
fi

if ! [[ "$interval" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
  error "--interval must be a positive number of seconds."
fi

need_cmd tmux

if [[ -z "$out_dir" ]]; then
  ts="$(date +%Y%m%d-%H%M%S)"
  out_dir="artifacts/analyze-${ts}"
fi

mkdir -p "$out_dir"
index_file="$out_dir/index.txt"

session_name="sv-analyze-${RANDOM}-$$"
socket_root="${SOUL_TMUX_SOCKET_DIR:-.tmux-sockets}"
socket_path="${SOUL_TMUX_SOCKET:-${socket_root%/}/soul-vault-${session_name}.sock}"
tmux_cmd=(tmux -S "$socket_path")

cleanup() {
  set +e
  if "${tmux_cmd[@]}" has-session -t "$session_name" >/dev/null 2>&1; then
    "${tmux_cmd[@]}" kill-session -t "$session_name" >/dev/null 2>&1
  fi
  "${tmux_cmd[@]}" kill-server >/dev/null 2>&1 || true
  rm -f "$socket_path" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

mkdir -p "$socket_root"
"${tmux_cmd[@]}" new-session -d -s "$session_name"
"${tmux_cmd[@]}" set-option -t "$session_name" remain-on-exit on >/dev/null
# Simulate user-entered command to keep interactive behavior consistent.
"${tmux_cmd[@]}" send-keys -t "${session_name}:0.0" "$command_to_run" C-m

echo "command: $command_to_run" > "$index_file"
echo "captures: $captures" >> "$index_file"
echo "interval_seconds: $interval" >> "$index_file"
echo "keys_between_captures: ${keys:-<none>}" >> "$index_file"
echo "" >> "$index_file"

for i in $(seq 1 "$captures"); do
  sleep "$interval"
  capture_file=$(printf "%s/capture-%02d.txt" "$out_dir" "$i")
  timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  "${tmux_cmd[@]}" capture-pane -p -t "${session_name}:0.0" -S - > "$capture_file"
  echo "$timestamp $capture_file" >> "$index_file"

  if [[ -n "$keys" && "$i" -lt "$captures" ]]; then
    read -r -a key_parts <<< "$keys"
    "${tmux_cmd[@]}" send-keys -t "${session_name}:0.0" "${key_parts[@]}"
  fi
done

echo "Analysis snapshots written to: $out_dir"
echo "Capture index: $index_file"

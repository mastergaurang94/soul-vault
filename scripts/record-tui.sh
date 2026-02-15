#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/record-tui.sh [options]

Record terminal output from a tmux-hosted TUI session.

Options:
  --duration SECONDS  Recording duration before auto-stop (default: 10)
  --out PATH          Output typescript file path
  --command CMD       Command to run in tmux (default: soul)
  --help              Show this help message

Notes:
  - Creates a timing file next to output: <out>.timing
  - Replay with: script -p <out>.timing <out>
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

duration="10"
out_file=""
command_to_run="soul"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --duration)
      [[ $# -ge 2 ]] || error "--duration requires a value."
      duration="$2"
      shift 2
      ;;
    --out)
      [[ $# -ge 2 ]] || error "--out requires a value."
      out_file="$2"
      shift 2
      ;;
    --command)
      [[ $# -ge 2 ]] || error "--command requires a value."
      command_to_run="$2"
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

if ! [[ "$duration" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
  error "--duration must be a positive number of seconds."
fi

need_cmd tmux
need_cmd script

if [[ -z "$out_file" ]]; then
  ts="$(date +%Y%m%d-%H%M%S)"
  out_file="artifacts/recordings/tui-${ts}.typescript"
fi

mkdir -p "$(dirname "$out_file")"
timing_file="${out_file}.timing"
session_name="sv-record-${RANDOM}-$$"
socket_root="${SOUL_TMUX_SOCKET_DIR:-.tmux-sockets}"
socket_path="${SOUL_TMUX_SOCKET:-${socket_root%/}/soul-vault-${session_name}.sock}"
tmux_cmd=(tmux -S "$socket_path")
script_pid=""

cleanup() {
  set +e
  if "${tmux_cmd[@]}" has-session -t "$session_name" >/dev/null 2>&1; then
    "${tmux_cmd[@]}" kill-session -t "$session_name" >/dev/null 2>&1
  fi
  "${tmux_cmd[@]}" kill-server >/dev/null 2>&1 || true
  rm -f "$socket_path" >/dev/null 2>&1 || true
  if [[ -n "$script_pid" ]]; then
    kill "$script_pid" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT INT TERM

mkdir -p "$socket_root"
"${tmux_cmd[@]}" new-session -d -s "$session_name"
"${tmux_cmd[@]}" set-option -t "$session_name" remain-on-exit on >/dev/null
# Send the command after session setup so short-lived commands are still visible.
"${tmux_cmd[@]}" send-keys -t "${session_name}:0.0" "$command_to_run" C-m

script -q -t "$timing_file" "$out_file" tmux -S "$socket_path" attach-session -t "$session_name" >/dev/null 2>&1 &
script_pid="$!"

sleep "$duration"

if "${tmux_cmd[@]}" has-session -t "$session_name" >/dev/null 2>&1; then
  "${tmux_cmd[@]}" kill-session -t "$session_name" >/dev/null 2>&1 || true
fi
wait "$script_pid" 2>/dev/null || true

echo "Recorded session: $out_file"
echo "Timing data:      $timing_file"
echo "Replay command:   script -p '$timing_file' '$out_file'"

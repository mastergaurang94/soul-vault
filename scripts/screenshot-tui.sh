#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/screenshot-tui.sh [options]

Capture a single tmux pane snapshot from a command-driven terminal UI.

Options:
  --wait SECONDS      Wait before capture (default: 2)
  --out PATH          Output snapshot file path
  --format FORMAT     text or ansi (default: text)
  --command CMD       Command to run in tmux (default: soul)
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

wait_seconds="2"
out_file=""
format="text"
command_to_run="soul"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --wait)
      [[ $# -ge 2 ]] || error "--wait requires a value."
      wait_seconds="$2"
      shift 2
      ;;
    --out)
      [[ $# -ge 2 ]] || error "--out requires a value."
      out_file="$2"
      shift 2
      ;;
    --format)
      [[ $# -ge 2 ]] || error "--format requires a value."
      format="$2"
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

if ! [[ "$wait_seconds" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
  error "--wait must be a positive number of seconds."
fi

if [[ "$format" != "text" && "$format" != "ansi" ]]; then
  error "--format must be either 'text' or 'ansi'."
fi

need_cmd tmux

if [[ -z "$out_file" ]]; then
  ts="$(date +%Y%m%d-%H%M%S)"
  ext="txt"
  if [[ "$format" == "ansi" ]]; then
    ext="ansi"
  fi
  out_file="artifacts/screenshots/snapshot-${ts}.${ext}"
fi

mkdir -p "$(dirname "$out_file")"
session_name="sv-shot-${RANDOM}-$$"
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
# Keep shell context alive and run command as if entered by a user.
"${tmux_cmd[@]}" send-keys -t "${session_name}:0.0" "$command_to_run" C-m

sleep "$wait_seconds"

capture_args=(-p -t "${session_name}:0.0" -S -)
if [[ "$format" == "ansi" ]]; then
  capture_args=(-e "${capture_args[@]}")
fi

"${tmux_cmd[@]}" capture-pane "${capture_args[@]}" > "$out_file"

echo "Captured snapshot: $out_file"

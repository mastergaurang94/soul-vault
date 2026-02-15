#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/extract-frames.sh --input VIDEO [options]

Extract JPG frames from a video at fixed intervals.

Options:
  --input PATH        Input video file (required)
  --interval SECONDS  Frame interval in seconds (default: 1)
  --out-dir DIR       Output directory (default: artifacts/frames-<timestamp>)
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

input_file=""
interval="1"
out_dir=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --input)
      [[ $# -ge 2 ]] || error "--input requires a value."
      input_file="$2"
      shift 2
      ;;
    --interval)
      [[ $# -ge 2 ]] || error "--interval requires a value."
      interval="$2"
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

[[ -n "$input_file" ]] || error "--input is required."
[[ -f "$input_file" ]] || error "Input file not found: $input_file"

if ! [[ "$interval" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
  error "--interval must be a positive number of seconds."
fi

need_cmd ffmpeg

if [[ -z "$out_dir" ]]; then
  ts="$(date +%Y%m%d-%H%M%S)"
  out_dir="artifacts/frames-${ts}"
fi

mkdir -p "$out_dir"

ffmpeg \
  -hide_banner \
  -loglevel error \
  -i "$input_file" \
  -vf "fps=1/${interval}" \
  "$out_dir/frame-%05d.jpg"

echo "Frames extracted to: $out_dir"

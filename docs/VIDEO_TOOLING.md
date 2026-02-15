# Video Tooling for Agent Development

This project includes zero-dependency terminal capture tooling built on:
- `tmux`
- `script` (macOS built-in)
- `ffmpeg`

The goal is to make TUI behavior observable for debugging, review, and automation.

## Scripts

### `scripts/record-tui.sh`
Record a tmux-hosted session into a replayable terminal recording.

What it outputs:
- `typescript` file: terminal output stream
- `timing` file: timing metadata for replay

Options:
- `--duration SECONDS` (default: `10`)
- `--out PATH`
- `--command CMD` (default: `soul`)

Example:
```bash
scripts/record-tui.sh --duration 15 --command soul --out artifacts/recordings/soul.typescript
script -p artifacts/recordings/soul.typescript.timing artifacts/recordings/soul.typescript
```

Use this when:
- You need a full timeline of UI behavior
- You want to replay what an agent saw during a run

### `scripts/screenshot-tui.sh`
Capture a single pane snapshot from a tmux session.

Options:
- `--wait SECONDS` (default: `2`)
- `--out PATH`
- `--format text|ansi` (default: `text`)
- `--command CMD` (default: `soul`)

Example:
```bash
scripts/screenshot-tui.sh --wait 2 --format text --out artifacts/screenshots/status.txt
```

Use this when:
- You need one specific screen state
- You want a quick, diffable text snapshot in CI/dev loops

### `scripts/extract-frames.sh`
Extract JPG frames from a video file for visual inspection or downstream analysis.

Options:
- `--input VIDEO` (required)
- `--interval SECONDS` (default: `1`)
- `--out-dir DIR`

Example:
```bash
scripts/extract-frames.sh --input demo.mp4 --interval 0.5 --out-dir artifacts/frames/demo
```

Use this when:
- You already have a video recording and need image samples
- You want frame-by-frame checks for transitions, color shifts, or layout jitter

### `scripts/analyze-tui.sh`
Capture a sequence of snapshots over time from a running tmux session.

Options:
- `--command CMD` (default: `soul`)
- `--captures N` (default: `5`)
- `--interval SECONDS` (default: `2`)
- `--keys KEYS` (keys sent between captures)
- `--out-dir DIR`

Example:
```bash
scripts/analyze-tui.sh --captures 6 --interval 1 --keys "2" --out-dir artifacts/analyze/page-switch
```

Use this when:
- You need to verify state changes over time
- You want to step the TUI between pages and inspect each resulting screen

## Example Workflows

### Verify a TUI animation/loading state
1. Run:
   ```bash
   scripts/analyze-tui.sh --command soul --captures 10 --interval 0.5 --out-dir artifacts/analyze/loading
   ```
2. Inspect `artifacts/analyze/loading/index.txt` and `capture-*.txt` to confirm progression.

### Check page layout after a change
1. Run:
   ```bash
   scripts/screenshot-tui.sh --command soul --wait 2 --format text --out artifacts/screenshots/layout.txt
   ```
2. Compare current output against a baseline snapshot.

### Debug a rendering bug reported by another agent
1. Record a run:
   ```bash
   scripts/record-tui.sh --duration 20 --command soul --out artifacts/recordings/bug.typescript
   ```
2. Replay the terminal stream:
   ```bash
   script -p artifacts/recordings/bug.typescript.timing artifacts/recordings/bug.typescript
   ```
3. Optionally convert external video captures into interval frames with `extract-frames.sh`.

## Interpreting Output

- `text` snapshots are best for structural checks, assertions, and diffs.
- `ansi` snapshots preserve escape codes, useful for color/style debugging.
- `record-tui.sh` timing + typescript pairs are best for temporal behavior and replay.
- `analyze-tui.sh` produces multi-point snapshots with timestamps for transition analysis.

## Notes

- All scripts fail fast with remediation-oriented errors when dependencies are missing.
- Output directories are auto-created.
- Commands are executed inside tmux so sessions can be captured consistently.

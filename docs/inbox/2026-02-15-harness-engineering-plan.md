# Harness Engineering Implementation Plan — 2026-02-15
Last updated: 2026-02-15

## Source
[OpenAI "Harness Engineering"](https://openai.com/index/harness-engineering/)

## Goals
Make the Soul Vault repo a first-class agent-friendly codebase where coding agents can autonomously work on features with high quality, minimal bugs, and great workflow practices.

## Three Parallel Workstreams

### 1. Video Tooling for Agents
- `scripts/record-tui.sh` — record a TUI session to video using `asciinema` + `agg` or `ttyrec` + `ttygif`, or just plain `script` + `ffmpeg`
- `scripts/screenshot-tui.sh` — capture a single frame of the TUI
- `scripts/analyze-video.sh` — extract frames from a video at intervals for agent analysis
- Document in `docs/VIDEO_TOOLING.md` how agents should use these tools
- Add AGENTS.md pointer to video tooling docs

### 2. Quality & Linting Infrastructure
- `scripts/lint-architecture.sh` — validate dependency direction (no upward imports)
- `scripts/lint-file-size.sh` — flag files over 200 lines
- `scripts/lint-quality.sh` — run all custom lints
- `docs/QUALITY.md` — quality grades per module (A/B/C/D)
- CI integration notes in docs

### 3. Knowledge Base Enhancement
- `docs/GOLDEN_PRINCIPLES.md` — mechanical rules for consistency
- `docs/EXECUTION_PLAN_TEMPLATE.md` — template for complex work
- `docs/plans/` directory for active/completed plans
- Update AGENTS.md to be a proper table of contents with progressive disclosure
- Add doc freshness markers

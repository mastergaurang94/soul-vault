# AGENTS.md — Soul Vault

> Start here. This file is a table of contents, not an encyclopedia.

## Quick Start

Soul Vault is a Rust CLI that converts AI conversations into a local markdown knowledge vault.
Build: `cargo build --release`
Test: `cargo test`

## Map

- `docs/STATUS.md` — Current state, active work, and backlog (read first).
- `docs/ARCHITECTURE.md` — Module map, dependency flow, vault layout.
- `docs/DESIGN_PRINCIPLES.md` — Engineering standards and coding guidance.
- `docs/GOLDEN_PRINCIPLES.md` — Mechanical, enforceable repo rules.
- `docs/EXECUTION_PLAN_TEMPLATE.md` — Template for complex multi-step work.
- `docs/plans/README.md` — Where active/completed execution plans live.
- `docs/COORDINATION.md` — Multi-agent handoff protocol.
- `docs/CHANGELOG.md` — Chronological record of completed changes.
- `docs/ADAPTERS_SPEC.md` — Provider adapter behavior and interfaces.
- `docs/TUI_SPEC.md` — Full-screen TUI product specification.
- `docs/QUALITY.md` — Module quality grading methodology and current lint baseline.
1. **`docs/STATUS.md`** — What's done, in progress, planned. **Read this before starting any work.**
2. **`docs/ARCHITECTURE.md`** — Module map, data flow, vault layout
3. **`docs/DESIGN_PRINCIPLES.md`** — Coding standards
4. **`docs/CHANGELOG.md`** — What was built and when
5. **`docs/COORDINATION.md`** — Multi-agent handoff protocol

## Workflow

**Before:** Read `docs/STATUS.md`. Don't duplicate in-progress work.
**During:** Follow `docs/DESIGN_PRINCIPLES.md`. Keep files under ~200 lines.
**After:** Update `docs/STATUS.md` + `docs/CHANGELOG.md`. Run verification. Leave it green.

## Repo Layout

```
src/
  main.rs           Entry point, clap CLI
  cli/              Commands: init, import, pull, export, status, watch, reset (+ legacy interactive)
  core/             Pipeline: processor → parser → merger + prompt template
  vault/            I/O: config, read, write, source tracking
  extractors/       File format handlers (local files, ChatGPT exports)
  types/            All types, enums, errors (leaf node — no internal imports)
  ui/               Terminal styling: theme colors, ratatui widgets
tests/              Integration tests + fixtures
docs/               All project documentation
.github/workflows/  CI + release automation
```
1. **`docs/STATUS.md`** — What's done, in progress, planned. **Read this before starting any work.**
2. **`docs/ARCHITECTURE.md`** — Module map, data flow, vault layout
3. **`docs/DESIGN_PRINCIPLES.md`** — Coding standards
4. **`docs/CHANGELOG.md`** — What was built and when
5. **`docs/COORDINATION.md`** — Multi-agent handoff protocol
6. **`docs/VIDEO_TOOLING.md`** — TUI recording, snapshots, and frame-analysis workflow

## Workflow

**Before:** Read `docs/STATUS.md`. Don't duplicate in-progress work.
**During:** Follow `docs/DESIGN_PRINCIPLES.md`. Keep files under ~200 lines.
**After:** Update `docs/STATUS.md` + `docs/CHANGELOG.md`. Run verification. Leave it green.

## Repo Layout

```
src/
  main.rs           Entry point, clap CLI
  cli/              Commands: init, import, pull, export, status, watch, reset (+ legacy interactive)
  core/             Pipeline: processor → parser → merger + prompt template
  vault/            I/O: config, read, write, source tracking
  extractors/       File format handlers (local files, ChatGPT exports)
  types/            All types, enums, errors (leaf node — no internal imports)
  ui/               Terminal styling: theme colors, ratatui widgets
tests/              Integration tests + fixtures
docs/               All project documentation
.github/workflows/  CI + release automation
```

## Rules

- Follow `docs/DESIGN_PRINCIPLES.md` for implementation standards.
- Follow `docs/GOLDEN_PRINCIPLES.md` for mechanical consistency checks.
- Validate external data at boundaries and return actionable errors.

## Current Focus

- Check `docs/STATUS.md` before writing code to avoid duplicate in-progress work.
- If work is complex or multi-session, create/update a plan under `docs/plans/`.

## Verify

```bash
cargo build --release && cargo test && cargo clippy --all-targets -- -D warnings
```

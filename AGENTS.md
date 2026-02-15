# AGENTS.md — Soul Vault

> Start here. Read this file first, then follow the links.

## What is Soul Vault?

Rust CLI that distills AI conversations into a structured local vault (`~/soul-vault/`).
Point it at any folder of transcripts, notes, or exports → it extracts structured memories into readable markdown.

## Orientation

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

1. **Dependency direction:** types → utils → core → cli. Never import upward.
2. **Validate at boundaries.** All external data (API, files, user input) parsed at entry.
3. **Error messages are remediation instructions.** Tell the user what happened AND what to do.
4. **No `process::exit()` in library code.** Use `anyhow::bail!()` or `Result`.
5. **Vault markdown must be beautiful.** It's the product.
6. **Test core logic.** Processor, merger, vault, source tracking need tests. CLI glue less critical.

## Verify

```bash
cargo build --release && cargo test && cargo clippy --all-targets -- -D warnings
```

## Reference

- [OpenAI "Harness Engineering"](https://openai.com/index/harness-engineering/) — the engineering philosophy behind this codebase's structure

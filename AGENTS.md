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

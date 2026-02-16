# Soul Vault — Project Status

Last updated: 2026-02-16

## ✅ Completed

- [x] Current command set is live in `src/main.rs`:
  - `init`, `import`, `export`, `status`, `watch`, `reset`, `login`, `logout`
  - No-args launch (`soul`) opens the full-screen TUI
- [x] TUI navigation currently has 7 pages:
  - Status, Import, Browse, Export, Watch, Reset, Settings
- [x] OAuth controls moved under Settings > Connections:
  - sidebar is stable (no separate Login/Logout pages)
  - connections copy now uses provider-scoped language (`Connect`/`Disconnect`) instead of product login framing
  - only valid next actions are shown per provider state
- [x] Export overhaul landed:
  - `soul export --format context|json|bundle`
  - `soul export --sections identity,preferences,topics,people,memories`
  - smart default output paths and bundle directory export
- [x] Import/auth updates landed:
  - providers mode (`soul import`) preflight API key validation
  - `soul login [provider]` / `soul logout [provider]`
  - cloud scaffold via `soul import --cloud [--provider ...]`
- [x] Vault reset safety command landed:
  - `soul reset` with confirmation flow
  - `soul reset --force` for non-interactive use
- [x] Source tracking + dedup is active via `sources.json`
- [x] Distribution scaffolding is in place (`install.sh`, CI/release workflows, Homebrew formula scaffold)
- [x] Agent-first documentation upgrade landed:
  - `AGENTS.md`/`CLAUDE.md` rewritten as progressive-disclosure TOC docs
  - Added `docs/GOLDEN_PRINCIPLES.md`, `docs/EXECUTION_PLAN_TEMPLATE.md`, `docs/plans/README.md`
  - Added `scripts/lint-docs.sh` doc gardening checks
- [x] Quality/linting infrastructure is in place (`scripts/lint-architecture.sh`, `scripts/lint-file-size.sh`, `scripts/lint-unwrap.sh`, `scripts/lint-all.sh`)
- [x] Video tooling for agent development is available:
  - `scripts/record-tui.sh` for replayable terminal recordings
  - `scripts/screenshot-tui.sh` for single-frame pane captures
  - `scripts/extract-frames.sh` for interval JPG extraction from video
  - `scripts/analyze-tui.sh` for time-series TUI snapshot analysis
- [x] Architecture boundary cleanup landed:
  - moved local file discovery/content extraction helpers from `extractors/` to `vault/`
  - updated `core/`, `tui/`, and CLI call sites to import from `vault::local`
  - added missing top-level `//!` module docs in `cli/`, `core/`, `extractors/`, `ui/`, and `vault/`
- [x] Init vault structure no longer pre-creates provider source folders:
  - `soul init` now creates only core directories plus `sources/`
  - provider-specific source directories are now lazy by usage, not scaffolded up front
- [x] First-run no-args UX now prompts initialization:
  - running `soul` before initialization prompts to run `soul init`
  - accepting runs init immediately, then launches the TUI
  - declining exits cleanly with a reminder to run `soul init`
- [x] API keys now validate during setup:
  - `soul init` validates Claude, ChatGPT, and Gemini keys immediately after entry
  - invalid keys are rejected with a re-enter prompt before saving
  - transient network/endpoint issues are marked unverified but can still be saved
- [x] Settings now reflects credential health, not just key presence:
  - persisted key validation state stored in `.config/key_status.json`
  - provider statuses now show `ready`, `key unverified`, or `key invalid` based on last validation
  - API Key section now shows Claude/ChatGPT/Gemini keys with masked values and health labels
- [x] CLI `soul status` provider health now matches credential reality:
  - green `+` appears only when provider credentials are truly ready (OAuth or verified key)
  - failed/unverified key states now show amber/red status instead of a misleading ready import state
- [x] TUI reset now exits after successful vault deletion:
  - resetting from the Reset page now closes the TUI immediately
  - prevents staying in an uninitialized post-reset UI session

## 🔨 In Progress

- None.

## 📋 Backlog

- [ ] Cloud API conversation fetch implementation for `soul import --cloud`
- [ ] `soul search` — full-text vault search
- [ ] `soul diff` — show changes since last import
- [ ] `SOUL_VAULT_VAULT_PATH` env var for multi-vault/test workflows
- [ ] Onchain backup (Arweave)
- [ ] SDK for other agents
- [ ] Chrome extension
- [ ] Multi-vault support
- [ ] Homebrew tap publishing

## Architecture Quick Reference

- **Binary:** `soul`
- **Crate:** `soul-vault`
- **Vault root:** `~/soul-vault/`
- **Config files:** `~/soul-vault/.config/config.json`, `keys.json`, `sources.json`
- **OAuth store:** `~/soul-vault/auth.yaml`
- **Current adapters:** Claude Code, OpenClaw, Gemini CLI, Codex

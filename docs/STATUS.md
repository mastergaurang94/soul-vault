# Soul Vault — Project Status

Last updated: 2026-02-15

## ✅ Completed

- [x] Export refactor: CLI + TUI redesign (2026-02-15):
  - `soul export` now supports `--format context|json|bundle` (default: `context`)
  - Added `--sections identity,preferences,topics,people,memories` (default all)
  - Added bundle export mode that creates a portable directory copy of vault markdown files
  - Added smart default export paths by format (`.md`, `.json`, or bundle folder)
  - Redesigned TUI Export page:
    - Removed topic/output text inputs
    - Added section checkboxes with Enter/Space toggle
    - Kept Enter-to-cycle format selector
    - Added output-path preview + selected-word-count preview
    - Export now always writes to file and shows destination
  - Added regression tests for section-filtered context export, bundle creation, TUI navigation, and smart default paths
- [x] `soul pull` CLI output polish (2026-02-15):
  - Reduced vertical noise across pull phases by tightening spacing and transition lines
  - Kept provider-by-provider discovery output intact
  - Upgraded Phase 4 progress bar to include elapsed time while keeping chunk name in-message
  - Added visible 30s rate-limit wait count-up (`1/30s` → `30/30s`) in the progress message
  - Grouped parse and processing errors into compact end-of-run summaries (no interleaving during progress)
  - Tightened final pull summary formatting with less padding/blank-line churn
- [x] Hardening pass (2026-02-15):
  - Fixed `soul init` API key prompt logic to always include the selected processing provider, including Gemini-only setups
  - Removed library-level `process::exit()` from `soul watch` auto-watch path; now returns actionable `Result` errors
  - Added regression tests for both fixes in `src/cli/init.rs` and `src/cli/watch.rs`
  - Updated docs to match current command set and architecture wording (`import`/`pull`/`reset`, full-screen TUI wording)
- [x] OAuth scaffolding + cloud pull gate (2026-02-15):
  - Added `soul login [provider]` and `soul logout [provider]`
  - Implemented local OAuth callback flow for Claude (browser open, localhost callback, code exchange, token storage)
  - Added credential storage at `~/soul-vault/auth.yaml` with `0600` file permissions
  - Added token validity + refresh checks in `src/auth/mod.rs`
  - Added `soul pull --cloud [--provider <claude|chatgpt|gemini>]`
  - Cloud pull now checks login status and prints API integration stub guidance
  - ChatGPT/Gemini OAuth flows are scaffolded with explicit "coming soon" output
- [x] Rust rewrite — full CLI with 7 commands (init, import, export, status, watch, reset, interactive TUI)
- [x] 200 tests (84 unit + 104 CLI UX + 12 integration), zero clippy warnings
- [x] Vault format: `~/soul-vault/` with structured markdown (identity/, preferences/, memories/, topics/, people/, sources/)
- [x] Verification: clean bill of health
- [x] Cleanup: TS archived to `soul-vault-ts-archive/`, Rust promoted to primary at `/Users/gaurangpatel/Documents/dev/soul-vault/`
- [x] Minor fixes: `anyhow::bail` instead of `process::exit`, ASCII slugify, removed unused `IngestResult`
- [x] Source tracking & dedup prevention — `sources.json` with file hashes, skip unchanged files on re-import
- [x] `--force` flag for import command (re-import regardless of changes)
- [x] `soul watch` command — file watcher with debounce, auto-import on changes
- [x] Distribution framework:
  - GitHub Actions CI (`.github/workflows/ci.yml`) — matrix build: macOS arm64, Linux x86_64/arm64
  - GitHub Actions Release (`.github/workflows/release.yml`) — builds binaries for 4 targets, SHA256 checksums
  - Pre-commit hooks (`.githooks/pre-commit`) — fmt, clippy, test
  - Install script (`install.sh`) — auto-detect OS/arch, download, verify checksum
  - Homebrew formula scaffold (`homebrew/soul-vault.rb`)
  - `rustfmt.toml` config
- [x] UX polish (2026-02-14):
  - Renamed `ingest` → `import` everywhere (CLI, help, status, docs); `ingest` kept as hidden alias
  - Fixed `soul status` box-drawing alignment — proper fixed-width formatting, emoji handling
  - Interactive menu reordered: Init → Status → Import → Export → Watch → Reset → Quit
  - Import from interactive menu now prompts for folder path instead of showing usage
  - Non-TTY fallback updated to use "import" terminology
  - `soul import` (no args) shows helpful error with usage example instead of clap error
  - 70 CLI UX regression tests added (tests/cli_ux_test.rs)
  - Binary symlinked to `~/.local/bin/soul` for global access
- [x] `soul reset` command (2026-02-14):
  - Destructive vault wipe with safety rails
  - Path validation (`is_safe_to_delete()`) prevents deleting outside home/soul
  - Interactive confirmation requires typing "reset" (not just y/n)
  - `--force` / `-f` flag for scripting/testing
  - Shows what will be deleted (path, config dir, file counts) before confirming
  - Added to interactive TUI menu as option 6 (after Watch, before Quit)
  - 21 tests (12 unit + 9 CLI UX) covering safety checks, temp vault deletion, non-TTY behavior

- [x] Full-screen ratatui TUI (2026-02-14):
  - `soul` (no args) now launches a full-screen alternate-screen TUI
  - Sidebar navigation: Status, Import, Browse, Export, Watch, Settings
  - Status page: vault overview (memories/topics/people/size/files/last activity), providers, imported sources
  - Browse page: tree view of vault directories + file content preview (scrollable)
  - Import page: folder path input with tilde expansion, path validation
  - Export page: format toggle (markdown/json), topic filter, output path, execute button, word count preview
  - Watch page: folder path input with terminal command guidance
  - Settings page: config display (vault path, processing LLM, providers)
  - Keyboard: j/k/arrows navigate, Enter select, Tab toggle sidebar/content, 1-6 jump to page, q/Esc quit/back
  - Non-TTY fallback preserved — prints help text with subcommand usage
  - All subcommands (`soul import`, `soul status`, etc.) still work unchanged
  - New files: `src/tui/` module (8 files: mod.rs, app.rs, sidebar.rs, pages/{mod,status,browse,import,export,watch,settings}.rs)
  - 205 tests (89 unit + 104 CLI UX + 12 integration), zero clippy warnings
  - Old `cli/interactive.rs` preserved as dead code reference

- [x] TUI async import pipeline (2026-02-14):
  - Import page now runs the full import pipeline within the TUI via tokio tasks
  - `src/core/pipeline.rs`: reusable import pipeline with `tokio::sync::mpsc` progress reporting
  - `ImportProgress` enum: Scanning → ScanComplete → Classifying → ClassifyComplete → Processing(current/total/file) → Writing → Done(result) | Error
  - Live progress bar during LLM processing (shows current/total chunks, file name)
  - Results summary: new/modified/skipped files, facts extracted, topics, people, errors
  - Handles: empty folders, all-unchanged, API key errors, rate limiting
  - Non-blocking: TUI remains responsive during import (50ms event poll timeout)
- [x] TUI live file watching (2026-02-14):
  - Watch page now runs `notify` file watcher directly in the TUI
  - `src/tui/watcher.rs`: background thread with debounced file system events
  - Events sent to TUI via `tokio::sync::mpsc` channels
  - Scrollable event log with timestamps, color-coded by event type (info/success/warning/error)
  - Auto-imports changed files using existing `run_for_files()` pipeline
  - Stop watcher with Esc (returns to sidebar)
  - Clean shutdown on quit (sends stop signal to watcher thread)
  - New files: `src/core/pipeline.rs`, `src/tui/watcher.rs` (2 new files)
  - Modified: `src/tui/mod.rs`, `src/tui/pages/mod.rs`, `src/tui/pages/import.rs`, `src/tui/pages/watch.rs`
  - 205 tests (89 unit + 104 CLI UX + 12 integration), zero clippy warnings

## 🔨 In Progress

*Check these before starting work — if an agent left work partially done, it should be noted here with details.*

## 📋 Backlog

- [ ] Cloud API implementation for `soul pull --cloud`
  - OAuth scaffolding is complete; conversation list/download API calls are still stubs
  - `extractors/chatgpt.rs` remains a placeholder for future ChatGPT API integration
- [ ] `soul search` — full-text vault search
- [ ] `soul diff` — show changes since last import
- [ ] `SOUL_VAULT_VAULT_PATH` env var for testability and multi-vault workflows
- [ ] Onchain backup (Arweave)
- [ ] SDK for other agents
- [ ] Chrome extension
- [ ] Multi-vault support
- [ ] Homebrew tap publishing (when ready — formula scaffold exists)

## Architecture Quick Reference

- **Binary:** `soul` (~4.3 MB release, LTO + strip)
- **Vault:** `~/soul-vault/` (identity/, preferences/, memories/, topics/, people/, sources/)
- **Config:** `~/soul-vault/.config/config.json` + `keys.json` (0600 perms) + `sources.json`
- **Default extraction model:** `claude-sonnet-4-20250514`
- **Codebase:** 34 source files, ~6,000+ lines of Rust
- **Stack:** clap, ratatui+crossterm, reqwest (rustls), serde, tokio, indicatif, colored, anyhow+thiserror, sha2, notify, chrono, regex, dirs

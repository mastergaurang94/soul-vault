# Soma — Project Status

Last updated: 2026-02-14

## ✅ Completed

- [x] Rust rewrite — full CLI with 7 commands (init, import, export, status, watch, reset, interactive TUI)
- [x] 200 tests (84 unit + 104 CLI UX + 12 integration), zero clippy warnings
- [x] Vault format: `~/soma/` with structured markdown (identity/, preferences/, memories/, topics/, people/, sources/)
- [x] Verification: clean bill of health
- [x] Cleanup: TS archived to `soma-ts-archive/`, Rust promoted to primary at `/Users/gaurangpatel/Documents/dev/soma/`
- [x] Minor fixes: `anyhow::bail` instead of `process::exit`, ASCII slugify, removed unused `IngestResult`
- [x] Source tracking & dedup prevention — `sources.json` with file hashes, skip unchanged files on re-import
- [x] `--force` flag for import command (re-import regardless of changes)
- [x] `soma watch` command — file watcher with debounce, auto-import on changes
- [x] Distribution framework:
  - GitHub Actions CI (`.github/workflows/ci.yml`) — matrix build: macOS arm64, Linux x86_64/arm64
  - GitHub Actions Release (`.github/workflows/release.yml`) — builds binaries for 4 targets, SHA256 checksums
  - Pre-commit hooks (`.githooks/pre-commit`) — fmt, clippy, test
  - Install script (`install.sh`) — auto-detect OS/arch, download, verify checksum
  - Homebrew formula scaffold (`homebrew/soma.rb`)
  - `rustfmt.toml` config
- [x] UX polish (2026-02-14):
  - Renamed `ingest` → `import` everywhere (CLI, help, status, docs); `ingest` kept as hidden alias
  - Fixed `soma status` box-drawing alignment — proper fixed-width formatting, emoji handling
  - Interactive menu reordered: Init → Status → Import → Export → Watch → Reset → Quit
  - Import from interactive menu now prompts for folder path instead of showing usage
  - Non-TTY fallback updated to use "import" terminology
  - `soma import` (no args) shows helpful error with usage example instead of clap error
  - 70 CLI UX regression tests added (tests/cli_ux_test.rs)
  - Binary symlinked to `~/.local/bin/soma` for global access

## 🔨 In Progress

*Check these before starting work — if an agent left work partially done, it should be noted here with details.*

- [ ] UI polish — Mole-inspired status output, progress bars
  - **Status:** Status box drawing is clean and aligned. `progress_bar()` and `format_time_ago()` helpers exist in `ui/theme.rs`. `label()` and `provider_line()` formatters exist but are unused. Could further polish with color-coded provider status.
- [ ] `soma browse` TUI — ratatui vault browser
  - **Status:** Not started. `ratatui` + `crossterm` are already dependencies. `ui/widgets.rs` has `MenuItem` and `soma_block` widgets. Would need a new `cli/browse.rs` command.

## 📋 Backlog

- [ ] `soma pull` — auto-pull from Claude/ChatGPT/Gemini APIs
  - `extractors/chatgpt.rs` is a placeholder for future ChatGPT API integration
- [ ] `soma search` — full-text vault search
- [ ] `soma diff` — show changes since last import
- [ ] `SOMA_VAULT_PATH` env var for testability and multi-vault workflows
- [ ] Onchain backup (Arweave)
- [ ] SDK for other agents
- [ ] Chrome extension
- [ ] Multi-vault support
- [ ] Homebrew tap publishing (when ready — formula scaffold exists)

## Architecture Quick Reference

- **Binary:** `soma` (~4.3 MB release, LTO + strip)
- **Vault:** `~/soma/` (identity/, preferences/, memories/, topics/, people/, sources/)
- **Config:** `~/soma/.config/config.json` + `keys.json` (0600 perms) + `sources.json`
- **Default extraction model:** `claude-sonnet-4-20250514`
- **Codebase:** 25 source files, ~4,500+ lines of Rust
- **Stack:** clap, ratatui+crossterm, reqwest (rustls), serde, tokio, indicatif, colored, anyhow+thiserror, sha2, notify, chrono, regex, dirs

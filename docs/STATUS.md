# Soma — Project Status

Last updated: 2026-02-14

## ✅ Completed

- [x] Rust rewrite — full CLI with 5 commands (init, ingest, export, status, interactive TUI)
- [x] 84 tests (72 unit + 12 integration), zero clippy warnings
- [x] Vault format: `~/soma/` with structured markdown (identity/, preferences/, memories/, topics/, people/, sources/)
- [x] Verification: clean bill of health
- [x] Cleanup: TS archived to `soma-ts-archive/`, Rust promoted to primary at `/Users/gaurangpatel/Documents/dev/soma/`
- [x] Minor fixes: `anyhow::bail` instead of `process::exit`, ASCII slugify, removed unused `IngestResult`
- [x] Source tracking & dedup prevention — `sources.json` with file hashes, skip unchanged files on re-ingest
- [x] `--force` flag for ingest command (re-ingest regardless of changes)
- [x] Distribution framework:
  - GitHub Actions CI (`.github/workflows/ci.yml`) — matrix build: macOS arm64, Linux x86_64/arm64
  - GitHub Actions Release (`.github/workflows/release.yml`) — builds binaries for 4 targets, SHA256 checksums
  - Pre-commit hooks (`.githooks/pre-commit`) — fmt, clippy, test
  - Install script (`install.sh`) — auto-detect OS/arch, download, verify checksum
  - Homebrew formula scaffold (`homebrew/soma.rb`)
  - `rustfmt.toml` config

## 🔨 In Progress

*Check these before starting work — if an agent left work partially done, it should be noted here with details.*

- [ ] `soma watch` command — file watcher for auto-ingest
  - **Status:** `notify` and `notify-debouncer-mini` crates already in `Cargo.toml`. `run_for_files()` helper exists in `cli/ingest.rs` for watch-triggered ingestion. The watch CLI command itself is **not yet wired up** in `main.rs` or implemented as a CLI subcommand.
- [ ] UI polish — Mole-inspired status output, progress bars
  - **Status:** `progress_bar()` and `format_time_ago()` helpers exist in `ui/theme.rs`. `label()` and `provider_line()` formatters exist but are unused (dead code warnings). Status command output is basic. No Mole-style formatting applied yet.
- [ ] `soma browse` TUI — ratatui vault browser
  - **Status:** Not started. `ratatui` + `crossterm` are already dependencies. `ui/widgets.rs` has `MenuItem` and `soma_block` widgets. Would need a new `cli/browse.rs` command.

## 📋 Backlog

- [ ] `soma pull` — auto-pull from Claude/ChatGPT/Gemini APIs
  - `extractors/chatgpt.rs` is a placeholder for future ChatGPT API integration
- [ ] `soma search` — full-text vault search
- [ ] `soma diff` — show changes since last ingest
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
- **Codebase:** 24 source files, ~4,431 lines of Rust
- **Stack:** clap, ratatui+crossterm, reqwest (rustls), serde, tokio, indicatif, colored, anyhow+thiserror, sha2, notify, chrono, regex, dirs

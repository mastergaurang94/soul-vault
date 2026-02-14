# Changelog

All notable changes to Soma will be documented in this file.
Agents: append entries here after completing work.

---

## 2026-02-14 — UX Polish

### Rename: ingest → import
- CLI subcommand renamed from `ingest` to `import`; `ingest` kept as hidden alias for backward compatibility
- All user-facing strings updated: help text, status output, error messages, watch output, non-TTY fallback
- `soma import` (no args) now shows helpful error with usage example and exit code 1 (instead of clap's generic error)
- Docs updated: README.md, STATUS.md, CHANGELOG.md, AGENTS.md, CLAUDE.md

### Status UI Fix
- Rewrote `src/cli/status.rs` with proper box-drawing alignment using fixed-width constants
- Added `visible_len()` function that strips ANSI escape codes and handles double-width emoji
- Dedicated box-drawing helpers: `print_box_top()`, `print_box_bottom()`, `print_box_header()`, `print_stat_row()`
- All `│` closing characters now properly aligned across all sections (overview, providers, sources)
- "Ingested Sources" renamed to "Imported Sources"

### Interactive Menu
- Reordered menu: Init → Status → Import → Export → Watch → Quit (was: Ingest → Watch → Export → Status → Init)
- Import action now prompts for folder path interactively (reads line from stdin) instead of just printing usage
- Watch action also prompts for folder path interactively
- All labels updated from "Ingest" to "Import"

### CLI UX Tests
- Added `tests/cli_ux_test.rs` with 70 comprehensive regression tests:
  - Help flag, version, subcommand help for all commands
  - Import: no args, nonexistent path, empty folder, unsupported files, hidden dirs, nested structures
  - Export: default/json/file/topic filter, edge cases
  - Status: sections, box-drawing consistency, no panics
  - Watch: no args, nonexistent path
  - Error message quality: no panics, actionable guidance, cross icons
  - Ingest alias: backward compatibility verified

### Distribution
- Binary symlinked to `~/.local/bin/soma` for global access

---

## 2026-02-14 — Initial Build

### TypeScript v0.1
- Built MVP with 14 source files, 5 commands
- 89 tests (vitest), Zod validation, Ink TUI
- Package: soma-vault (npm)

### Rust Rewrite
- Full rewrite: 24 source files, ~4,431 lines
- 84 tests (72 unit + 12 integration)
- ~4.3 MB release binary (LTO + strip), zero clippy warnings
- Vault-compatible with TS version
- Stack: clap, ratatui+crossterm, reqwest, serde, tokio, indicatif, anyhow+thiserror

### Cleanup
- TS version archived to `soma-ts-archive/`
- Rust promoted to primary at `/Users/gaurangpatel/Documents/dev/soma/`
- Fixed: `process::exit` → `anyhow::bail`, ASCII slugify, removed unused `IngestResult`

### Source Tracking & Dedup
- `vault/sources.rs`: SHA-256 file hashing, `sources.json` tracking
- `IngestClassification`: classifies files as new/modified/unchanged
- `cli/ingest.rs`: skip unchanged files, `--force` flag to override
- 15 tests covering hashing, classification, roundtrip serialization

### Distribution Framework
- GitHub Actions CI (`.github/workflows/ci.yml`): matrix build across macOS arm64, Linux x86_64/arm64
- GitHub Actions Release (`.github/workflows/release.yml`): 4-target binary builds, SHA256 checksums, auto GitHub Release
- Pre-commit hooks (`.githooks/pre-commit`): fmt + clippy + test gate
- Install script (`install.sh`): auto-detect OS/arch, download from GitHub Releases, checksum verification
- Homebrew formula scaffold (`homebrew/soma.rb`): platform-specific binary blocks
- `rustfmt.toml`, `.gitignore`, `.cargo/config.toml`

### Documentation
- Updated `README.md` with 5 installation methods, development section, CI/CD workflow
- Created `AGENTS.md` as agent entry point with repository map and key rules
- Created `docs/STATUS.md` — living project status document
- Created `docs/CHANGELOG.md` (this file)
- Created `docs/ARCHITECTURE.md` — module map, data flow, dependency rules
- Created `docs/DESIGN_PRINCIPLES.md` — coding standards
- Created `docs/COORDINATION.md` — multi-agent workflow protocol
- Fixed build error: added `--force` flag to `Ingest` CLI command (was missing from `main.rs` after `ingest.rs` signature update)

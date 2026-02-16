# Changelog
Last updated: 2026-02-16


All notable changes to Soul Vault will be documented in this file.
Agents: append entries here after completing work.

---

## 2026-02-16 — TUI Auth UX Consolidation Under Settings Connections

### Sidebar and page model
- Removed standalone TUI `Login` and `Logout` pages.
- Sidebar now keeps a stable 7-page structure:
  - Status, Import, Browse, Export, Watch, Reset, Settings
- Updated TUI page wiring:
  - `src/tui/app.rs` page enum/list and tests
  - `src/tui/layout.rs` page set routing
  - `src/tui/pages/mod.rs` module exports

### Settings > Connections language and action model
- Reworked OAuth status section in `src/tui/pages/settings.rs` into a `Connections` section.
- Replaced product-account framing (`logged in`) with provider-connection framing:
  - `connected`, `ready`, `not set up`, `coming soon`
- Added selective next-action hints per provider state:
  - `Connect via soul login`
  - `Disconnect via soul logout`
  - `Set up in soul init`
- Hides impossible actions (for example, no disconnect prompt when not connected).

### Documentation sync
- Updated TUI page counts/lists in:
  - `docs/STATUS.md`
  - `docs/ARCHITECTURE.md`
  - `README.md`

## 2026-02-16 — API Key Validation During `soul init` (All Providers)

### Validation behavior
- Added provider-aware API key validation in `src/cli/init_validate.rs` and wired into `src/cli/init.rs`.
- `soul init` now validates keys immediately after entry for:
  - Claude: Anthropic API check
  - ChatGPT: OpenAI models API check
  - Gemini: Google Generative Language models API check
- Validation outcomes now drive setup UX:
  - `verified`: key is accepted and saved
  - `invalid`: key is rejected and user is prompted to re-enter
  - `unverified` (network/transient): warning shown, key can still be saved

### Persisted key health + Settings UX
- Added persisted key health tracking in `src/vault/config.rs`:
  - `.config/key_status.json` with per-provider `status`, `checked_at`, and optional message
- Updated Settings page (`src/tui/pages/settings.rs`) to use key health state instead of key presence:
  - provider rows now reflect `ready`, `key unverified`, or `key invalid`
  - API Key section now shows Claude/ChatGPT/Gemini with masked key + health label

### Status command accuracy fix
- Updated CLI status provider rendering in `src/cli/status.rs` to match credential reality:
  - green `+` now requires OAuth login or verified API key state
  - providers with invalid/unverified/missing keys no longer show as ready
  - `no imports yet` now appears only when provider credentials are actually ready

### TUI reset UX
- Updated Reset page behavior in `src/tui/pages/reset.rs`:
  - after successful vault deletion, TUI now exits immediately
  - avoids leaving users inside a now-uninitialized vault session

### Runtime wiring
- Converted `cli::init::run` to async and updated call sites:
  - `src/main.rs` direct `soul init` command path
  - `src/main.rs` first-run no-args init prompt flow
  - `src/cli/interactive.rs` legacy menu init action

---

## 2026-02-16 — First-Run Init Prompt for `soul` (No Args)

### UX behavior
- Updated no-subcommand flow in `src/main.rs`:
  - if the vault is not initialized and stdin is a TTY, `soul` now prompts:
    - `Vault not initialized. Run setup now with soul init? (Y/n)`
  - accepting runs `soul init` immediately, then continues into the TUI
  - declining exits cleanly with a reminder to run `soul init`
- Non-TTY behavior remains unchanged through existing TUI non-interactive help path

### Tests
- Added unit coverage in `src/main.rs` for init-prompt decision logic (`should_run_init`)

---

## 2026-02-16 — Lazy Provider Source Directory Creation

### Init behavior change
- Updated `create_vault_structure()` to stop pre-creating provider-specific source folders:
  - removed eager creation of `sources/chatgpt`, `sources/claude`, and `sources/gemini`
  - keeps creation of core vault directories and `sources/` root unchanged
- Result: running `soul init` no longer creates integration/provider folders before the user has chosen or used those providers

---

## 2026-02-15 — Architecture Boundary Fix + Module Docs

### Dependency rule compliance
- Moved local file ingestion helpers:
  - `src/extractors/local.rs` → `src/vault/local.rs`
  - `src/extractors/chatgpt.rs` → `src/vault/chatgpt.rs`
- Rewired imports to the new boundary-safe location:
  - `src/core/pipeline.rs` now imports from `crate::vault::local`
  - `src/tui/watcher.rs` now imports from `crate::vault::local`
  - CLI ingestion/watch paths now import from `crate::vault::local`
- Updated `src/vault/mod.rs` to export `chatgpt` and `local` modules

### Docs lint compliance
- Added missing top-level module docs to:
  - `src/cli/mod.rs`
  - `src/core/mod.rs`
  - `src/extractors/mod.rs`
  - `src/ui/mod.rs`
  - `src/vault/mod.rs`

## 2026-02-15 — Knowledge Base Enhancement for Agent-First Development

### Documentation and planning system
- Reworked `AGENTS.md` into a compact table-of-contents entry point with progressive disclosure
- Kept `CLAUDE.md` byte-identical to `AGENTS.md`
- Added `docs/GOLDEN_PRINCIPLES.md` with concrete, verifiable repository rules
- Added `docs/EXECUTION_PLAN_TEMPLATE.md` for complex multi-session execution
- Added `docs/plans/README.md` and `docs/plans/completed/` workflow conventions
- Added `Last updated: 2026-02-15` freshness markers to docs missing them

### Doc gardening automation
- Added `scripts/lint-docs.sh` to check:
  - top-level `//!` module docs for all `src/**/*.rs`
  - `Last updated` markers for all docs markdown files
  - broken internal markdown links under `docs/`
  - `AGENTS.md` coverage of stable docs markdown files
- Script currently reports one failing group in existing codebase:
  - Missing top-level `//!` module docs in:
    - `src/cli/mod.rs`
    - `src/core/mod.rs`
    - `src/extractors/mod.rs`
    - `src/ui/mod.rs`
    - `src/vault/mod.rs`

## 2026-02-15 — Quality Grading + Lint Infrastructure

### New quality enforcement scripts
- Added `scripts/lint-architecture.sh`
  - Parses `use crate::...` imports and validates dependency direction against `docs/ARCHITECTURE.md`
  - Reports actionable file:line remediation guidance
- Added `scripts/lint-file-size.sh`
  - Enforces default 200-line file limit with `--limit N` override
  - Reports oversized files with split-by-concern guidance
- Added `scripts/lint-unwrap.sh`
  - Flags non-test `.unwrap()` usage in `src/`
  - Flags `process::exit` outside `src/main.rs`
- Added `scripts/lint-all.sh`
  - Runs all custom lints, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt -- --check`
  - Produces an aggregated pass/fail report

### Documentation updates
- Added `docs/QUALITY.md` with module letter grades (A/B/C/D), grading methodology, and current lint baseline.
- Updated `docs/DESIGN_PRINCIPLES.md` with an **Automated Enforcement** section that points to lint scripts and actionable lint output standards.
- Updated `docs/STATUS.md` to reflect quality infrastructure as completed work.

## 2026-02-15 — Video Tooling for Agent Development Workflow

### New script tooling
- Added `scripts/record-tui.sh`:
  - Launches command in detached `tmux`
  - Records session output via `script` with timing metadata
  - Supports `--duration`, `--out`, `--command`
- Added `scripts/screenshot-tui.sh`:
  - Launches command in `tmux`, waits for render, captures pane output
  - Supports `--wait`, `--out`, `--format text|ansi`, `--command`
- Added `scripts/extract-frames.sh`:
  - Extracts numbered JPG frames from videos using `ffmpeg`
  - Supports `--input`, `--interval`, `--out-dir`
- Added `scripts/analyze-tui.sh`:
  - Runs command in `tmux` and captures a time-series of pane snapshots
  - Supports `--command`, `--captures`, `--interval`, `--keys`, `--out-dir`

### Documentation updates
- Added `docs/VIDEO_TOOLING.md` with usage guidance, workflows, and output interpretation
- Updated `AGENTS.md` orientation to link `docs/VIDEO_TOOLING.md`

### Validation
- Verified all new scripts respond to `--help`
- Attempted runtime `tmux` snapshot validation (`scripts/screenshot-tui.sh --command ls`) but current sandbox disallows socket creation; requires host-level run for end-to-end capture validation

## 2026-02-15 — Documentation Sync for Current Codebase State

### Docs refreshed against implementation
- Updated `README.md` command documentation to match clap definitions in `src/main.rs`
  - Covers all commands: `init`, `import`, `pull`, `export`, `status`, `watch`, `reset`, `login`, `logout`
  - Documents export flags: `--format context|json|bundle` and `--sections`
  - Calls out local `soul pull` preflight API key validation
  - Adds `soul login` / `soul logout` usage and provider defaults
  - Updates TUI section to list all 9 pages: Status, Pull, Import, Browse, Export, Watch, Login, Logout, Settings
  - Replaced placeholder repo URLs with `mastergaurang94/soul-vault`
- Rewrote `docs/ARCHITECTURE.md` to include current modules and wiring:
  - Added `src/auth/mod.rs`
  - Added `src/adapters/` map (`claude`, `openclaw`, `gemini`, `codex`, `mod`)
  - Added TUI login/logout page modules in the page map
  - Updated dependency direction to include auth/adapters/tui relationships
- Updated `docs/ADAPTERS_SPEC.md` to reflect actual trait and registry API in code
- Cleaned `docs/STATUS.md` to remove stale in-progress items and align current command/page state
- Audited `docs/CHANGELOG.md` ordering and duplicate headings (chronological order preserved; no duplicate headings)

## 2026-02-15 — Hardening: unwrap/panic-path cleanup

### Robustness updates
- Replaced non-test `unwrap()` calls in progress-style creation with safe fallbacks:
  - `src/cli/pull.rs`
  - `src/cli/ingest.rs`
- Hardened regex utilities to avoid panic paths by gracefully degrading when regex construction fails:
  - `src/core/parser.rs`
  - `src/cli/export.rs`
- Removed a non-test `expect()` panic path in vault root resolution:
  - `src/vault/config.rs`

### Validation notes
- Kept API key/config/auth paths on `anyhow::Result` flows with existing actionable errors for missing files and invalid config/keys content.

## 2026-02-15 — TUI Login/Logout Pages + Copy Clarity

### New TUI pages
- Added `src/tui/pages/login.rs` and `src/tui/pages/logout.rs`
- Extended `Page` enum and page wiring to include `Login` and `Logout`
- Sidebar now includes 9 pages in this order:
  - Status, Pull, Import, Browse, Export, Watch, Login, Logout, Settings
- Updated sidebar number shortcuts in `src/tui/mod.rs` from `1-7` to `1-9`

### Login page behavior
- Added "Login — Cloud Provider OAuth" page with:
  - Claude login state from `auth::is_logged_in(&Provider::Claude)`
  - ChatGPT/Gemini "coming soon" status rows
  - Enter action message: "Starting OAuth login... Check your browser."
- This page is intentionally UI-only for now (actual OAuth flow remains in `soul login`)

### Logout page behavior
- Added "Logout — Clear Credentials" page with session-aware messaging:
  - Logged in: prompt to press Enter to logout Claude
  - Not logged in: "No active sessions."
- Enter on logged-in state now clears Claude credentials via `auth::remove_credentials`
- Shows completion message: "Credentials cleared."

### Pull vs Import clarity improvements
- Pull page ready-state title changed to:
  - `Pull — Auto-import from AI apps`
- Added pull description:
  - `Automatically discovers and imports sessions from local AI applications.`
- Import page input-state title changed to:
  - `Import — Manual file & folder import`
- Added import description:
  - `Import transcript files, ChatGPT exports, or any text files into your vault.`

## 2026-02-15 — Pull/Status Bug Fixes

### `soul pull` preflight API key check
- Added an early local-pull guard in `src/cli/pull.rs` that checks `get_api_key("claude")` before Phase 1 discovery
- If the key is missing or empty, command now fails immediately with:
  - `No API key configured. Run \`soul init\` to set up your Claude API key.`
- Cloud pull path remains unchanged

### `config.json` sync metadata updates after successful pull
- Added post-success config persistence in `src/cli/pull.rs`:
  - `last_sync` now updates to current RFC 3339 timestamp
  - Matching provider entries now update `last_pull` to the same timestamp when sessions were discovered
- Uses existing `read_config()` / `write_config()` without adding dependencies

### `soul status` vault path correctness
- Updated `src/cli/status.rs` to display `vault_root()` instead of `stats.vault_path` from config
- Prevents stale/typoed path display when `config.json` `vaultPath` drifts from actual runtime vault root

## 2026-02-15 — Export CLI/TUI Refactor

### CLI export formats and sections
- `soul export` now supports `--format context|json|bundle` with `context` as default
- Added `--sections` flag (`identity,preferences,topics,people,memories`) to control included export sections
- Kept `--topic` support for topic-filtered context output
- Added smart default export paths for file outputs:
  - context: `~/soul-vault-export-YYYY-MM-DD.md`
  - json: `~/soul-vault-export-YYYY-MM-DD.json`
  - bundle: `~/soul-vault-export-YYYY-MM-DD/`
- Added `bundle` mode that creates a directory containing raw vault markdown files for backup/migration (no new dependencies)

### TUI Export page redesign
- Removed raw text inputs for topic filter and output path
- Added section checkbox toggles for Identity, Preferences, Topics, People, Memories
- Kept Enter-to-cycle format selector and j/k navigation
- Added smart output-path preview by format and selected-word-count preview
- Export action now writes to a file path every time and reports the destination

### Tests
- Added section-filtered context export test coverage in `src/cli/export.rs`
- Added bundle directory creation tests in `src/cli/export.rs`
- Added TUI export navigation + section toggle tests in `src/tui/pages/export.rs`
- Added smart default path generation tests in `src/tui/pages/export.rs`
- Updated CLI UX export invalid-format regression test to match new validation behavior

## 2026-02-15 — Pull Output Polish

### `soul pull` UX tightening
- Reduced excess blank lines and redundant spacing in local pull flow output (`src/cli/pull.rs`)
- Kept Phase 1 provider discovery lines unchanged (per-provider session counts remain visible)
- Kept spinner behavior for parse/write/source-tracking phases while tightening surrounding transitions

### LLM progress improvements (Phase 4)
- Progress bar template now includes elapsed time (`{elapsed_precise}`)
- Message field continues to show current chunk source label
- Preserved warm gold/amber + cyan accent styling in progress output

### Rate limit feedback
- On 429/rate-limit responses, progress message now shows a live wait count-up from `1/30s` to `30/30s` before retry

### End-of-run reporting
- Parse errors and processing errors are now shown together at the end in grouped sections
- Final summary block is more compact (less padding and fewer blank separator lines)

## 2026-02-15 — OAuth Scaffolding for Cloud Pull

### New CLI commands
- Added `soul login [provider]` (defaults to Claude)
- Added `soul logout [provider]` (provider-specific or global credential removal)
- Wired both commands into clap in `src/main.rs` and module exports in `src/cli/mod.rs`

### New auth module
- Added `src/auth/mod.rs` with:
  - `AuthCredentials` model (`access_token`, `refresh_token`, `expires_at`, `provider`)
  - `save_credentials()` / `load_credentials()` / `is_logged_in()` / `remove_credentials()` / `clear_credentials()`
  - Credential storage at `~/soul-vault/auth.yaml` with enforced `0600` permissions
  - Expiry check + refresh path (`ensure_valid_credentials()` + refresh token grant)

### OAuth flow scaffolding
- Implemented real localhost callback flow for Claude:
  - Starts random local callback listener on `127.0.0.1:0`
  - Opens browser to provider authorization URL
  - Receives callback and validates `state`
  - Exchanges auth code for token and persists credentials
- Added provider stubs for ChatGPT and Gemini with explicit "coming soon" guidance
- OAuth endpoints/client IDs are scaffolded with placeholders and Claude env-var overrides

### Pull command integration
- Extended `soul pull` with:
  - `--cloud` flag to enable cloud path
  - `--provider` flag (`claude`, `chatgpt`, `gemini`; default Claude)
- `soul pull --cloud` now:
  - Checks login state
  - Validates/refreshes tokens when possible
  - Prints provider API stub: "Coming soon — use soul import with your exported data"
- Existing local pull behavior remains default unchanged

### UX/help updates
- Added `login/logout` lines to non-TTY help output (`src/tui/layout.rs`, `src/cli/interactive.rs`)
- Added OAuth hints to Settings page (`src/tui/pages/settings.rs`)
- Extended legacy interactive menu with Pull/Login/Logout actions (`src/cli/interactive.rs`)

## 2026-02-15 — Hardening & QA Pass

### Reliability fixes
- `src/cli/init.rs`: fixed API key collection flow so the selected processing provider is always included in the key prompt set (covers Gemini-only selections)
- `src/cli/watch.rs`: replaced library-level `process::exit()` calls in auto-watch with actionable `anyhow::bail!()` errors returned to `main`

### Regression tests
- `src/cli/init.rs`: added tests for key-provider selection/dedup behavior
- `src/cli/watch.rs`: added tests for auto-watch precondition validation (TTY + provider directory checks)

### Documentation sync
- Updated command-set wording in `README.md`, `AGENTS.md`, and `CLAUDE.md`
- Updated pipeline/CLI descriptions in `docs/ARCHITECTURE.md` to reflect `import`, `pull`, `watch`, and `reset`

## 2026-02-14 — TUI Async Import & Live Watch

### Async import pipeline in TUI
- Import page now runs the full import pipeline within the TUI
- New `src/core/pipeline.rs`: reusable import pipeline with `tokio::sync::mpsc` progress reporting
- `ImportProgress` enum with structured states: Scanning → Classifying → Processing → Writing → Done/Error
- Live progress bar during LLM processing (current/total chunks, file name, percentage)
- Results summary on completion: files (new/modified/skipped), facts extracted, topics, people
- Error handling: API key errors abort with guidance, rate limits auto-retry after 30s
- TUI event loop polls channels every 50ms — remains responsive during long imports

### Live file watching in TUI
- Watch page now runs `notify` file watcher directly in the TUI (no separate terminal needed)
- New `src/tui/watcher.rs`: background thread with 2s debounced file system events
- Events sent to TUI via `tokio::sync::mpsc` channels
- Scrollable event log with timestamps, color-coded icons by type (info/success/warning/error)
- Auto-imports changed files using existing `run_for_files()` pipeline
- Stop watcher with Esc (sends stop signal, returns to sidebar)
- Clean shutdown on app quit

### Architecture changes
- New `PageAction` variants: `StartImport(String)`, `StartWatch(String)`, `StopWatch`
- `Channels` struct in TUI event loop holds `mpsc::Receiver` endpoints
- Event loop changed from blocking `event::read()` to `event::poll(50ms)` + `try_recv()` pattern
- New files: `src/core/pipeline.rs` (~170 lines), `src/tui/watcher.rs` (~200 lines)
- Modified: `src/tui/mod.rs`, `src/tui/pages/mod.rs`, `src/tui/pages/import.rs`, `src/tui/pages/watch.rs`
- 205 tests (89 unit + 104 CLI UX + 12 integration), zero clippy warnings

---

## 2026-02-14 — Full-Screen TUI

### `soul` (no args) → ratatui full-screen TUI
- Replaced inline menu (`cli/interactive.rs`) with full-screen alternate-screen TUI
- New `src/tui/` module (8 files):
  - `mod.rs` — entry point, event loop, layout (header / sidebar+content / footer)
  - `app.rs` — App state (current page, sidebar selection, focus, vault status)
  - `sidebar.rs` — Navigation widget with selection highlight, gold theme
  - `pages/mod.rs` — PageWidget trait (render + handle_key + PageAction)
  - `pages/status.rs` — Vault overview (memories/topics/people/size/files/last activity), providers, imported sources
  - `pages/browse.rs` — Tree view of vault directories + scrollable file preview (40/60 split)
  - `pages/import.rs` — Folder path input with tilde expansion and path validation
  - `pages/export.rs` — Format toggle, topic filter, output path, execute button, word count preview
  - `pages/watch.rs` — Folder path input with terminal command guidance
  - `pages/settings.rs` — Config display (vault path, processing LLM, created date, providers)
- Keyboard bindings:
  - Sidebar: j/k/arrows navigate, Enter select, q/Esc quit
  - Global: Tab toggle sidebar/content, 1-6 jump to page, ? help toggle
  - Content: Esc back to sidebar, page-specific keys
- Non-TTY fallback preserved (prints help text with subcommand usage)
- All subcommands (`soul import <folder>`, `soul status`, etc.) unchanged
- Old `cli/interactive.rs` kept as dead code reference
- 205 tests (89 unit + 104 CLI UX + 12 integration), zero clippy warnings

---

## 2026-02-14 — Reset Command

### `soul reset`
- New command to completely wipe vault and config, returning to pre-init state
- Safety-first design:
  - `is_safe_to_delete()` validates path is inside home directory and contains "soul"
  - Rejects `/`, `~`, home dir, and paths outside home
  - Interactive confirmation requires typing "reset" (not just y/n)
  - Non-TTY environments require `--force` flag
- Shows detailed summary before deletion: vault path, config dir, file counts
- `--force` / `-f` flag skips confirmation (for scripting/CI)
- Graceful handling of non-existent vault: "Nothing to reset — vault not initialized."
- Added to interactive TUI menu as option 6 (after Watch, before Quit)
- Added to non-TTY help text
- New file: `src/cli/reset.rs` (~165 lines)
- 21 tests: 12 unit tests (safety validation, file counting) + 9 CLI UX tests (temp vault deletion, non-TTY behavior, help text)
- Total test count: 200 (84 unit + 104 CLI UX + 12 integration)

---

## 2026-02-14 — UX Polish

### Rename: ingest → import
- CLI subcommand renamed from `ingest` to `import`; `ingest` kept as hidden alias for backward compatibility
- All user-facing strings updated: help text, status output, error messages, watch output, non-TTY fallback
- `soul import` (no args) now shows helpful error with usage example and exit code 1 (instead of clap's generic error)
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
- Binary symlinked to `~/.local/bin/soul` for global access

---

## 2026-02-14 — Initial Build

### TypeScript v0.1
- Built MVP with 14 source files, 5 commands
- 89 tests (vitest), Zod validation, Ink TUI
- Package: soul-vault (npm)

### Rust Rewrite
- Full rewrite: 24 source files, ~4,431 lines
- 84 tests (72 unit + 12 integration)
- ~4.3 MB release binary (LTO + strip), zero clippy warnings
- Vault-compatible with TS version
- Stack: clap, ratatui+crossterm, reqwest, serde, tokio, indicatif, anyhow+thiserror

### Cleanup
- TS version archived to `soul-vault-ts-archive/`
- Rust promoted to primary at `/Users/gaurangpatel/Documents/dev/soul-vault/`
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
- Homebrew formula scaffold (`homebrew/soul-vault.rb`): platform-specific binary blocks
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

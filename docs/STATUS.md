# Soul Vault — Project Status

Last updated: 2026-02-15

## ✅ Completed

- [x] Current command set is live in `src/main.rs`:
  - `init`, `import`, `pull`, `export`, `status`, `watch`, `reset`, `login`, `logout`
  - No-args launch (`soul`) opens the full-screen TUI
- [x] TUI navigation currently has 9 pages:
  - Status, Pull, Import, Browse, Export, Watch, Login, Logout, Settings
- [x] Export overhaul landed:
  - `soul export --format context|json|bundle`
  - `soul export --sections identity,preferences,topics,people,memories`
  - smart default output paths and bundle directory export
- [x] Pull/auth updates landed:
  - local `soul pull` preflight API key validation
  - `soul login [provider]` / `soul logout [provider]`
  - cloud pull scaffold via `soul pull --cloud [--provider ...]`
- [x] Vault reset safety command landed:
  - `soul reset` with confirmation flow
  - `soul reset --force` for non-interactive use
- [x] Source tracking + dedup is active via `sources.json`
- [x] Distribution scaffolding is in place (`install.sh`, CI/release workflows, Homebrew formula scaffold)
- [x] Video tooling for agent development is available:
  - `scripts/record-tui.sh` for replayable terminal recordings
  - `scripts/screenshot-tui.sh` for single-frame pane captures
  - `scripts/extract-frames.sh` for interval JPG extraction from video
  - `scripts/analyze-tui.sh` for time-series TUI snapshot analysis

## 🔨 In Progress

- None.

## 📋 Backlog

- [ ] Cloud API conversation fetch implementation for `soul pull --cloud`
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

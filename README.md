# Soma — Your AI Memory, Unified

A CLI tool that distills AI conversations into a structured local vault. Point it at any folder of transcripts, notes, or ChatGPT exports and Soma extracts structured memories — identity, preferences, decisions, relationships, topics — into readable markdown files at `~/soma/`.

Built in Rust for instant startup, single-binary distribution, and premium terminal experience.

## Install

### Quick Install (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/user/soma/main/install.sh | bash
```

Downloads the latest release binary for your platform and installs to `~/.local/bin/`.

Set a custom install directory or version:

```bash
SOMA_INSTALL_DIR=/usr/local/bin SOMA_VERSION=v0.1.0 bash install.sh
```

### Homebrew (coming soon)

```bash
brew install user/tap/soma
```

### Cargo install

```bash
cargo install soma-vault
```

### From source

```bash
git clone https://github.com/user/soma.git
cd soma
cargo build --release
# Binary at target/release/soma (~4.3 MB)
```

### GitHub Releases

Download pre-built binaries directly from [GitHub Releases](https://github.com/user/soma/releases):

| Platform | Binary |
|----------|--------|
| macOS (Apple Silicon) | `soma-macos-arm64` |
| macOS (Intel) | `soma-macos-x86_64` |
| Linux (x86_64) | `soma-linux-x86_64` |
| Linux (ARM64) | `soma-linux-arm64` |

```bash
# Example: download, make executable, move to PATH
chmod +x soma-macos-arm64
mv soma-macos-arm64 ~/.local/bin/soma
```

## Usage

### Interactive Menu

```bash
soma
```

Launches a beautiful TUI menu with arrow key / vim binding navigation.

### Initialize Vault

```bash
soma init
```

First-time setup wizard:
1. Creates `~/soma/` directory structure
2. Asks which providers to connect (Claude, ChatGPT, Gemini)
3. Selects which LLM to use for memory extraction
4. Collects API keys (stored locally in `~/soma/.config/keys.json`)

### Ingest Files

```bash
soma ingest ~/Documents/ai-conversations/
```

**The killer feature.** Point at any folder and Soma:
- Discovers `.md`, `.txt`, `.json`, `.jsonl` files recursively
- Chunks them for LLM processing
- Sends through Claude to extract structured memories
- Merges and deduplicates with existing vault
- Writes to `~/soma/` as clean markdown

Supports: raw transcripts, ChatGPT JSON exports, session logs, Obsidian notes, and any text.

### Export Context

```bash
# Print to stdout
soma export

# Write to file
soma export -o ~/context.md

# JSON format
soma export --format json

# Filter by topic
soma export --topic crypto
```

Outputs your entire vault as a single context document that any AI can consume.

### Status

```bash
soma status
```

Shows vault summary: memory counts, topics, people, provider status.

## Vault Structure

```
~/soma/
  .config/
    config.json          # Settings, provider config
    keys.json            # API keys (0600 permissions, gitignored)
  identity/
    profile.md           # Core facts: name, location, family, work
    preferences.md       # Communication style, interests, values
  memories/
    2026-02-14.md        # Daily memory digests
  topics/
    crypto.md            # Accumulated context per topic
    rust.md
  people/
    avni.md              # People mentioned in conversations
  sources/
    chatgpt/
    claude/
    gemini/
```

All files are human-readable markdown. Open them in any text editor.

## Tech Stack

| Concern | Library |
|---------|---------|
| CLI | clap (derive macros) |
| TUI | ratatui + crossterm |
| HTTP | reqwest (direct Anthropic API) |
| JSON | serde + serde_json |
| Async | tokio |
| Progress | indicatif |
| Colors | colored |
| Errors | anyhow + thiserror |

## Color Palette

- **Purple** `#7C3AED` — primary brand
- **Cyan** `#06B6D4` — secondary / links
- **Amber** `#F59E0B` — accents / warnings
- **Emerald** `#10B981` — success
- **Red** `#EF4444` — errors

## Development

```bash
# Build
cargo build

# Run
cargo run

# Test
cargo test

# Lint
cargo clippy --all-targets -- -D warnings

# Format
cargo fmt

# Release build (~4.3 MB binary)
cargo build --release
```

### Git Hooks

After cloning, set up pre-commit hooks:

```bash
./scripts/setup-hooks.sh
```

This runs `cargo fmt --check`, `cargo clippy`, and `cargo test` before each commit.

### CI/CD

- **CI** runs on every push to `main` and PRs: build, test, clippy across macOS (arm64), Linux (x86_64, arm64)
- **Release** triggers on `v*` tags: builds binaries for all platforms, creates GitHub Release with SHA256 checksums

To cut a release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Architecture

```
src/
  main.rs              # Entry point, clap CLI
  cli/                 # Command implementations
    interactive.rs     # Ratatui TUI menu
    init.rs            # Setup wizard
    ingest.rs          # File ingestion pipeline
    export.rs          # Vault export
    status.rs          # Vault summary
  core/                # Business logic
    processor.rs       # Claude API calls via reqwest
    parser.rs          # JSON response parsing
    merger.rs          # Memory dedup + text chunking
    prompt.rs          # Extraction prompt template
  vault/               # Vault I/O
    config.rs          # Config/keys management
    read.rs            # Read vault content
    write.rs           # Write memories to vault
  extractors/          # File format handlers
    local.rs           # .md, .txt, .json, .jsonl
  types/               # All types, enums, errors
    mod.rs
  ui/                  # Terminal styling
    theme.rs           # Colors, icons, formatting
    widgets.rs         # Ratatui widgets
```

## License

MIT

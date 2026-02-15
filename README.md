# Soul Vault — Your AI Memory, Unified

Soul Vault is a Rust CLI (`soul`) that distills AI conversations into a structured local vault at `~/soul-vault/`.

It supports both command-driven workflows and a full-screen TUI (`soul` with no args).

## Install

### Quick Install (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/mastergaurang94/soul-vault/main/install.sh | bash
```

Installs the latest release binary to `~/.local/bin/soul`.

### Cargo

```bash
cargo install soul-vault
```

### From source

```bash
git clone https://github.com/mastergaurang94/soul-vault.git
cd soul-vault
cargo build --release
# Binary: target/release/soul
```

### GitHub Releases

Download prebuilt binaries from:
`https://github.com/mastergaurang94/soul-vault/releases`

## Usage

### Full-Screen TUI (`soul`)

```bash
soul
```

The no-args TUI has 9 pages:
- Status
- Pull
- Import
- Browse
- Export
- Watch
- Login
- Logout
- Settings

### Commands

#### `soul init`

Initializes the vault, provider config, and API key setup.

#### `soul import <folder> [--force]`

Imports local files recursively (`.md`, `.txt`, `.json`, `.jsonl`) into the vault.

Flags:
- `-f, --force`: re-import files even if source tracking says unchanged

#### `soul pull [--force] [--cloud] [--provider <claude|chatgpt|gemini>]`

Discovers provider sessions (Claude Code, OpenClaw, Gemini CLI, Codex) and imports them.

Notes:
- Local pull path performs a preflight Claude API key check and fails early if missing.
- `--cloud` enables OAuth/cloud scaffold flow.
- `--provider` applies to cloud mode (defaults to `claude`).

Flags:
- `-f, --force`: re-import all discovered sessions
- `--cloud`: use provider cloud mode instead of local session files
- `--provider <claude|chatgpt|gemini>`

#### `soul export [-o|--output <path>] [-f|--format <context|json|bundle>] [--topic <topic>] [--sections <csv>]`

Exports vault data.

Flags:
- `-o, --output <path>`: write to file/directory instead of stdout
- `-f, --format <context|json|bundle>`: output format (default `context`)
- `-t, --topic <topic>`: filter by topic
- `--sections <csv>`: comma-separated subset of `identity,preferences,topics,people,memories`

Examples:

```bash
# Context markdown to stdout
soul export

# JSON export
soul export --format json

# Bundle export (directory)
soul export --format bundle --output ~/soul-vault-bundle

# Only selected sections
soul export --sections identity,topics,memories
```

#### `soul status`

Shows vault summary, providers, and source stats.

#### `soul watch [folder]`

Watches files and auto-imports changes.

- With a folder: watches that folder recursively.
- Without a folder: auto-discovers provider base directories and watches them.

#### `soul reset [--force]`

Deletes vault/config state and returns to pre-init state.

Flags:
- `-f, --force`: skip confirmation prompt

#### `soul login [provider]`

Starts OAuth login (default provider: `claude`).

Supported provider values:
- `claude`
- `chatgpt`
- `gemini`

#### `soul logout [provider]`

Removes saved OAuth credentials.

- `soul logout`: clears all saved provider credentials
- `soul logout <provider>`: clears one provider

## Vault Structure

```text
~/soul-vault/
  .config/
    config.json
    keys.json
    sources.json
  auth.yaml
  identity/
    profile.md
    preferences.md
  memories/
    YYYY-MM-DD.md
  topics/
    <slug>.md
  people/
    <slug>.md
  sources/
    claude/
    chatgpt/
    gemini/
```

## Development

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

## License

MIT

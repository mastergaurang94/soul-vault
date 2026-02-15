# Adapters & Provider Integration — Current Spec
Last updated: 2026-02-15


## Overview

Soul Vault uses a pluggable adapter registry for `soul import` (providers mode) and auto-watch provider discovery.
Each adapter discovers local session files and parses them into a normalized `Conversation`.

## Current Adapter Set

```text
src/adapters/
├── mod.rs          # SessionAdapter trait, normalized types, AdapterRegistry
├── claude.rs       # ~/.claude/projects/**/*.jsonl
├── openclaw.rs     # ~/.openclaw/agents/*/sessions/*.jsonl
├── gemini.rs       # ~/.gemini/tmp/*/chats/session-*.json
└── codex.rs        # ~/.codex/sessions/**/rollout-*.jsonl
```

Notes:
- There is no ChatGPT adapter in `src/adapters/` right now.
- ChatGPT import support currently lives in the local extractor path (`soul import ...`), not adapter-based provider discovery.

## SessionAdapter Trait (Actual)

```rust
pub trait SessionAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn display_name(&self) -> &str;
    fn discover_sessions(&self) -> Result<Vec<SessionFile>>;
    fn parse_session(&self, path: &Path) -> Result<Conversation>;
    fn can_handle(&self, path: &Path) -> bool;
}
```

## Normalized Types

```rust
pub struct SessionFile {
    pub path: PathBuf,
    pub provider: String,
    pub project: Option<String>,
    pub modified: SystemTime,
}

pub struct Conversation {
    pub id: String,
    pub title: Option<String>,
    pub provider: String,
    pub created_at: Option<DateTime<Utc>>,
    pub messages: Vec<Message>,
}

pub struct Message {
    pub role: Role, // User | Assistant | System | Tool
    pub content: String,
    pub timestamp: Option<DateTime<Utc>>,
}
```

## AdapterRegistry API (Actual)

```rust
pub struct AdapterRegistry {
    adapters: Vec<Box<dyn SessionAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self;
    pub fn discover_all(&self) -> Vec<(String, Vec<SessionFile>)>;
    pub fn auto_detect(&self, path: &Path) -> Option<&dyn SessionAdapter>;
    pub fn base_dirs(&self) -> Vec<(String, PathBuf)>;
}
```

Behavior:
- `discover_all()` returns grouped results by adapter display name.
- Adapter-level discovery errors are swallowed as empty results (`unwrap_or_default`) to keep provider import resilient.
- `base_dirs()` is used by `soul watch` (no folder mode) to auto-watch provider roots.

## CLI Integration

### `soul import` (no folder)

Default path (no `--cloud`):
- Discovers sessions from Claude Code, OpenClaw, Gemini CLI, Codex.
- Runs preflight API key validation for Claude before discovery.
- Parses provider sessions via adapters, normalizes to text, then runs the memory extraction pipeline.

### `soul watch` (no folder argument)

Auto mode uses `AdapterRegistry::base_dirs()` and watches detected provider roots.

## Provider Parsing Notes

### Claude Code
- Root: `~/.claude/projects/`
- Session files: `*.jsonl` (excluding `agent-*.jsonl`)
- Parses message/event lines; attempts title from summary or first user message

### OpenClaw
- Root: `~/.openclaw/agents/*/sessions/`
- Session files: `*.jsonl` (excluding deleted/backup variants)
- Parses `session` and `message` records with role/content extraction

### Gemini CLI
- Root: `~/.gemini/tmp/<projectHash>/chats/`
- Session files: `session-*.json`
- Parses `messages[]` entries (`user`, `gemini`, `system`)

### Codex
- Root: `~/.codex/sessions/`
- Session files: `rollout-*.jsonl`
- Parses `session_meta`, `event_msg.user_message`, and `response_item` assistant content

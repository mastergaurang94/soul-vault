# Adapters & Provider Integration — Design Spec

## Overview

Soul Vault needs to import AI conversations from multiple sources. Inspired by
OneContext's adapter/registry pattern, we build a pluggable system where each
AI provider gets its own adapter that handles discovery, reading, and parsing.

## Architecture

```
src/adapters/
├── mod.rs          # AdapterRegistry + SessionAdapter trait
├── claude.rs       # Claude Code: reads ~/.claude/projects/**/*.jsonl
├── chatgpt.rs      # ChatGPT: reads conversations.json from data export
├── codex.rs        # OpenAI Codex: reads ~/.codex/sessions/**/*.jsonl
├── gemini.rs       # Gemini: reads session files (format TBD)
└── openclaw.rs     # OpenClaw: reads ~/.openclaw/agents/*/sessions/*.jsonl
```

## SessionAdapter Trait

```rust
pub trait SessionAdapter: Send + Sync {
    /// Unique name: "claude", "chatgpt", "codex", "gemini", "openclaw"
    fn name(&self) -> &str;
    
    /// Human-readable display name
    fn display_name(&self) -> &str;
    
    /// Auto-discover session files on disk (returns paths)
    fn discover_sessions(&self) -> Result<Vec<SessionFile>>;
    
    /// Parse a session file into normalized conversations
    fn parse_session(&self, path: &Path) -> Result<Vec<Conversation>>;
    
    /// Check if a file belongs to this adapter
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
    pub role: Role, // User, Assistant, System, Tool
    pub content: String,
    pub timestamp: Option<DateTime<Utc>>,
}
```

## Registry

```rust
pub struct AdapterRegistry {
    adapters: Vec<Box<dyn SessionAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self { /* register all built-in adapters */ }
    pub fn discover_all(&self) -> Result<Vec<SessionFile>> { /* merge all */ }
    pub fn auto_detect(&self, path: &Path) -> Option<&dyn SessionAdapter> { /* try each */ }
}
```

## CLI Integration

### `soul pull` (new command)
Auto-discovers sessions from all providers on disk:
```
$ soul pull
Discovering sessions...
  Claude Code: 47 sessions found
  OpenClaw: 12 sessions found
  ChatGPT: (no local sessions — use `soul import` with export folder)

Processing 59 sessions...
[████████░░] 42/59  Processing claude session abc123...
```

### `soul import <path>` (enhanced)
Existing command, now smarter:
- Auto-detects if path is a ChatGPT export zip/folder
- Auto-detects if path contains Claude/Codex/OpenClaw sessions  
- Falls back to generic file import for raw .md/.txt files

### `soul watch` (enhanced)  
Now watches provider session directories automatically:
```
$ soul watch
Watching:
  ~/.claude/projects/     (Claude Code)
  ~/.openclaw/agents/     (OpenClaw)
  ~/Downloads/            (ChatGPT exports)

[17:04:32] New session detected: ~/.claude/projects/-Users-gaurang-myproject/abc123.jsonl
[17:04:33] Importing 3 new turns...
```

## Provider Details

### Claude Code
- Location: `~/.claude/projects/<encoded-path>/*.jsonl`
- Format: JSONL, each line is a message event
- Key fields: `type`, `role`, `content`, `cwd`
- Skip: `agent-*.jsonl` (subtask files)

### ChatGPT Export
- Location: User downloads zip from Settings → Data Controls → Export
- Format: `conversations.json` — array of conversation objects with `mapping` tree
- Each node has `message.author.role` and `message.content.parts`
- Also includes `chat.html` (we ignore this, use JSON)

### OpenClaw
- Location: `~/.openclaw/agents/*/sessions/*.jsonl`
- Format: JSONL with message objects
- Key fields: `role`, `content`, `model`

### Codex (OpenAI)
- Location: `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`
- Format: JSONL with `session_meta` header, then message events

### Gemini
- TBD — need to research session storage format

## Non-Technical User Path

For users who aren't technical (Lumen UI target):
1. Go to ChatGPT → Settings → Export Data
2. Download the zip
3. Open Soul Vault (or Lumen) → Import → Select the downloaded zip
4. Soul Vault auto-detects it's a ChatGPT export, parses conversations.json, distills memories

The CLI equivalent: `soul import ~/Downloads/chatgpt-export.zip`

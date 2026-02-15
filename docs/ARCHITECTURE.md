# Soul Vault — Architecture
Last updated: 2026-02-15


## Module Map

```text
src/
├── main.rs                    CLI entrypoint (clap), tokio runtime, command dispatch
├── auth/
│   └── mod.rs                 OAuth credential storage + refresh lifecycle (`~/soul-vault/auth.yaml`)
├── adapters/                  Provider session discovery/parsing for `soul pull`
│   ├── mod.rs                 `SessionAdapter`, `AdapterRegistry`, normalized conversation types
│   ├── claude.rs              Claude Code adapter (`~/.claude/projects/**/*.jsonl`)
│   ├── openclaw.rs            OpenClaw adapter (`~/.openclaw/agents/*/sessions/*.jsonl`)
│   ├── gemini.rs              Gemini CLI adapter (`~/.gemini/tmp/*/chats/session-*.json`)
│   └── codex.rs               Codex adapter (`~/.codex/sessions/**/rollout-*.jsonl`)
├── cli/                       Command handlers
│   ├── mod.rs
│   ├── init.rs                `soul init`
│   ├── ingest.rs              `soul import <folder>` (hidden alias: `soul ingest`)
│   ├── pull.rs                `soul pull [--force] [--cloud] [--provider ...]`
│   ├── export.rs              `soul export --format context|json|bundle --sections ...`
│   ├── status.rs              `soul status`
│   ├── watch.rs               `soul watch [folder]`
│   ├── reset.rs               `soul reset [--force]`
│   ├── login.rs               `soul login [provider]`
│   ├── logout.rs              `soul logout [provider]`
│   └── interactive.rs         Legacy non-fullscreen menu path
├── core/                      Pipeline logic and LLM processing
│   ├── mod.rs
│   ├── pipeline.rs            Async import pipeline with progress channels
│   ├── processor.rs           LLM call orchestration
│   ├── parser.rs              Response parsing/validation
│   ├── merger.rs              Chunking + memory merge/dedup
│   └── prompt.rs              Extraction prompt template
├── extractors/
│   ├── mod.rs
│   └──                        Reserved for provider-specific extractor boundaries
├── tui/                       Full-screen ratatui app (`soul` with no args)
│   ├── mod.rs                 Event loop, key handling, async channel draining
│   ├── app.rs                 App/page/focus state
│   ├── layout.rs              Header/body/footer + non-TTY fallback help
│   ├── sidebar.rs             Sidebar navigation widget
│   ├── watcher.rs             Background file watching for TUI watch mode
│   └── pages/
│       ├── mod.rs             `PageWidget` trait + `PageAction`
│       ├── status.rs
│       ├── pull.rs
│       ├── import.rs
│       ├── import_render.rs
│       ├── browse.rs
│       ├── export.rs
│       ├── watch.rs
│       ├── watch_render.rs
│       ├── login.rs
│       ├── logout.rs
│       └── settings.rs
├── types/
│   └── mod.rs                 Shared domain/config/fact types (`Provider`, `ExtractedMemories`, etc.)
├── ui/
│   ├── mod.rs
│   ├── theme.rs               CLI/TUI color + icon helpers
│   └── widgets.rs             Reusable ratatui widget helpers
└── vault/
    ├── mod.rs
    ├── local.rs               Local file discovery/content extraction
    ├── chatgpt.rs             ChatGPT export parsing/formatting helpers
    ├── config.rs              Vault paths/config/keys helpers
    ├── read.rs                Vault reads/stats/content assembly
    ├── write.rs               Markdown write/update routines
    └── sources.rs             Source hashing/tracking for dedup
```

## TUI Page Model

The TUI currently has **9 pages** in this order:
1. Status
2. Pull
3. Import
4. Browse
5. Export
6. Watch
7. Login
8. Logout
9. Settings

## Dependency Direction

```text
types/         ← leaf (no internal imports)
ui/            ← standalone presentation helpers
vault/         ← depends on types/
auth/          ← depends on types/, vault/
extractors/    ← depends on types/
adapters/      ← normalized provider parsing layer
core/          ← depends on types/, vault/, ui/
tui/           ← depends on core/, vault/, ui/, auth/, adapters/, types/
cli/           ← depends on core/, vault/, extractors/, adapters/, auth/, ui/, types/
main.rs        ← depends on cli/, tui/, ui/
```

Rules:
- No upward imports from low-level modules into CLI.
- `types/` remains the shared leaf module.
- External input is validated at command/adapter/extractor boundaries.

## Import Pipeline (Local Files)

`import` path data flow:
1. Discover supported files (`vault/local.rs`)
2. Classify new/modified/skipped via source hashes (`vault/sources.rs`)
3. Extract content + chunk text (`vault/local.rs`, `core/merger.rs`)
4. Process each chunk via LLM (`core/processor.rs`, `core/parser.rs`)
5. Merge deduplicated memories (`core/merger.rs`)
6. Write markdown updates (`vault/write.rs`)
7. Update source tracking (`vault/sources.rs`)

## Vault Layout

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

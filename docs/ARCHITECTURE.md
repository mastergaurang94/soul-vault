# Soul Vault — Architecture

## Module Map

```
src/
├── main.rs                    Entry point. Clap CLI definition, tokio runtime, dispatch to commands.
├── tui/                       Full-screen ratatui TUI (launched by `soul` with no args)
│   ├── mod.rs                 Entry point, event loop, layout (header + sidebar + content + footer)
│   ├── app.rs                 App state (current page, sidebar selection, focus, vault status)
│   ├── sidebar.rs             Sidebar navigation widget with gold selection highlight
│   └── pages/                 Page implementations (each: render + handle_key)
│       ├── mod.rs             PageWidget trait + PageAction enum
│       ├── status.rs          Status dashboard (vault stats, providers, sources)
│       ├── browse.rs          Vault file browser (tree + preview)
│       ├── import.rs          Import workflow (folder input + validation)
│       ├── export.rs          Export options (format, topic filter, output path)
│       ├── watch.rs           Watch mode (folder input + guidance)
│       └── settings.rs        Config display (providers, LLM, vault path)
├── cli/                       Command implementations (thin wrappers orchestrating core + vault)
│   ├── mod.rs                 Module declarations
│   ├── init.rs                `soul init` — interactive setup wizard (providers, API keys, vault creation)
│   ├── ingest.rs              `soul import <folder>` — scan → classify → chunk → LLM → merge → write pipeline
│   ├── pull.rs                `soul pull` — discover provider sessions and import them
│   ├── export.rs              `soul export` — reads vault, builds context document (markdown or JSON)
│   ├── status.rs              `soul status` — vault summary (counts, providers, last sync)
│   ├── watch.rs               `soul watch [folder]` — watch local/provider folders and auto-import changes
│   ├── reset.rs               `soul reset` — destructive vault reset with safety checks
│   └── interactive.rs         Legacy inline menu (replaced by tui/, kept for reference)
├── core/                      Business logic (no I/O, no UI)
│   ├── mod.rs                 Module declarations
│   ├── processor.rs           LLM API calls — sends chunks to Claude via reqwest, returns ExtractedMemories
│   ├── parser.rs              JSON response parsing — handles markdown fencing, validates schema, enriches with source/date
│   ├── merger.rs              Memory dedup + text chunking — merge_all_memories(), chunk_text()
│   └── prompt.rs              EXTRACTION_PROMPT constant — the system prompt for memory extraction
├── vault/                     Vault I/O (filesystem operations)
│   ├── mod.rs                 Module declarations
│   ├── config.rs              Paths (vault_root, config_dir, etc.), config.json/keys.json read/write, scaffolding
│   ├── read.rs                Read vault content — stats, full export, markdown file access
│   ├── write.rs               Write memories — daily digests, topic/people files, identity/prefs append
│   └── sources.rs             Source tracking — SHA-256 hashing, sources.json, file classification for dedup
├── extractors/                File format handlers
│   ├── mod.rs                 Module declarations
│   ├── local.rs               Discovers .md/.txt/.json/.jsonl files, reads content, parses ChatGPT exports
│   └── chatgpt.rs             Placeholder for future ChatGPT API direct integration
├── types/                     All shared types (leaf node — no internal imports)
│   └── mod.rs                 Provider, Confidence, SoulVaultConfig, ExtractedMemories, all fact types, SoulVaultError
└── ui/                        Terminal presentation
    ├── mod.rs                 Module declarations
    ├── theme.rs               Color palette (gold/cyan/amber/emerald/red), icons, text formatters
    └── widgets.rs             Ratatui widgets: MenuItem, soul_vault_block
```

## Dependency Direction

```
types/     ← everything depends on this, it depends on nothing internal
core/      ← depends on types/, vault/config (for API keys)
vault/     ← depends on types/
extractors/← depends on types/
cli/       ← depends on core/, vault/, extractors/, ui/, types/
ui/        ← depends on nothing internal (standalone formatters)
main.rs    ← depends on cli/, ui/
```

**Rule:** Core never imports CLI. Types are leaf nodes. UI is standalone. TUI depends on vault/, ui/, cli/ (for export).

## Key Types (all in `types/mod.rs`)

| Type | Purpose |
|------|---------|
| `Provider` | Enum: Claude, ChatGpt, Gemini |
| `Confidence` | Enum: High, Medium, Low |
| `SoulVaultConfig` | Vault config (providers, processing LLM, vault path) |
| `KeysConfig` | HashMap<String, String> for API keys |
| `ExtractedMemories` | Container for all extracted facts (identity, preferences, decisions, relationships, topics, emotional_context) |
| `IdentityFact` | Identity fact (content, category, confidence, source, date) |
| `PreferenceFact` | Preference (content, type, confidence, source, date) |
| `DecisionFact` | Decision (content, context, confidence, source, date) |
| `RelationshipFact` | Relationship (person, content, role, confidence, source, date) |
| `TopicFact` | Topic (topic, content, opinion, confidence, source, date) |
| `FileInfo` | Discovered file metadata (path, name, extension, size) |
| `ChunkInfo` | Text chunk for LLM processing (content, source, index, total) |
| `VaultStats` | Vault summary for status command |
| `VaultContent` | Full vault content for export |
| `SoulVaultError` | Typed errors with actionable messages (thiserror) |

## Import Pipeline

The core data flow when you run `soul import <folder>`:

```
1. SCAN           discover_files(folder)                     → Vec<FileInfo>
   extractors/local.rs — recursive walk, filter by extension (.md/.txt/.json/.jsonl)

2. CLASSIFY       classify_files(base_path, file_paths)      → IngestClassification
   vault/sources.rs — compare SHA-256 hashes against sources.json, split into new/modified/skipped

3. READ & CHUNK   extract_file_content(file) + chunk_text()  → Vec<ChunkInfo>
   extractors/local.rs — read files, handle JSON/JSONL/text formats
   core/merger.rs — split at paragraph boundaries, max 80K chars/chunk

4. LLM EXTRACT    process_chunk(client, chunk)                → ExtractedMemories
   core/processor.rs — POST to Anthropic API with EXTRACTION_PROMPT
   core/parser.rs — parse JSON response, strip markdown fences, enrich with source/date

5. MERGE          merge_all_memories(results)                 → ExtractedMemories
   core/merger.rs — combine all extractions, deduplicate by normalized content key

6. WRITE          write_memories_to_vault(merged, date)       → WriteResult
   vault/write.rs — daily digest, topic files, people files, identity/preferences append
   Dedup: won't write content that already exists in the vault file

7. TRACK          update_source_tracking(base_path, files)
   vault/sources.rs — record file hashes in sources.json for future dedup
```

## Vault Layout

```
~/soul-vault/
  .config/
    config.json      Settings, provider config (created by `soul init`)
    keys.json        API keys (0600 permissions, gitignored)
    sources.json     File hash tracking for ingestion dedup
  identity/
    profile.md       Core identity facts
    preferences.md   Communication style, interests, values
  memories/
    2026-02-14.md    Daily memory digests
  topics/
    <slug>.md        Accumulated context per topic
  people/
    <slug>.md        People mentioned in conversations
  sources/
    chatgpt/         (reserved for provider-specific source storage)
    claude/
    gemini/
```

## Error Handling

- **Library code:** `anyhow::Result` for propagation, `thiserror` for typed errors in `SoulVaultError`
- **main.rs only:** `std::process::exit(1)` — the single place where errors become exit codes
- **Every error message tells the user what to do:** e.g., `"Run \`soul init\` to create your vault."`

# Soul Vault TUI — Full Application Spec
Last updated: 2026-02-15


## Vision

`soul` (no args) launches a persistent, full-screen ratatui TUI application.
This is the **primary user experience** — not a menu launcher, but the app itself.

Individual subcommands (`soul import <folder>`, `soul status`, etc.) remain for
scripting/CI/power-users and produce plain terminal output as they do today.

## Architecture

### Layout

```
┌─────────────────────────────────────────────────────────────────┐
│  Soul Vault ✦ Your AI memory, unified.                     [?] Help  │
├──────────────┬──────────────────────────────────────────────────┤
│              │                                                  │
│   Sidebar    │              Main Content Area                   │
│              │                                                  │
│  > Status    │   (renders the selected page)                    │
│    Import    │                                                  │
│    Browse    │                                                  │
│    Export    │                                                  │
│    Watch    │                                                  │
│    Settings  │                                                  │
│              │                                                  │
│              │                                                  │
│              │                                                  │
│              │                                                  │
│              │                                                  │
├──────────────┴──────────────────────────────────────────────────┤
│  j/k navigate  enter select  q quit  ? help                    │
└─────────────────────────────────────────────────────────────────┘
```

### Pages

1. **Status** (home/default) — vault overview, provider status, imported sources
   - Mirrors current `soul status` output but rendered as ratatui widgets
   - Shows: memory/topic/people counts, vault size, providers, last activity

2. **Import** — import workflow
   - Text input for folder path (with ~ expansion)
   - Shows progress inline: scanning → classifying → processing → writing
   - Displays results summary when complete

3. **Browse** — vault file browser
   - Tree view: identity/, memories/, topics/, people/, sources/
   - Navigate with j/k, enter to open file
   - File content rendered as scrollable markdown in the right pane
   - Search/filter with `/`

4. **Export** — export options
   - Format selection (markdown/json)
   - Topic filter input
   - Output path input (or stdout preview)
   - Shows word count / size preview

5. **Watch** — watch mode
   - Folder path input
   - Live event log showing file changes and import status
   - Stop with `q` or Esc (returns to sidebar)

6. **Settings** — replaces `init` for existing vaults
   - Provider configuration (enable/disable, API keys)
   - Processing LLM selection
   - Vault path display
   - Reset option (with confirmation)

### First-Run Experience

If vault is not initialized, TUI shows a setup wizard page instead of sidebar:
- Step-by-step init flow rendered in the TUI
- After completion, transitions to normal sidebar + Status page

### Keyboard Bindings

**Global:**
- `q` / `Esc` — quit (from sidebar) or back (from page)
- `?` — toggle help overlay
- `1-6` — jump to page by number
- `Tab` — toggle focus between sidebar and content

**Sidebar:**
- `j` / `↓` — move down
- `k` / `↑` — move up
- `Enter` — select page

**Content area (page-specific):**
- `j/k` or `↑/↓` — scroll
- `/` — search (in Browse)
- `Enter` — confirm/submit
- `Esc` — back to sidebar

### Colors

- Gold (#F5A623) — borders, selections, branding (warm, soulful)
- Cyan (#06B6D4) — links, paths, commands, accents
- Amber (#F59E0B) — warnings, stars, highlights
- Emerald (#10B981) — success, checkmarks
- Red (#EF4444) — errors
- DarkGray — dim/secondary text

### File Structure

```
src/
  tui/
    mod.rs          — TUI app entry point, event loop, layout
    app.rs          — App state struct
    sidebar.rs      — Sidebar navigation widget
    pages/
      mod.rs        — Page trait + registry
      status.rs     — Status dashboard page
      import.rs     — Import workflow page
      browse.rs     — Vault browser page
      export.rs     — Export options page
      watch.rs      — Watch mode page
      settings.rs   — Settings/config page
      init_wizard.rs — First-run setup wizard
    input.rs        — Text input widget
    help.rs         — Help overlay
```

### Key Design Decisions

1. **Alternate screen** — TUI uses ratatui's alternate screen (full takeover)
2. **Page trait** — each page implements a `Page` trait with `render()`, `handle_key()`, `tick()`
3. **Async support** — Import/Watch need async; use tokio channels to communicate between async tasks and TUI event loop
4. **Existing code reuse** — vault/read, vault/write, core/* are all reusable; we're replacing cli/* rendering, not business logic
5. **No breaking changes** — subcommands still work exactly as before

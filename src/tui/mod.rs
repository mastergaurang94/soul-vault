//! TUI entry point — alternate screen, event loop, async channel integration.

pub mod app;
pub mod layout;
pub mod pages;
pub mod sidebar;
pub mod watcher;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::io::{self, IsTerminal};
use std::time::Duration;
use tokio::sync::mpsc;

use self::app::{App, Focus, Page};
use self::layout::PageSet;
use self::pages::import::ImportPage;
use self::pages::watch::WatchPage;
use crate::core::pipeline::ImportProgress;
use crate::tui::pages::watch::WatchEvent;

// ─── Public Entry Point ───────────────────────────────────────────────────────

pub fn run() -> Result<()> {
    if !io::stdin().is_terminal() {
        layout::print_non_tty_help();
        return Ok(());
    }

    terminal::enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;

    let backend = ratatui::backend::CrosstermBackend::new(io::stdout());
    let terminal = ratatui::Terminal::new(backend)?;
    let result = run_app(terminal);

    io::stdout().execute(LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    result
}

// ─── App Loop ─────────────────────────────────────────────────────────────────

fn run_app(
    mut terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    let mut app = App::new();
    let mut pages = PageSet::new();
    let mut channels = Channels::default();

    loop {
        // 1. Drain async channels (non-blocking)
        drain_folder_import_progress(&mut pages.import, &mut channels);
        drain_watch_events(&mut pages.watch, &mut channels);
        drain_provider_import_progress(&mut pages.import, &mut channels);

        // 2. Draw
        terminal.draw(|frame| {
            layout::render_layout(frame.area(), frame.buffer_mut(), &app, &pages);
        })?;

        if app.should_quit {
            if let Some(tx) = channels.watch_stop_tx.take() {
                let _ = tx.try_send(());
            }
            break;
        }

        // 3. Poll for crossterm events (short timeout for responsive channel draining)
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                handle_key(key, &mut app, &mut pages, &mut channels);
            }
        }
    }

    Ok(())
}

// ─── Channels ─────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Channels {
    import_rx: Option<mpsc::Receiver<ImportProgress>>,
    watch_event_rx: Option<mpsc::Receiver<WatchEvent>>,
    watch_stop_tx: Option<mpsc::Sender<()>>,
    pull_rx: Option<mpsc::Receiver<String>>,
}

fn drain_folder_import_progress(import: &mut ImportPage, channels: &mut Channels) {
    if let Some(rx) = &mut channels.import_rx {
        while let Ok(progress) = rx.try_recv() {
            let is_terminal = matches!(
                progress,
                ImportProgress::Done(_)
                    | ImportProgress::Error(_)
                    | ImportProgress::NothingToImport { .. }
            );
            import.on_folder_progress(progress);
            if is_terminal {
                channels.import_rx = None;
                break;
            }
        }
    }
}

fn drain_watch_events(watch: &mut WatchPage, channels: &mut Channels) {
    if let Some(rx) = &mut channels.watch_event_rx {
        while let Ok(event) = rx.try_recv() {
            watch.on_event(event);
        }
    }
}

fn drain_provider_import_progress(import: &mut ImportPage, channels: &mut Channels) {
    if let Some(rx) = &mut channels.pull_rx {
        while let Ok(msg) = rx.try_recv() {
            if let Some(summary_str) = msg.strip_prefix("DONE:") {
                let summary: Vec<String> = summary_str.split('\n').map(String::from).collect();
                import.on_provider_done(summary);
                channels.pull_rx = None;
                return;
            } else if let Some(error_str) = msg.strip_prefix("ERROR:") {
                import.on_provider_error(error_str.to_string());
                channels.pull_rx = None;
                return;
            } else {
                import.on_provider_progress(msg);
            }
        }
    }
}

// ─── Key Handling ─────────────────────────────────────────────────────────────

fn handle_key(
    key: crossterm::event::KeyEvent,
    app: &mut App,
    pages: &mut PageSet,
    channels: &mut Channels,
) {
    // Global keys
    match key.code {
        KeyCode::Char('q') if app.focus == Focus::Sidebar => {
            app.should_quit = true;
            return;
        }
        KeyCode::Char('?') => {
            app.show_help = !app.show_help;
            return;
        }
        KeyCode::Tab if !(app.focus == Focus::Content && app.current_page == Page::Import) => {
            app.toggle_focus();
            return;
        }
        KeyCode::Char(c @ '1'..='9') if app.focus == Focus::Sidebar => {
            let idx = (c as usize) - ('1' as usize);
            app.select_page(idx);
            app.focus = Focus::Content;
            return;
        }
        _ => {}
    }

    // Sidebar handling
    if app.focus == Focus::Sidebar {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => app.sidebar_down(),
            KeyCode::Char('k') | KeyCode::Up => app.sidebar_up(),
            KeyCode::Enter => app.confirm_sidebar(),
            KeyCode::Esc => app.should_quit = true,
            _ => {}
        }
        return;
    }

    // Content handling — delegate to current page
    let action = pages.current_mut(app.current_page).handle_key(key, app);
    match action {
        pages::PageAction::BackToSidebar => app.focus = Focus::Sidebar,
        pages::PageAction::StartImport(folder) => {
            start_import(&folder, &mut pages.import, channels);
        }
        pages::PageAction::StartWatch(folder) => {
            start_watch(&folder, &mut pages.watch, channels);
        }
        pages::PageAction::StopWatch => {
            stop_watch(&mut pages.watch, channels);
            app.focus = Focus::Sidebar;
        }
        pages::PageAction::StartProviderImport => {
            start_provider_import(&mut pages.import, channels);
        }
        pages::PageAction::Consumed | pages::PageAction::Ignored => {}
    }
}

// ─── Async Task Spawners ──────────────────────────────────────────────────────

fn start_import(folder: &str, import_page: &mut ImportPage, channels: &mut Channels) {
    let (tx, rx) = mpsc::channel(64);
    channels.import_rx = Some(rx);
    import_page.on_folder_progress(ImportProgress::Scanning);

    let folder = folder.to_string();
    tokio::spawn(async move {
        crate::core::pipeline::run_import(folder, tx).await;
    });
}

fn start_watch(folder: &str, watch_page: &mut WatchPage, channels: &mut Channels) {
    if let Some(tx) = channels.watch_stop_tx.take() {
        let _ = tx.try_send(());
    }

    let (event_tx, event_rx) = mpsc::channel(256);
    let (stop_tx, stop_rx) = mpsc::channel(1);

    channels.watch_event_rx = Some(event_rx);
    channels.watch_stop_tx = Some(stop_tx);
    watch_page.start_watching(folder);

    let folder = folder.to_string();
    watcher::start_watcher(folder, event_tx, stop_rx);
}

fn start_provider_import(import_page: &mut ImportPage, channels: &mut Channels) {
    let (tx, rx) = mpsc::channel(64);
    channels.pull_rx = Some(rx);
    import_page.on_provider_progress("Discovering AI sessions...".to_string());

    tokio::spawn(async move {
        use crate::adapters::{conversation_to_text, AdapterRegistry};
        use crate::core::merger::{chunk_text, merge_all_memories};
        use crate::core::processor::process_chunk;
        use crate::vault::write::write_memories_to_vault;

        let registry = AdapterRegistry::new();
        let discovered = registry.discover_all();

        let mut total = 0;
        for (name, sessions) in &discovered {
            total += sessions.len();
            let _ = tx
                .send(format!("{}: {} sessions", name, sessions.len()))
                .await;
        }

        if total == 0 {
            let _ = tx.send("ERROR:No AI sessions found.".to_string()).await;
            return;
        }

        let all_sessions: Vec<_> = discovered.into_iter().flat_map(|(_, s)| s).collect();

        // Parse sessions into chunks
        let mut all_chunks = Vec::new();
        for session in &all_sessions {
            if let Some(adapter) = registry.auto_detect(&session.path) {
                if let Ok(conv) = adapter.parse_session(&session.path) {
                    if !conv.messages.is_empty() {
                        let text = conversation_to_text(&conv);
                        if !text.trim().is_empty() {
                            all_chunks.extend(chunk_text(&text, &conv.id));
                        }
                    }
                }
            }
        }

        let _ = tx.send(format!("Parsed {} chunks", all_chunks.len())).await;

        if all_chunks.is_empty() {
            let _ = tx
                .send("DONE:No meaningful content found.".to_string())
                .await;
            return;
        }

        // Process through LLM
        let client = reqwest::Client::new();
        let mut all_memories = Vec::new();
        let chunk_count = all_chunks.len();

        for (i, chunk) in all_chunks.iter().enumerate() {
            let _ = tx
                .send(format!("Processing {}/{}", i + 1, chunk_count))
                .await;
            if let Ok(memories) = process_chunk(&client, chunk).await {
                all_memories.push(memories);
            }
        }

        let merged = merge_all_memories(&all_memories);
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        match write_memories_to_vault(&merged, &today) {
            Ok(result) => {
                let summary = format!(
                    "DONE:{} sessions processed\n{} memories extracted\n{} topics\n{} people",
                    all_sessions.len(),
                    merged.fact_count(),
                    result.topics_written.len(),
                    result.people_written.len()
                );
                let _ = tx.send(summary).await;
            }
            Err(e) => {
                let _ = tx.send(format!("ERROR:{}", e)).await;
            }
        }
    });
}

fn stop_watch(watch_page: &mut WatchPage, channels: &mut Channels) {
    if let Some(tx) = channels.watch_stop_tx.take() {
        let _ = tx.try_send(());
    }
    channels.watch_event_rx = None;
    watch_page.stop_watching();
}

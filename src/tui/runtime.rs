//! TUI runtime loop — terminal setup, draw cycle, and key routing.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::io::{self, IsTerminal};
use std::time::Duration;

use super::app::{App, Focus, Page};
use super::layout;
use super::layout::PageSet;
use super::pages;
use super::runtime_tasks::{self, Channels};

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
        runtime_tasks::drain_folder_import_progress(&mut pages.import, &mut channels);
        runtime_tasks::drain_watch_events(&mut pages.watch, &mut channels);
        runtime_tasks::drain_provider_import_progress(&mut pages.import, &mut channels);

        terminal.draw(|frame| {
            layout::render_layout(frame.area(), frame.buffer_mut(), &app, &pages);
        })?;

        if app.should_quit {
            runtime_tasks::shutdown_watcher(&mut channels);
            break;
        }

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

// ─── Key Handling ─────────────────────────────────────────────────────────────

fn handle_key(
    key: crossterm::event::KeyEvent,
    app: &mut App,
    pages: &mut PageSet,
    channels: &mut Channels,
) {
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

    let action = pages.current_mut(app.current_page).handle_key(key, app);
    match action {
        pages::PageAction::BackToSidebar => app.focus = Focus::Sidebar,
        pages::PageAction::StartImport(folder) => {
            runtime_tasks::start_import(&folder, &mut pages.import, channels);
        }
        pages::PageAction::StartWatch(folder) => {
            runtime_tasks::start_watch(&folder, &mut pages.watch, channels);
        }
        pages::PageAction::StopWatch => {
            runtime_tasks::stop_watch(&mut pages.watch, channels);
            app.focus = Focus::Sidebar;
        }
        pages::PageAction::StartProviderImport => {
            runtime_tasks::start_provider_import(&mut pages.import, channels);
        }
        pages::PageAction::Consumed | pages::PageAction::Ignored => {}
    }
}

//! Page modules — each page implements rendering and key handling.

pub mod browse;
pub mod export;
pub mod import;
pub mod import_render;
pub mod pull;
pub mod settings;
pub mod status;
pub mod watch;
pub mod watch_render;

use crossterm::event::KeyEvent;
use ratatui::{buffer::Buffer, layout::Rect};

use crate::tui::app::App;

// ─── Page Action ──────────────────────────────────────────────────────────────

/// Result of a page handling a key event.
pub enum PageAction {
    /// Key was consumed by the page.
    Consumed,
    /// Key was not handled — bubble up to global handler.
    Ignored,
    /// Page wants to return focus to sidebar.
    BackToSidebar,
    /// Start an async import for the given folder path.
    StartImport(String),
    /// Start file watching for the given folder path.
    StartWatch(String),
    /// Stop the active file watcher.
    StopWatch,
    /// Start pulling AI sessions from all providers.
    StartPull,
}

// ─── Page Trait ───────────────────────────────────────────────────────────────

/// Each page implements this trait for rendering and input handling.
pub trait PageWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, app: &App);
    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> PageAction;
}

//! Watch page — folder input, live event log for file changes.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Widget},
};

use super::watch_render;
use crate::tui::app::App;
use crate::tui::pages::{PageAction, PageWidget};
use crate::ui::theme::rat;

// ─── Watch Event Types ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum EventKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct WatchEvent {
    pub timestamp: String,
    pub message: String,
    pub kind: EventKind,
}

// ─── Watch State ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum WatchPhase {
    Input,
    Watching {
        #[allow(dead_code)]
        folder: String,
        events: Vec<WatchEvent>,
        scroll: usize,
    },
    #[allow(dead_code)]
    Error(String),
}

pub struct WatchPage {
    input: String,
    cursor: usize,
    pub phase: WatchPhase,
}

impl Default for WatchPage {
    fn default() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            phase: WatchPhase::Input,
        }
    }
}

impl WatchPage {
    /// Called by the event loop when a watch event arrives.
    pub fn on_event(&mut self, event: WatchEvent) {
        if let WatchPhase::Watching { events, scroll, .. } = &mut self.phase {
            events.push(event);
            *scroll = events.len().saturating_sub(1);
        }
    }

    /// Transition to watching state.
    pub fn start_watching(&mut self, folder: &str) {
        let now = chrono::Local::now().format("%H:%M:%S").to_string();
        self.phase = WatchPhase::Watching {
            folder: folder.to_string(),
            events: vec![WatchEvent {
                timestamp: now,
                message: format!("Watching {} for changes...", folder),
                kind: EventKind::Info,
            }],
            scroll: 0,
        };
    }

    /// Return to input state (when watcher is stopped).
    pub fn stop_watching(&mut self) {
        self.phase = WatchPhase::Input;
        self.input.clear();
        self.cursor = 0;
    }
}

// ─── PageWidget ───────────────────────────────────────────────────────────────

impl PageWidget for WatchPage {
    fn render(&self, area: Rect, buf: &mut Buffer, app: &App) {
        if !app.vault_initialized {
            watch_render::render_not_init(area, buf);
            return;
        }
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(rat::GOLD))
            .title(" Watch ")
            .title_style(Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD));
        let inner = block.inner(area);
        block.render(area, buf);

        match &self.phase {
            WatchPhase::Input => watch_render::render_input(inner, buf, &self.input),
            WatchPhase::Watching { events, scroll, .. } => {
                watch_render::render_watching(inner, buf, events, *scroll)
            }
            WatchPhase::Error(msg) => watch_render::render_error(inner, buf, msg),
        }
    }

    fn handle_key(&mut self, key: KeyEvent, _app: &mut App) -> PageAction {
        match &self.phase {
            WatchPhase::Input => handle_input_key(key, &mut self.input, &mut self.cursor),
            WatchPhase::Watching { .. } => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if let WatchPhase::Watching { scroll, events, .. } = &mut self.phase {
                        if *scroll < events.len().saturating_sub(1) {
                            *scroll += 1;
                        }
                    }
                    PageAction::Consumed
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if let WatchPhase::Watching { scroll, .. } = &mut self.phase {
                        *scroll = scroll.saturating_sub(1);
                    }
                    PageAction::Consumed
                }
                KeyCode::Esc => PageAction::StopWatch,
                _ => PageAction::Ignored,
            },
            WatchPhase::Error(_) => match key.code {
                KeyCode::Enter | KeyCode::Char('r') => {
                    self.phase = WatchPhase::Input;
                    self.input.clear();
                    self.cursor = 0;
                    PageAction::Consumed
                }
                KeyCode::Esc => {
                    self.phase = WatchPhase::Input;
                    self.input.clear();
                    self.cursor = 0;
                    PageAction::BackToSidebar
                }
                _ => PageAction::Ignored,
            },
        }
    }
}

// ─── Input Key Handling ───────────────────────────────────────────────────────

fn handle_input_key(key: KeyEvent, input: &mut String, cursor: &mut usize) -> PageAction {
    match key.code {
        KeyCode::Char(c) => {
            input.insert(*cursor, c);
            *cursor += 1;
            PageAction::Consumed
        }
        KeyCode::Backspace => {
            if *cursor > 0 {
                *cursor -= 1;
                input.remove(*cursor);
            }
            PageAction::Consumed
        }
        KeyCode::Left => {
            *cursor = cursor.saturating_sub(1);
            PageAction::Consumed
        }
        KeyCode::Right => {
            if *cursor < input.len() {
                *cursor += 1;
            }
            PageAction::Consumed
        }
        KeyCode::Enter => {
            if input.trim().is_empty() {
                return PageAction::Consumed;
            }
            let path = expand_tilde(input);
            let abs = std::path::Path::new(&path);
            if !abs.exists() {
                return PageAction::Consumed;
            }
            PageAction::StartWatch(path)
        }
        KeyCode::Esc => {
            input.clear();
            *cursor = 0;
            PageAction::BackToSidebar
        }
        _ => PageAction::Ignored,
    }
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            return path.replacen('~', &home.display().to_string(), 1);
        }
    }
    path.to_string()
}

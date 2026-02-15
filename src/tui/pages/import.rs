//! Import page — folder input, async import with progress, results summary.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Widget},
};

use super::import_render;
use crate::core::pipeline::{ImportProgress, ImportResult};
use crate::tui::app::App;
use crate::tui::pages::{PageAction, PageWidget};
use crate::ui::theme::rat;

// ─── Import State ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ImportPhase {
    Input,
    Scanning,
    Classifying,
    Processing {
        current: usize,
        total: usize,
        current_file: String,
    },
    Writing,
    Done(ImportResult),
    NothingToImport { skipped_count: usize },
    Error(String),
}

pub struct ImportPage {
    input: String,
    cursor: usize,
    pub phase: ImportPhase,
}

impl Default for ImportPage {
    fn default() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            phase: ImportPhase::Input,
        }
    }
}

impl ImportPage {
    /// Called by the event loop when an ImportProgress message arrives.
    pub fn on_progress(&mut self, progress: ImportProgress) {
        self.phase = match progress {
            ImportProgress::Scanning | ImportProgress::ScanComplete { .. } => {
                ImportPhase::Scanning
            }
            ImportProgress::Classifying | ImportProgress::ClassifyComplete { .. } => {
                ImportPhase::Classifying
            }
            ImportProgress::NothingToImport { skipped_count } => {
                ImportPhase::NothingToImport { skipped_count }
            }
            ImportProgress::Processing {
                current,
                total,
                current_file,
            } => ImportPhase::Processing {
                current,
                total,
                current_file,
            },
            ImportProgress::Writing => ImportPhase::Writing,
            ImportProgress::Done(result) => ImportPhase::Done(result),
            ImportProgress::Error(msg) => ImportPhase::Error(msg),
        };
    }
}

// ─── PageWidget ───────────────────────────────────────────────────────────────

impl PageWidget for ImportPage {
    fn render(&self, area: Rect, buf: &mut Buffer, app: &App) {
        if !app.vault_initialized {
            import_render::render_not_init(area, buf);
            return;
        }
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(rat::GOLD))
            .title(" Import ")
            .title_style(
                Style::default()
                    .fg(rat::GOLD)
                    .add_modifier(Modifier::BOLD),
            );
        let inner = block.inner(area);
        block.render(area, buf);

        match &self.phase {
            ImportPhase::Input => import_render::render_input(inner, buf, &self.input),
            ImportPhase::Scanning => {
                import_render::render_phase(inner, buf, "Scanning for files...")
            }
            ImportPhase::Classifying => {
                import_render::render_phase(inner, buf, "Checking for changes...")
            }
            ImportPhase::Processing {
                current,
                total,
                current_file,
            } => import_render::render_processing(inner, buf, *current, *total, current_file),
            ImportPhase::Writing => {
                import_render::render_phase(inner, buf, "Merging and writing to vault...")
            }
            ImportPhase::Done(result) => import_render::render_done(inner, buf, result),
            ImportPhase::NothingToImport { skipped_count } => {
                import_render::render_nothing(inner, buf, *skipped_count)
            }
            ImportPhase::Error(msg) => import_render::render_error(inner, buf, msg),
        }
    }

    fn handle_key(&mut self, key: KeyEvent, _app: &mut App) -> PageAction {
        match &self.phase {
            ImportPhase::Input => handle_input_key(key, &mut self.input, &mut self.cursor),
            ImportPhase::Scanning
            | ImportPhase::Classifying
            | ImportPhase::Processing { .. }
            | ImportPhase::Writing => PageAction::Consumed,
            ImportPhase::Done(_)
            | ImportPhase::NothingToImport { .. }
            | ImportPhase::Error(_) => match key.code {
                KeyCode::Enter | KeyCode::Char('r') => {
                    self.phase = ImportPhase::Input;
                    self.input.clear();
                    self.cursor = 0;
                    PageAction::Consumed
                }
                KeyCode::Esc => {
                    self.phase = ImportPhase::Input;
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
            PageAction::StartImport(path)
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

//! Export page — format selection, section toggles, preview, and execute.

use crossterm::event::{KeyCode, KeyEvent};

use super::export_render;
use super::export_state::{ExportField, ExportFormat};
use crate::tui::app::App;
use crate::tui::pages::{PageAction, PageWidget};

pub struct ExportPage {
    pub(super) format: ExportFormat,
    pub(super) include_identity: bool,
    pub(super) include_preferences: bool,
    pub(super) include_topics: bool,
    pub(super) include_people: bool,
    pub(super) include_memories: bool,
    pub(super) active_field: ExportField,
    pub(super) result_msg: Option<(bool, String)>,
}

impl Default for ExportPage {
    fn default() -> Self {
        Self {
            format: ExportFormat::Context,
            include_identity: true,
            include_preferences: true,
            include_topics: true,
            include_people: true,
            include_memories: true,
            active_field: ExportField::Format,
            result_msg: None,
        }
    }
}

// ─── PageWidget ───────────────────────────────────────────────────────────────

impl PageWidget for ExportPage {
    fn render(&self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer, app: &App) {
        if !app.vault_initialized {
            export_render::render_not_init(area, buf);
            return;
        }
        export_render::render_form(area, buf, self);
    }

    fn handle_key(&mut self, key: KeyEvent, _app: &mut App) -> PageAction {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.active_field = self.active_field.next();
                PageAction::Consumed
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.active_field = self.active_field.prev();
                PageAction::Consumed
            }
            KeyCode::Enter => {
                self.select_active();
                PageAction::Consumed
            }
            KeyCode::Char(' ') => {
                if self.toggle_active_section() {
                    PageAction::Consumed
                } else {
                    PageAction::Ignored
                }
            }
            KeyCode::Esc => {
                self.result_msg = None;
                PageAction::BackToSidebar
            }
            _ => PageAction::Ignored,
        }
    }
}

impl ExportPage {
    fn selected_sections_count(&self) -> usize {
        [
            self.include_identity,
            self.include_preferences,
            self.include_topics,
            self.include_people,
            self.include_memories,
        ]
        .iter()
        .filter(|enabled| **enabled)
        .count()
    }

    fn select_active(&mut self) {
        match self.active_field {
            ExportField::Format => self.format = self.format.next(),
            ExportField::Execute => self.execute_export(),
            _ => {
                self.toggle_active_section();
            }
        }
    }

    fn toggle_active_section(&mut self) -> bool {
        match self.active_field {
            ExportField::Identity => self.toggle_identity(),
            ExportField::Preferences => self.toggle_preferences(),
            ExportField::Topics => self.toggle_topics(),
            ExportField::People => self.toggle_people(),
            ExportField::Memories => self.toggle_memories(),
            _ => false,
        }
    }

    fn toggle_identity(&mut self) -> bool {
        if self.include_identity && self.selected_sections_count() == 1 {
            self.result_msg = Some((false, "Select at least one section to export.".to_string()));
            return true;
        }
        self.include_identity = !self.include_identity;
        self.result_msg = None;
        true
    }

    fn toggle_preferences(&mut self) -> bool {
        if self.include_preferences && self.selected_sections_count() == 1 {
            self.result_msg = Some((false, "Select at least one section to export.".to_string()));
            return true;
        }
        self.include_preferences = !self.include_preferences;
        self.result_msg = None;
        true
    }

    fn toggle_topics(&mut self) -> bool {
        if self.include_topics && self.selected_sections_count() == 1 {
            self.result_msg = Some((false, "Select at least one section to export.".to_string()));
            return true;
        }
        self.include_topics = !self.include_topics;
        self.result_msg = None;
        true
    }

    fn toggle_people(&mut self) -> bool {
        if self.include_people && self.selected_sections_count() == 1 {
            self.result_msg = Some((false, "Select at least one section to export.".to_string()));
            return true;
        }
        self.include_people = !self.include_people;
        self.result_msg = None;
        true
    }

    fn toggle_memories(&mut self) -> bool {
        if self.include_memories && self.selected_sections_count() == 1 {
            self.result_msg = Some((false, "Select at least one section to export.".to_string()));
            return true;
        }
        self.include_memories = !self.include_memories;
        self.result_msg = None;
        true
    }

    pub(super) fn output_path(&self) -> String {
        match crate::cli::export::smart_default_output_path(self.format.arg()) {
            Ok(path) => path.display().to_string(),
            Err(_) => "<unable to build path>".to_string(),
        }
    }

    fn selected_sections_csv(&self) -> String {
        let mut sections = Vec::new();
        if self.include_identity {
            sections.push("identity");
        }
        if self.include_preferences {
            sections.push("preferences");
        }
        if self.include_topics {
            sections.push("topics");
        }
        if self.include_people {
            sections.push("people");
        }
        if self.include_memories {
            sections.push("memories");
        }
        sections.join(",")
    }

    fn execute_export(&mut self) {
        let output_path = self.output_path();
        let sections = self.selected_sections_csv();
        match crate::cli::export::run(Some(&output_path), self.format.arg(), None, Some(&sections))
        {
            Ok(()) => {
                self.result_msg = Some((true, format!("Exported to {}", output_path)));
            }
            Err(e) => {
                self.result_msg = Some((false, e.to_string()));
            }
        }
    }
}

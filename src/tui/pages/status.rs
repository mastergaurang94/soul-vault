//! Status page — vault overview, providers, imported sources.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{buffer::Buffer, layout::Rect};

use super::status_render;
use crate::tui::app::App;
use crate::tui::pages::{PageAction, PageWidget};

#[derive(Default)]
pub struct StatusPage {
    pub scroll: u16,
}

impl PageWidget for StatusPage {
    fn render(&self, area: Rect, buf: &mut Buffer, app: &App) {
        if !app.vault_initialized {
            status_render::render_not_initialized(area, buf);
            return;
        }
        status_render::render_dashboard(area, buf, self.scroll);
    }

    fn handle_key(&mut self, key: KeyEvent, _app: &mut App) -> PageAction {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1);
                PageAction::Consumed
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                PageAction::Consumed
            }
            KeyCode::Esc => PageAction::BackToSidebar,
            _ => PageAction::Ignored,
        }
    }
}

//! Logout page — clear saved OAuth credentials.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::auth::{is_logged_in, remove_credentials};
use crate::tui::app::App;
use crate::tui::pages::{PageAction, PageWidget};
use crate::types::Provider;
use crate::ui::theme::rat;

#[derive(Default)]
pub struct LogoutPage {
    message: Option<String>,
}

impl PageWidget for LogoutPage {
    fn render(&self, area: Rect, buf: &mut Buffer, app: &App) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(rat::GOLD))
            .title(" Logout — Credentials ")
            .title_style(Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD));
        let inner = block.inner(area);
        block.render(area, buf);

        if !app.vault_initialized {
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Vault not initialized. Run `soul init` first.",
                    Style::default().fg(rat::AMBER),
                )),
            ])
            .render(inner, buf);
            return;
        }

        render_content(inner, buf, self.message.as_deref());
    }

    fn handle_key(&mut self, key: KeyEvent, _app: &mut App) -> PageAction {
        let logged_in = is_logged_in(&Provider::Claude).unwrap_or(false);

        match key.code {
            KeyCode::Enter if logged_in => {
                self.message = match remove_credentials(&Provider::Claude) {
                    Ok(_) => Some("Credentials cleared.".to_string()),
                    Err(e) => Some(format!("Failed to clear credentials: {}", e)),
                };
                PageAction::Consumed
            }
            KeyCode::Esc => {
                self.message = None;
                PageAction::BackToSidebar
            }
            KeyCode::Enter => {
                self.message = Some("No active Claude credentials to clear.".to_string());
                PageAction::Consumed
            }
            _ => PageAction::Ignored,
        }
    }
}

fn render_content(area: Rect, buf: &mut Buffer, message: Option<&str>) {
    let logged_in = is_logged_in(&Provider::Claude).unwrap_or(false);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Logout — Clear Credentials",
            Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if logged_in {
        lines.push(Line::from(Span::styled(
            "  You are currently logged in to Claude. Press Enter to logout.",
            Style::default(),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  No active sessions.",
            Style::default().fg(rat::DIM),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press Esc to go back",
        Style::default().fg(rat::DIM),
    )));

    if let Some(msg) = message {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", msg),
            Style::default().fg(rat::CYAN),
        )));
    }

    Paragraph::new(lines).render(area, buf);
}

//! Login page — show OAuth provider status and login guidance.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::auth::is_logged_in;
use crate::tui::app::App;
use crate::tui::pages::{PageAction, PageWidget};
use crate::types::Provider;
use crate::ui::theme::rat;

#[derive(Default)]
pub struct LoginPage {
    message: Option<String>,
}

impl PageWidget for LoginPage {
    fn render(&self, area: Rect, buf: &mut Buffer, app: &App) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(rat::GOLD))
            .title(" Login ")
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
        match key.code {
            KeyCode::Enter => {
                self.message = Some("Starting OAuth login... Check your browser.".to_string());
                PageAction::Consumed
            }
            KeyCode::Esc => {
                self.message = None;
                PageAction::BackToSidebar
            }
            _ => PageAction::Ignored,
        }
    }
}

fn render_content(area: Rect, buf: &mut Buffer, message: Option<&str>) {
    let claude_logged_in = is_logged_in(&Provider::Claude).unwrap_or(false);
    let claude_status = if claude_logged_in {
        "✓ logged in"
    } else {
        "x not logged in"
    };
    let claude_color = if claude_logged_in {
        rat::EMERALD
    } else {
        rat::RED
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Login — Cloud Provider OAuth",
            Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Authenticate with a cloud provider to pull conversations via API.",
            Style::default(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Providers",
            Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("    Claude      "),
            Span::styled(claude_status, Style::default().fg(claude_color)),
        ]),
        Line::from(Span::styled(
            "    ChatGPT     x coming soon",
            Style::default().fg(rat::DIM),
        )),
        Line::from(Span::styled(
            "    Gemini      x coming soon",
            Style::default().fg(rat::DIM),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Press Enter to start OAuth login, Esc to go back",
            Style::default().fg(rat::DIM),
        )),
    ];

    if let Some(msg) = message {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", msg),
            Style::default().fg(rat::CYAN),
        )));
    }

    Paragraph::new(lines).render(area, buf);
}

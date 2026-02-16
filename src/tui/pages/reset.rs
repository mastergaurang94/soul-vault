//! TUI page for vault reset — deletion with confirmation.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::tui::app::App;
use crate::tui::pages::{PageAction, PageWidget};
use crate::ui::theme::rat;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Phase {
    Confirm,
    Done,
    Error(String),
}

pub struct ResetPage {
    phase: Phase,
    selected: usize, // 0 = Cancel, 1 = Confirm
}

impl Default for ResetPage {
    fn default() -> Self {
        Self {
            phase: Phase::Confirm,
            selected: 0,
        }
    }
}

impl PageWidget for ResetPage {
    fn render(&self, area: Rect, buf: &mut Buffer, app: &App) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(rat::GOLD))
            .title(" Reset ")
            .title_style(Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD));
        let inner = block.inner(area);
        block.render(area, buf);

        if !app.vault_initialized {
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Vault not initialized. Nothing to reset.",
                    Style::default().fg(rat::AMBER),
                )),
            ])
            .render(inner, buf);
            return;
        }

        match &self.phase {
            Phase::Confirm => render_confirm(inner, buf, self.selected),
            Phase::Done => render_done(inner, buf),
            Phase::Error(msg) => render_error(inner, buf, msg),
        }
    }

    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> PageAction {
        match &self.phase {
            Phase::Confirm => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.selected = 1;
                    PageAction::Consumed
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.selected = 0;
                    PageAction::Consumed
                }
                KeyCode::Enter => {
                    if self.selected == 0 {
                        self.phase = Phase::Confirm;
                        self.selected = 0;
                        PageAction::BackToSidebar
                    } else {
                        match crate::cli::reset::delete_vault() {
                            Ok(()) => {
                                self.phase = Phase::Done;
                                app.vault_initialized = false;
                                app.should_quit = true;
                            }
                            Err(e) => self.phase = Phase::Error(e.to_string()),
                        }
                        PageAction::Consumed
                    }
                }
                KeyCode::Esc => {
                    self.selected = 0;
                    PageAction::BackToSidebar
                }
                _ => PageAction::Ignored,
            },
            Phase::Done | Phase::Error(_) => match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.phase = Phase::Confirm;
                    self.selected = 0;
                    PageAction::BackToSidebar
                }
                _ => PageAction::Ignored,
            },
        }
    }
}

fn render_confirm(area: Rect, buf: &mut Buffer, selected: usize) {
    let options = ["Cancel", "Yes, delete everything"];
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  ⚠ This will delete your entire vault.",
            Style::default().fg(rat::AMBER).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  All memories, topics, people, and config will be removed.",
            Style::default().fg(ratatui::style::Color::White),
        )),
        Line::from(Span::styled(
            "  This action cannot be undone.",
            Style::default().fg(rat::DIM),
        )),
        Line::from(""),
    ];

    for (i, label) in options.iter().enumerate() {
        let style = if i == selected {
            Style::default()
                .fg(if i == 1 { rat::RED } else { rat::GOLD })
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(rat::DIM)
        };
        let prefix = if i == selected { "  > " } else { "    " };
        lines.push(Line::from(Span::styled(format!("{prefix}{label}"), style)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  j/k select · Enter confirm · Esc cancel",
        Style::default().fg(rat::DIM),
    )));

    Paragraph::new(lines).render(area, buf);
}

fn render_done(area: Rect, buf: &mut Buffer) {
    Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  ✓ Vault deleted successfully.",
            Style::default()
                .fg(rat::EMERALD)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Run `soul init` to start fresh.",
            Style::default().fg(rat::DIM),
        )),
    ])
    .render(area, buf);
}

fn render_error(area: Rect, buf: &mut Buffer, msg: &str) {
    Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  ✗ Reset failed: {msg}"),
            Style::default().fg(rat::RED).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Press Esc to go back.",
            Style::default().fg(rat::DIM),
        )),
    ])
    .render(area, buf);
}

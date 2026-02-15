//! Settings page — show config, providers, vault path.

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
use crate::vault::config::{read_config, vault_root};

// ─── Settings Page ────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct SettingsPage {
    scroll: u16,
}

impl PageWidget for SettingsPage {
    fn render(&self, area: Rect, buf: &mut Buffer, app: &App) {
        if !app.vault_initialized {
            render_not_init(area, buf);
            return;
        }
        render_settings(area, buf, self.scroll);
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

// ─── Rendering ────────────────────────────────────────────────────────────────

fn render_not_init(area: Rect, buf: &mut Buffer) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Vault not initialized.",
            Style::default().fg(rat::AMBER).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Run `soul init` to create your vault and configure providers.",
            Style::default().fg(rat::DIM),
        )),
    ];
    Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(rat::AMBER))
                .title(" Settings — Config "),
        )
        .render(area, buf);
}

fn render_settings(area: Rect, buf: &mut Buffer, scroll: u16) {
    let config = match read_config() {
        Ok(c) => c,
        Err(_) => {
            Paragraph::new("  Failed to read config.")
                .style(Style::default().fg(rat::RED))
                .render(area, buf);
            return;
        }
    };

    let root = vault_root();
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "  Configuration",
            Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        config_line("  Vault path", root.display().to_string()),
        config_line(
            "  Processing LLM",
            config.processing_llm.display_name().to_string(),
        ),
        config_line("  Created", config.created_at.clone()),
        Line::from(""),
    ];

    // ─── Providers ────────────────────────────────────────────────────
    lines.push(Line::from(Span::styled(
        "  Providers",
        Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for p in &config.providers {
        let status = if p.enabled { "+ enabled" } else { "- disabled" };
        let color = if p.enabled { rat::EMERALD } else { rat::DIM };
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("{:<14}", p.name.display_name()),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(status, Style::default().fg(color)),
        ]));
    }
    lines.push(Line::from(""));

    // ─── Hints ────────────────────────────────────────────────────────
    lines.push(Line::from(Span::styled(
        "  To reconfigure, run `soul init` in a terminal.",
        Style::default().fg(rat::DIM),
    )));
    lines.push(Line::from(Span::styled(
        "  OAuth cloud login: `soul login [provider]` and `soul logout [provider]`.",
        Style::default().fg(rat::DIM),
    )));
    lines.push(Line::from(Span::styled(
        "  To reset everything, run `soul reset`.",
        Style::default().fg(rat::DIM),
    )));

    let visible: Vec<Line> = lines.into_iter().skip(scroll as usize).collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(rat::GOLD))
        .title(" Settings — Config ")
        .title_style(Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD));

    Paragraph::new(visible).block(block).render(area, buf);
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn config_line(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<20}", label), Style::default().fg(rat::DIM)),
        Span::styled(value, Style::default().add_modifier(Modifier::BOLD)),
    ])
}

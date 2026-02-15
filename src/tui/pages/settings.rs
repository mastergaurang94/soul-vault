//! Settings page — vault config overview with provider connection status.

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
use crate::vault::config::{get_api_key, read_config, vault_root};

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

fn render_not_init(area: Rect, buf: &mut Buffer) {
    Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Vault not initialized.",
            Style::default().fg(rat::AMBER).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Run `soul init` to get started.",
            Style::default().fg(rat::DIM),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(rat::AMBER))
            .title(" Settings "),
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
        section_header("  Vault"),
        Line::from(""),
        kv_line("  Path", &root.display().to_string()),
        kv_line("  LLM", config.processing_llm.display_name()),
        kv_line("  Created", &config.created_at),
        Line::from(""),
        section_header("  Providers"),
        Line::from(""),
    ];

    for p in &config.providers {
        let (status, color) = provider_status(&p.name, p.enabled);
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("{:<12}", p.name.display_name()),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(status, Style::default().fg(color)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(section_header("  API Key"));
    lines.push(Line::from(""));
    lines.extend(api_key_lines());

    lines.push(Line::from(""));
    lines.push(section_header("  OAuth"));
    lines.push(Line::from(""));
    lines.extend(oauth_lines());

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Manage keys: `soul init` · OAuth: `soul login`",
        Style::default().fg(rat::DIM),
    )));

    let visible: Vec<Line> = lines.into_iter().skip(scroll as usize).collect();
    Paragraph::new(visible)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(rat::GOLD))
                .title(" Settings ")
                .title_style(Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD)),
        )
        .render(area, buf);
}

fn provider_status(provider: &Provider, enabled: bool) -> (String, ratatui::style::Color) {
    if !enabled {
        return ("disabled".into(), rat::DIM);
    }
    let has_key = get_api_key(&provider.to_string())
        .ok()
        .flatten()
        .map(|k| !k.trim().is_empty())
        .unwrap_or(false);
    let has_oauth = is_logged_in(provider).unwrap_or(false);

    if has_key || has_oauth {
        ("ready".into(), rat::EMERALD)
    } else {
        ("enabled · no credentials".into(), rat::AMBER)
    }
}

fn api_key_lines() -> Vec<Line<'static>> {
    let key = get_api_key("claude")
        .ok()
        .flatten()
        .unwrap_or_default();
    if key.trim().is_empty() {
        vec![Line::from(Span::styled(
            "    Claude: not set",
            Style::default().fg(rat::DIM),
        ))]
    } else {
        let masked = mask_key(&key);
        vec![Line::from(vec![
            Span::styled("    Claude: ", Style::default().fg(rat::DIM)),
            Span::styled(masked, Style::default().fg(rat::EMERALD)),
        ])]
    }
}

fn oauth_lines() -> Vec<Line<'static>> {
    let providers = [Provider::Claude, Provider::ChatGpt, Provider::Gemini];
    providers
        .iter()
        .map(|p| {
            let logged_in = is_logged_in(p).unwrap_or(false);
            let (status, color) = if logged_in {
                ("logged in", rat::EMERALD)
            } else {
                ("not connected", rat::DIM)
            };
            Line::from(vec![
                Span::styled(format!("    {:<12}", p.display_name()), Style::default().fg(rat::DIM)),
                Span::styled(status, Style::default().fg(color)),
            ])
        })
        .collect()
}

fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        "••••••••".to_string()
    } else {
        format!("{}••••{}", &key[..4], &key[key.len() - 4..])
    }
}

fn section_header(label: &str) -> Line<'static> {
    Line::from(Span::styled(
        label.to_string(),
        Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
    ))
}

fn kv_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<18}", label), Style::default().fg(rat::DIM)),
        Span::styled(value.to_string(), Style::default().add_modifier(Modifier::BOLD)),
    ])
}

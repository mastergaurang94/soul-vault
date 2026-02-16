//! Settings page — vault config overview with provider connections.

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
use crate::types::{Provider, SoulVaultConfig};
use crate::ui::theme::rat;
use crate::vault::config::{
    get_api_key, get_key_health, read_config, vault_root, ApiKeyHealth, ApiKeyHealthRecord,
};

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
    lines.push(section_header("  Connections"));
    lines.push(Line::from(""));
    lines.extend(connection_lines(&config));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Configure: `soul init` · Connect: `soul login <provider>` · Disconnect: `soul logout <provider>`",
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

    if has_oauth {
        ("ready".into(), rat::EMERALD)
    } else if !has_key {
        ("enabled · no credentials".into(), rat::AMBER)
    } else {
        match get_key_health(provider).ok().flatten() {
            Some(record) => match record.status {
                ApiKeyHealth::Verified => ("ready".into(), rat::EMERALD),
                ApiKeyHealth::Unverified => ("key unverified".into(), rat::AMBER),
                ApiKeyHealth::Invalid => ("key invalid".into(), rat::RED),
            },
            None => ("key set · unknown".into(), rat::AMBER),
        }
    }
}

fn api_key_lines() -> Vec<Line<'static>> {
    let providers = [Provider::Claude, Provider::ChatGpt, Provider::Gemini];
    providers
        .iter()
        .map(|provider| {
            let key = get_api_key(&provider.to_string())
                .ok()
                .flatten()
                .unwrap_or_default();
            let health = get_key_health(provider).ok().flatten();
            api_key_line(provider, &key, health.as_ref())
        })
        .collect()
}

fn connection_lines(config: &SoulVaultConfig) -> Vec<Line<'static>> {
    let providers = [Provider::Claude, Provider::ChatGpt, Provider::Gemini];
    providers
        .iter()
        .map(|p| {
            let (status, status_color, action) = connection_state(config, p);
            let mut spans = vec![
                Span::styled(
                    format!("    {:<12}", p.display_name()),
                    Style::default().fg(rat::DIM),
                ),
                Span::styled(status, Style::default().fg(status_color)),
            ];
            if let Some(action) = action {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(action, Style::default().fg(rat::CYAN)));
            }
            Line::from(spans)
        })
        .collect()
}

fn connection_state(
    config: &SoulVaultConfig,
    provider: &Provider,
) -> (&'static str, ratatui::style::Color, Option<&'static str>) {
    if !oauth_supported(provider) {
        return ("coming soon", rat::DIM, None);
    }

    if !provider_enabled(config, provider) {
        return ("not set up", rat::AMBER, Some("Set up in `soul init`"));
    }

    if is_logged_in(provider).unwrap_or(false) {
        return (
            "connected",
            rat::EMERALD,
            Some("Disconnect via `soul logout`"),
        );
    }

    ("ready", rat::CYAN, Some("Connect via `soul login`"))
}

fn oauth_supported(provider: &Provider) -> bool {
    matches!(provider, Provider::Claude)
}

fn provider_enabled(config: &SoulVaultConfig, provider: &Provider) -> bool {
    config
        .providers
        .iter()
        .find(|entry| entry.name == *provider)
        .map(|entry| entry.enabled)
        .unwrap_or(false)
}

fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        "••••••••".to_string()
    } else {
        format!("{}••••{}", &key[..4], &key[key.len() - 4..])
    }
}

fn api_key_line(
    provider: &Provider,
    key: &str,
    health: Option<&ApiKeyHealthRecord>,
) -> Line<'static> {
    if key.trim().is_empty() {
        return Line::from(vec![
            Span::styled(
                format!("    {:<12}", provider.display_name()),
                Style::default().fg(rat::DIM),
            ),
            Span::styled("not set", Style::default().fg(rat::DIM)),
        ]);
    }

    let masked = mask_key(key);
    let (label, color) = match health.map(|h| &h.status) {
        Some(ApiKeyHealth::Verified) => ("verified", rat::EMERALD),
        Some(ApiKeyHealth::Unverified) => ("unverified", rat::AMBER),
        Some(ApiKeyHealth::Invalid) => ("invalid", rat::RED),
        None => ("unknown", rat::AMBER),
    };

    Line::from(vec![
        Span::styled(
            format!("    {:<12}", provider.display_name()),
            Style::default().fg(rat::DIM),
        ),
        Span::styled(masked, Style::default().fg(rat::EMERALD)),
        Span::raw("  "),
        Span::styled(format!("[{}]", label), Style::default().fg(color)),
    ])
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
        Span::styled(
            value.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ])
}

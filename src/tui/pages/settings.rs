//! Settings page — vault config overview with provider connections.

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
use crate::types::{ProcessingMode, Provider, SoulVaultConfig};
use crate::ui::theme::rat;
use crate::vault::config::{
    get_api_key, get_key_health, read_config, vault_root, write_config, ApiKeyHealth,
    ApiKeyHealthRecord,
};

#[derive(Default)]
pub struct SettingsPage {
    selected_connection: usize,
    pending_oauth: bool,
    status_message: Option<(bool, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionAction {
    Connect,
    Disconnect,
    Setup,
    None,
}

struct ConnectionState {
    label: &'static str,
    color: ratatui::style::Color,
    action: ConnectionAction,
    action_label: Option<&'static str>,
}

impl PageWidget for SettingsPage {
    fn render(&self, area: Rect, buf: &mut Buffer, app: &App) {
        if !app.vault_initialized {
            render_not_init(area, buf);
            return;
        }
        render_settings(area, buf, self);
    }

    fn handle_key(&mut self, key: KeyEvent, _app: &mut App) -> PageAction {
        if self.pending_oauth {
            return match key.code {
                KeyCode::Esc => PageAction::BackToSidebar,
                _ => PageAction::Consumed,
            };
        }

        match key.code {
            KeyCode::Char('1') => self.apply_processing_mode(ProcessingMode::Disabled),
            KeyCode::Char('2') => self.apply_processing_mode(ProcessingMode::Claude),
            KeyCode::Char('3') => self.apply_processing_mode(ProcessingMode::ChatGpt),
            KeyCode::Char('4') => self.apply_processing_mode(ProcessingMode::Gemini),
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected_connection =
                    (self.selected_connection + 1) % connection_providers().len();
                PageAction::Consumed
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let len = connection_providers().len();
                self.selected_connection = (self.selected_connection + len - 1) % len;
                PageAction::Consumed
            }
            KeyCode::Enter => self.select_connection_action(),
            KeyCode::Esc => PageAction::BackToSidebar,
            _ => PageAction::Ignored,
        }
    }
}

impl SettingsPage {
    pub fn on_oauth_complete(&mut self, ok: bool, message: String) {
        self.pending_oauth = false;
        self.status_message = Some((ok, message));
    }

    fn selected_provider(&self) -> Provider {
        connection_providers()[self.selected_connection].clone()
    }

    fn select_connection_action(&mut self) -> PageAction {
        let config = match read_config() {
            Ok(cfg) => cfg,
            Err(e) => {
                self.status_message = Some((false, format!("Failed to read config: {e}")));
                return PageAction::Consumed;
            }
        };

        let provider = self.selected_provider();
        let state = connection_state(&config, &provider);
        match state.action {
            ConnectionAction::Connect => {
                self.pending_oauth = true;
                self.status_message = Some((
                    true,
                    format!("Starting OAuth for {}...", provider.display_name()),
                ));
                PageAction::StartOAuthConnect(provider)
            }
            ConnectionAction::Disconnect => {
                match remove_credentials(&provider) {
                    Ok(true) => {
                        self.status_message =
                            Some((true, format!("Disconnected {}.", provider.display_name())))
                    }
                    Ok(false) => {
                        self.status_message = Some((
                            false,
                            format!("No active {} connection.", provider.display_name()),
                        ))
                    }
                    Err(e) => {
                        self.status_message = Some((false, format!("Disconnect failed: {e}")));
                    }
                }
                PageAction::Consumed
            }
            ConnectionAction::Setup => {
                self.status_message = Some((
                    false,
                    format!("Set up {} in `soul init` first.", provider.display_name()),
                ));
                PageAction::Consumed
            }
            ConnectionAction::None => {
                self.status_message = Some((
                    false,
                    format!("OAuth for {} is coming soon.", provider.display_name()),
                ));
                PageAction::Consumed
            }
        }
    }

    fn apply_processing_mode(&mut self, mode: ProcessingMode) -> PageAction {
        let mut config = match read_config() {
            Ok(cfg) => cfg,
            Err(e) => {
                self.status_message = Some((false, format!("Failed to read config: {e}")));
                return PageAction::Consumed;
            }
        };

        if let Some(provider) = mode.as_provider() {
            if let Some(entry) = config.providers.iter_mut().find(|p| p.name == provider) {
                entry.enabled = true;
            }
        }

        config.processing_mode = mode.clone();
        if let Err(e) = write_config(&config) {
            self.status_message = Some((false, format!("Failed to update config: {e}")));
            return PageAction::Consumed;
        }

        self.status_message = Some((true, format!("Processing set to {}.", mode.display_name())));
        if let Some(provider) = mode.as_provider() {
            if !provider_has_credentials(&provider) {
                self.status_message = Some((
                    false,
                    format!(
                        "Processing set to {}, but credentials are not configured yet.",
                        provider.display_name()
                    ),
                ));
            }
        }

        PageAction::Consumed
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

fn render_settings(area: Rect, buf: &mut Buffer, page: &SettingsPage) {
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
        kv_line("  Processing", config.processing_mode.display_name()),
        kv_line("  Created", &config.created_at),
        Line::from(""),
        section_header("  Processing mode"),
        Line::from(""),
    ];

    lines.extend(processing_mode_lines(&config.processing_mode));
    lines.extend([
        Line::from(""),
        section_header("  Providers"),
        Line::from(""),
    ]);

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
    lines.extend(connection_lines(&config, page.selected_connection));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Processing: 1 Disabled · 2 Claude · 3 ChatGPT · 4 Gemini",
        Style::default().fg(rat::DIM),
    )));
    lines.push(Line::from(Span::styled(
        "  Connections: Up/Down select · Enter action · Esc back",
        Style::default().fg(rat::DIM),
    )));
    if page.pending_oauth {
        lines.push(Line::from(Span::styled(
            "  Waiting for OAuth callback in your browser...",
            Style::default().fg(rat::CYAN),
        )));
    }
    if let Some((ok, msg)) = &page.status_message {
        let color = if *ok { rat::EMERALD } else { rat::RED };
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", msg),
            Style::default().fg(color),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Configure: `soul init` · Connect: `soul login <provider>` · Disconnect: `soul logout <provider>`",
        Style::default().fg(rat::DIM),
    )));

    Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(rat::GOLD))
                .title(" Settings ")
                .title_style(Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD)),
        )
        .render(area, buf);
}

fn connection_providers() -> Vec<Provider> {
    vec![Provider::Claude, Provider::ChatGpt, Provider::Gemini]
}

fn connection_lines(config: &SoulVaultConfig, selected_idx: usize) -> Vec<Line<'static>> {
    connection_providers()
        .into_iter()
        .enumerate()
        .map(|(idx, provider)| {
            let state = connection_state(config, &provider);
            let selected = idx == selected_idx;
            let prefix = if selected { "  > " } else { "    " };
            let name_style = if selected {
                Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(rat::DIM)
            };
            let mut spans = vec![
                Span::styled(prefix, name_style),
                Span::styled(format!("{:<12}", provider.display_name()), name_style),
                Span::styled(state.label, Style::default().fg(state.color)),
            ];
            if let Some(label) = state.action_label {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(label, Style::default().fg(rat::CYAN)));
            }
            Line::from(spans)
        })
        .collect()
}

fn connection_state(config: &SoulVaultConfig, provider: &Provider) -> ConnectionState {
    if !oauth_supported(provider) {
        return ConnectionState {
            label: "coming soon",
            color: rat::DIM,
            action: ConnectionAction::None,
            action_label: None,
        };
    }

    if !provider_enabled(config, provider) {
        return ConnectionState {
            label: "not set up",
            color: rat::AMBER,
            action: ConnectionAction::Setup,
            action_label: Some("Run `soul init`"),
        };
    }

    if is_logged_in(provider).unwrap_or(false) {
        return ConnectionState {
            label: "connected",
            color: rat::EMERALD,
            action: ConnectionAction::Disconnect,
            action_label: Some("[Enter] Disconnect"),
        };
    }

    ConnectionState {
        label: "ready",
        color: rat::CYAN,
        action: ConnectionAction::Connect,
        action_label: Some("[Enter] Connect via OAuth"),
    }
}

fn processing_mode_lines(mode: &ProcessingMode) -> Vec<Line<'static>> {
    let options = [
        (
            1usize,
            ProcessingMode::Disabled,
            "Disabled (raw sessions only)",
        ),
        (2usize, ProcessingMode::Claude, "Claude"),
        (3usize, ProcessingMode::ChatGpt, "ChatGPT"),
        (4usize, ProcessingMode::Gemini, "Gemini"),
    ];

    options
        .iter()
        .map(|(idx, candidate, label)| {
            let selected = *mode == *candidate;
            let marker = if selected { "•" } else { " " };
            let color = if selected { rat::EMERALD } else { rat::DIM };
            Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    format!("{} {}. {}", marker, idx, label),
                    Style::default().fg(color),
                ),
            ])
        })
        .collect()
}

fn oauth_supported(provider: &Provider) -> bool {
    matches!(provider, Provider::Claude)
}

fn provider_has_credentials(provider: &Provider) -> bool {
    let has_key = get_api_key(&provider.to_string())
        .ok()
        .flatten()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    let has_oauth = is_logged_in(provider).unwrap_or(false);
    has_key || has_oauth
}

fn provider_enabled(config: &SoulVaultConfig, provider: &Provider) -> bool {
    config
        .providers
        .iter()
        .find(|entry| entry.name == *provider)
        .map(|entry| entry.enabled)
        .unwrap_or(false)
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

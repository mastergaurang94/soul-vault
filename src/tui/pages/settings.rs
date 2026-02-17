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
    setup_flow: Option<SetupFlow>,
    reset_flow: Option<ResetFlow>,
    pending_processing_provider: Option<Provider>,
}

#[derive(Debug, Clone)]
enum SetupFlow {
    ChooseAuth {
        provider: Provider,
        selected: usize,
    },
    EnterApiKey {
        provider: Provider,
        input: String,
        cursor: usize,
        submitting: bool,
    },
}

#[derive(Debug, Clone)]
enum ResetFlow {
    Confirm { selected: usize }, // 0 = Cancel, 1 = Reset vault
    TypeConfirm { input: String },
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

    fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> PageAction {
        if self.reset_flow.is_some() {
            return self.handle_reset_flow_key(key, app);
        }

        if self.setup_flow.is_some() {
            return self.handle_setup_flow_key(key);
        }

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
            KeyCode::Char('5') => {
                self.status_message = Some((
                    false,
                    "Soul Vault Cloud processing is coming soon. Choose 1-4 for now.".to_string(),
                ));
                PageAction::Consumed
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.reset_flow = Some(ResetFlow::Confirm { selected: 0 });
                self.status_message = None;
                PageAction::Consumed
            }
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
        if ok {
            if let Some(provider) = self.pending_processing_provider.take() {
                self.finalize_processing_provider(provider);
                self.setup_flow = None;
                return;
            }
        } else {
            self.pending_processing_provider = None;
        }
        self.status_message = Some((ok, message));
    }

    pub fn on_api_key_complete(&mut self, ok: bool, provider: Provider, message: String) {
        if ok {
            self.finalize_processing_provider(provider);
            self.setup_flow = None;
            self.status_message = Some((true, message));
            return;
        }
        self.status_message = Some((false, message));
    }

    pub fn on_api_key_error(&mut self, message: String) {
        if let Some(SetupFlow::EnterApiKey { submitting, .. }) = &mut self.setup_flow {
            *submitting = false;
        }
        self.status_message = Some((false, message));
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
        if matches!(mode, ProcessingMode::Disabled) {
            let mut config = match read_config() {
                Ok(cfg) => cfg,
                Err(e) => {
                    self.status_message = Some((false, format!("Failed to read config: {e}")));
                    return PageAction::Consumed;
                }
            };
            config.processing_mode = ProcessingMode::Disabled;
            if let Err(e) = write_config(&config) {
                self.status_message = Some((false, format!("Failed to update config: {e}")));
                return PageAction::Consumed;
            }
            self.status_message =
                Some((true, "Processing set to Disabled (raw mode).".to_string()));
            return PageAction::Consumed;
        }

        let provider = match mode.as_provider() {
            Some(provider) => provider,
            None => return PageAction::Consumed,
        };

        if provider_has_credentials(&provider) {
            self.finalize_processing_provider(provider);
            return PageAction::Consumed;
        }

        self.setup_flow = Some(SetupFlow::ChooseAuth {
            provider,
            selected: 0,
        });
        self.status_message = Some((
            false,
            "Credentials required. Choose API key or OAuth to continue.".to_string(),
        ));
        PageAction::Consumed
    }

    fn finalize_processing_provider(&mut self, provider: Provider) {
        let mut config = match read_config() {
            Ok(cfg) => cfg,
            Err(e) => {
                self.status_message = Some((false, format!("Failed to read config: {e}")));
                return;
            }
        };

        if let Some(entry) = config.providers.iter_mut().find(|p| p.name == provider) {
            entry.enabled = true;
        }
        config.processing_mode = ProcessingMode::from_provider(&provider);
        if let Err(e) = write_config(&config) {
            self.status_message = Some((false, format!("Failed to update config: {e}")));
            return;
        }
        self.status_message = Some((
            true,
            format!("Processing set to {}.", provider.display_name()),
        ));
    }

    fn handle_setup_flow_key(&mut self, key: KeyEvent) -> PageAction {
        let flow = match self.setup_flow.clone() {
            Some(flow) => flow,
            None => return PageAction::Ignored,
        };

        match flow {
            SetupFlow::ChooseAuth {
                provider,
                mut selected,
            } => {
                let oauth_available = oauth_supported(&provider);
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        selected = (selected + 1) % 3;
                        self.setup_flow = Some(SetupFlow::ChooseAuth { provider, selected });
                        PageAction::Consumed
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        selected = (selected + 2) % 3;
                        self.setup_flow = Some(SetupFlow::ChooseAuth { provider, selected });
                        PageAction::Consumed
                    }
                    KeyCode::Enter => {
                        match selected {
                            0 => {
                                self.setup_flow = Some(SetupFlow::EnterApiKey {
                                    provider,
                                    input: String::new(),
                                    cursor: 0,
                                    submitting: false,
                                });
                            }
                            1 => {
                                if oauth_available {
                                    self.pending_oauth = true;
                                    self.pending_processing_provider = Some(provider.clone());
                                    self.status_message = Some((
                                        true,
                                        format!(
                                            "Starting OAuth for {}...",
                                            provider.display_name()
                                        ),
                                    ));
                                    self.setup_flow = None;
                                    return PageAction::StartOAuthConnect(provider);
                                } else {
                                    self.status_message = Some((
                                        false,
                                        format!(
                                            "OAuth for {} is coming soon. Use API key for now.",
                                            provider.display_name()
                                        ),
                                    ));
                                }
                            }
                            _ => {
                                self.setup_flow = None;
                                self.pending_processing_provider = None;
                                self.status_message = Some((
                                    false,
                                    "Processing mode unchanged. Setup cancelled.".to_string(),
                                ));
                            }
                        }
                        PageAction::Consumed
                    }
                    KeyCode::Esc => {
                        self.setup_flow = None;
                        self.pending_processing_provider = None;
                        self.status_message = Some((
                            false,
                            "Processing mode unchanged. Setup cancelled.".to_string(),
                        ));
                        PageAction::Consumed
                    }
                    _ => PageAction::Ignored,
                }
            }
            SetupFlow::EnterApiKey {
                provider,
                mut input,
                mut cursor,
                submitting,
            } => {
                if submitting {
                    return match key.code {
                        KeyCode::Esc => {
                            self.setup_flow = None;
                            self.pending_processing_provider = None;
                            self.status_message =
                                Some((false, "API key setup cancelled.".to_string()));
                            PageAction::Consumed
                        }
                        _ => PageAction::Consumed,
                    };
                }

                match key.code {
                    KeyCode::Esc => {
                        self.setup_flow = Some(SetupFlow::ChooseAuth {
                            provider,
                            selected: 0,
                        });
                        PageAction::Consumed
                    }
                    KeyCode::Left => {
                        cursor = cursor.saturating_sub(1);
                        self.setup_flow = Some(SetupFlow::EnterApiKey {
                            provider,
                            input,
                            cursor,
                            submitting: false,
                        });
                        PageAction::Consumed
                    }
                    KeyCode::Right => {
                        if cursor < input.len() {
                            cursor += 1;
                        }
                        self.setup_flow = Some(SetupFlow::EnterApiKey {
                            provider,
                            input,
                            cursor,
                            submitting: false,
                        });
                        PageAction::Consumed
                    }
                    KeyCode::Backspace => {
                        if cursor > 0 {
                            input.remove(cursor - 1);
                            cursor -= 1;
                        }
                        self.setup_flow = Some(SetupFlow::EnterApiKey {
                            provider,
                            input,
                            cursor,
                            submitting: false,
                        });
                        PageAction::Consumed
                    }
                    KeyCode::Char(c) => {
                        input.insert(cursor, c);
                        cursor += 1;
                        self.setup_flow = Some(SetupFlow::EnterApiKey {
                            provider,
                            input,
                            cursor,
                            submitting: false,
                        });
                        PageAction::Consumed
                    }
                    KeyCode::Enter => {
                        if input.trim().is_empty() {
                            self.status_message =
                                Some((false, "API key cannot be empty.".to_string()));
                            return PageAction::Consumed;
                        }
                        self.setup_flow = Some(SetupFlow::EnterApiKey {
                            provider: provider.clone(),
                            input: input.clone(),
                            cursor,
                            submitting: true,
                        });
                        self.status_message = Some((
                            true,
                            format!("Validating {} API key...", provider.display_name()),
                        ));
                        PageAction::StartApiKeySetup(provider, input)
                    }
                    _ => PageAction::Ignored,
                }
            }
        }
    }

    fn handle_reset_flow_key(&mut self, key: KeyEvent, app: &mut App) -> PageAction {
        match self.reset_flow.clone() {
            Some(ResetFlow::Confirm { mut selected }) => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    selected = 1;
                    self.reset_flow = Some(ResetFlow::Confirm { selected });
                    PageAction::Consumed
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    selected = 0;
                    self.reset_flow = Some(ResetFlow::Confirm { selected });
                    PageAction::Consumed
                }
                KeyCode::Enter => {
                    if selected == 0 {
                        self.reset_flow = None;
                        self.status_message = Some((false, "Reset cancelled.".to_string()));
                    } else {
                        self.reset_flow = Some(ResetFlow::TypeConfirm {
                            input: String::new(),
                        });
                    }
                    PageAction::Consumed
                }
                KeyCode::Esc => {
                    self.reset_flow = None;
                    self.status_message = Some((false, "Reset cancelled.".to_string()));
                    PageAction::Consumed
                }
                _ => PageAction::Ignored,
            },
            Some(ResetFlow::TypeConfirm { mut input }) => match key.code {
                KeyCode::Esc => {
                    self.reset_flow = None;
                    self.status_message = Some((false, "Reset cancelled.".to_string()));
                    PageAction::Consumed
                }
                KeyCode::Backspace => {
                    input.pop();
                    self.reset_flow = Some(ResetFlow::TypeConfirm { input });
                    PageAction::Consumed
                }
                KeyCode::Char(c) => {
                    if c.is_ascii_alphabetic() {
                        input.push(c.to_ascii_uppercase());
                    }
                    self.reset_flow = Some(ResetFlow::TypeConfirm { input });
                    PageAction::Consumed
                }
                KeyCode::Enter => {
                    if input.trim() != "RESET" {
                        self.status_message = Some((
                            false,
                            "Confirmation mismatch. Type RESET exactly.".to_string(),
                        ));
                        return PageAction::Consumed;
                    }

                    match crate::cli::reset::move_vault_to_trash() {
                        Ok(_) => {
                            app.vault_initialized = false;
                            app.should_quit = true;
                            self.reset_flow = None;
                            self.status_message =
                                Some((true, "Vault moved to Trash successfully.".to_string()));
                        }
                        Err(e) => {
                            self.status_message = Some((false, format!("Reset failed: {e}")));
                        }
                    }
                    PageAction::Consumed
                }
                _ => PageAction::Ignored,
            },
            None => PageAction::Ignored,
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
    lines.push(section_header("  Danger zone"));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "    Press X to reset vault (typed confirmation required)",
        Style::default().fg(rat::RED),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Processing: 1 Disabled · 2 Claude · 3 ChatGPT · 4 Gemini · 5 Cloud (coming soon)",
        Style::default().fg(rat::DIM),
    )));
    if page.reset_flow.is_some() {
        lines.push(Line::from(Span::styled(
            "  Reset: j/k choose · Enter confirm · Esc cancel",
            Style::default().fg(rat::DIM),
        )));
    } else if page.setup_flow.is_some() {
        lines.push(Line::from(Span::styled(
            "  Setup: Up/Down choose · Enter confirm · Esc cancel",
            Style::default().fg(rat::DIM),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  Connections: Up/Down select · Enter action · Esc back",
            Style::default().fg(rat::DIM),
        )));
    }
    lines.extend(setup_flow_lines(page));
    lines.extend(reset_flow_lines(page));
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
        "  You can configure processing and connections directly in Settings.",
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

    let mut lines: Vec<Line<'static>> = options
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
        .collect();

    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled(
            "  5. Soul Vault Cloud (coming soon)",
            Style::default().fg(rat::DIM),
        ),
    ]));
    lines
}

fn setup_flow_lines(page: &SettingsPage) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    match &page.setup_flow {
        Some(SetupFlow::ChooseAuth { provider, selected }) => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {} setup", provider.display_name()),
                Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
            )));
            let options = ["API key", "OAuth", "Back"];
            for (idx, option) in options.iter().enumerate() {
                let prefix = if idx == *selected { "  > " } else { "    " };
                let color = if idx == *selected {
                    rat::GOLD
                } else {
                    rat::DIM
                };
                lines.push(Line::from(Span::styled(
                    format!("{prefix}{option}"),
                    Style::default().fg(color),
                )));
            }
        }
        Some(SetupFlow::EnterApiKey {
            provider,
            input,
            cursor,
            submitting,
        }) => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  Enter {} API key", provider.display_name()),
                Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                format!("  Hint: {}", provider.api_key_hint()),
                Style::default().fg(rat::DIM),
            )));
            let mut shown = input.clone();
            if *cursor <= shown.len() {
                shown.insert(*cursor, '|');
            }
            lines.push(Line::from(Span::styled(
                format!("  {}", shown),
                Style::default().fg(rat::CYAN),
            )));
            if *submitting {
                lines.push(Line::from(Span::styled(
                    "  Validating and saving...",
                    Style::default().fg(rat::DIM),
                )));
            }
        }
        None => {}
    }
    lines
}

fn reset_flow_lines(page: &SettingsPage) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    match &page.reset_flow {
        Some(ResetFlow::Confirm { selected }) => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Reset vault?",
                Style::default().fg(rat::RED).add_modifier(Modifier::BOLD),
            )));
            let options = ["Cancel", "Yes, move vault to Trash"];
            for (idx, option) in options.iter().enumerate() {
                let prefix = if idx == *selected { "  > " } else { "    " };
                let color = if idx == *selected {
                    if idx == 1 {
                        rat::RED
                    } else {
                        rat::GOLD
                    }
                } else {
                    rat::DIM
                };
                lines.push(Line::from(Span::styled(
                    format!("{prefix}{option}"),
                    Style::default().fg(color),
                )));
            }
        }
        Some(ResetFlow::TypeConfirm { input }) => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Type RESET to confirm:",
                Style::default().fg(rat::RED).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                format!("    {}", input),
                Style::default().fg(rat::CYAN),
            )));
        }
        None => {}
    }
    lines
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

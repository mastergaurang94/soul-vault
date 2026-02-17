//! Settings page — vault config overview with provider credential and processing controls.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::auth::{is_logged_in, oauth_connect_available, remove_credentials, save_setup_token};
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
    selected_item: usize,
    pending_oauth: bool,
    status_message: Option<(bool, String)>,
    status_from_subflow: bool,
    setup_flow: Option<SetupFlow>,
    reset_flow: Option<ResetFlow>,
    pending_processing_provider: Option<Provider>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SelectableItem {
    Credential(Provider),
    Processing(ProcessingChoice),
    DangerReset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessingChoice {
    Disabled,
    Claude,
    ChatGpt,
    Gemini,
    Cloud,
}

#[derive(Debug, Clone)]
enum SetupFlow {
    AuthMenu {
        provider: Provider,
        selected: usize,
        set_processing_on_success: bool,
    },
    EnterApiKey {
        provider: Provider,
        input: String,
        cursor: usize,
        submitting: bool,
        set_processing_on_success: bool,
    },
    EnterSetupToken {
        provider: Provider,
        input: String,
        cursor: usize,
        set_processing_on_success: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthAction {
    ApiKey,
    ClaudeSetupToken,
    OAuth,
    Back,
}

#[derive(Debug, Clone)]
enum ResetFlow {
    Confirm { selected: usize },
    TypeConfirm { input: String },
}

impl PageWidget for SettingsPage {
    fn render(&self, area: Rect, buf: &mut Buffer, app: &App) {
        if !app.vault_initialized {
            render_not_init(area, buf);
            return;
        }

        if let Some(flow) = &self.setup_flow {
            render_setup_flow(area, buf, flow, self.status_message.as_ref());
            return;
        }

        if let Some(flow) = &self.reset_flow {
            render_reset_flow(area, buf, flow, self.status_message.as_ref());
            return;
        }

        render_settings_main(area, buf, self);
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
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_selection_down();
                PageAction::Consumed
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_selection_up();
                PageAction::Consumed
            }
            KeyCode::Enter => self.activate_selected_item(),
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.reset_flow = Some(ResetFlow::Confirm { selected: 0 });
                self.clear_status();
                PageAction::Consumed
            }
            KeyCode::Esc => PageAction::BackToSidebar,
            _ => PageAction::Ignored,
        }
    }
}

impl SettingsPage {
    fn clear_status(&mut self) {
        self.status_message = None;
        self.status_from_subflow = false;
    }

    fn set_status_main(&mut self, ok: bool, message: impl Into<String>) {
        self.status_message = Some((ok, message.into()));
        self.status_from_subflow = false;
    }

    fn set_status_subflow(&mut self, ok: bool, message: impl Into<String>) {
        self.status_message = Some((ok, message.into()));
        self.status_from_subflow = true;
    }

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
        self.set_status_subflow(ok, message);
    }

    pub fn on_api_key_complete(&mut self, ok: bool, provider: Provider, message: String) {
        if ok {
            if self.pending_processing_provider.as_ref() == Some(&provider) {
                self.finalize_processing_provider(provider);
                self.pending_processing_provider = None;
                self.setup_flow = None;
                self.set_status_subflow(true, message);
                return;
            }
            self.setup_flow = None;
            self.set_status_subflow(true, message);
            return;
        }
        self.set_status_subflow(false, message);
    }

    pub fn on_api_key_error(&mut self, message: String) {
        if let Some(SetupFlow::EnterApiKey { submitting, .. }) = &mut self.setup_flow {
            *submitting = false;
        }
        self.set_status_subflow(false, message);
    }

    fn selectable_items() -> Vec<SelectableItem> {
        vec![
            SelectableItem::Credential(Provider::Claude),
            SelectableItem::Credential(Provider::ChatGpt),
            SelectableItem::Credential(Provider::Gemini),
            SelectableItem::Processing(ProcessingChoice::Disabled),
            SelectableItem::Processing(ProcessingChoice::Claude),
            SelectableItem::Processing(ProcessingChoice::ChatGpt),
            SelectableItem::Processing(ProcessingChoice::Gemini),
            SelectableItem::Processing(ProcessingChoice::Cloud),
            SelectableItem::DangerReset,
        ]
    }

    fn current_item(&self) -> SelectableItem {
        Self::selectable_items()
            .get(self.selected_item)
            .cloned()
            .unwrap_or(SelectableItem::Credential(Provider::Claude))
    }

    fn move_selection_down(&mut self) {
        let len = Self::selectable_items().len();
        self.selected_item = (self.selected_item + 1) % len;
    }

    fn move_selection_up(&mut self) {
        let len = Self::selectable_items().len();
        self.selected_item = (self.selected_item + len - 1) % len;
    }

    fn activate_selected_item(&mut self) -> PageAction {
        match self.current_item() {
            SelectableItem::Credential(provider) => {
                self.open_auth_menu(provider, false);
                PageAction::Consumed
            }
            SelectableItem::Processing(choice) => self.apply_processing_choice(choice),
            SelectableItem::DangerReset => {
                self.reset_flow = Some(ResetFlow::Confirm { selected: 0 });
                self.clear_status();
                PageAction::Consumed
            }
        }
    }

    fn open_auth_menu(&mut self, provider: Provider, set_processing_on_success: bool) {
        // Avoid leaking stale main-screen status into auth-specific subflows.
        self.clear_status();
        self.setup_flow = Some(SetupFlow::AuthMenu {
            provider,
            selected: 0,
            set_processing_on_success,
        });
    }

    fn apply_processing_choice(&mut self, choice: ProcessingChoice) -> PageAction {
        match choice {
            ProcessingChoice::Disabled => {
                let mut config = match read_config() {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        self.set_status_main(false, format!("Failed to read config: {e}"));
                        return PageAction::Consumed;
                    }
                };
                config.processing_mode = ProcessingMode::Disabled;
                if let Err(e) = write_config(&config) {
                    self.set_status_main(false, format!("Failed to update config: {e}"));
                    return PageAction::Consumed;
                }
                self.set_status_main(true, "Processing set to Disabled (raw mode).");
                PageAction::Consumed
            }
            ProcessingChoice::Cloud => {
                self.set_status_main(
                    false,
                    "Soul Vault Cloud processing is coming soon. Choose Claude, ChatGPT, Gemini, or Disabled.",
                );
                PageAction::Consumed
            }
            ProcessingChoice::Claude | ProcessingChoice::ChatGpt | ProcessingChoice::Gemini => {
                let provider = choice.provider().expect("provider choice");
                if provider_has_credentials(&provider) {
                    self.finalize_processing_provider(provider);
                    return PageAction::Consumed;
                }

                self.open_auth_menu(provider.clone(), true);
                self.set_status_subflow(
                    false,
                    format!(
                        "{} needs credentials before it can be used for processing.",
                        provider.display_name()
                    ),
                );
                PageAction::Consumed
            }
        }
    }

    fn finalize_processing_provider(&mut self, provider: Provider) {
        let mut config = match read_config() {
            Ok(cfg) => cfg,
            Err(e) => {
                self.set_status_main(false, format!("Failed to read config: {e}"));
                return;
            }
        };

        if let Some(entry) = config.providers.iter_mut().find(|p| p.name == provider) {
            entry.enabled = true;
        }
        config.processing_mode = ProcessingMode::from_provider(&provider);
        if let Err(e) = write_config(&config) {
            self.set_status_main(false, format!("Failed to update config: {e}"));
            return;
        }
        self.set_status_main(true, format!("Processing set to {}.", provider.display_name()));
    }

    fn handle_setup_flow_key(&mut self, key: KeyEvent) -> PageAction {
        let flow = match self.setup_flow.clone() {
            Some(flow) => flow,
            None => return PageAction::Ignored,
        };

        match flow {
            SetupFlow::AuthMenu {
                provider,
                mut selected,
                set_processing_on_success,
            } => {
                let actions = auth_actions_for_provider(&provider);
                if actions.is_empty() {
                    self.setup_flow = None;
                    return PageAction::Consumed;
                }

                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        selected = (selected + 1) % actions.len();
                        self.setup_flow = Some(SetupFlow::AuthMenu {
                            provider,
                            selected,
                            set_processing_on_success,
                        });
                        PageAction::Consumed
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        selected = (selected + actions.len() - 1) % actions.len();
                        self.setup_flow = Some(SetupFlow::AuthMenu {
                            provider,
                            selected,
                            set_processing_on_success,
                        });
                        PageAction::Consumed
                    }
                    KeyCode::Enter => match actions[selected] {
                        AuthAction::ApiKey => {
                            if set_processing_on_success {
                                self.pending_processing_provider = Some(provider.clone());
                            } else {
                                self.pending_processing_provider = None;
                            }
                            self.setup_flow = Some(SetupFlow::EnterApiKey {
                                provider: provider.clone(),
                                input: String::new(),
                                cursor: 0,
                                submitting: false,
                                set_processing_on_success,
                            });
                            PageAction::Consumed
                        }
                        AuthAction::ClaudeSetupToken => {
                            if set_processing_on_success {
                                self.pending_processing_provider = Some(provider.clone());
                            } else {
                                self.pending_processing_provider = None;
                            }
                            self.setup_flow = Some(SetupFlow::EnterSetupToken {
                                provider: provider.clone(),
                                input: String::new(),
                                cursor: 0,
                                set_processing_on_success,
                            });
                            PageAction::Consumed
                        }
                        AuthAction::OAuth => self.handle_oauth_action(&provider, set_processing_on_success),
                        AuthAction::Back => {
                            self.setup_flow = None;
                            self.pending_processing_provider = None;
                            if set_processing_on_success {
                                self.set_status_subflow(
                                    false,
                                    "Processing mode unchanged. Setup cancelled.",
                                );
                            }
                            PageAction::Consumed
                        }
                    },
                    KeyCode::Esc => {
                        self.setup_flow = None;
                        self.pending_processing_provider = None;
                        if set_processing_on_success {
                            self.set_status_subflow(
                                false,
                                "Processing mode unchanged. Setup cancelled.",
                            );
                        }
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
                set_processing_on_success,
            } => {
                if submitting {
                    return match key.code {
                        KeyCode::Esc => {
                            self.setup_flow = None;
                            self.pending_processing_provider = None;
                            self.set_status_subflow(false, "API key setup cancelled.");
                            PageAction::Consumed
                        }
                        _ => PageAction::Consumed,
                    };
                }

                match key.code {
                    KeyCode::Esc => {
                            self.open_auth_menu(provider.clone(), set_processing_on_success);
                        PageAction::Consumed
                    }
                    KeyCode::Left => {
                        cursor = cursor.saturating_sub(1);
                        self.setup_flow = Some(SetupFlow::EnterApiKey {
                            provider,
                            input,
                            cursor,
                            submitting: false,
                            set_processing_on_success,
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
                            set_processing_on_success,
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
                            set_processing_on_success,
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
                            set_processing_on_success,
                        });
                        PageAction::Consumed
                    }
                    KeyCode::Enter => {
                        if input.trim().is_empty() {
                            self.set_status_subflow(false, "API key cannot be empty.");
                            return PageAction::Consumed;
                        }
                        self.setup_flow = Some(SetupFlow::EnterApiKey {
                            provider: provider.clone(),
                            input: input.clone(),
                            cursor,
                            submitting: true,
                            set_processing_on_success,
                        });
                        self.set_status_subflow(
                            true,
                            format!("Validating {} API key...", provider.display_name()),
                        );
                        PageAction::StartApiKeySetup(provider, input)
                    }
                    _ => PageAction::Ignored,
                }
            }
            SetupFlow::EnterSetupToken {
                provider,
                mut input,
                mut cursor,
                set_processing_on_success,
            } => match key.code {
                KeyCode::Esc => {
                    self.open_auth_menu(provider.clone(), set_processing_on_success);
                    PageAction::Consumed
                }
                KeyCode::Left => {
                    cursor = cursor.saturating_sub(1);
                    self.setup_flow = Some(SetupFlow::EnterSetupToken {
                        provider,
                        input,
                        cursor,
                        set_processing_on_success,
                    });
                    PageAction::Consumed
                }
                KeyCode::Right => {
                    if cursor < input.len() {
                        cursor += 1;
                    }
                    self.setup_flow = Some(SetupFlow::EnterSetupToken {
                        provider,
                        input,
                        cursor,
                        set_processing_on_success,
                    });
                    PageAction::Consumed
                }
                KeyCode::Backspace => {
                    if cursor > 0 {
                        input.remove(cursor - 1);
                        cursor -= 1;
                    }
                    self.setup_flow = Some(SetupFlow::EnterSetupToken {
                        provider,
                        input,
                        cursor,
                        set_processing_on_success,
                    });
                    PageAction::Consumed
                }
                KeyCode::Char(c) => {
                    input.insert(cursor, c);
                    cursor += 1;
                    self.setup_flow = Some(SetupFlow::EnterSetupToken {
                        provider,
                        input,
                        cursor,
                        set_processing_on_success,
                    });
                    PageAction::Consumed
                }
                KeyCode::Enter => {
                    let token = input.trim();
                    if token.is_empty() {
                        self.set_status_subflow(false, "Setup-token cannot be empty.");
                        return PageAction::Consumed;
                    }
                    match save_setup_token(&provider, token) {
                        Ok(()) => {
                            if self.pending_processing_provider.as_ref() == Some(&provider) {
                                self.finalize_processing_provider(provider);
                                self.pending_processing_provider = None;
                            }
                            self.setup_flow = None;
                            self.set_status_subflow(
                                true,
                                "Setup-token saved. Cloud import is now ready.",
                            );
                        }
                        Err(e) => {
                            self.set_status_subflow(
                                false,
                                format!("Failed to save setup-token: {e}"),
                            );
                        }
                    }
                    PageAction::Consumed
                }
                _ => PageAction::Ignored,
            },
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
                        self.set_status_subflow(false, "Reset cancelled.");
                    } else {
                        self.reset_flow = Some(ResetFlow::TypeConfirm {
                            input: String::new(),
                        });
                    }
                    PageAction::Consumed
                }
                KeyCode::Esc => {
                    self.reset_flow = None;
                    self.set_status_subflow(false, "Reset cancelled.");
                    PageAction::Consumed
                }
                _ => PageAction::Ignored,
            },
            Some(ResetFlow::TypeConfirm { mut input }) => match key.code {
                KeyCode::Esc => {
                    self.reset_flow = None;
                    self.set_status_subflow(false, "Reset cancelled.");
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
                        self.set_status_subflow(false, "Confirmation mismatch. Type RESET exactly.");
                        return PageAction::Consumed;
                    }

                    match crate::cli::reset::move_vault_to_trash() {
                        Ok(_) => {
                            app.vault_initialized = false;
                            app.should_quit = true;
                            self.reset_flow = None;
                            self.set_status_subflow(true, "Vault moved to Trash successfully.");
                        }
                        Err(e) => {
                            self.set_status_subflow(false, format!("Reset failed: {e}"));
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

impl SettingsPage {
    fn handle_oauth_action(
        &mut self,
        provider: &Provider,
        set_processing_on_success: bool,
    ) -> PageAction {
        if is_logged_in(provider).unwrap_or(false) {
            match remove_credentials(provider) {
                Ok(true) => {
                    self.set_status_subflow(true, format!("Disconnected {}.", provider.display_name()));
                }
                Ok(false) => {
                    self.set_status_subflow(
                        false,
                        format!("No active {} connection.", provider.display_name()),
                    );
                }
                Err(e) => {
                    self.set_status_subflow(false, format!("Disconnect failed: {e}"));
                }
            }
            self.setup_flow = None;
            self.pending_processing_provider = None;
            return PageAction::Consumed;
        }

        if !oauth_supported(provider) {
            self.set_status_subflow(
                false,
                match provider {
                    Provider::ChatGpt => "OAuth requires Codex CLI (`codex login`). Install Codex CLI or use API key.".to_string(),
                    Provider::Gemini => "OAuth requires Gemini CLI (`gemini`). Install Gemini CLI or use API key.".to_string(),
                    Provider::Claude => "Claude browser OAuth is not configured. Use API key or setup-token.".to_string(),
                },
            );
            return PageAction::Consumed;
        }

        self.pending_oauth = true;
        if set_processing_on_success {
            self.pending_processing_provider = Some(provider.clone());
        } else {
            self.pending_processing_provider = None;
        }
        self.set_status_subflow(true, format!("Starting OAuth for {}...", provider.display_name()));
        self.setup_flow = None;
        PageAction::StartOAuthConnect(provider.clone())
    }
}

impl ProcessingChoice {
    fn provider(self) -> Option<Provider> {
        match self {
            ProcessingChoice::Disabled | ProcessingChoice::Cloud => None,
            ProcessingChoice::Claude => Some(Provider::Claude),
            ProcessingChoice::ChatGpt => Some(Provider::ChatGpt),
            ProcessingChoice::Gemini => Some(Provider::Gemini),
        }
    }

    fn label(self) -> &'static str {
        match self {
            ProcessingChoice::Disabled => "Disabled (raw sessions only)",
            ProcessingChoice::Claude => "Claude",
            ProcessingChoice::ChatGpt => "ChatGPT",
            ProcessingChoice::Gemini => "Gemini",
            ProcessingChoice::Cloud => "Soul Vault Cloud (coming soon)",
        }
    }
}

fn auth_actions_for_provider(provider: &Provider) -> Vec<AuthAction> {
    if *provider == Provider::Claude {
        return vec![AuthAction::ApiKey, AuthAction::ClaudeSetupToken, AuthAction::Back];
    }
    vec![AuthAction::ApiKey, AuthAction::OAuth, AuthAction::Back]
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

fn render_settings_main(area: Rect, buf: &mut Buffer, page: &SettingsPage) {
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
    let current = page.current_item();

    let mut lines: Vec<Line> = vec![
        section_header("  Vault"),
        Line::from(""),
        kv_line("  Path", &root.display().to_string()),
        kv_line("  Processing", config.processing_mode.display_name()),
        kv_line("  Created", &config.created_at),
        Line::from(""),
        section_header("  Credentials & Connections"),
        Line::from(""),
    ];

    for provider in [Provider::Claude, Provider::ChatGpt, Provider::Gemini] {
        let selected = current == SelectableItem::Credential(provider.clone());
        lines.push(credential_line(&config, &provider, selected));
    }

    lines.push(Line::from(""));
    lines.push(section_header("  Processing Mode"));
    lines.push(Line::from(""));

    for choice in [
        ProcessingChoice::Disabled,
        ProcessingChoice::Claude,
        ProcessingChoice::ChatGpt,
        ProcessingChoice::Gemini,
        ProcessingChoice::Cloud,
    ] {
        let selected = current == SelectableItem::Processing(choice);
        let active = match choice {
            ProcessingChoice::Disabled => config.processing_mode == ProcessingMode::Disabled,
            ProcessingChoice::Cloud => false,
            _ => choice.provider() == config.processing_mode.as_provider(),
        };
        lines.push(processing_line(choice, selected, active));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Danger zone",
        Style::default().fg(rat::RED).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    lines.push(danger_line(current == SelectableItem::DangerReset));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Up/Down select · Enter open/apply · X reset · Esc back",
        Style::default().fg(rat::DIM),
    )));

    if page.pending_oauth {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Waiting for OAuth callback in your browser...",
            Style::default().fg(rat::CYAN),
        )));
    }

    if !page.status_from_subflow {
        if let Some((ok, msg)) = &page.status_message {
        let color = if *ok { rat::EMERALD } else { rat::RED };
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", msg),
            Style::default().fg(color),
        )));
        }
    }

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

fn render_setup_flow(
    area: Rect,
    buf: &mut Buffer,
    flow: &SetupFlow,
    status_message: Option<&(bool, String)>,
) {
    let mut lines: Vec<Line> = vec![section_header("  Credential Setup"), Line::from("")];

    match flow {
        SetupFlow::AuthMenu {
            provider,
            selected,
            set_processing_on_success,
        } => {
            lines.push(Line::from(Span::styled(
                format!("  {}", provider.display_name()),
                Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                if *set_processing_on_success {
                    "  Choose how to set credentials for processing"
                } else {
                    "  Manage provider credentials"
                },
                Style::default().fg(rat::DIM),
            )));
            lines.push(Line::from(""));

            let actions = auth_actions_for_provider(provider);
            for (idx, action) in actions.iter().enumerate() {
                let is_selected = idx == *selected;
                let prefix = if is_selected { "  > " } else { "    " };
                let color = if is_selected { rat::GOLD } else { rat::DIM };
                lines.push(Line::from(Span::styled(
                    format!("{}{}", prefix, auth_action_label(*action)),
                    Style::default().fg(color),
                )));
            }
        }
        SetupFlow::EnterApiKey {
            provider,
            input,
            cursor,
            submitting,
            ..
        } => {
            lines.push(Line::from(Span::styled(
                format!("  Enter {} API key", provider.display_name()),
                Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                format!("  Hint: {}", provider.api_key_hint()),
                Style::default().fg(rat::DIM),
            )));
            lines.push(Line::from(""));

            let mut shown = input.clone();
            if *cursor <= shown.len() {
                shown.insert(*cursor, '|');
            }
            lines.push(Line::from(Span::styled(
                format!("  {}", shown),
                Style::default().fg(rat::CYAN),
            )));

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                if *submitting {
                    "  Validating and saving..."
                } else {
                    "  Enter save · Esc back"
                },
                Style::default().fg(rat::DIM),
            )));
        }
        SetupFlow::EnterSetupToken {
            provider,
            input,
            cursor,
            ..
        } => {
            lines.push(Line::from(Span::styled(
                format!("  Enter {} setup-token", provider.display_name()),
                Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "  Generate it with `claude setup-token`, then paste it here.",
                Style::default().fg(rat::DIM),
            )));
            lines.push(Line::from(""));

            let mut shown = input.clone();
            if *cursor <= shown.len() {
                shown.insert(*cursor, '|');
            }
            lines.push(Line::from(Span::styled(
                format!("  {}", shown),
                Style::default().fg(rat::CYAN),
            )));

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Enter save · Esc back",
                Style::default().fg(rat::DIM),
            )));
        }
    }

    if let Some((ok, msg)) = status_message {
        let color = if *ok { rat::EMERALD } else { rat::RED };
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", msg),
            Style::default().fg(color),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Up/Down choose · Enter confirm · Esc back",
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

fn render_reset_flow(
    area: Rect,
    buf: &mut Buffer,
    flow: &ResetFlow,
    status_message: Option<&(bool, String)>,
) {
    let mut lines: Vec<Line> = vec![
        section_header("  Reset Vault"),
        Line::from(""),
        Line::from(Span::styled(
            "  This will move your vault to Trash.",
            Style::default().fg(rat::DIM),
        )),
        Line::from(""),
    ];

    match flow {
        ResetFlow::Confirm { selected } => {
            let options = ["Cancel", "Yes, move vault to Trash"];
            for (idx, option) in options.iter().enumerate() {
                let is_selected = idx == *selected;
                let prefix = if is_selected { "  > " } else { "    " };
                let color = if is_selected {
                    if idx == 1 {
                        rat::RED
                    } else {
                        rat::GOLD
                    }
                } else {
                    rat::DIM
                };
                lines.push(Line::from(Span::styled(
                    format!("{}{}", prefix, option),
                    Style::default().fg(color),
                )));
            }
        }
        ResetFlow::TypeConfirm { input } => {
            lines.push(Line::from(Span::styled(
                "  Type RESET to confirm:",
                Style::default().fg(rat::RED).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                format!("  {}", input),
                Style::default().fg(rat::CYAN),
            )));
        }
    }

    if let Some((ok, msg)) = status_message {
        let color = if *ok { rat::EMERALD } else { rat::RED };
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", msg),
            Style::default().fg(color),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Up/Down choose · Enter confirm · Esc back",
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

fn auth_action_label(action: AuthAction) -> &'static str {
    match action {
        AuthAction::ApiKey => "Set API key",
        AuthAction::ClaudeSetupToken => "Paste setup-token",
        AuthAction::OAuth => "OAuth",
        AuthAction::Back => "Back",
    }
}

fn credential_line(config: &SoulVaultConfig, provider: &Provider, selected: bool) -> Line<'static> {
    let prefix = if selected { "  > " } else { "    " };
    let name_style = if selected {
        Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(rat::DIM)
    };

    let key_label = api_key_status_label(provider);
    let oauth_label = oauth_status_label(config, provider);

    Line::from(vec![
        Span::styled(prefix, name_style),
        Span::styled(format!("{:<8}", provider.display_name()), name_style),
        Span::styled(format!(" API: {:<12}", key_label), Style::default().fg(rat::DIM)),
        Span::styled(format!(" OAuth: {:<15}", oauth_label), Style::default().fg(rat::DIM)),
    ])
}

fn processing_line(choice: ProcessingChoice, selected: bool, active: bool) -> Line<'static> {
    let prefix = if selected { "  > " } else { "    " };
    let marker = if active { "●" } else { " " };
    let color = if selected {
        rat::GOLD
    } else if active {
        rat::EMERALD
    } else {
        rat::DIM
    };

    Line::from(Span::styled(
        format!("{} {} {}", prefix, marker, choice.label()),
        Style::default().fg(color),
    ))
}

fn danger_line(selected: bool) -> Line<'static> {
    let prefix = if selected { "  > " } else { "    " };
    let style = if selected {
        Style::default().fg(rat::RED).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(rat::RED)
    };
    Line::from(Span::styled(
        format!("{}Reset vault (typed confirmation required)", prefix),
        style,
    ))
}

fn api_key_status_label(provider: &Provider) -> String {
    let key = get_api_key(&provider.to_string())
        .ok()
        .flatten()
        .unwrap_or_default();
    if key.trim().is_empty() {
        return "not set".to_string();
    }

    match get_key_health(provider).ok().flatten().map(|h| h.status) {
        Some(ApiKeyHealth::Verified) => "verified".to_string(),
        Some(ApiKeyHealth::Unverified) => "unverified".to_string(),
        Some(ApiKeyHealth::Invalid) => "invalid".to_string(),
        None => "set".to_string(),
    }
}

fn oauth_status_label(config: &SoulVaultConfig, provider: &Provider) -> &'static str {
    if *provider == Provider::Claude {
        return if is_logged_in(provider).unwrap_or(false) {
            "token saved"
        } else {
            "setup-token"
        };
    }
    if !oauth_supported(provider) {
        return "unavailable";
    }
    if is_logged_in(provider).unwrap_or(false) {
        return "connected";
    }
    if !provider_enabled(config, provider) {
        return "not setup";
    }
    "ready"
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
        Span::styled(value.to_string(), Style::default().fg(rat::CYAN)),
    ])
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

fn oauth_supported(provider: &Provider) -> bool {
    oauth_connect_available(provider)
}

#[allow(dead_code)]
fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        "••••••••".to_string()
    } else {
        format!("{}••••{}", &key[..4], &key[key.len() - 4..])
    }
}

#[allow(dead_code)]
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

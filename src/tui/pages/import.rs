//! Unified import page — provider auto-discovery and manual folder import.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Widget},
};

use super::import_provider_render;
use super::import_render;
use crate::core::pipeline::{ImportProgress, ImportResult};
use crate::tui::app::App;
use crate::tui::pages::{PageAction, PageWidget};
use crate::types::Provider;
use crate::ui::theme::rat;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportMode {
    Providers,
    Folder,
    Cloud,
}

#[derive(Debug, Clone)]
pub enum ProviderPhase {
    Ready,
    Running { progress: Vec<String> },
    Processing { current: usize, total: usize },
    Done { summary: Vec<String> },
    Error(String),
}

#[derive(Debug, Clone)]
pub enum FolderPhase {
    Input,
    Scanning,
    Classifying,
    Processing {
        current: usize,
        total: usize,
        current_file: String,
    },
    Writing,
    Done(ImportResult),
    NothingToImport {
        skipped_count: usize,
    },
    Error(String),
}

#[derive(Debug, Clone)]
pub enum CloudPhase {
    Ready {
        provider_index: usize,
    },
    Running {
        provider: Provider,
        state_label: String,
        current: usize,
        total: usize,
        progress: Vec<String>,
    },
    Done {
        summary: Vec<String>,
    },
    Error(String),
}

pub struct ImportPage {
    mode: ImportMode,
    input: String,
    cursor: usize,
    pub provider_phase: ProviderPhase,
    pub folder_phase: FolderPhase,
    pub cloud_phase: CloudPhase,
}

impl Default for ImportPage {
    fn default() -> Self {
        Self {
            mode: ImportMode::Providers,
            input: String::new(),
            cursor: 0,
            provider_phase: ProviderPhase::Ready,
            folder_phase: FolderPhase::Input,
            cloud_phase: CloudPhase::Ready { provider_index: 0 },
        }
    }
}

impl ImportPage {
    pub fn on_folder_progress(&mut self, progress: ImportProgress) {
        self.folder_phase = match progress {
            ImportProgress::Scanning | ImportProgress::ScanComplete { .. } => FolderPhase::Scanning,
            ImportProgress::Classifying | ImportProgress::ClassifyComplete { .. } => {
                FolderPhase::Classifying
            }
            ImportProgress::NothingToImport { skipped_count } => {
                FolderPhase::NothingToImport { skipped_count }
            }
            ImportProgress::Processing {
                current,
                total,
                current_file,
            } => FolderPhase::Processing {
                current,
                total,
                current_file,
            },
            ImportProgress::Writing => FolderPhase::Writing,
            ImportProgress::Done(result) => FolderPhase::Done(result),
            ImportProgress::Error(msg) => FolderPhase::Error(msg),
        };
    }

    pub fn on_provider_progress(&mut self, msg: String) {
        match &mut self.provider_phase {
            ProviderPhase::Running { progress } => progress.push(msg),
            _ => {
                self.provider_phase = ProviderPhase::Running {
                    progress: vec![msg],
                }
            }
        }
    }
    pub fn on_provider_processing(&mut self, current: usize, total: usize) {
        self.provider_phase = ProviderPhase::Processing { current, total };
    }
    pub fn on_provider_done(&mut self, summary: Vec<String>) {
        self.provider_phase = ProviderPhase::Done { summary };
    }
    pub fn on_provider_error(&mut self, msg: String) {
        self.provider_phase = ProviderPhase::Error(msg);
    }
    fn switch_mode(&mut self) {
        self.mode = match self.mode {
            ImportMode::Providers => ImportMode::Folder,
            ImportMode::Folder => ImportMode::Cloud,
            ImportMode::Cloud => ImportMode::Providers,
        };
    }

    pub fn on_cloud_progress(
        &mut self,
        provider: Provider,
        state_label: String,
        current: usize,
        total: usize,
        msg: String,
    ) {
        match &mut self.cloud_phase {
            CloudPhase::Running {
                provider: running_provider,
                progress,
                state_label: running_state,
                current: running_current,
                total: running_total,
            } if *running_provider == provider => {
                *running_state = state_label;
                *running_current = current;
                *running_total = total;
                progress.push(msg);
            }
            _ => {
                self.cloud_phase = CloudPhase::Running {
                    provider,
                    state_label,
                    current,
                    total,
                    progress: vec![msg],
                };
            }
        }
    }

    pub fn on_cloud_done(&mut self, summary: Vec<String>) {
        self.cloud_phase = CloudPhase::Done { summary };
    }

    pub fn on_cloud_error(&mut self, msg: String) {
        self.cloud_phase = CloudPhase::Error(msg);
    }
}

impl PageWidget for ImportPage {
    fn render(&self, area: Rect, buf: &mut Buffer, app: &App) {
        if !app.vault_initialized {
            import_render::render_not_init(area, buf);
            return;
        }

        let mode_label = match self.mode {
            ImportMode::Providers => " Import — Providers ",
            ImportMode::Folder => " Import — Folder ",
            ImportMode::Cloud => " Import — Cloud ",
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(rat::GOLD))
            .title(mode_label)
            .title_style(Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD));
        let inner = block.inner(area);
        block.render(area, buf);

        match self.mode {
            ImportMode::Providers => match &self.provider_phase {
                ProviderPhase::Ready => import_provider_render::render_ready(inner, buf),
                ProviderPhase::Running { progress } => {
                    import_provider_render::render_running(inner, buf, progress)
                }
                ProviderPhase::Processing { current, total } => {
                    import_provider_render::render_processing(inner, buf, *current, *total)
                }
                ProviderPhase::Done { summary } => {
                    import_provider_render::render_done(inner, buf, summary)
                }
                ProviderPhase::Error(msg) => import_provider_render::render_error(inner, buf, msg),
            },
            ImportMode::Folder => match &self.folder_phase {
                FolderPhase::Input => import_render::render_input(inner, buf, &self.input),
                FolderPhase::Scanning => {
                    import_render::render_phase(inner, buf, "Scanning for files...")
                }
                FolderPhase::Classifying => {
                    import_render::render_phase(inner, buf, "Checking for changes...")
                }
                FolderPhase::Processing {
                    current,
                    total,
                    current_file,
                } => import_render::render_processing(inner, buf, *current, *total, current_file),
                FolderPhase::Writing => {
                    import_render::render_phase(inner, buf, "Merging and writing to vault...")
                }
                FolderPhase::Done(result) => import_render::render_done(inner, buf, result),
                FolderPhase::NothingToImport { skipped_count } => {
                    import_render::render_nothing(inner, buf, *skipped_count)
                }
                FolderPhase::Error(msg) => import_render::render_error(inner, buf, msg),
            },
            ImportMode::Cloud => render_cloud(inner, buf, &self.cloud_phase),
        }
    }

    fn handle_key(&mut self, key: KeyEvent, _app: &mut App) -> PageAction {
        if matches!(key.code, KeyCode::Tab) {
            self.switch_mode();
            return PageAction::Consumed;
        }

        match self.mode {
            ImportMode::Providers => handle_provider_key(key, &mut self.provider_phase),
            ImportMode::Folder => handle_folder_key(
                key,
                &mut self.input,
                &mut self.cursor,
                &mut self.folder_phase,
            ),
            ImportMode::Cloud => handle_cloud_key(key, &mut self.cloud_phase),
        }
    }
}

fn render_cloud(area: Rect, buf: &mut Buffer, phase: &CloudPhase) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Paragraph, Widget};

    match phase {
        CloudPhase::Ready { provider_index } => {
            let providers = [Provider::Claude, Provider::ChatGpt, Provider::Gemini];
            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Cloud mode — explicit provider selection required",
                    Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Up/Down to choose provider, Enter to start",
                    Style::default(),
                )),
                Line::from(Span::styled(
                    "  Tab to switch mode, Esc to go back",
                    Style::default().fg(rat::DIM),
                )),
                Line::from(""),
            ];

            for (i, provider) in providers.iter().enumerate() {
                let selected = i == *provider_index;
                let prefix = if selected { ">" } else { " " };
                let style = if selected {
                    Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(rat::DIM)
                };
                let label = if matches!(provider, Provider::Claude) {
                    format!("{} (export-only)", provider.display_name())
                } else {
                    provider.display_name().to_string()
                };
                lines.push(Line::from(Span::styled(
                    format!("  {} {}", prefix, label),
                    style,
                )));
            }

            Paragraph::new(lines).render(area, buf);
        }
        CloudPhase::Running {
            provider,
            state_label,
            current,
            total,
            progress,
        } => {
            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {} cloud import running", provider.display_name()),
                    Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("  State: {} ({}/{})", state_label, current, total),
                    Style::default().fg(rat::DIM),
                )),
                Line::from(Span::styled(
                    "  Press x to cancel",
                    Style::default().fg(rat::DIM),
                )),
                Line::from(""),
            ];

            let max_lines = area.height.saturating_sub(lines.len() as u16) as usize;
            let start = progress.len().saturating_sub(max_lines);
            for line in progress.iter().skip(start) {
                lines.push(Line::from(Span::styled(
                    format!("  {}", line),
                    Style::default().fg(rat::DIM),
                )));
            }
            Paragraph::new(lines).render(area, buf);
        }
        CloudPhase::Done { summary } => {
            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  + Cloud import complete!",
                    Style::default()
                        .fg(rat::EMERALD)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
            ];
            for msg in summary {
                lines.push(Line::from(format!("  {}", msg)));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Enter to run again, Tab to switch mode, Esc to go back",
                Style::default().fg(rat::DIM),
            )));
            Paragraph::new(lines).render(area, buf);
        }
        CloudPhase::Error(msg) => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  x {}", msg),
                    Style::default().fg(rat::RED),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Enter to try again, Tab to switch mode, Esc to go back",
                    Style::default().fg(rat::DIM),
                )),
            ];
            Paragraph::new(lines).render(area, buf);
        }
    }
}

fn handle_provider_key(key: KeyEvent, phase: &mut ProviderPhase) -> PageAction {
    match phase {
        ProviderPhase::Ready => match key.code {
            KeyCode::Left => PageAction::BackToSidebar,
            KeyCode::Enter => PageAction::StartProviderImport,
            KeyCode::Esc => PageAction::BackToSidebar,
            _ => PageAction::Ignored,
        },
        ProviderPhase::Running { .. } | ProviderPhase::Processing { .. } => match key.code {
            KeyCode::Left => PageAction::BackToSidebar,
            _ => PageAction::Ignored,
        },
        ProviderPhase::Done { .. } | ProviderPhase::Error(_) => match key.code {
            KeyCode::Left => PageAction::BackToSidebar,
            KeyCode::Enter | KeyCode::Char('r') => {
                *phase = ProviderPhase::Ready;
                PageAction::Consumed
            }
            KeyCode::Esc => {
                *phase = ProviderPhase::Ready;
                PageAction::BackToSidebar
            }
            _ => PageAction::Ignored,
        },
    }
}

fn handle_folder_key(
    key: KeyEvent,
    input: &mut String,
    cursor: &mut usize,
    phase: &mut FolderPhase,
) -> PageAction {
    match phase {
        FolderPhase::Input => import_render::handle_input_key(key, input, cursor),
        FolderPhase::Scanning
        | FolderPhase::Classifying
        | FolderPhase::Processing { .. }
        | FolderPhase::Writing => match key.code {
            KeyCode::Left => PageAction::BackToSidebar,
            _ => PageAction::Consumed,
        },
        FolderPhase::Done(_) | FolderPhase::NothingToImport { .. } | FolderPhase::Error(_) => {
            match key.code {
                KeyCode::Left => PageAction::BackToSidebar,
                KeyCode::Enter | KeyCode::Char('r') => {
                    *phase = FolderPhase::Input;
                    input.clear();
                    *cursor = 0;
                    PageAction::Consumed
                }
                KeyCode::Esc => {
                    *phase = FolderPhase::Input;
                    input.clear();
                    *cursor = 0;
                    PageAction::BackToSidebar
                }
                _ => PageAction::Ignored,
            }
        }
    }
}

fn handle_cloud_key(key: KeyEvent, phase: &mut CloudPhase) -> PageAction {
    match phase {
        CloudPhase::Ready { provider_index } => match key.code {
            KeyCode::Left | KeyCode::Esc => PageAction::BackToSidebar,
            KeyCode::Up => {
                *provider_index = provider_index.saturating_sub(1);
                PageAction::Consumed
            }
            KeyCode::Down => {
                *provider_index = (*provider_index + 1).min(2);
                PageAction::Consumed
            }
            KeyCode::Enter => {
                let provider = match *provider_index {
                    0 => Provider::Claude,
                    1 => Provider::ChatGpt,
                    _ => Provider::Gemini,
                };
                PageAction::StartCloudImport(provider)
            }
            _ => PageAction::Ignored,
        },
        CloudPhase::Running { .. } => match key.code {
            KeyCode::Left => PageAction::BackToSidebar,
            KeyCode::Char('x') => PageAction::CancelCloudImport,
            _ => PageAction::Consumed,
        },
        CloudPhase::Done { .. } | CloudPhase::Error(_) => match key.code {
            KeyCode::Left => PageAction::BackToSidebar,
            KeyCode::Enter | KeyCode::Char('r') => {
                *phase = CloudPhase::Ready { provider_index: 0 };
                PageAction::Consumed
            }
            KeyCode::Esc => {
                *phase = CloudPhase::Ready { provider_index: 0 };
                PageAction::BackToSidebar
            }
            _ => PageAction::Ignored,
        },
    }
}

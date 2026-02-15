//! Export page — format selection, topic filter, output path, preview and execute.

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
use crate::vault::read::read_vault_content;

// ─── Export State ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportFormat {
    Markdown,
    Json,
}

impl ExportFormat {
    fn label(self) -> &'static str {
        match self {
            ExportFormat::Markdown => "Markdown",
            ExportFormat::Json => "JSON",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportField {
    Format,
    TopicFilter,
    OutputPath,
    Execute,
}

pub struct ExportPage {
    format: ExportFormat,
    topic_filter: String,
    output_path: String,
    active_field: ExportField,
    editing: bool,
    result_msg: Option<(bool, String)>, // (success, message)
}

impl Default for ExportPage {
    fn default() -> Self {
        Self {
            format: ExportFormat::Markdown,
            topic_filter: String::new(),
            output_path: String::new(),
            active_field: ExportField::Format,
            editing: false,
            result_msg: None,
        }
    }
}

// ─── PageWidget ───────────────────────────────────────────────────────────────

impl PageWidget for ExportPage {
    fn render(&self, area: Rect, buf: &mut Buffer, app: &App) {
        if !app.vault_initialized {
            render_not_init(area, buf);
            return;
        }
        render_form(area, buf, self);
    }

    fn handle_key(&mut self, key: KeyEvent, _app: &mut App) -> PageAction {
        if self.editing {
            return self.handle_editing(key);
        }
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.next_field();
                PageAction::Consumed
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.prev_field();
                PageAction::Consumed
            }
            KeyCode::Enter => {
                match self.active_field {
                    ExportField::Format => {
                        self.format = match self.format {
                            ExportFormat::Markdown => ExportFormat::Json,
                            ExportFormat::Json => ExportFormat::Markdown,
                        };
                    }
                    ExportField::TopicFilter => self.editing = true,
                    ExportField::OutputPath => self.editing = true,
                    ExportField::Execute => self.execute_export(),
                }
                PageAction::Consumed
            }
            KeyCode::Esc => {
                self.result_msg = None;
                PageAction::BackToSidebar
            }
            _ => PageAction::Ignored,
        }
    }
}

impl ExportPage {
    fn next_field(&mut self) {
        self.active_field = match self.active_field {
            ExportField::Format => ExportField::TopicFilter,
            ExportField::TopicFilter => ExportField::OutputPath,
            ExportField::OutputPath => ExportField::Execute,
            ExportField::Execute => ExportField::Format,
        };
    }

    fn prev_field(&mut self) {
        self.active_field = match self.active_field {
            ExportField::Format => ExportField::Execute,
            ExportField::TopicFilter => ExportField::Format,
            ExportField::OutputPath => ExportField::TopicFilter,
            ExportField::Execute => ExportField::OutputPath,
        };
    }

    fn handle_editing(&mut self, key: KeyEvent) -> PageAction {
        let target = match self.active_field {
            ExportField::TopicFilter => &mut self.topic_filter,
            ExportField::OutputPath => &mut self.output_path,
            _ => return PageAction::Ignored,
        };
        match key.code {
            KeyCode::Char(c) => target.push(c),
            KeyCode::Backspace => {
                target.pop();
            }
            KeyCode::Enter | KeyCode::Esc => self.editing = false,
            _ => {}
        }
        PageAction::Consumed
    }

    fn execute_export(&mut self) {
        let fmt = match self.format {
            ExportFormat::Markdown => "markdown",
            ExportFormat::Json => "json",
        };
        let topic = if self.topic_filter.trim().is_empty() {
            None
        } else {
            Some(self.topic_filter.trim())
        };
        let output = if self.output_path.trim().is_empty() {
            None
        } else {
            Some(expand_tilde(self.output_path.trim()))
        };

        match crate::cli::export::run(output.as_deref(), fmt, topic) {
            Ok(()) => {
                let dest = output.unwrap_or_else(|| "stdout".into());
                self.result_msg = Some((true, format!("Exported {} to {}", fmt, dest)));
            }
            Err(e) => {
                self.result_msg = Some((false, e.to_string()));
            }
        }
    }
}

// ─── Rendering ────────────────────────────────────────────────────────────────

fn render_not_init(area: Rect, buf: &mut Buffer) {
    Paragraph::new(Span::styled(
        "  Vault not initialized. Run `soul init` first.",
        Style::default().fg(rat::AMBER),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(rat::AMBER))
            .title(" Export "),
    )
    .render(area, buf);
}

fn render_form(area: Rect, buf: &mut Buffer, page: &ExportPage) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(rat::GOLD))
        .title(" Export ")
        .title_style(Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD));
    let inner = block.inner(area);
    block.render(area, buf);

    let active = page.active_field;
    let cursor = |f| if page.editing && active == f { "_" } else { "" };

    let mut lines = vec![
        Line::from(""),
        field_line(active, ExportField::Format, "Format", page.format.label(), ""),
        Line::from(""),
        field_line(
            active,
            ExportField::TopicFilter,
            "Topic filter",
            if page.topic_filter.is_empty() { "(none)" } else { &page.topic_filter },
            cursor(ExportField::TopicFilter),
        ),
        Line::from(""),
        field_line(
            active,
            ExportField::OutputPath,
            "Output path",
            if page.output_path.is_empty() { "(stdout)" } else { &page.output_path },
            cursor(ExportField::OutputPath),
        ),
        Line::from(""),
        field_line(active, ExportField::Execute, "[ Export ]", "", ""),
    ];

    if let Ok(vault) = read_vault_content() {
        let wc: usize = vault.identity.split_whitespace().count()
            + vault.preferences.split_whitespace().count()
            + vault.topics.iter().map(|t| t.content.split_whitespace().count()).sum::<usize>()
            + vault.people.iter().map(|p| p.content.split_whitespace().count()).sum::<usize>()
            + vault.memories.iter().map(|m| m.content.split_whitespace().count()).sum::<usize>();
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  ~{} words in vault", wc),
            Style::default().fg(rat::DIM),
        )));
    }

    if let Some((ok, msg)) = &page.result_msg {
        lines.push(Line::from(""));
        let color = if *ok { rat::EMERALD } else { rat::RED };
        lines.push(Line::from(Span::styled(format!("  {}", msg), Style::default().fg(color))));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  j/k navigate  Enter select/toggle  Esc back",
        Style::default().fg(rat::DIM),
    )));

    Paragraph::new(lines).render(inner, buf);
}

fn field_line<'a>(
    active: ExportField,
    field: ExportField,
    label: &'a str,
    value: &'a str,
    cursor: &'a str,
) -> Line<'a> {
    let sel = if active == field { " > " } else { "   " };
    let style = if active == field {
        Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let mut spans = vec![Span::styled(sel, style)];
    if value.is_empty() {
        spans.push(Span::styled(label, style));
    } else {
        spans.push(Span::styled(format!("{}: ", label), Style::default().fg(rat::DIM)));
        spans.push(Span::styled(value, style));
    }
    if !cursor.is_empty() {
        spans.push(Span::styled(cursor, Style::default().fg(rat::GOLD)));
    }
    Line::from(spans)
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn expand_tilde(path: &str) -> String {
    if path.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            return path.replacen('~', &home.display().to_string(), 1);
        }
    }
    path.to_string()
}

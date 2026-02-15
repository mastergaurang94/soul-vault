//! Export page — format selection, section toggles, preview, and execute.

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
    Context,
    Json,
    Bundle,
}

impl ExportFormat {
    fn next(self) -> Self {
        match self {
            Self::Context => Self::Json,
            Self::Json => Self::Bundle,
            Self::Bundle => Self::Context,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Context => "Markdown Context",
            Self::Json => "JSON",
            Self::Bundle => "Bundle (folder)",
        }
    }

    fn arg(self) -> &'static str {
        match self {
            Self::Context => "context",
            Self::Json => "json",
            Self::Bundle => "bundle",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportField {
    Format,
    Identity,
    Preferences,
    Topics,
    People,
    Memories,
    Execute,
}

impl ExportField {
    const ALL: [Self; 7] = [
        Self::Format,
        Self::Identity,
        Self::Preferences,
        Self::Topics,
        Self::People,
        Self::Memories,
        Self::Execute,
    ];

    fn next(self) -> Self {
        let idx = Self::ALL
            .iter()
            .position(|field| *field == self)
            .unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    fn prev(self) -> Self {
        let idx = Self::ALL
            .iter()
            .position(|field| *field == self)
            .unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

pub struct ExportPage {
    format: ExportFormat,
    include_identity: bool,
    include_preferences: bool,
    include_topics: bool,
    include_people: bool,
    include_memories: bool,
    active_field: ExportField,
    result_msg: Option<(bool, String)>, // (success, message)
}

impl Default for ExportPage {
    fn default() -> Self {
        Self {
            format: ExportFormat::Context,
            include_identity: true,
            include_preferences: true,
            include_topics: true,
            include_people: true,
            include_memories: true,
            active_field: ExportField::Format,
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
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.active_field = self.active_field.next();
                PageAction::Consumed
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.active_field = self.active_field.prev();
                PageAction::Consumed
            }
            KeyCode::Enter => {
                self.select_active();
                PageAction::Consumed
            }
            KeyCode::Char(' ') => {
                if self.toggle_active_section() {
                    PageAction::Consumed
                } else {
                    PageAction::Ignored
                }
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
    fn select_active(&mut self) {
        match self.active_field {
            ExportField::Format => self.format = self.format.next(),
            ExportField::Execute => self.execute_export(),
            _ => {
                self.toggle_active_section();
            }
        }
    }

    fn toggle_active_section(&mut self) -> bool {
        match self.active_field {
            ExportField::Identity => self.include_identity = !self.include_identity,
            ExportField::Preferences => self.include_preferences = !self.include_preferences,
            ExportField::Topics => self.include_topics = !self.include_topics,
            ExportField::People => self.include_people = !self.include_people,
            ExportField::Memories => self.include_memories = !self.include_memories,
            _ => return false,
        }
        true
    }

    fn selected_sections_csv(&self) -> String {
        let mut sections = Vec::new();
        if self.include_identity {
            sections.push("identity");
        }
        if self.include_preferences {
            sections.push("preferences");
        }
        if self.include_topics {
            sections.push("topics");
        }
        if self.include_people {
            sections.push("people");
        }
        if self.include_memories {
            sections.push("memories");
        }
        sections.join(",")
    }

    fn output_path(&self) -> String {
        match crate::cli::export::smart_default_output_path(self.format.arg()) {
            Ok(path) => path.display().to_string(),
            Err(_) => "<unable to build path>".to_string(),
        }
    }

    fn execute_export(&mut self) {
        let output_path = self.output_path();
        let sections = self.selected_sections_csv();
        match crate::cli::export::run(Some(&output_path), self.format.arg(), None, Some(&sections))
        {
            Ok(()) => {
                self.result_msg = Some((true, format!("Exported to {}", output_path)));
            }
            Err(e) => {
                self.result_msg = Some((false, e.to_string()));
            }
        }
    }

    fn selected_word_count(&self) -> Option<usize> {
        let vault = read_vault_content().ok()?;
        let mut count = 0;
        if self.include_identity {
            count += vault.identity.split_whitespace().count();
        }
        if self.include_preferences {
            count += vault.preferences.split_whitespace().count();
        }
        if self.include_topics {
            count += vault
                .topics
                .iter()
                .map(|t| t.content.split_whitespace().count())
                .sum::<usize>();
        }
        if self.include_people {
            count += vault
                .people
                .iter()
                .map(|p| p.content.split_whitespace().count())
                .sum::<usize>();
        }
        if self.include_memories {
            count += vault
                .memories
                .iter()
                .map(|m| m.content.split_whitespace().count())
                .sum::<usize>();
        }
        Some(count)
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

    let mut lines = vec![
        Line::from(""),
        field_line(
            page.active_field,
            ExportField::Format,
            "Format",
            page.format.label(),
        ),
        Line::from(""),
        Line::from(Span::styled(
            "  Sections",
            Style::default().fg(rat::DIM).add_modifier(Modifier::BOLD),
        )),
        checkbox_line(
            page.active_field,
            ExportField::Identity,
            "Identity",
            page.include_identity,
        ),
        checkbox_line(
            page.active_field,
            ExportField::Preferences,
            "Preferences",
            page.include_preferences,
        ),
        checkbox_line(
            page.active_field,
            ExportField::Topics,
            "Topics",
            page.include_topics,
        ),
        checkbox_line(
            page.active_field,
            ExportField::People,
            "People",
            page.include_people,
        ),
        checkbox_line(
            page.active_field,
            ExportField::Memories,
            "Memories",
            page.include_memories,
        ),
        Line::from(""),
        Line::from(Span::styled(
            format!("  Output: {}", page.output_path()),
            Style::default().fg(rat::CYAN),
        )),
    ];

    if let Some(words) = page.selected_word_count() {
        lines.push(Line::from(Span::styled(
            format!("  Preview: ~{} words selected", words),
            Style::default().fg(rat::DIM),
        )));
    }

    lines.push(Line::from(""));
    lines.push(field_line(
        page.active_field,
        ExportField::Execute,
        "[ Export ]",
        "",
    ));

    if let Some((ok, msg)) = &page.result_msg {
        lines.push(Line::from(""));
        let color = if *ok { rat::EMERALD } else { rat::RED };
        lines.push(Line::from(Span::styled(
            format!("  {}", msg),
            Style::default().fg(color),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  j/k navigate  Enter select/toggle  Space toggle section  Esc back",
        Style::default().fg(rat::DIM),
    )));

    Paragraph::new(lines).render(inner, buf);
}

fn checkbox_line(
    active: ExportField,
    field: ExportField,
    label: &str,
    enabled: bool,
) -> Line<'static> {
    let marker = if enabled { "[x]" } else { "[ ]" };
    field_line(active, field, format!("{marker} {label}"), "")
}

fn field_line(
    active: ExportField,
    field: ExportField,
    label: impl Into<String>,
    value: impl Into<String>,
) -> Line<'static> {
    let label = label.into();
    let value = value.into();
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
        spans.push(Span::styled(
            format!("{}: ", label),
            Style::default().fg(rat::DIM),
        ));
        spans.push(Span::styled(value, style));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn tui_navigation_wraps() {
        let mut page = ExportPage::default();
        let mut app = App::new();

        page.handle_key(key(KeyCode::Char('k')), &mut app);
        assert_eq!(page.active_field, ExportField::Execute);

        page.handle_key(key(KeyCode::Char('j')), &mut app);
        assert_eq!(page.active_field, ExportField::Format);
    }

    #[test]
    fn tui_space_toggles_sections() {
        let mut page = ExportPage::default();
        let mut app = App::new();

        page.active_field = ExportField::Identity;
        page.handle_key(key(KeyCode::Char(' ')), &mut app);
        assert!(!page.include_identity);
    }

    #[test]
    fn smart_default_paths_match_format() {
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let context = crate::cli::export::smart_default_output_path("context").unwrap();
        let json = crate::cli::export::smart_default_output_path("json").unwrap();
        let bundle = crate::cli::export::smart_default_output_path("bundle").unwrap();

        assert!(context.ends_with(format!("soul-vault-export-{date}.md")));
        assert!(json.ends_with(format!("soul-vault-export-{date}.json")));
        assert!(bundle.ends_with(format!("soul-vault-export-{date}")));
    }
}

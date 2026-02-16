//! Export page rendering helpers.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use super::export::ExportPage;
use super::export_state::ExportField;
use crate::ui::theme::rat;
use crate::vault::read::read_vault_content;

pub fn render_not_init(area: Rect, buf: &mut Buffer) {
    Paragraph::new(Span::styled(
        "  Vault not initialized. Run `soul init` first.",
        Style::default().fg(rat::AMBER),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(rat::AMBER))
            .title(" Export — Context "),
    )
    .render(area, buf);
}

pub fn render_form(area: Rect, buf: &mut Buffer, page: &ExportPage) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(rat::GOLD))
        .title(" Export ")
        .title_style(Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD));
    let inner = block.inner(area);
    block.render(area, buf);

    let max_w = inner.width.saturating_sub(4) as usize;

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
            truncate_line(&format!("  Output: {}", page.output_path()), max_w),
            Style::default().fg(rat::CYAN),
        )),
    ];

    if let Some(words) = selected_word_count(page) {
        lines.push(Line::from(Span::styled(
            format!("  ~{words} words selected"),
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
            truncate_line(&format!("  {msg}"), max_w),
            Style::default().fg(color),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  j/k navigate · Enter/Space toggle · Esc back",
        Style::default().fg(rat::DIM),
    )));

    Paragraph::new(lines).render(inner, buf);
}

fn truncate_line(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max > 3 {
        format!("{}…", &s[..max - 1])
    } else {
        s[..max].to_string()
    }
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

fn selected_word_count(page: &ExportPage) -> Option<usize> {
    let vault = read_vault_content().ok()?;
    let mut count = 0;
    if page.include_identity {
        count += vault.identity.split_whitespace().count();
    }
    if page.include_preferences {
        count += vault.preferences.split_whitespace().count();
    }
    if page.include_topics {
        count += vault
            .topics
            .iter()
            .map(|t| t.content.split_whitespace().count())
            .sum::<usize>();
    }
    if page.include_people {
        count += vault
            .people
            .iter()
            .map(|p| p.content.split_whitespace().count())
            .sum::<usize>();
    }
    if page.include_memories {
        count += vault
            .memories
            .iter()
            .map(|m| m.content.split_whitespace().count())
            .sum::<usize>();
    }
    Some(count)
}

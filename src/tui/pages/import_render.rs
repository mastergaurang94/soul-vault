//! Import page rendering helpers — progress states and result summaries.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::core::pipeline::ImportResult;
use crate::ui::theme::rat;

pub fn render_not_init(area: Rect, buf: &mut Buffer) {
    Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Vault not initialized. Run `soul init` first.",
            Style::default().fg(rat::AMBER),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(rat::AMBER))
            .title(" Import "),
    )
    .render(area, buf);
}

pub fn render_input(area: Rect, buf: &mut Buffer, input: &str) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Enter folder path to import:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  > "),
            Span::styled(input, Style::default().fg(rat::CYAN)),
            Span::styled("_", Style::default().fg(rat::GOLD)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Enter to confirm, Esc to cancel",
            Style::default().fg(rat::DIM),
        )),
        Line::from(Span::styled(
            "  Example: ~/Documents/chatgpt-exports",
            Style::default().fg(rat::DIM),
        )),
    ];
    Paragraph::new(lines).render(area, buf);
}

pub fn render_phase(area: Rect, buf: &mut Buffer, message: &str) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  * {}", message),
            Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
        )),
    ];
    Paragraph::new(lines).render(area, buf);
}

pub fn render_processing(area: Rect, buf: &mut Buffer, cur: usize, total: usize, file: &str) {
    let pct = if total > 0 { (cur * 100) / total } else { 0 };
    let bar_w = 20;
    let filled = if total > 0 { (cur * bar_w) / total } else { 0 };
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_w - filled));

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Processing through Claude...",
            Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {}", bar), Style::default().fg(rat::GOLD)),
            Span::styled(
                format!(" {}/{} ({}%)", cur, total, pct),
                Style::default().fg(rat::DIM),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", file),
            Style::default().fg(rat::DIM),
        )),
    ];
    Paragraph::new(lines).render(area, buf);
}

pub fn render_done(area: Rect, buf: &mut Buffer, r: &ImportResult) {
    let files_str = format!(
        "{} new, {} modified, {} skipped",
        r.new_count, r.modified_count, r.skipped_count
    );
    let facts_str = r.facts_extracted.to_string();
    let topics_str = r.topics.len().to_string();
    let people_str = r.people.len().to_string();

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  + Import complete!",
            Style::default()
                .fg(rat::EMERALD)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        stat_line("  Files", &files_str),
        stat_line("  Facts extracted", &facts_str),
        stat_line("  Topics", &topics_str),
        stat_line("  People", &people_str),
    ];

    if !r.errors.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  {} errors", r.errors.len()),
            Style::default().fg(rat::AMBER),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press Enter to import another, Esc to go back",
        Style::default().fg(rat::DIM),
    )));
    Paragraph::new(lines).render(area, buf);
}

pub fn render_nothing(area: Rect, buf: &mut Buffer, skipped: usize) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  + All {} files unchanged. Nothing to import.", skipped),
            Style::default().fg(rat::EMERALD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Use `soul import --force <folder>` to re-import everything.",
            Style::default().fg(rat::DIM),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Press Enter to try again, Esc to go back",
            Style::default().fg(rat::DIM),
        )),
    ];
    Paragraph::new(lines).render(area, buf);
}

pub fn render_error(area: Rect, buf: &mut Buffer, msg: &str) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  x {}", msg),
            Style::default().fg(rat::RED),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Press Enter to try again, Esc to go back",
            Style::default().fg(rat::DIM),
        )),
    ];
    Paragraph::new(lines).render(area, buf);
}

fn stat_line<'a>(label: &'a str, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{:<20}", label), Style::default().fg(rat::DIM)),
        Span::styled(
            value.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ])
}

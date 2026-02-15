//! Watch page rendering helpers — event log and states.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use super::watch::{EventKind, WatchEvent};
use crate::ui::theme::rat;

pub fn render_not_init(area: Rect, buf: &mut Buffer) {
    Paragraph::new(Span::styled(
        "  Vault not initialized. Run `soul init` first.",
        Style::default().fg(rat::AMBER),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(rat::AMBER))
            .title(" Watch — Live "),
    )
    .render(area, buf);
}

pub fn render_input(area: Rect, buf: &mut Buffer, input: &str) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Enter folder path to watch:",
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
            "  Enter to start, Esc to cancel",
            Style::default().fg(rat::DIM),
        )),
        Line::from(Span::styled(
            "  Example: ~/Documents/chatgpt-exports",
            Style::default().fg(rat::DIM),
        )),
    ];
    Paragraph::new(lines).render(area, buf);
}

pub fn render_watching(area: Rect, buf: &mut Buffer, events: &[WatchEvent], scroll: usize) {
    let max_visible = area.height.saturating_sub(3) as usize;
    let start = if events.len() > max_visible {
        scroll.min(events.len().saturating_sub(max_visible))
    } else {
        0
    };

    let mut lines = vec![
        Line::from(Span::styled(
            "  Watching for changes (Esc to stop, j/k to scroll)",
            Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for event in events.iter().skip(start).take(max_visible) {
        let color = match event.kind {
            EventKind::Info => rat::DIM,
            EventKind::Success => rat::EMERALD,
            EventKind::Warning => rat::AMBER,
            EventKind::Error => rat::RED,
        };
        let icon = match event.kind {
            EventKind::Info => "-",
            EventKind::Success => "+",
            EventKind::Warning => "!",
            EventKind::Error => "x",
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", event.timestamp),
                Style::default().fg(rat::DIM),
            ),
            Span::styled(format!("{} ", icon), Style::default().fg(color)),
            Span::styled(&event.message, Style::default().fg(color)),
        ]));
    }

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

//! Import page provider-mode rendering helpers.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::ui::theme::rat;

pub fn render_ready(area: Rect, buf: &mut Buffer) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Providers mode — auto-discover AI app sessions",
            Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Sources: Claude Code, OpenClaw, Gemini CLI, Codex",
            Style::default().fg(rat::DIM),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Enter to start import",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  Tab to switch mode, Esc to go back",
            Style::default().fg(rat::DIM),
        )),
    ];
    Paragraph::new(lines).render(area, buf);
}

pub fn render_running(area: Rect, buf: &mut Buffer, progress: &[String]) {
    let max_lines = area.height.saturating_sub(2) as usize;
    let start = progress.len().saturating_sub(max_lines);

    let mut lines = vec![
        Line::from(Span::styled(
            "  Importing provider sessions...",
            Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    for msg in progress.iter().skip(start) {
        lines.push(Line::from(Span::styled(
            format!("  {}", msg),
            Style::default().fg(rat::DIM),
        )));
    }
    Paragraph::new(lines).render(area, buf);
}

pub fn render_processing(area: Rect, buf: &mut Buffer, current: usize, total: usize) {
    let pct = if total > 0 { (current * 100) / total } else { 0 };
    let bar_w = 20;
    let filled = if total > 0 {
        (current * bar_w) / total
    } else {
        0
    };
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_w - filled));

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Processing through Claude...",
            Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {bar}"), Style::default().fg(rat::GOLD)),
            Span::styled(
                format!(" {current}/{total} ({pct}%)"),
                Style::default().fg(rat::DIM),
            ),
        ]),
    ];
    Paragraph::new(lines).render(area, buf);
}

pub fn render_done(area: Rect, buf: &mut Buffer, summary: &[String]) {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  + Provider import complete!",
            Style::default()
                .fg(rat::EMERALD)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for msg in summary {
        lines.push(Line::from(Span::styled(
            format!("  {}", msg),
            Style::default(),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Enter to run again, Tab to switch mode, Esc to go back",
        Style::default().fg(rat::DIM),
    )));
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
            "  Enter to try again, Tab to switch mode, Esc to go back",
            Style::default().fg(rat::DIM),
        )),
    ];
    Paragraph::new(lines).render(area, buf);
}

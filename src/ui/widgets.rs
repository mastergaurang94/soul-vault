//! Reusable ratatui widgets for Soma's TUI.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use super::theme::rat;

/// A styled menu item for the interactive menu.
#[allow(dead_code)]
pub struct MenuItem<'a> {
    pub label: &'a str,
    pub description: &'a str,
    pub icon: &'a str,
    pub selected: bool,
}

impl<'a> Widget for MenuItem<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (prefix, label_style, desc_style) = if self.selected {
            (
                "  ▶ ",
                Style::default().fg(rat::PURPLE).add_modifier(Modifier::BOLD),
                Style::default().fg(rat::DIM),
            )
        } else {
            (
                "    ",
                Style::default().fg(ratatui::style::Color::White),
                Style::default().fg(rat::DIM),
            )
        };

        let line = Line::from(vec![
            Span::styled(prefix, label_style),
            Span::styled(format!("{} {}", self.icon, self.label), label_style),
            Span::styled(format!("  {}", self.description), desc_style),
        ]);

        Paragraph::new(line).render(area, buf);
    }
}

/// A themed block with Soma's purple border.
#[allow(dead_code)]
pub fn soma_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(rat::PURPLE))
        .title_style(Style::default().fg(rat::PURPLE).add_modifier(Modifier::BOLD))
        .title(title)
}

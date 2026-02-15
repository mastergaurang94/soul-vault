//! Sidebar navigation widget — page list with selection highlight.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::tui::app::{App, Focus, Page};
use crate::ui::theme::rat;

// ─── Sidebar Widget ───────────────────────────────────────────────────────────

pub struct Sidebar<'a> {
    app: &'a App,
}

impl<'a> Sidebar<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
}

impl Widget for Sidebar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.app.focus == Focus::Sidebar {
            rat::GOLD
        } else {
            rat::DIM
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(" Soul Vault ")
            .title_style(Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD));

        let inner = block.inner(area);
        block.render(area, buf);

        // Render each page as a menu item
        for (i, &page) in Page::ALL.iter().enumerate() {
            if i as u16 >= inner.height {
                break;
            }
            let item_area = Rect {
                x: inner.x,
                y: inner.y + i as u16,
                width: inner.width,
                height: 1,
            };
            render_item(item_area, buf, i + 1, page, i == self.app.sidebar_selected);
        }

        // Footer hint at bottom of sidebar
        let hint_y = inner.y + inner.height.saturating_sub(1);
        if hint_y > inner.y + Page::ALL.len() as u16 {
            let hint = Line::from(Span::styled(" j/k ent q", Style::default().fg(rat::DIM)));
            let hint_area = Rect {
                x: inner.x,
                y: hint_y,
                width: inner.width,
                height: 1,
            };
            Paragraph::new(hint).render(hint_area, buf);
        }
    }
}

// ─── Item Rendering ───────────────────────────────────────────────────────────

fn render_item(area: Rect, buf: &mut Buffer, index: usize, page: Page, selected: bool) {
    let (prefix, style) = if selected {
        (
            " > ",
            Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
        )
    } else {
        ("   ", Style::default().fg(ratatui::style::Color::White))
    };

    let mut spans = vec![
        Span::styled(prefix, style),
        Span::styled(format!("{index}"), style),
        Span::styled(" ", style),
    ];
    let icon = page.icon();
    if !icon.is_empty() {
        spans.push(Span::styled(icon, style));
        spans.push(Span::styled(" ", style));
    }
    spans.push(Span::styled(page.label(), style));
    let line = Line::from(spans);

    Paragraph::new(line).render(area, buf);
}

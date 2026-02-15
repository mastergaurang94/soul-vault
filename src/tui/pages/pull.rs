//! Pull page — discover and import AI sessions from providers.

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

// ─── Pull State ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum PullPhase {
    Ready,
    Running { progress: Vec<String> },
    Done { summary: Vec<String> },
    Error(String),
}

pub struct PullPage {
    pub phase: PullPhase,
}

impl Default for PullPage {
    fn default() -> Self {
        Self {
            phase: PullPhase::Ready,
        }
    }
}

impl PullPage {
    /// Called when pull progress updates arrive.
    pub fn on_progress(&mut self, msg: String) {
        match &mut self.phase {
            PullPhase::Running { progress } => {
                progress.push(msg);
            }
            _ => {
                self.phase = PullPhase::Running {
                    progress: vec![msg],
                };
            }
        }
    }

    /// Called when pull completes.
    pub fn on_done(&mut self, summary: Vec<String>) {
        self.phase = PullPhase::Done { summary };
    }

    /// Called on error.
    pub fn on_error(&mut self, msg: String) {
        self.phase = PullPhase::Error(msg);
    }
}

// ─── PageWidget ───────────────────────────────────────────────────────────────

impl PageWidget for PullPage {
    fn render(&self, area: Rect, buf: &mut Buffer, app: &App) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(rat::GOLD))
            .title(" Pull ")
            .title_style(
                Style::default()
                    .fg(rat::GOLD)
                    .add_modifier(Modifier::BOLD),
            );
        let inner = block.inner(area);
        block.render(area, buf);

        if !app.vault_initialized {
            let msg = Paragraph::new(Span::styled(
                "  Vault not initialized. Run `soul init` first.",
                Style::default().fg(rat::AMBER),
            ));
            msg.render(inner, buf);
            return;
        }

        match &self.phase {
            PullPhase::Ready => render_ready(inner, buf),
            PullPhase::Running { progress } => render_running(inner, buf, progress),
            PullPhase::Done { summary } => render_done(inner, buf, summary),
            PullPhase::Error(msg) => render_error(inner, buf, msg),
        }
    }

    fn handle_key(&mut self, key: KeyEvent, _app: &mut App) -> PageAction {
        match &self.phase {
            PullPhase::Ready => match key.code {
                KeyCode::Enter => PageAction::StartPull,
                KeyCode::Esc => PageAction::BackToSidebar,
                _ => PageAction::Ignored,
            },
            PullPhase::Done { .. } | PullPhase::Error(_) => match key.code {
                KeyCode::Enter | KeyCode::Char('r') => {
                    self.phase = PullPhase::Ready;
                    PageAction::Consumed
                }
                KeyCode::Esc => {
                    self.phase = PullPhase::Ready;
                    PageAction::BackToSidebar
                }
                _ => PageAction::Ignored,
            },
            PullPhase::Running { .. } => PageAction::Ignored,
        }
    }
}

// ─── Render Helpers ───────────────────────────────────────────────────────────

fn render_ready(area: Rect, buf: &mut Buffer) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Pull AI sessions from all providers",
            Style::default()
                .fg(rat::GOLD)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Discovers sessions from:",
            Style::default(),
        )),
        Line::from(Span::styled(
            "    - Claude Code  (~/.claude/projects/)",
            Style::default().fg(rat::DIM),
        )),
        Line::from(Span::styled(
            "    - OpenClaw     (~/.openclaw/agents/)",
            Style::default().fg(rat::DIM),
        )),
        Line::from(Span::styled(
            "    - Gemini CLI   (~/.gemini/tmp/)",
            Style::default().fg(rat::DIM),
        )),
        Line::from(Span::styled(
            "    - Codex        (~/.codex/sessions/)",
            Style::default().fg(rat::DIM),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Press Enter to start, Esc to go back",
            Style::default().fg(rat::DIM),
        )),
    ];
    Paragraph::new(lines).render(area, buf);
}

fn render_running(area: Rect, buf: &mut Buffer, progress: &[String]) {
    let max_lines = area.height.saturating_sub(2) as usize;
    let start = progress.len().saturating_sub(max_lines);

    let mut lines = vec![
        Line::from(Span::styled(
            "  Pulling sessions...",
            Style::default()
                .fg(rat::GOLD)
                .add_modifier(Modifier::BOLD),
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

fn render_done(area: Rect, buf: &mut Buffer, summary: &[String]) {
    let mut lines = vec![
        Line::from(Span::styled(
            "  Pull complete!",
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
        "  Press Enter to pull again, Esc to go back",
        Style::default().fg(rat::DIM),
    )));

    Paragraph::new(lines).render(area, buf);
}

fn render_error(area: Rect, buf: &mut Buffer, msg: &str) {
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

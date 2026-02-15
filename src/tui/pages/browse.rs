//! Browse page — vault file browser with tree navigation and file preview.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use std::fs;
use std::path::PathBuf;

use crate::tui::app::App;
use crate::tui::pages::{PageAction, PageWidget};
use crate::ui::theme::rat;
use crate::vault::config::vault_root;

// ─── Browse State ─────────────────────────────────────────────────────────────

pub struct BrowsePage {
    entries: Vec<BrowseEntry>,
    selected: usize,
    preview_scroll: u16,
    preview_content: Option<String>,
}

struct BrowseEntry {
    display: String,
    path: PathBuf,
    is_dir: bool,
    depth: usize,
}

impl Default for BrowsePage {
    fn default() -> Self {
        let mut page = Self {
            entries: Vec::new(),
            selected: 0,
            preview_scroll: 0,
            preview_content: None,
        };
        page.refresh();
        page
    }
}

impl BrowsePage {
    fn refresh(&mut self) {
        self.entries.clear();
        let root = vault_root();
        if !root.exists() {
            return;
        }
        let dirs = ["identity", "memories", "topics", "people", "sources"];
        for dir_name in &dirs {
            let dir_path = root.join(dir_name);
            self.entries.push(BrowseEntry {
                display: format!("{}/", dir_name),
                path: dir_path.clone(),
                is_dir: true,
                depth: 0,
            });
            if dir_path.exists() {
                if let Ok(mut files) = read_dir_sorted(&dir_path) {
                    for file in files.drain(..) {
                        self.entries.push(BrowseEntry {
                            display: file
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            path: file,
                            is_dir: false,
                            depth: 1,
                        });
                    }
                }
            }
        }
        self.load_preview();
    }

    fn load_preview(&mut self) {
        self.preview_scroll = 0;
        if let Some(entry) = self.entries.get(self.selected) {
            if !entry.is_dir && entry.path.exists() {
                self.preview_content = fs::read_to_string(&entry.path).ok();
            } else {
                self.preview_content = None;
            }
        } else {
            self.preview_content = None;
        }
    }

    fn move_down(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1) % self.entries.len();
            self.load_preview();
        }
    }

    fn move_up(&mut self) {
        if !self.entries.is_empty() {
            if self.selected > 0 {
                self.selected -= 1;
            } else {
                self.selected = self.entries.len() - 1;
            }
            self.load_preview();
        }
    }
}

// ─── PageWidget ───────────────────────────────────────────────────────────────

impl PageWidget for BrowsePage {
    fn render(&self, area: Rect, buf: &mut Buffer, app: &App) {
        if !app.vault_initialized || self.entries.is_empty() {
            render_empty(area, buf, app.vault_initialized);
            return;
        }

        let chunks = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        render_tree(chunks[0], buf, &self.entries, self.selected);
        render_preview(chunks[1], buf, &self.preview_content, self.preview_scroll);
    }

    fn handle_key(&mut self, key: KeyEvent, _app: &mut App) -> PageAction {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_down();
                PageAction::Consumed
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_up();
                PageAction::Consumed
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.preview_scroll = self.preview_scroll.saturating_add(1);
                PageAction::Consumed
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.preview_scroll = self.preview_scroll.saturating_sub(1);
                PageAction::Consumed
            }
            KeyCode::Char('r') => {
                self.refresh();
                PageAction::Consumed
            }
            KeyCode::Esc => PageAction::BackToSidebar,
            _ => PageAction::Ignored,
        }
    }
}

// ─── Rendering ────────────────────────────────────────────────────────────────

fn render_empty(area: Rect, buf: &mut Buffer, initialized: bool) {
    let msg = if initialized {
        "  No vault files found."
    } else {
        "  Vault not initialized. Run `soul init` first."
    };
    Paragraph::new(msg)
        .style(Style::default().fg(rat::DIM))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(rat::GOLD))
                .title(" Browse — Vault "),
        )
        .render(area, buf);
}

fn render_tree(area: Rect, buf: &mut Buffer, entries: &[BrowseEntry], selected: usize) {
    let lines: Vec<Line> = entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let indent = "  ".repeat(entry.depth);
            let icon = if entry.is_dir { "+" } else { " " };
            let text = format!(" {}{} {}", indent, icon, entry.display);

            if i == selected {
                Line::from(Span::styled(
                    text,
                    Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
                ))
            } else if entry.is_dir {
                Line::from(Span::styled(
                    text,
                    Style::default().fg(rat::AMBER).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(text, Style::default()))
            }
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(rat::GOLD))
        .title(" Files ")
        .title_style(Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD));

    Paragraph::new(lines).block(block).render(area, buf);
}

fn render_preview(area: Rect, buf: &mut Buffer, content: &Option<String>, scroll: u16) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(rat::DIM))
        .title(" Preview ")
        .title_style(Style::default().fg(rat::DIM));

    match content {
        Some(text) => {
            let lines: Vec<Line> = text
                .lines()
                .skip(scroll as usize)
                .map(|l| Line::from(Span::raw(format!(" {}", l))))
                .collect();
            Paragraph::new(lines).block(block).render(area, buf);
        }
        None => {
            Paragraph::new(Span::styled(
                "  Select a file to preview",
                Style::default().fg(rat::DIM),
            ))
            .block(block)
            .render(area, buf);
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn read_dir_sorted(dir: &PathBuf) -> std::io::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    Ok(files)
}

//! Status page rendering helpers and dashboard data formatting.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use std::fs;
use std::path::Path;

use crate::ui::theme::{format_bytes, format_time_ago, rat};
use crate::vault::config::vault_root;
use crate::vault::read::get_vault_stats;
use crate::vault::sources::get_source_stats;

pub fn render_not_initialized(area: Rect, buf: &mut Buffer) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Vault not initialized",
            Style::default().fg(rat::AMBER).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Run `soul init` to create your vault,",
            Style::default().fg(rat::DIM),
        )),
        Line::from(Span::styled(
            "  or select Settings to configure.",
            Style::default().fg(rat::DIM),
        )),
    ];
    Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(rat::AMBER))
                .title(" Status — Overview ")
                .title_style(Style::default().fg(rat::AMBER)),
        )
        .render(area, buf);
}

pub fn render_dashboard(area: Rect, buf: &mut Buffer, scroll: u16) {
    let stats = match get_vault_stats() {
        Ok(s) => s,
        Err(_) => {
            Paragraph::new("  Failed to read vault stats.")
                .style(Style::default().fg(rat::RED))
                .render(area, buf);
            return;
        }
    };

    let sources = get_source_stats().unwrap_or_default();
    let root = vault_root();
    let vault_size = compute_dir_size(&root);
    let file_count = count_files(&root);
    let last_activity = stats
        .last_sync
        .as_deref()
        .map(format_time_ago)
        .unwrap_or_else(|| "never".into());

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "  Vault Overview",
            Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        stat_line("  Memories", stats.memory_count.to_string()),
        stat_line("  Topics", stats.topic_count.to_string()),
        stat_line("  People", stats.people_count.to_string()),
        stat_line("  Vault size", format_bytes(vault_size)),
        stat_line("  Total files", file_count.to_string()),
        stat_line("  Last activity", last_activity),
        Line::from(""),
    ];

    lines.push(Line::from(Span::styled(
        "  Providers",
        Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    for p in &stats.providers {
        let (icon, icon_color) = if p.connected {
            ("+", rat::EMERALD)
        } else {
            ("-", rat::DIM)
        };
        let status_text = if p.connected {
            p.last_import
                .as_deref()
                .map(|lp| format!("last: {}", format_time_ago(lp)))
                .unwrap_or_else(|| "no imports yet".into())
        } else {
            "not connected".into()
        };
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(icon, Style::default().fg(icon_color)),
            Span::raw(" "),
            Span::styled(
                format!("{:<14}", p.name.display_name()),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(status_text, Style::default().fg(rat::DIM)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Imported Sources",
        Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    if sources.is_empty() {
        lines.push(Line::from(Span::styled(
            "    No sources imported yet.",
            Style::default().fg(rat::DIM),
        )));
        lines.push(Line::from(Span::styled(
            "    Use Import to get started.",
            Style::default().fg(rat::DIM),
        )));
    } else {
        for s in &sources {
            let display = truncate_path(&s.path, 42);
            let last = format_time_ago(&s.last_ingested);
            lines.push(Line::from(Span::styled(
                format!("    {}", display),
                Style::default().fg(rat::CYAN),
            )));
            lines.push(Line::from(Span::styled(
                format!("      {} files, last: {}", s.files_ingested, last),
                Style::default().fg(rat::DIM),
            )));
        }
    }

    let visible: Vec<Line> = lines.into_iter().skip(scroll as usize).collect();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(rat::GOLD))
        .title(" Status — Overview ")
        .title_style(Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD));
    Paragraph::new(visible).block(block).render(area, buf);
}

fn stat_line(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<18}", label), Style::default().fg(rat::DIM)),
        Span::styled(value, Style::default().add_modifier(Modifier::BOLD)),
    ])
}

fn truncate_path(path: &str, max: usize) -> String {
    if path.len() <= max {
        return path.to_string();
    }
    let home = dirs::home_dir()
        .map(|h| h.display().to_string())
        .unwrap_or_default();
    let display = if path.starts_with(&home) {
        format!("~{}", &path[home.len()..])
    } else {
        path.to_string()
    };
    if display.len() <= max {
        return display;
    }
    format!("...{}", &display[display.len() - (max - 3)..])
}

fn compute_dir_size(dir: &Path) -> u64 {
    if !dir.exists() {
        return 0;
    }
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                total += compute_dir_size(&path);
            } else if path.is_file() {
                total += fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}

fn count_files(dir: &Path) -> usize {
    if !dir.exists() {
        return 0;
    }
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += count_files(&path);
            } else if path.is_file() {
                count += 1;
            }
        }
    }
    count
}

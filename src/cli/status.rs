//! `soul status` — displays vault summary with source tracking and provider status.

use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::ui::theme::*;
use crate::vault::config::{assert_initialized, vault_root};
use crate::vault::read::get_vault_stats;
use crate::vault::sources::get_source_stats;

// ─── Constants ────────────────────────────────────────────────────────────────

/// Inner content width (between the │ borders). Total line = "  │" + content + "│"
const INNER_WIDTH: usize = 50;

// ─── Status Command ───────────────────────────────────────────────────────────

pub fn run() -> Result<()> {
    println!("{}", banner());
    assert_initialized()?;

    let stats = get_vault_stats()?;
    let sources = get_source_stats().unwrap_or_default();

    // ─── Header ───────────────────────────────────────────────────────────
    println!(
        "  {} {} {}",
        bold_gold("Soul Vault"),
        amber(ICON_STAR),
        dim(&stats.vault_path)
    );
    println!();

    // ─── Vault Overview Box ───────────────────────────────────────────────
    print_box_top();
    print_box_header("Vault Overview");
    print_box_sep();

    let vault_size = compute_vault_size(&vault_root());
    let vault_file_count = count_all_files(&vault_root());

    print_stat_row("Memories", &stats.memory_count.to_string());
    print_stat_row("Topics", &stats.topic_count.to_string());
    print_stat_row("People", &stats.people_count.to_string());
    print_stat_row("Vault size", &format_bytes(vault_size));
    print_stat_row("Total files", &vault_file_count.to_string());

    let last_sync_display = match &stats.last_sync {
        Some(ls) => format_time_ago(ls),
        None => "never".to_string(),
    };
    print_stat_row("Last activity", &last_sync_display);

    print_box_bottom();
    println!();

    // ─── Providers Box ────────────────────────────────────────────────────
    print_box_top();
    print_box_header("Providers");
    print_box_sep();

    for p in &stats.providers {
        let name = p.name.display_name();
        let (icon, status_text) = if p.connected {
            let pull_info = match &p.last_pull {
                Some(lp) => format!("last: {}", format_time_ago(lp)),
                None => "no pulls yet".to_string(),
            };
            (emerald(ICON_CHECK), pull_info)
        } else {
            (dim(ICON_DOT), "not connected".to_string())
        };

        // Build the visible content: "  ✓ Claude         no pulls yet"
        // icon(1) + space(1) + name(padded to 14) + status
        let visible_content = format!("  {} {:<14}{}", "X", name, &status_text);
        let vis_len = visible_content.len(); // no ANSI here, plain measurement

        // Now build with colors
        let colored_content = format!("  {} {:<14}{}", icon, name, dim(&status_text));
        let pad = if vis_len < INNER_WIDTH {
            INNER_WIDTH - vis_len
        } else {
            1
        };
        println!("  │{}{}│", colored_content, " ".repeat(pad));
    }

    print_box_bottom();
    println!();

    // ─── Imported Sources Box ─────────────────────────────────────────────
    if !sources.is_empty() {
        print_box_top();
        print_box_header("Imported Sources");
        print_box_sep();

        for source in &sources {
            let display_path = truncate_path(&source.path, 40);
            let last = format_time_ago(&source.last_ingested);
            print_content_row(&format!("  {}", &display_path), Some(&cyan));
            print_content_row(
                &format!(
                    "    {} files, last: {}",
                    source.files_ingested, &last
                ),
                Some(&dim),
            );
        }

        print_box_bottom();
    } else {
        println!("  {}", dim("No sources imported yet."));
        println!(
            "  {} {} {}",
            dim("Run"),
            cyan("soul import <folder>"),
            dim("to get started.")
        );
    }

    println!();

    Ok(())
}

// ─── Box Drawing ──────────────────────────────────────────────────────────────

fn print_box_top() {
    println!("  ┌{}┐", "─".repeat(INNER_WIDTH));
}

fn print_box_bottom() {
    println!("  └{}┘", "─".repeat(INNER_WIDTH));
}

fn print_box_sep() {
    println!("  ├{}┤", "─".repeat(INNER_WIDTH));
}

fn print_box_header(title: &str) {
    // Simple header: "  │  TITLE                           │"
    // No emoji — they cause inconsistent terminal widths across terminals.
    // Visible: 2 (indent) + title.len()
    let visible_used = 2 + title.len();
    let pad = if visible_used < INNER_WIDTH {
        INNER_WIDTH - visible_used
    } else {
        1
    };
    println!("  │  {}{}│", bold_white(title), " ".repeat(pad));
}

fn print_stat_row(label: &str, value: &str) {
    // Layout: "    Label:         Value"
    // 4 spaces indent + label + ":" + padding to col 20 + value
    let label_with_colon = format!("{}:", label);
    let visible = format!("    {:<16}{}", label_with_colon, value);
    let vis_len = visible.len();
    let pad = if vis_len < INNER_WIDTH {
        INNER_WIDTH - vis_len
    } else {
        1
    };
    // Apply colors: label dim, value bold
    println!(
        "  │    {:<16}{}{}│",
        dim(&label_with_colon),
        bold_white(value),
        " ".repeat(pad)
    );
}

fn print_content_row(text: &str, color_fn: Option<&dyn Fn(&str) -> String>) {
    let vis_len = text.len();
    let colored = match color_fn {
        Some(f) => f(text),
        None => text.to_string(),
    };
    let pad = if vis_len < INNER_WIDTH {
        INNER_WIDTH - vis_len
    } else {
        1
    };
    println!("  │{}{}│", colored, " ".repeat(pad));
}

// ─── Path & Size Helpers ──────────────────────────────────────────────────────

fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
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

    if display.len() <= max_len {
        return display;
    }
    format!("...{}", &display[display.len() - (max_len - 3)..])
}

fn compute_vault_size(root: &Path) -> u64 {
    if !root.exists() {
        return 0;
    }
    walkdir_size(root)
}

fn walkdir_size(dir: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                total += walkdir_size(&path);
            } else if path.is_file() {
                total += fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}

fn count_all_files(root: &Path) -> usize {
    if !root.exists() {
        return 0;
    }
    walkdir_count(root)
}

fn walkdir_count(dir: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += walkdir_count(&path);
            } else if path.is_file() {
                count += 1;
            }
        }
    }
    count
}

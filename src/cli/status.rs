//! `soul status` — displays vault summary with source tracking and provider status.

use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::ui::theme::*;
use crate::vault::config::{
    assert_initialized, get_api_key, get_key_health, read_config, vault_root, ApiKeyHealth,
};
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
    let config = read_config()?;
    let sources = get_source_stats().unwrap_or_default();

    // ─── Header ───────────────────────────────────────────────────────────
    println!(
        "  {} {} {}",
        bold_gold("Soul Vault"),
        amber(ICON_STAR),
        dim(&vault_root().display().to_string())
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
    print_stat_row("Processing", config.processing_mode.display_name());

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
        let (icon, status_text, status_colored) =
            provider_line_state(p.connected, &p.name, &p.last_import);

        // Build the visible content: "  ✓ Claude         no imports yet"
        // icon(1) + space(1) + name(padded to 14) + status
        let visible_content = format!("  {} {:<14}{}", "X", name, status_text);
        let vis_len = visible_len(&visible_content); // no ANSI here, plain measurement

        // Now build with colors
        let colored_content = format!("  {} {:<14}{}", icon, name, status_colored);
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
                &format!("    {} files, last: {}", source.files_ingested, &last),
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

fn provider_line_state(
    enabled: bool,
    provider: &crate::types::Provider,
    last_import: &Option<String>,
) -> (String, String, String) {
    if !enabled {
        let status = "disabled".to_string();
        return (dim(ICON_DOT), status.clone(), dim(&status));
    }

    let has_key = get_api_key(&provider.to_string())
        .ok()
        .flatten()
        .map(|k| !k.trim().is_empty())
        .unwrap_or(false);
    if !has_key {
        return not_connected();
    }

    match get_key_health(provider).ok().flatten().map(|r| r.status) {
        Some(ApiKeyHealth::Verified) => {
            let status = match last_import {
                Some(lp) => format!("last: {}", format_time_ago(lp)),
                None => "no imports yet".to_string(),
            };
            (emerald(ICON_CHECK), status.clone(), dim(&status))
        }
        Some(ApiKeyHealth::Unverified) | Some(ApiKeyHealth::Invalid) => not_connected(),
        None => not_connected(),
    }
}

fn not_connected() -> (String, String, String) {
    let status = "not connected".to_string();
    (dim(ICON_DOT), status.clone(), dim(&status))
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
    let visible_used = 2 + visible_len(title);
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
    let vis_len = visible_len(&visible);
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
    let vis_len = visible_len(text);
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

fn visible_len(s: &str) -> usize {
    s.chars().count()
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

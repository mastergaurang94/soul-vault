//! `soma status` — displays vault summary with source tracking and provider status.

use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::ui::theme::*;
use crate::vault::config::{assert_initialized, vault_root};
use crate::vault::read::get_vault_stats;
use crate::vault::sources::get_source_stats;

// ─── Status Command ───────────────────────────────────────────────────────────

pub fn run() -> Result<()> {
    println!("{}", banner());
    assert_initialized()?;

    let stats = get_vault_stats()?;
    let sources = get_source_stats().unwrap_or_default();

    // ─── Header ───────────────────────────────────────────────────────────
    println!(
        "  {} {} {}",
        bold_purple("Soma Vault"),
        amber(ICON_STAR),
        dim(&stats.vault_path)
    );
    println!();

    // ─── Vault Overview Box ───────────────────────────────────────────────
    println!("  {}", dim("┌─────────────────────────────────────────────────────┐"));
    println!(
        "  {}  {}{}  {}",
        dim("│"),
        purple("📊"),
        bold_white("  Vault Overview"),
        dim(&format!(
            "{}│",
            " ".repeat(35 - "  Vault Overview".len())
        ))
    );
    println!("  {}", dim("├─────────────────────────────────────────────────────┤"));

    // Stats rows
    let vault_size = compute_vault_size(&vault_root());
    let vault_file_count = count_all_files(&vault_root());

    print_box_row("Memories", &stats.memory_count.to_string(), false);
    print_box_row("Topics", &stats.topic_count.to_string(), false);
    print_box_row("People", &stats.people_count.to_string(), false);
    print_box_row("Vault size", &format_bytes(vault_size), false);
    print_box_row("Total files", &vault_file_count.to_string(), false);

    let last_sync_display = match &stats.last_sync {
        Some(ls) => format_time_ago(ls),
        None => dim("never"),
    };
    print_box_row("Last activity", &last_sync_display, false);

    println!("  {}", dim("└─────────────────────────────────────────────────────┘"));
    println!();

    // ─── Providers Box ────────────────────────────────────────────────────
    println!("  {}", dim("┌─────────────────────────────────────────────────────┐"));
    println!(
        "  {}  {}{}  {}",
        dim("│"),
        purple("🔌"),
        bold_white("  Providers"),
        dim(&format!(
            "{}│",
            " ".repeat(35 - "  Providers".len())
        ))
    );
    println!("  {}", dim("├─────────────────────────────────────────────────────┤"));

    for p in &stats.providers {
        let name = p.name.display_name();
        let (icon, status) = if p.connected {
            let pull_info = match &p.last_pull {
                Some(lp) => format!("last: {}", format_time_ago(lp)),
                None => "no pulls yet".to_string(),
            };
            (emerald(ICON_CHECK), dim(&pull_info))
        } else {
            (dim(ICON_DOT), dim("not connected"))
        };
        println!(
            "  {}    {} {:<14}{}{}",
            dim("│"),
            icon,
            bold_white(name),
            status,
            pad_to_box_end(&format!("    {} {:<14}{}", icon, name, &status_stripped_len(&status)), 53)
        );
    }

    println!("  {}", dim("└─────────────────────────────────────────────────────┘"));
    println!();

    // ─── Ingested Sources Box ─────────────────────────────────────────────
    if !sources.is_empty() {
        println!("  {}", dim("┌─────────────────────────────────────────────────────┐"));
        println!(
            "  {}  {}{}  {}",
            dim("│"),
            purple("📁"),
            bold_white("  Ingested Sources"),
            dim(&format!(
                "{}│",
                " ".repeat(35 - "  Ingested Sources".len())
            ))
        );
        println!("  {}", dim("├─────────────────────────────────────────────────────┤"));

        for source in &sources {
            // Truncate long paths
            let display_path = truncate_path(&source.path, 40);
            let last = format_time_ago(&source.last_ingested);
            println!(
                "  {}    {} {}",
                dim("│"),
                cyan(&display_path),
                dim("│")
            );
            println!(
                "  {}      {} files, last: {}{}",
                dim("│"),
                bold_white(&source.files_ingested.to_string()),
                dim(&last),
                dim(" │")
            );
        }

        println!("  {}", dim("└─────────────────────────────────────────────────────┘"));
    } else {
        println!("  {}", dim("No sources ingested yet."));
        println!(
            "  {} {} {}",
            dim("Run"),
            cyan("soma ingest <folder>"),
            dim("to get started.")
        );
    }

    println!();

    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn print_box_row(label: &str, value: &str, _highlight: bool) {
    println!(
        "  {}    {:<16}{}{}",
        dim("│"),
        dim(&format!("{}:", label)),
        bold_white(value),
        dim(" │")
    );
}

fn pad_to_box_end(_content: &str, _width: usize) -> String {
    // For the provider rows, we just close the box
    dim(" │").to_string()
}

fn status_stripped_len(s: &str) -> String {
    // Strip ANSI codes for length calculation (approximate)
    let _ = s;
    String::new()
}

fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }
    // Show ~/... prefix
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

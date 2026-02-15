//! `soul watch [folder]` — file watcher that auto-ingests on changes.
//!
//! When no folder is given, auto-discovers provider session directories
//! using the adapter registry.

use anyhow::Result;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::io::IsTerminal;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use crate::adapters::AdapterRegistry;
use crate::cli::watch_events::{
    collect_supported_changed_files, process_auto_changes, process_folder_changes,
};
use crate::cli::watch_validate::validate_auto_watch_prereqs;
use crate::ui::theme::*;
use crate::vault::config::{assert_initialized, assert_path_exists};

// ─── Watch Command ────────────────────────────────────────────────────────────

pub async fn run(folder_path: &str) -> Result<()> {
    println!("{}", banner());
    assert_initialized()?;

    let abs_path =
        std::fs::canonicalize(folder_path).unwrap_or_else(|_| Path::new(folder_path).to_path_buf());
    assert_path_exists(&abs_path)?;

    println!(
        "  👁  Watching {} for changes\n",
        cyan(&abs_path.display().to_string())
    );
    println!("{}", line());
    println!("  {} {}", dim("Press"), bold_white("Ctrl+C to stop"));
    println!();

    let (tx, rx) = mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_secs(2), tx)?;
    debouncer
        .watcher()
        .watch(&abs_path, RecursiveMode::Recursive)?;

    println!(
        "{}",
        check(&format!(
            "Watching {} — waiting for changes...",
            dim(&abs_path.display().to_string())
        ))
    );
    println!();

    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                let changed_files = collect_supported_changed_files(&events);
                process_folder_changes(&abs_path, changed_files).await?;
            }
            Ok(Err(error)) => {
                eprintln!("  {} Watch error: {}", red(ICON_CROSS), error);
            }
            Err(e) => {
                eprintln!("  {} Channel error: {}", red(ICON_CROSS), e);
                break;
            }
        }
    }

    Ok(())
}

// ─── Auto-Watch (no args) ─────────────────────────────────────────────────────

/// Watch all provider session directories automatically.
pub async fn run_auto() -> Result<()> {
    let registry = AdapterRegistry::new();
    let base_dirs = registry.base_dirs();
    validate_auto_watch_prereqs(std::io::stdin().is_terminal(), &base_dirs)?;

    println!("{}", banner());
    assert_initialized()?;

    println!("  👁  Auto-watching AI provider directories\n");
    println!("{}", line());

    for (name, dir) in &base_dirs {
        println!(
            "  {} {} {}",
            emerald(ICON_CHECK),
            bold_white(name),
            dim(&dir.display().to_string())
        );
    }

    println!("\n  {} {}", dim("Press"), bold_white("Ctrl+C to stop"));
    println!();

    let (tx, rx) = mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_secs(2), tx)?;

    for (name, dir) in &base_dirs {
        match debouncer.watcher().watch(dir, RecursiveMode::Recursive) {
            Ok(()) => {
                println!(
                    "{}",
                    check(&format!(
                        "Watching {} — {}",
                        name,
                        dim(&dir.display().to_string())
                    ))
                );
            }
            Err(e) => {
                println!("  {} Failed to watch {}: {}", red(ICON_CROSS), name, e);
            }
        }
    }
    println!();

    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                let changed_files = collect_supported_changed_files(&events);
                process_auto_changes(&registry, &base_dirs, changed_files).await?;
            }
            Ok(Err(error)) => {
                eprintln!("  {} Watch error: {}", red(ICON_CROSS), error);
            }
            Err(e) => {
                eprintln!("  {} Channel error: {}", red(ICON_CROSS), e);
                break;
            }
        }
    }

    Ok(())
}

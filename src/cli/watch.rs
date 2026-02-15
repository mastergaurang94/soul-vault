//! `soul watch [folder]` — file watcher that auto-ingests on changes.
//!
//! When no folder is given, auto-discovers provider session directories
//! using the adapter registry.

use anyhow::Result;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use crate::adapters::AdapterRegistry;
use crate::ui::theme::*;
use crate::vault::config::{assert_initialized, assert_path_exists};
use crate::vault::local::discover_files;
use crate::vault::sources::{classify_files, update_source_tracking};

// ─── Supported Extensions ─────────────────────────────────────────────────────

const SUPPORTED_EXTENSIONS: &[&str] = &["md", "txt", "json", "jsonl"];

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

    // Set up file watcher with debounce
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

    // Main event loop
    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                // Collect unique changed files that are supported
                let changed_files: HashSet<_> = events
                    .iter()
                    .filter_map(|event| {
                        let path = &event.path;
                        if path.is_file() {
                            let ext = path
                                .extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("")
                                .to_lowercase();
                            if SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                                // Skip hidden files and .config
                                let path_str = path.to_string_lossy();
                                if !path_str.contains("/.") && !path_str.contains("\\.") {
                                    return Some(path.clone());
                                }
                            }
                        }
                        None
                    })
                    .collect();

                if changed_files.is_empty() {
                    continue;
                }

                let now = chrono::Local::now().format("%H:%M:%S").to_string();
                println!(
                    "  {} {} {} changed",
                    dim(&now),
                    amber("⟳"),
                    bold_white(&changed_files.len().to_string())
                );

                // Classify changes
                let file_paths: Vec<_> = changed_files.into_iter().collect();
                match classify_files(&abs_path, &file_paths) {
                    Ok(classification) => {
                        let to_ingest = classification.all_to_ingest();
                        let skipped = classification.skipped_files.len();

                        if to_ingest.is_empty() {
                            println!(
                                "  {}   {} unchanged — skipped",
                                dim(&" ".repeat(8)),
                                dim(&skipped.to_string())
                            );
                            continue;
                        }

                        // Build FileInfo for changed files
                        let all_files = discover_files(&abs_path)?;
                        let files_to_ingest: Vec<_> = all_files
                            .into_iter()
                            .filter(|f| to_ingest.contains(&f.path))
                            .collect();

                        if files_to_ingest.is_empty() {
                            continue;
                        }

                        for f in &files_to_ingest {
                            let rel = f.path.strip_prefix(&abs_path).unwrap_or(&f.path).display();
                            println!(
                                "  {}   {} {}",
                                dim(&" ".repeat(8)),
                                emerald(ICON_CHECK),
                                dim(&rel.to_string())
                            );
                        }

                        // Run import
                        match crate::cli::ingest::run_for_files(&abs_path, &files_to_ingest).await {
                            Ok(()) => {
                                println!(
                                    "  {}   {} imported, {} skipped",
                                    dim(&" ".repeat(8)),
                                    bold_white(&files_to_ingest.len().to_string()),
                                    dim(&skipped.to_string())
                                );
                            }
                            Err(e) => {
                                println!(
                                    "  {}   {} {}",
                                    dim(&" ".repeat(8)),
                                    red(ICON_CROSS),
                                    red(&e.to_string())
                                );
                            }
                        }
                    }
                    Err(e) => {
                        // Fall back to simple hash check + ingest
                        println!(
                            "  {}   {} Classification error: {}",
                            dim(&" ".repeat(8)),
                            amber("⚠"),
                            e
                        );
                    }
                }

                // Update source tracking for all files
                if let Ok(all_files) = discover_files(&abs_path) {
                    let all_paths: Vec<_> = all_files.iter().map(|f| f.path.clone()).collect();
                    let _ = update_source_tracking(&abs_path, &all_paths);
                }

                println!();
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

    // Set up file watcher on all provider directories
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

    // Main event loop — same pattern as run()
    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                let changed_files: HashSet<_> = events
                    .iter()
                    .filter_map(|event| {
                        let path = &event.path;
                        if path.is_file() {
                            let ext = path
                                .extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("")
                                .to_lowercase();
                            if SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                                let path_str = path.to_string_lossy();
                                if !path_str.contains("/.") && !path_str.contains("\\.") {
                                    return Some(path.clone());
                                }
                            }
                        }
                        None
                    })
                    .collect();

                if changed_files.is_empty() {
                    continue;
                }

                let now = chrono::Local::now().format("%H:%M:%S").to_string();
                println!(
                    "  {} {} {} changed",
                    dim(&now),
                    amber("⟳"),
                    bold_white(&changed_files.len().to_string())
                );

                // For auto-watch, use adapters to parse and import
                let file_paths: Vec<_> = changed_files.into_iter().collect();
                for file_path in &file_paths {
                    if let Some(adapter) = registry.auto_detect(file_path) {
                        let rel = file_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        println!(
                            "  {}   {} {} ({})",
                            dim(&" ".repeat(8)),
                            emerald(ICON_CHECK),
                            dim(&rel),
                            adapter.display_name()
                        );
                    }
                }

                // Find a common base for source tracking
                for (_, dir) in &base_dirs {
                    let relevant: Vec<_> = file_paths
                        .iter()
                        .filter(|f| f.starts_with(dir))
                        .cloned()
                        .collect();

                    if relevant.is_empty() {
                        continue;
                    }

                    match classify_files(dir, &relevant) {
                        Ok(classification) => {
                            let to_ingest = classification.all_to_ingest();
                            if to_ingest.is_empty() {
                                continue;
                            }

                            let all_files = match discover_files(dir) {
                                Ok(f) => f,
                                Err(_) => continue,
                            };
                            let files_to_ingest: Vec<_> = all_files
                                .into_iter()
                                .filter(|f| to_ingest.contains(&f.path))
                                .collect();

                            if files_to_ingest.is_empty() {
                                continue;
                            }

                            match crate::cli::ingest::run_for_files(dir, &files_to_ingest).await {
                                Ok(()) => {
                                    println!(
                                        "  {}   {} imported",
                                        dim(&" ".repeat(8)),
                                        bold_white(&files_to_ingest.len().to_string())
                                    );
                                }
                                Err(e) => {
                                    println!(
                                        "  {}   {} {}",
                                        dim(&" ".repeat(8)),
                                        red(ICON_CROSS),
                                        red(&e.to_string())
                                    );
                                }
                            }

                            if let Ok(all) = discover_files(dir) {
                                let paths: Vec<_> = all.iter().map(|f| f.path.clone()).collect();
                                let _ = update_source_tracking(dir, &paths);
                            }
                        }
                        Err(e) => {
                            println!("  {}   {} {}", dim(&" ".repeat(8)), amber("⚠"), e);
                        }
                    }
                }

                println!();
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

fn validate_auto_watch_prereqs(is_tty: bool, base_dirs: &[(String, PathBuf)]) -> Result<()> {
    if !is_tty {
        anyhow::bail!(
            "Auto-watch requires a terminal.\n      \
             → Usage: soul watch <folder>\n      \
             → Or run `soul import` for one-time provider import."
        );
    }

    if base_dirs.is_empty() {
        anyhow::bail!(
            "No provider directories found.\n      \
             → Looked for: ~/.claude/projects/, ~/.openclaw/agents/, ~/.gemini/tmp/, ~/.codex/sessions/\n      \
             → You can also specify a folder: soul watch <folder>"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_auto_watch_requires_tty() {
        let dirs = vec![("Claude Code".to_string(), PathBuf::from("/tmp"))];
        let err = validate_auto_watch_prereqs(false, &dirs).unwrap_err();
        assert!(err.to_string().contains("Auto-watch requires a terminal"));
    }

    #[test]
    fn validate_auto_watch_requires_provider_dirs() {
        let err = validate_auto_watch_prereqs(true, &[]).unwrap_err();
        assert!(err.to_string().contains("No provider directories found"));
    }

    #[test]
    fn validate_auto_watch_ok_when_tty_and_dirs_present() {
        let dirs = vec![("Codex".to_string(), PathBuf::from("/tmp"))];
        assert!(validate_auto_watch_prereqs(true, &dirs).is_ok());
    }
}

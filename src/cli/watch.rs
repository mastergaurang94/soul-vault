//! `soma watch <folder>` — file watcher that auto-ingests on changes.

use anyhow::Result;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::collections::HashSet;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use crate::extractors::local::discover_files;
use crate::ui::theme::*;
use crate::vault::config::{assert_initialized, assert_path_exists};
use crate::vault::sources::{classify_files, update_source_tracking};

// ─── Supported Extensions ─────────────────────────────────────────────────────

const SUPPORTED_EXTENSIONS: &[&str] = &["md", "txt", "json", "jsonl"];

// ─── Watch Command ────────────────────────────────────────────────────────────

pub async fn run(folder_path: &str) -> Result<()> {
    println!("{}", banner());
    assert_initialized()?;

    let abs_path = std::fs::canonicalize(folder_path)
        .unwrap_or_else(|_| Path::new(folder_path).to_path_buf());
    assert_path_exists(&abs_path)?;

    println!(
        "  👁 Watching {} for changes\n",
        cyan(&abs_path.display().to_string())
    );
    println!("{}", line());
    println!(
        "  {} {}",
        dim("Press"),
        bold_white("Ctrl+C to stop")
    );
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
                            let rel = f
                                .path
                                .strip_prefix(&abs_path)
                                .unwrap_or(&f.path)
                                .display();
                            println!(
                                "  {}   {} {}",
                                dim(&" ".repeat(8)),
                                emerald(ICON_CHECK),
                                dim(&rel.to_string())
                            );
                        }

                        // Run ingestion
                        match crate::cli::ingest::run_for_files(&abs_path, &files_to_ingest).await
                        {
                            Ok(()) => {
                                println!(
                                    "  {}   {} ingested, {} skipped",
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
                eprintln!(
                    "  {} Watch error: {}",
                    red(ICON_CROSS),
                    error
                );
            }
            Err(e) => {
                eprintln!("  {} Channel error: {}", red(ICON_CROSS), e);
                break;
            }
        }
    }

    Ok(())
}

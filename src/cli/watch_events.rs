//! Event filtering and change-processing helpers for watch mode.

use anyhow::Result;
use notify_debouncer_mini::DebouncedEvent;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::adapters::AdapterRegistry;
use crate::ui::theme::*;
use crate::vault::local::discover_files;
use crate::vault::sources::{classify_files, update_source_tracking};

const SUPPORTED_EXTENSIONS: &[&str] = &["md", "txt", "json", "jsonl"];

pub(crate) fn collect_supported_changed_files(events: &[DebouncedEvent]) -> HashSet<PathBuf> {
    events
        .iter()
        .filter_map(|event| {
            let path = &event.path;
            if !path.is_file() {
                return None;
            }

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                return None;
            }

            let path_str = path.to_string_lossy();
            if path_str.contains("/.") || path_str.contains("\\.") {
                return None;
            }

            Some(path.clone())
        })
        .collect()
}

pub(crate) async fn process_folder_changes(
    abs_path: &Path,
    changed_files: HashSet<PathBuf>,
) -> Result<()> {
    if changed_files.is_empty() {
        return Ok(());
    }

    let now = chrono::Local::now().format("%H:%M:%S").to_string();
    println!(
        "  {} {} {} changed",
        dim(&now),
        amber("⟳"),
        bold_white(&changed_files.len().to_string())
    );

    let file_paths: Vec<_> = changed_files.into_iter().collect();
    match classify_files(abs_path, &file_paths) {
        Ok(classification) => {
            let to_ingest = classification.all_to_ingest();
            let skipped = classification.skipped_files.len();

            if to_ingest.is_empty() {
                println!(
                    "  {}   {} unchanged — skipped",
                    dim(&" ".repeat(8)),
                    dim(&skipped.to_string())
                );
                return Ok(());
            }

            let all_files = discover_files(abs_path)?;
            let files_to_ingest: Vec<_> = all_files
                .into_iter()
                .filter(|f| to_ingest.contains(&f.path))
                .collect();

            if files_to_ingest.is_empty() {
                return Ok(());
            }

            for f in &files_to_ingest {
                let rel = f.path.strip_prefix(abs_path).unwrap_or(&f.path).display();
                println!(
                    "  {}   {} {}",
                    dim(&" ".repeat(8)),
                    emerald(ICON_CHECK),
                    dim(&rel.to_string())
                );
            }

            if run_ingest(abs_path, &files_to_ingest).await {
                println!(
                    "  {}   {} imported, {} skipped",
                    dim(&" ".repeat(8)),
                    bold_white(&files_to_ingest.len().to_string()),
                    dim(&skipped.to_string())
                );
            }
        }
        Err(e) => {
            println!(
                "  {}   {} Classification error: {}",
                dim(&" ".repeat(8)),
                amber("⚠"),
                e
            );
        }
    }

    refresh_source_tracking(abs_path);

    println!();
    Ok(())
}

pub(crate) async fn process_auto_changes(
    registry: &AdapterRegistry,
    base_dirs: &[(String, PathBuf)],
    changed_files: HashSet<PathBuf>,
) -> Result<()> {
    if changed_files.is_empty() {
        return Ok(());
    }

    let now = chrono::Local::now().format("%H:%M:%S").to_string();
    println!(
        "  {} {} {} changed",
        dim(&now),
        amber("⟳"),
        bold_white(&changed_files.len().to_string())
    );

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

    for (_, dir) in base_dirs {
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

                if run_ingest(dir, &files_to_ingest).await {
                    println!(
                        "  {}   {} imported",
                        dim(&" ".repeat(8)),
                        bold_white(&files_to_ingest.len().to_string())
                    );
                    refresh_source_tracking(dir);
                }
            }
            Err(e) => {
                println!("  {}   {} {}", dim(&" ".repeat(8)), amber("⚠"), e);
            }
        }
    }

    println!();
    Ok(())
}

async fn run_ingest(base_dir: &Path, files: &[crate::types::FileInfo]) -> bool {
    match crate::cli::ingest::run_for_files(base_dir, files).await {
        Ok(()) => true,
        Err(e) => {
            println!(
                "  {}   {} {}",
                dim(&" ".repeat(8)),
                red(ICON_CROSS),
                red(&e.to_string())
            );
            false
        }
    }
}

fn refresh_source_tracking(base_dir: &Path) {
    if let Ok(all_files) = discover_files(base_dir) {
        let all_paths: Vec<_> = all_files.iter().map(|f| f.path.clone()).collect();
        let _ = update_source_tracking(base_dir, &all_paths);
    }
}

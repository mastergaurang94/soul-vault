//! Local folder import implementation for `soul import <folder>`.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::cli::ingest_process::{process_all_chunks, spinner};
use crate::cli::ingest_scan::{classify_and_filter, read_and_chunk, scan_files};
use crate::cli::ingest_summary::print_summary;
use crate::core::merger::merge_all_memories;
use crate::types::FileInfo;
use crate::ui::theme::*;
use crate::vault::config::{assert_initialized, assert_path_exists, processing_enabled};
use crate::vault::sources::update_source_tracking;
use crate::vault::write::write_memories_to_vault;

// ─── Ingest Command ───────────────────────────────────────────────────────────

pub async fn run(folder_path: &str, force: bool) -> Result<()> {
    println!("{}", banner());
    assert_initialized()?;

    let abs_path =
        std::fs::canonicalize(folder_path).unwrap_or_else(|_| Path::new(folder_path).to_path_buf());
    assert_path_exists(&abs_path)?;

    println!(
        "  {} Importing from {}\n",
        ICON_FOLDER,
        cyan(&abs_path.display().to_string())
    );
    println!("{}", line());

    let files = scan_files(&abs_path)?;
    let (files_to_ingest, new_count, modified_count, skipped_count) =
        classify_and_filter(&abs_path, &files, force)?;

    if files_to_ingest.is_empty() {
        println!(
            "\n{}",
            check(&format!(
                "All {} files unchanged. Nothing to import.",
                skipped_count
            ))
        );
        println!(
            "\n  {} {} {}",
            dim("Use"),
            cyan("soul import --force <folder>"),
            dim("to re-import everything.")
        );
        println!();
        return Ok(());
    }

    let all_chunks = read_and_chunk(&files_to_ingest)?;
    if !processing_enabled()? {
        println!(
            "\n  {} Processing disabled. Imported raw sessions only (no memory extraction).",
            amber(ICON_STAR)
        );
        let pb = spinner("Updating source tracking...");
        let all_file_paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();
        match update_source_tracking(&abs_path, &all_file_paths) {
            Ok(()) => pb.finish_with_message(check("Source tracking updated")),
            Err(e) => {
                pb.finish_with_message(amber("Source tracking skipped"));
                eprintln!(
                    "{}",
                    amber(&format!("  ⚠ Could not update source tracking: {}", e))
                );
            }
        }
        println!(
            "{}",
            check(&format!(
                "Raw import complete ({} files: {} new, {} modified, {} skipped).",
                files.len(),
                new_count,
                modified_count,
                skipped_count
            ))
        );
        println!();
        return Ok(());
    }

    let (all_memories, errors) = process_all_chunks(&all_chunks).await?;

    let merged = merge_all_memories(&all_memories);

    println!();
    let pb = spinner("Merging memories...");
    pb.finish_with_message(check("Memories merged and deduplicated"));

    let pb = spinner("Writing to vault...");
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let write_result = write_memories_to_vault(&merged, &today)?;
    pb.finish_with_message(check("Vault updated"));

    let pb = spinner("Updating source tracking...");
    let all_file_paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();
    match update_source_tracking(&abs_path, &all_file_paths) {
        Ok(()) => pb.finish_with_message(check("Source tracking updated")),
        Err(e) => {
            pb.finish_with_message(amber("Source tracking skipped"));
            eprintln!(
                "{}",
                amber(&format!("  ⚠ Could not update source tracking: {}", e))
            );
        }
    }

    print_summary(
        new_count,
        modified_count,
        skipped_count,
        &merged,
        &write_result.topics_written,
        &write_result.people_written,
        &errors,
    );

    Ok(())
}

/// Run ingestion for specific files only (used by watch command).
pub async fn run_for_files(base_path: &Path, files: &[FileInfo]) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    let all_chunks = read_and_chunk(files)?;
    if all_chunks.is_empty() {
        return Ok(());
    }

    if !processing_enabled()? {
        let all_file_paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();
        if let Err(e) = update_source_tracking(base_path, &all_file_paths) {
            eprintln!(
                "{}",
                amber(&format!("  ⚠ Could not update source tracking: {}", e))
            );
        }
        return Ok(());
    }

    let (all_memories, _errors) = process_all_chunks(&all_chunks).await?;

    let merged = merge_all_memories(&all_memories);
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    write_memories_to_vault(&merged, &today)?;

    let all_file_paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();
    if let Err(e) = update_source_tracking(base_path, &all_file_paths) {
        eprintln!(
            "{}",
            amber(&format!("  ⚠ Could not update source tracking: {}", e))
        );
    }

    Ok(())
}

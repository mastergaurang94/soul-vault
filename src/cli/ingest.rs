//! `soma ingest <folder>` — file ingestion with source tracking and dedup.

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::core::merger::{chunk_text, merge_all_memories};
use crate::core::processor::process_chunk;
use crate::extractors::local::{discover_files, extract_file_content};
use crate::types::{ChunkInfo, ExtractedMemories, FileInfo};
use crate::ui::theme::*;
use crate::vault::config::{assert_initialized, assert_path_exists};
use crate::vault::sources::{classify_files, update_source_tracking};
use crate::vault::write::write_memories_to_vault;

// ─── Ingest Command ───────────────────────────────────────────────────────────

pub async fn run(folder_path: &str, force: bool) -> Result<()> {
    println!("{}", banner());
    assert_initialized()?;

    let abs_path = std::fs::canonicalize(folder_path)
        .unwrap_or_else(|_| Path::new(folder_path).to_path_buf());
    assert_path_exists(&abs_path)?;

    println!(
        "  {} Ingesting from {}\n",
        ICON_FOLDER,
        cyan(&abs_path.display().to_string())
    );
    println!("{}", line());

    // Phase 1: Scan
    let files = scan_files(&abs_path)?;

    // Phase 2: Source tracking & dedup
    let (files_to_ingest, new_count, modified_count, skipped_count) =
        classify_and_filter(&abs_path, &files, force)?;

    if files_to_ingest.is_empty() {
        println!(
            "\n{}",
            check(&format!(
                "All {} files unchanged. Nothing to ingest.",
                skipped_count
            ))
        );
        println!(
            "\n  {} {} {}",
            dim("Use"),
            cyan("soma ingest --force <folder>"),
            dim("to re-ingest everything.")
        );
        println!();
        return Ok(());
    }

    // Phase 3: Read & Chunk (only files to ingest)
    let all_chunks = read_and_chunk(&files_to_ingest)?;

    // Phase 4: LLM Processing
    let (all_memories, errors) = process_all_chunks(&all_chunks).await?;

    // Phase 5: Merge & Write
    let merged = merge_all_memories(&all_memories);

    println!();
    let pb = spinner("Merging memories...");
    pb.finish_with_message(check("Memories merged and deduplicated"));

    let pb = spinner("Writing to vault...");
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let write_result = write_memories_to_vault(&merged, &today)?;
    pb.finish_with_message(check("Vault updated"));

    // Phase 6: Update source tracking
    let pb = spinner("Updating source tracking...");
    let all_file_paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();
    update_source_tracking(&abs_path, &all_file_paths)?;
    pb.finish_with_message(check("Source tracking updated"));

    // Summary
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

    // Read & Chunk
    let all_chunks = read_and_chunk(files)?;
    if all_chunks.is_empty() {
        return Ok(());
    }

    // LLM Processing
    let (all_memories, _errors) = process_all_chunks(&all_chunks).await?;

    // Merge & Write
    let merged = merge_all_memories(&all_memories);
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    write_memories_to_vault(&merged, &today)?;

    // Update source tracking
    let all_file_paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();
    update_source_tracking(base_path, &all_file_paths)?;

    Ok(())
}

// ─── Phase 1: Scan ────────────────────────────────────────────────────────────

fn scan_files(abs_path: &Path) -> Result<Vec<FileInfo>> {
    let pb = spinner("Scanning for files...");
    let files = discover_files(abs_path)?;

    if files.is_empty() {
        pb.finish_with_message(cross("No supported files found (.md, .txt, .json, .jsonl)"));
        anyhow::bail!("No files to process");
    }

    let total_size: u64 = files.iter().map(|f| f.size).sum();
    pb.finish_with_message(check(&format!(
        "Found {} files ({})",
        bold_white(&files.len().to_string()),
        format_bytes(total_size)
    )));

    // File type breakdown
    let mut counts = std::collections::HashMap::new();
    for f in &files {
        *counts.entry(f.extension.clone()).or_insert(0usize) += 1;
    }
    for (ext, count) in &counts {
        println!("{}", dim(&format!("      {}: {} files", ext, count)));
    }
    println!();

    Ok(files)
}

// ─── Phase 2: Source Tracking & Classification ────────────────────────────────

fn classify_and_filter(
    abs_path: &Path,
    files: &[FileInfo],
    force: bool,
) -> Result<(Vec<FileInfo>, usize, usize, usize)> {
    if force {
        println!(
            "{}",
            amber("  ⚠ Force mode: re-ingesting all files regardless of changes")
        );
        println!();
        let count = files.len();
        return Ok((files.to_vec(), count, 0, 0));
    }

    let pb = spinner("Checking for changes...");
    let file_paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();
    let classification = classify_files(abs_path, &file_paths)?;

    let new_count = classification.new_files.len();
    let modified_count = classification.modified_files.len();
    let skipped_count = classification.skipped_files.len();

    let to_ingest_paths = classification.all_to_ingest();
    let files_to_ingest: Vec<FileInfo> = files
        .iter()
        .filter(|f| to_ingest_paths.contains(&f.path))
        .cloned()
        .collect();

    let mut status_parts = Vec::new();
    if new_count > 0 {
        status_parts.push(format!("{} new", bold_white(&new_count.to_string())));
    }
    if modified_count > 0 {
        status_parts.push(format!(
            "{} modified",
            amber(&modified_count.to_string())
        ));
    }
    if skipped_count > 0 {
        status_parts.push(format!(
            "{} unchanged",
            dim(&skipped_count.to_string())
        ));
    }

    pb.finish_with_message(check(&status_parts.join(", ")));
    println!();

    Ok((files_to_ingest, new_count, modified_count, skipped_count))
}

// ─── Phase 3: Read & Chunk ────────────────────────────────────────────────────

fn read_and_chunk(files: &[FileInfo]) -> Result<Vec<ChunkInfo>> {
    let pb = spinner("Reading and chunking files...");
    let mut all_chunks = Vec::new();
    let mut read_errors = Vec::new();

    for file in files {
        match extract_file_content(file) {
            Ok(content) => {
                if content.trim().is_empty() {
                    read_errors.push(format!("{}: empty file, skipped", file.path.display()));
                    continue;
                }
                all_chunks.extend(chunk_text(&content, &file.name));
            }
            Err(e) => {
                read_errors.push(format!("{}: {}", file.path.display(), e));
            }
        }
    }

    pb.finish_with_message(check(&format!(
        "Prepared {} chunks from {} files",
        bold_white(&all_chunks.len().to_string()),
        files.len()
    )));

    if !read_errors.is_empty() {
        println!(
            "{}",
            amber(&format!("\n    ⚠ {} files had issues:", read_errors.len()))
        );
        for err in read_errors.iter().take(5) {
            println!("{}", dim(&format!("      {}", err)));
        }
        if read_errors.len() > 5 {
            println!(
                "{}",
                dim(&format!("      ... and {} more", read_errors.len() - 5))
            );
        }
        println!();
    }

    if all_chunks.is_empty() {
        eprintln!("{}", cross("No content to process."));
        anyhow::bail!("No content to process");
    }

    Ok(all_chunks)
}

// ─── Phase 4: LLM Processing ─────────────────────────────────────────────────

async fn process_all_chunks(
    all_chunks: &[ChunkInfo],
) -> Result<(Vec<ExtractedMemories>, Vec<String>)> {
    println!(
        "\n  {} {}\n",
        ICON_BRAIN,
        purple("Processing through Claude...")
    );

    let client = reqwest::Client::new();
    let mut all_memories = Vec::new();
    let mut errors = Vec::new();

    let pb = ProgressBar::new(all_chunks.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "  {spinner:.magenta} [{bar:20.magenta/dark_gray}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("█░░"),
    );

    for (i, chunk) in all_chunks.iter().enumerate() {
        let label = if chunk.total > 1 {
            format!("{} ({}/{})", chunk.source, chunk.index + 1, chunk.total)
        } else {
            chunk.source.clone()
        };

        pb.set_message(dim(&format!("Processing {}", label)));

        match process_chunk(&client, chunk).await {
            Ok(memories) => {
                let fact_count = memories.fact_count();
                all_memories.push(memories);
                pb.set_message(format!(
                    "{} → {} facts",
                    dim(&chunk.source),
                    bold_white(&fact_count.to_string())
                ));
            }
            Err(e) => {
                let msg = e.to_string();
                pb.set_message(format!("{} → {}", dim(&chunk.source), red(&msg)));
                errors.push(format!("Processing {}: {}", chunk.source, msg));

                // Fatal errors
                if msg.contains("API key") || msg.contains("401") {
                    pb.finish_and_clear();
                    anyhow::bail!("API key error. Run `soma init` to reconfigure.");
                }

                // Rate limit — wait and retry
                if msg.contains("Rate limited") || msg.contains("429") {
                    pb.set_message(amber("Rate limited. Waiting 30s..."));
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    if let Ok(memories) = process_chunk(&client, chunk).await {
                        all_memories.push(memories);
                    }
                }
            }
        }

        pb.set_position((i + 1) as u64);
    }

    pb.finish_and_clear();
    println!(
        "{}",
        check(&format!(
            "Processed {} chunks",
            bold_white(&all_chunks.len().to_string())
        ))
    );

    Ok((all_memories, errors))
}

// ─── Summary ──────────────────────────────────────────────────────────────────

fn print_summary(
    new_count: usize,
    modified_count: usize,
    skipped_count: usize,
    merged: &ExtractedMemories,
    topics_written: &[String],
    people_written: &[String],
    errors: &[String],
) {
    let total = merged.fact_count();
    println!("\n{}", line());
    println!(
        "\n  {} {}\n",
        amber(ICON_STAR),
        bold_purple("Ingestion complete!")
    );

    // Ingestion stats
    println!(
        "  {} {} new, {} updated, {} skipped",
        dim(&format!("{:<18}", "Ingested")),
        bold_white(&new_count.to_string()),
        amber(&modified_count.to_string()),
        dim(&skipped_count.to_string())
    );
    println!(
        "  {} {}",
        dim(&format!("{:<18}", "Memories extracted")),
        bold_white(&total.to_string())
    );
    println!(
        "  {} {}{}",
        dim(&format!("{:<18}", "Topics found")),
        bold_white(&topics_written.len().to_string()),
        summarize_list(topics_written)
    );
    println!(
        "  {} {}{}",
        dim(&format!("{:<18}", "People found")),
        bold_white(&people_written.len().to_string()),
        summarize_list(people_written)
    );
    if !errors.is_empty() {
        println!(
            "  {} {}",
            dim(&format!("{:<18}", "Errors")),
            amber(&errors.len().to_string())
        );
    }
    println!(
        "\n  {} {} {}",
        dim("Run"),
        cyan("soma status"),
        dim("to see your vault.")
    );
    println!();
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("  {spinner:.magenta} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

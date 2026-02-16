//! File scan, change classification, and chunk preparation for ingest.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::cli::ingest_process::spinner;
use crate::core::merger::chunk_text;
use crate::types::{ChunkInfo, FileInfo};
use crate::ui::theme::*;
use crate::vault::local::{discover_files, extract_file_content};
use crate::vault::sources::classify_files;

pub(crate) fn scan_files(abs_path: &Path) -> Result<Vec<FileInfo>> {
    let pb = spinner("Scanning for files...");
    let files = discover_files(abs_path)?;

    if files.is_empty() {
        pb.finish_with_message(cross("No supported files found (.md, .txt, .json, .jsonl)"));
        anyhow::bail!("No files to process");
    }

    let total_size: u64 = files.iter().map(|f| f.size).sum();

    let mut counts = std::collections::HashMap::new();
    for f in &files {
        *counts.entry(f.extension.clone()).or_insert(0usize) += 1;
    }
    let mut breakdown: Vec<String> = counts
        .iter()
        .map(|(ext, count)| format!("{count} {ext}"))
        .collect();
    breakdown.sort();
    let detail = breakdown.join(", ");

    pb.finish_with_message(check(&format!(
        "Found {} files ({}) — {}",
        bold_white(&files.len().to_string()),
        format_bytes(total_size),
        dim(&detail),
    )));

    Ok(files)
}

pub(crate) fn classify_and_filter(
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
        status_parts.push(format!("{} modified", amber(&modified_count.to_string())));
    }
    if skipped_count > 0 {
        status_parts.push(format!("{} unchanged", dim(&skipped_count.to_string())));
    }

    pb.finish_with_message(check(&status_parts.join(", ")));
    println!();

    Ok((files_to_ingest, new_count, modified_count, skipped_count))
}

pub(crate) fn read_and_chunk(files: &[FileInfo]) -> Result<Vec<ChunkInfo>> {
    use indicatif::{ProgressBar, ProgressStyle};

    let pb = ProgressBar::new(files.len() as u64);
    let style = ProgressStyle::with_template(
        "  {spinner:.yellow} [{bar:20.yellow/dark_gray}] {pos}/{len} Reading files…",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar())
    .progress_chars("█░░");
    pb.set_style(style);

    let mut all_chunks = Vec::new();
    let mut read_errors = Vec::new();

    for (i, file) in files.iter().enumerate() {
        match extract_file_content(file) {
            Ok(content) => {
                if content.trim().is_empty() {
                    read_errors.push(format!("{}: empty file, skipped", file.path.display()));
                } else {
                    all_chunks.extend(chunk_text(&content, &file.name));
                }
            }
            Err(e) => {
                read_errors.push(format!("{}: {}", file.path.display(), e));
            }
        }
        pb.set_position((i + 1) as u64);
    }

    let mut msg = check(&format!(
        "Prepared {} chunks from {} files",
        bold_white(&all_chunks.len().to_string()),
        files.len()
    ));
    if !read_errors.is_empty() {
        msg.push_str(&format!(
            " ({})",
            amber(&format!("{} skipped", read_errors.len()))
        ));
    }
    pb.finish_and_clear();
    println!("{msg}");

    if all_chunks.is_empty() {
        eprintln!("{}", cross("No content to process."));
        anyhow::bail!("No content to process");
    }

    Ok(all_chunks)
}

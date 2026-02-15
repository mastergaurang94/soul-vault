//! Async import pipeline with progress reporting via channels.
//!
//! Reuses existing scan/classify/process/merge/write functions
//! but sends structured progress messages instead of printing to stdout.

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::core::merger::{chunk_text, merge_all_memories};
use crate::core::processor::process_chunk;
use crate::types::{ChunkInfo, ExtractedMemories, FileInfo};
use crate::vault::config::{assert_initialized, assert_path_exists};
use crate::vault::local::{discover_files, extract_file_content};
use crate::vault::sources::{classify_files, update_source_tracking};
use crate::vault::write::write_memories_to_vault;

// ─── Progress Messages ───────────────────────────────────────────────────────

/// Structured progress updates sent from the pipeline to the TUI.
#[derive(Debug, Clone)]
pub enum ImportProgress {
    Scanning,
    ScanComplete {
        #[allow(dead_code)]
        file_count: usize,
    },
    Classifying,
    ClassifyComplete {
        #[allow(dead_code)]
        new_count: usize,
        #[allow(dead_code)]
        modified_count: usize,
        #[allow(dead_code)]
        skipped_count: usize,
    },
    /// All files unchanged — nothing to do.
    NothingToImport {
        skipped_count: usize,
    },
    Processing {
        current: usize,
        total: usize,
        current_file: String,
    },
    Writing,
    Done(ImportResult),
    Error(String),
}

/// Final result of a completed import.
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub new_count: usize,
    pub modified_count: usize,
    pub skipped_count: usize,
    pub facts_extracted: usize,
    pub topics: Vec<String>,
    pub people: Vec<String>,
    pub errors: Vec<String>,
}

// ─── Pipeline Entry Point ─────────────────────────────────────────────────────

/// Runs the full import pipeline, sending progress over `tx`.
///
/// Designed to be spawned on a tokio task from the TUI event loop.
pub async fn run_import(folder: String, tx: mpsc::Sender<ImportProgress>) {
    if let Err(e) = run_import_inner(&folder, &tx).await {
        let _ = tx.send(ImportProgress::Error(e.to_string())).await;
    }
}

async fn run_import_inner(folder: &str, tx: &mpsc::Sender<ImportProgress>) -> Result<()> {
    assert_initialized()?;

    let abs_path =
        std::fs::canonicalize(folder).unwrap_or_else(|_| Path::new(folder).to_path_buf());
    assert_path_exists(&abs_path)?;

    // Phase 1: Scan
    tx.send(ImportProgress::Scanning).await.ok();
    let files = discover_files(&abs_path)?;
    if files.is_empty() {
        anyhow::bail!("No supported files found (.md, .txt, .json, .jsonl)");
    }
    tx.send(ImportProgress::ScanComplete {
        file_count: files.len(),
    })
    .await
    .ok();

    // Phase 2: Classify
    tx.send(ImportProgress::Classifying).await.ok();
    let file_paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();
    let classification = classify_files(&abs_path, &file_paths)?;

    let new_count = classification.new_files.len();
    let modified_count = classification.modified_files.len();
    let skipped_count = classification.skipped_files.len();

    tx.send(ImportProgress::ClassifyComplete {
        new_count,
        modified_count,
        skipped_count,
    })
    .await
    .ok();

    let to_ingest = classification.all_to_ingest();
    let files_to_ingest: Vec<FileInfo> = files
        .iter()
        .filter(|f| to_ingest.contains(&f.path))
        .cloned()
        .collect();

    if files_to_ingest.is_empty() {
        tx.send(ImportProgress::NothingToImport { skipped_count })
            .await
            .ok();
        return Ok(());
    }

    // Phase 3: Read & Chunk
    let all_chunks = read_and_chunk(&files_to_ingest)?;

    // Phase 4: Process through LLM
    let (all_memories, errors) = process_chunks_with_progress(&all_chunks, tx).await?;

    // Phase 5: Merge & Write
    tx.send(ImportProgress::Writing).await.ok();
    let merged = merge_all_memories(&all_memories);
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let write_result = write_memories_to_vault(&merged, &today)?;

    // Update source tracking
    update_source_tracking(&abs_path, &file_paths)?;

    tx.send(ImportProgress::Done(ImportResult {
        new_count,
        modified_count,
        skipped_count,
        facts_extracted: merged.fact_count(),
        topics: write_result.topics_written,
        people: write_result.people_written,
        errors,
    }))
    .await
    .ok();

    Ok(())
}

// ─── Chunking ─────────────────────────────────────────────────────────────────

fn read_and_chunk(files: &[FileInfo]) -> Result<Vec<ChunkInfo>> {
    let mut all_chunks = Vec::new();
    for file in files {
        if let Ok(content) = extract_file_content(file) {
            if !content.trim().is_empty() {
                all_chunks.extend(chunk_text(&content, &file.name));
            }
        }
    }
    if all_chunks.is_empty() {
        anyhow::bail!("No content to process after reading files");
    }
    Ok(all_chunks)
}

// ─── LLM Processing with Progress ────────────────────────────────────────────

async fn process_chunks_with_progress(
    chunks: &[ChunkInfo],
    tx: &mpsc::Sender<ImportProgress>,
) -> Result<(Vec<ExtractedMemories>, Vec<String>)> {
    let client = reqwest::Client::new();
    let mut all_memories = Vec::new();
    let mut errors = Vec::new();
    let total = chunks.len();

    for (i, chunk) in chunks.iter().enumerate() {
        let label = if chunk.total > 1 {
            format!("{} ({}/{})", chunk.source, chunk.index + 1, chunk.total)
        } else {
            chunk.source.clone()
        };

        tx.send(ImportProgress::Processing {
            current: i + 1,
            total,
            current_file: label.clone(),
        })
        .await
        .ok();

        match process_chunk(&client, chunk).await {
            Ok(memories) => all_memories.push(memories),
            Err(e) => {
                let msg = e.to_string();
                // Fatal API key errors — abort
                if msg.contains("API key") || msg.contains("401") {
                    anyhow::bail!("API key error. Run `soul init` to reconfigure.");
                }
                // Rate limit — wait and retry once
                if msg.contains("Rate limited") || msg.contains("429") {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    if let Ok(memories) = process_chunk(&client, chunk).await {
                        all_memories.push(memories);
                        continue;
                    }
                }
                errors.push(format!("{}: {}", label, msg));
            }
        }
    }

    Ok((all_memories, errors))
}

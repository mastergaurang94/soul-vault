//! Hashing/classification/update logic for source tracking.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::vault::sources_store::{read_sources, write_sources};
use crate::vault::sources_types::{IngestClassification, SourceEntry};

/// Computes SHA-256 hash of a file's contents.
pub fn compute_file_hash(path: &Path) -> Result<String> {
    let content = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Classifies files into new, modified, or unchanged based on source tracking.
pub fn classify_files(base_path: &Path, file_paths: &[PathBuf]) -> Result<IngestClassification> {
    let sources = read_sources()?;
    let base_str = base_path.to_string_lossy().to_string();
    let existing = sources.sources.iter().find(|s| s.path == base_str);

    let mut classification = IngestClassification::default();
    match existing {
        None => classification.new_files = file_paths.to_vec(),
        Some(entry) => {
            for file_path in file_paths {
                let rel_path = file_path
                    .strip_prefix(base_path)
                    .unwrap_or(file_path)
                    .to_string_lossy()
                    .to_string();

                match entry.file_hashes.get(&rel_path) {
                    None => classification.new_files.push(file_path.clone()),
                    Some(old_hash) => match compute_file_hash(file_path) {
                        Ok(current_hash) if &current_hash == old_hash => {
                            classification.skipped_files.push(file_path.clone())
                        }
                        Ok(_) | Err(_) => classification.modified_files.push(file_path.clone()),
                    },
                }
            }
        }
    }

    Ok(classification)
}

/// Updates source tracking after a successful ingestion.
pub fn update_source_tracking(base_path: &Path, all_files: &[PathBuf]) -> Result<()> {
    let mut sources = read_sources()?;
    let base_str = base_path.to_string_lossy().to_string();

    let mut file_hashes = HashMap::new();
    for file_path in all_files {
        let rel_path = file_path
            .strip_prefix(base_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();
        if let Ok(hash) = compute_file_hash(file_path) {
            file_hashes.insert(rel_path, hash);
        }
    }

    let now = chrono::Utc::now().to_rfc3339();

    if let Some(entry) = sources.sources.iter_mut().find(|s| s.path == base_str) {
        entry.files_ingested = all_files.len();
        entry.last_ingested = now;
        entry.file_hashes = file_hashes;
    } else {
        sources.sources.push(SourceEntry {
            path: base_str,
            files_ingested: all_files.len(),
            last_ingested: now,
            file_hashes,
        });
    }

    write_sources(&sources)?;
    Ok(())
}

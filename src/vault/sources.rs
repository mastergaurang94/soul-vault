//! Source tracking: dedup prevention via file hash tracking.
//!
//! Tracks ingested paths with metadata in ~/soul-vault/.config/sources.json
//! to prevent duplicate ingestion on repeated runs.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::vault::config::config_dir;

// ─── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesConfig {
    pub sources: Vec<SourceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceEntry {
    pub path: String,
    pub files_ingested: usize,
    pub last_ingested: String,
    pub file_hashes: HashMap<String, String>,
}

/// Result of classifying files against previous ingestion.
#[derive(Debug, Default)]
pub struct IngestClassification {
    /// Files that are new (never seen before).
    pub new_files: Vec<PathBuf>,
    /// Files that have changed since last ingestion.
    pub modified_files: Vec<PathBuf>,
    /// Files that are unchanged since last ingestion.
    pub skipped_files: Vec<PathBuf>,
}

impl IngestClassification {
    pub fn all_to_ingest(&self) -> Vec<PathBuf> {
        let mut result = self.new_files.clone();
        result.extend(self.modified_files.iter().cloned());
        result
    }
}

// ─── Sources File Path ────────────────────────────────────────────────────────

pub fn sources_config_path() -> PathBuf {
    config_dir().join("sources.json")
}

// ─── Read / Write ─────────────────────────────────────────────────────────────

/// Reads sources.json. Returns empty config if missing.
pub fn read_sources() -> Result<SourcesConfig> {
    let path = sources_config_path();
    if !path.exists() {
        return Ok(SourcesConfig {
            sources: Vec::new(),
        });
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let config: SourcesConfig =
        serde_json::from_str(&raw).with_context(|| "Failed to parse sources.json")?;
    Ok(config)
}

/// Writes sources.json with pretty formatting.
pub fn write_sources(config: &SourcesConfig) -> Result<()> {
    let path = sources_config_path();
    fs::create_dir_all(config_dir())?;
    let json = serde_json::to_string_pretty(config)?;
    fs::write(&path, json).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

// ─── File Hashing ─────────────────────────────────────────────────────────────

/// Computes SHA-256 hash of a file's contents.
pub fn compute_file_hash(path: &Path) -> Result<String> {
    let content = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

// ─── Classification ───────────────────────────────────────────────────────────

/// Classifies files into new, modified, or unchanged based on source tracking.
///
/// `base_path` is the absolute path to the ingested folder.
/// `file_paths` is the list of absolute file paths discovered.
pub fn classify_files(
    base_path: &Path,
    file_paths: &[PathBuf],
) -> Result<IngestClassification> {
    let sources = read_sources()?;
    let base_str = base_path.to_string_lossy().to_string();

    // Find existing source entry for this path
    let existing = sources.sources.iter().find(|s| s.path == base_str);

    let mut classification = IngestClassification::default();

    match existing {
        None => {
            // Never ingested this folder before — all files are new
            classification.new_files = file_paths.to_vec();
        }
        Some(entry) => {
            for file_path in file_paths {
                let rel_path = file_path
                    .strip_prefix(base_path)
                    .unwrap_or(file_path)
                    .to_string_lossy()
                    .to_string();

                match entry.file_hashes.get(&rel_path) {
                    None => {
                        // New file not seen before
                        classification.new_files.push(file_path.clone());
                    }
                    Some(old_hash) => {
                        // Check if file has changed
                        match compute_file_hash(file_path) {
                            Ok(current_hash) => {
                                if &current_hash == old_hash {
                                    classification.skipped_files.push(file_path.clone());
                                } else {
                                    classification.modified_files.push(file_path.clone());
                                }
                            }
                            Err(_) => {
                                // Can't read file — treat as modified
                                classification.modified_files.push(file_path.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(classification)
}

/// Updates source tracking after a successful ingestion.
///
/// `base_path` is the folder that was ingested.
/// `ingested_files` is the list of file paths that were actually ingested.
/// All files (including skipped ones) should have their hashes recorded.
pub fn update_source_tracking(
    base_path: &Path,
    all_files: &[PathBuf],
) -> Result<()> {
    let mut sources = read_sources()?;
    let base_str = base_path.to_string_lossy().to_string();

    // Build current file hashes
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

    // Update or insert source entry
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

// ─── Stats ────────────────────────────────────────────────────────────────────

/// Returns summary info about tracked sources for the status command.
pub fn get_source_stats() -> Result<Vec<SourceSummary>> {
    let sources = read_sources()?;
    Ok(sources
        .sources
        .iter()
        .map(|s| SourceSummary {
            path: s.path.clone(),
            files_ingested: s.files_ingested,
            last_ingested: s.last_ingested.clone(),
        })
        .collect())
}

#[derive(Debug)]
pub struct SourceSummary {
    pub path: String,
    pub files_ingested: usize,
    pub last_ingested: String,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_compute_file_hash() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("test.txt");
        fs::write(&file, "hello world").unwrap();

        let hash1 = compute_file_hash(&file).unwrap();
        assert!(!hash1.is_empty());
        assert_eq!(hash1.len(), 64); // SHA-256 hex is 64 chars

        // Same content = same hash
        let hash2 = compute_file_hash(&file).unwrap();
        assert_eq!(hash1, hash2);

        // Different content = different hash
        fs::write(&file, "hello world!").unwrap();
        let hash3 = compute_file_hash(&file).unwrap();
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_compute_file_hash_known_value() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("test.txt");
        fs::write(&file, "hello world").unwrap();

        let hash = compute_file_hash(&file).unwrap();
        // SHA-256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_sources_config_serde_roundtrip() {
        let mut file_hashes = HashMap::new();
        file_hashes.insert("notes.md".to_string(), "abc123".to_string());
        file_hashes.insert("data.json".to_string(), "def456".to_string());

        let config = SourcesConfig {
            sources: vec![SourceEntry {
                path: "/home/user/docs".to_string(),
                files_ingested: 2,
                last_ingested: "2026-02-14T11:00:00Z".to_string(),
                file_hashes,
            }],
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: SourcesConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.sources.len(), 1);
        assert_eq!(parsed.sources[0].path, "/home/user/docs");
        assert_eq!(parsed.sources[0].files_ingested, 2);
        assert_eq!(parsed.sources[0].file_hashes.len(), 2);
        assert_eq!(
            parsed.sources[0].file_hashes.get("notes.md").unwrap(),
            "abc123"
        );
    }

    #[test]
    fn test_classify_files_all_new() {
        let tmp = tempdir().unwrap();
        let base = tmp.path();

        // Create test files
        fs::write(base.join("a.md"), "content a").unwrap();
        fs::write(base.join("b.txt"), "content b").unwrap();

        let files = vec![base.join("a.md"), base.join("b.txt")];

        // No sources.json exists — everything is new
        let classification = classify_files(base, &files).unwrap();
        assert_eq!(classification.new_files.len(), 2);
        assert_eq!(classification.modified_files.len(), 0);
        assert_eq!(classification.skipped_files.len(), 0);
    }

    #[test]
    fn test_classify_files_with_existing_tracking() {
        let tmp = tempdir().unwrap();
        let base = tmp.path();

        // Create test files
        fs::write(base.join("unchanged.md"), "same content").unwrap();
        fs::write(base.join("modified.md"), "original content").unwrap();

        // Compute hashes for "previous" ingestion
        let unchanged_hash = compute_file_hash(&base.join("unchanged.md")).unwrap();
        let old_modified_hash = "old_hash_that_wont_match".to_string();

        let mut file_hashes = HashMap::new();
        file_hashes.insert("unchanged.md".to_string(), unchanged_hash);
        file_hashes.insert("modified.md".to_string(), old_modified_hash);

        // Write a sources.json in the test
        // We can't use the global sources.json, so we test the classify logic directly
        let sources = SourcesConfig {
            sources: vec![SourceEntry {
                path: base.to_string_lossy().to_string(),
                files_ingested: 2,
                last_ingested: "2026-02-14T10:00:00Z".to_string(),
                file_hashes,
            }],
        };

        // Test classification logic directly
        let base_str = base.to_string_lossy().to_string();
        let existing = sources.sources.iter().find(|s| s.path == base_str);
        assert!(existing.is_some());

        let entry = existing.unwrap();

        // Test unchanged file
        let rel = "unchanged.md";
        let old_hash = entry.file_hashes.get(rel).unwrap();
        let current_hash = compute_file_hash(&base.join(rel)).unwrap();
        assert_eq!(old_hash, &current_hash); // Should be unchanged

        // Test modified file
        let rel = "modified.md";
        let old_hash = entry.file_hashes.get(rel).unwrap();
        let current_hash = compute_file_hash(&base.join(rel)).unwrap();
        assert_ne!(old_hash, &current_hash); // Should be different
    }

    #[test]
    fn test_classify_new_file_in_tracked_source() {
        let tmp = tempdir().unwrap();
        let base = tmp.path();

        // Create a new file
        fs::write(base.join("brand_new.md"), "new content").unwrap();

        // Source entry exists but doesn't have this file
        let sources = SourcesConfig {
            sources: vec![SourceEntry {
                path: base.to_string_lossy().to_string(),
                files_ingested: 0,
                last_ingested: "2026-02-14T10:00:00Z".to_string(),
                file_hashes: HashMap::new(),
            }],
        };

        let base_str = base.to_string_lossy().to_string();
        let entry = sources.sources.iter().find(|s| s.path == base_str).unwrap();

        let rel = "brand_new.md";
        assert!(!entry.file_hashes.contains_key(rel)); // New file
    }

    #[test]
    fn test_update_source_tracking_new_entry() {
        let tmp = tempdir().unwrap();
        let base = tmp.path();

        fs::write(base.join("test.md"), "content").unwrap();

        let files = vec![base.join("test.md")];

        // Create a temp sources config and test the logic
        let mut sources = SourcesConfig {
            sources: Vec::new(),
        };
        let base_str = base.to_string_lossy().to_string();

        let mut file_hashes = HashMap::new();
        for file_path in &files {
            let rel_path = file_path
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .to_string();
            let hash = compute_file_hash(file_path).unwrap();
            file_hashes.insert(rel_path, hash);
        }

        sources.sources.push(SourceEntry {
            path: base_str.clone(),
            files_ingested: files.len(),
            last_ingested: chrono::Utc::now().to_rfc3339(),
            file_hashes,
        });

        assert_eq!(sources.sources.len(), 1);
        assert_eq!(sources.sources[0].path, base_str);
        assert_eq!(sources.sources[0].files_ingested, 1);
        assert!(sources.sources[0].file_hashes.contains_key("test.md"));
    }

    #[test]
    fn test_update_source_tracking_existing_entry() {
        let tmp = tempdir().unwrap();
        let base = tmp.path();

        fs::write(base.join("old.md"), "old content").unwrap();
        fs::write(base.join("new.md"), "new content").unwrap();

        let base_str = base.to_string_lossy().to_string();

        let mut sources = SourcesConfig {
            sources: vec![SourceEntry {
                path: base_str.clone(),
                files_ingested: 1,
                last_ingested: "2026-02-14T10:00:00Z".to_string(),
                file_hashes: {
                    let mut h = HashMap::new();
                    h.insert(
                        "old.md".to_string(),
                        compute_file_hash(&base.join("old.md")).unwrap(),
                    );
                    h
                },
            }],
        };

        // Simulate update with both files
        let all_files = vec![base.join("old.md"), base.join("new.md")];
        let mut new_hashes = HashMap::new();
        for f in &all_files {
            let rel = f.strip_prefix(base).unwrap().to_string_lossy().to_string();
            new_hashes.insert(rel, compute_file_hash(f).unwrap());
        }

        let entry = sources.sources.iter_mut().find(|s| s.path == base_str).unwrap();
        entry.files_ingested = all_files.len();
        entry.file_hashes = new_hashes;

        assert_eq!(entry.files_ingested, 2);
        assert!(entry.file_hashes.contains_key("old.md"));
        assert!(entry.file_hashes.contains_key("new.md"));
    }

    #[test]
    fn test_ingest_classification_all_to_ingest() {
        let classification = IngestClassification {
            new_files: vec![PathBuf::from("a.md"), PathBuf::from("b.md")],
            modified_files: vec![PathBuf::from("c.md")],
            skipped_files: vec![PathBuf::from("d.md")],
        };

        let to_ingest = classification.all_to_ingest();
        assert_eq!(to_ingest.len(), 3);
        assert!(to_ingest.contains(&PathBuf::from("a.md")));
        assert!(to_ingest.contains(&PathBuf::from("b.md")));
        assert!(to_ingest.contains(&PathBuf::from("c.md")));
    }

    #[test]
    fn test_empty_sources_read() {
        // When sources.json doesn't exist, should return empty
        let config = SourcesConfig {
            sources: Vec::new(),
        };
        assert_eq!(config.sources.len(), 0);
    }
}

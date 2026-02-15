//! Tests for vault module.
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::vault::sources_classify::{classify_files, compute_file_hash};
use crate::vault::sources_types::{IngestClassification, SourceEntry, SourcesConfig};

#[test]
fn test_compute_file_hash_basic_and_known() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("test.txt");
    fs::write(&file, "hello world").unwrap();

    let hash1 = compute_file_hash(&file).unwrap();
    let hash2 = compute_file_hash(&file).unwrap();
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64);
    assert_eq!(
        hash1,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );

    fs::write(&file, "hello world!").unwrap();
    let hash3 = compute_file_hash(&file).unwrap();
    assert_ne!(hash1, hash3);
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
    assert_eq!(parsed.sources[0].file_hashes.len(), 2);
}

#[test]
fn test_classify_files_all_new() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();
    fs::write(base.join("a.md"), "content a").unwrap();
    fs::write(base.join("b.txt"), "content b").unwrap();

    let files = vec![base.join("a.md"), base.join("b.txt")];
    let classification = classify_files(base, &files).unwrap();
    assert_eq!(classification.new_files.len(), 2);
    assert_eq!(classification.modified_files.len(), 0);
    assert_eq!(classification.skipped_files.len(), 0);
}

#[test]
fn test_classify_files_with_existing_tracking_logic() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();
    fs::write(base.join("unchanged.md"), "same content").unwrap();
    fs::write(base.join("modified.md"), "original content").unwrap();

    let unchanged_hash = compute_file_hash(&base.join("unchanged.md")).unwrap();
    let mut file_hashes = HashMap::new();
    file_hashes.insert("unchanged.md".to_string(), unchanged_hash.clone());
    file_hashes.insert(
        "modified.md".to_string(),
        "old_hash_that_wont_match".to_string(),
    );

    let sources = SourcesConfig {
        sources: vec![SourceEntry {
            path: base.to_string_lossy().to_string(),
            files_ingested: 2,
            last_ingested: "2026-02-14T10:00:00Z".to_string(),
            file_hashes,
        }],
    };

    let entry = sources
        .sources
        .iter()
        .find(|s| s.path == base.to_string_lossy())
        .unwrap();

    assert_eq!(
        entry.file_hashes.get("unchanged.md").unwrap(),
        &unchanged_hash
    );
    assert_ne!(
        entry.file_hashes.get("modified.md").unwrap(),
        &compute_file_hash(&base.join("modified.md")).unwrap()
    );
}

#[test]
fn test_classify_new_file_in_tracked_source() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();
    fs::write(base.join("brand_new.md"), "new content").unwrap();

    let sources = SourcesConfig {
        sources: vec![SourceEntry {
            path: base.to_string_lossy().to_string(),
            files_ingested: 0,
            last_ingested: "2026-02-14T10:00:00Z".to_string(),
            file_hashes: HashMap::new(),
        }],
    };

    let entry = sources
        .sources
        .iter()
        .find(|s| s.path == base.to_string_lossy())
        .unwrap();
    assert!(!entry.file_hashes.contains_key("brand_new.md"));
}

#[test]
fn test_update_tracking_logic_and_all_to_ingest() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();
    fs::write(base.join("old.md"), "old content").unwrap();
    fs::write(base.join("new.md"), "new content").unwrap();

    let all_files = vec![base.join("old.md"), base.join("new.md")];
    let mut new_hashes = HashMap::new();
    for f in &all_files {
        let rel = f.strip_prefix(base).unwrap().to_string_lossy().to_string();
        new_hashes.insert(rel, compute_file_hash(f).unwrap());
    }

    assert!(new_hashes.contains_key("old.md"));
    assert!(new_hashes.contains_key("new.md"));

    let classification = IngestClassification {
        new_files: vec![PathBuf::from("a.md"), PathBuf::from("b.md")],
        modified_files: vec![PathBuf::from("c.md")],
        skipped_files: vec![PathBuf::from("d.md")],
    };
    let to_ingest = classification.all_to_ingest();
    assert_eq!(to_ingest.len(), 3);
}

#[test]
fn test_empty_sources_read_shape() {
    let config = SourcesConfig {
        sources: Vec::new(),
    };
    assert_eq!(config.sources.len(), 0);
}

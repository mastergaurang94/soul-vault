use std::fs;

use crate::types::*;
use crate::vault::write_digest::build_digest_sections;
use crate::vault::write_entries::append_entry;
use crate::vault::write_slug::slugify;

#[test]
fn test_slugify() {
    assert_eq!(slugify("Hello World"), "hello-world");
    assert_eq!(slugify("crypto & AI"), "crypto-ai");
    assert_eq!(slugify("  spaces  "), "spaces");
    assert_eq!(slugify(""), "");
}

#[test]
fn test_build_digest_sections_empty() {
    let m = ExtractedMemories::default();
    assert_eq!(build_digest_sections(&m), "");
}

#[test]
fn test_build_digest_sections() {
    let m = ExtractedMemories {
        decisions: vec![DecisionFact {
            content: "Chose Rust".to_string(),
            context: None,
            confidence: Confidence::High,
            source: "test".to_string(),
            date: "2026-02-14".to_string(),
        }],
        topics: vec![TopicFact {
            topic: "Rust".to_string(),
            content: "Learning Rust for CLI tools".to_string(),
            opinion: None,
            confidence: Confidence::Medium,
            source: "test".to_string(),
            date: "2026-02-14".to_string(),
        }],
        ..Default::default()
    };

    let sections = build_digest_sections(&m);
    assert!(sections.contains("## Decisions"));
    assert!(sections.contains("- Chose Rust"));
    assert!(sections.contains("## Topics"));
    assert!(sections.contains("**Rust**"));
}

#[test]
fn test_write_daily_digest_indirectly_via_sections() {
    let m = ExtractedMemories {
        identity: vec![IdentityFact {
            content: "Name is Test".to_string(),
            category: "name".to_string(),
            confidence: Confidence::High,
            source: "test".to_string(),
            date: "2026-02-14".to_string(),
        }],
        ..Default::default()
    };
    let result = build_digest_sections(&m);
    assert!(result.contains("## Identity"));
    assert!(result.contains("Name is Test"));
}

#[test]
fn test_append_entry_new_file() {
    let tmp = tempfile::tempdir().unwrap();
    append_entry(
        tmp.path(),
        "rust",
        "Rust",
        "Learning Rust",
        "2026-02-14",
        "high",
        None,
    )
    .unwrap();

    let content = fs::read_to_string(tmp.path().join("rust.md")).unwrap();
    assert!(content.contains("# Rust"));
    assert!(content.contains("Learning Rust"));
    assert!(content.contains("(2026-02-14, high)"));
}

#[test]
fn test_append_entry_dedup() {
    let tmp = tempfile::tempdir().unwrap();

    append_entry(
        tmp.path(),
        "rust",
        "Rust",
        "Learning Rust",
        "2026-02-14",
        "high",
        None,
    )
    .unwrap();
    append_entry(
        tmp.path(),
        "rust",
        "Rust",
        "Learning Rust",
        "2026-02-14",
        "high",
        None,
    )
    .unwrap();

    let content = fs::read_to_string(tmp.path().join("rust.md")).unwrap();
    assert_eq!(content.matches("Learning Rust").count(), 1);
}

#[test]
fn test_append_entry_people_with_role() {
    let tmp = tempfile::tempdir().unwrap();
    let people_path = tmp.path().join("people");
    fs::create_dir_all(&people_path).unwrap();

    append_entry(
        &people_path,
        "avni",
        "Avni",
        "Daughter, light of my life",
        "2026-02-14",
        "high",
        Some("daughter"),
    )
    .unwrap();

    let content = fs::read_to_string(people_path.join("avni.md")).unwrap();
    assert!(content.contains("person: Avni"));
    assert!(content.contains("role: daughter"));
}

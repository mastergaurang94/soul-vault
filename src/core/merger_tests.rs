use crate::core::merger::{chunk_text, merge_all_memories};
use crate::core::merger_dedup::normalize;
use crate::types::*;

#[test]
fn test_merge_empty() {
    let results: Vec<ExtractedMemories> = vec![];
    let merged = merge_all_memories(&results);
    assert!(merged.is_empty());
}

#[test]
fn test_merge_single() {
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

    let merged = merge_all_memories(&[m]);
    assert_eq!(merged.identity.len(), 1);
}

#[test]
fn test_merge_dedup_case_insensitive() {
    let m1 = ExtractedMemories {
        identity: vec![IdentityFact {
            content: "Lives in Houston".to_string(),
            category: "location".to_string(),
            confidence: Confidence::High,
            source: "a".to_string(),
            date: "2026-02-14".to_string(),
        }],
        ..Default::default()
    };
    let m2 = ExtractedMemories {
        identity: vec![IdentityFact {
            content: "lives in houston".to_string(),
            category: "location".to_string(),
            confidence: Confidence::Medium,
            source: "b".to_string(),
            date: "2026-02-14".to_string(),
        }],
        ..Default::default()
    };

    let merged = merge_all_memories(&[m1, m2]);
    assert_eq!(merged.identity.len(), 1);
}

#[test]
fn test_merge_relationships_dedup() {
    let m1 = ExtractedMemories {
        relationships: vec![RelationshipFact {
            person: "Avni".to_string(),
            content: "His daughter".to_string(),
            role: Some("daughter".to_string()),
            confidence: Confidence::High,
            source: "a".to_string(),
            date: "2026-02-14".to_string(),
        }],
        ..Default::default()
    };
    let m2 = ExtractedMemories {
        relationships: vec![RelationshipFact {
            person: "Avni".to_string(),
            content: "His daughter".to_string(),
            role: Some("daughter".to_string()),
            confidence: Confidence::High,
            source: "b".to_string(),
            date: "2026-02-14".to_string(),
        }],
        ..Default::default()
    };

    let merged = merge_all_memories(&[m1, m2]);
    assert_eq!(merged.relationships.len(), 1);
}

#[test]
fn test_chunk_text_small_and_source() {
    let text = "Hello world";
    let chunks = chunk_text(text, "test");
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content, "Hello world");
    assert_eq!(chunks[0].source, "test");
    assert_eq!(chunks[0].index, 0);
    assert_eq!(chunks[0].total, 1);
}

#[test]
fn test_chunk_text_large() {
    let paragraph = "A".repeat(50_000);
    let text = format!("{}\n\n{}", paragraph, paragraph);
    let chunks = chunk_text(&text, "test");
    assert!(chunks.len() >= 2);

    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.index, i);
        assert_eq!(chunk.total, chunks.len());
    }
}

#[test]
fn test_normalize() {
    assert_eq!(normalize("  Hello   World  "), "hello world");
    assert_eq!(normalize("UPPER"), "upper");
}

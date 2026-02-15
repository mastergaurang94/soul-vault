//! Memory merging and deduplication.

use std::collections::HashMap;

use crate::types::ExtractedMemories;

// ─── Memory Merger ────────────────────────────────────────────────────────────

/// Merges multiple extraction results into a single deduplicated set.
pub fn merge_all_memories(results: &[ExtractedMemories]) -> ExtractedMemories {
    let mut merged = ExtractedMemories::default();

    for result in results {
        merged.identity.extend(result.identity.iter().cloned());
        merged
            .preferences
            .extend(result.preferences.iter().cloned());
        merged.decisions.extend(result.decisions.iter().cloned());
        merged
            .relationships
            .extend(result.relationships.iter().cloned());
        merged.topics.extend(result.topics.iter().cloned());
        merged
            .emotional_context
            .extend(result.emotional_context.iter().cloned());
    }

    ExtractedMemories {
        identity: deduplicate_by(merged.identity, |f| f.content.clone()),
        preferences: deduplicate_by(merged.preferences, |f| f.content.clone()),
        decisions: deduplicate_by(merged.decisions, |f| f.content.clone()),
        relationships: deduplicate_by(merged.relationships, |f| {
            format!("{}:{}", f.person, f.content)
        }),
        topics: deduplicate_by(merged.topics, |f| format!("{}:{}", f.topic, f.content)),
        emotional_context: deduplicate_by(merged.emotional_context, |f| f.content.clone()),
    }
}

// ─── Deduplication ────────────────────────────────────────────────────────────

/// Generic deduplication by a key function. Keeps first occurrence.
fn deduplicate_by<T, F>(items: Vec<T>, key_fn: F) -> Vec<T>
where
    F: Fn(&T) -> String,
{
    let mut seen = HashMap::new();
    let mut result = Vec::new();

    for item in items {
        let key = normalize(&key_fn(&item));
        if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(key) {
            e.insert(true);
            result.push(item);
        }
    }

    result
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

// ─── Text Chunking ────────────────────────────────────────────────────────────

/// Max characters per chunk (~20K tokens, safe for Claude).
const MAX_CHUNK_CHARS: usize = 80_000;

use crate::types::ChunkInfo;

/// Splits text into LLM-friendly chunks, breaking at paragraph boundaries.
pub fn chunk_text(text: &str, source: &str) -> Vec<ChunkInfo> {
    if text.len() <= MAX_CHUNK_CHARS {
        return vec![ChunkInfo {
            content: text.to_string(),
            source: source.to_string(),
            index: 0,
            total: 1,
        }];
    }

    let raw_chunks = split_at_paragraphs(text, MAX_CHUNK_CHARS);
    let final_chunks = force_split_oversized(&raw_chunks, MAX_CHUNK_CHARS);

    let total = final_chunks.len();
    final_chunks
        .into_iter()
        .enumerate()
        .map(|(index, content)| ChunkInfo {
            content,
            source: source.to_string(),
            index,
            total,
        })
        .collect()
}

/// Splits text at paragraph boundaries, respecting max size.
fn split_at_paragraphs(text: &str, max_size: usize) -> Vec<String> {
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let mut chunks = Vec::new();
    let mut current = String::new();

    for para in paragraphs {
        let combined = if current.is_empty() {
            para.to_string()
        } else {
            format!("{}\n\n{}", current, para)
        };

        if combined.len() > max_size && !current.is_empty() {
            chunks.push(current.trim().to_string());
            current = para.to_string();
        } else {
            current = combined;
        }
    }

    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }

    chunks
}

/// Force-splits any chunks still over the size limit.
fn force_split_oversized(chunks: &[String], max_size: usize) -> Vec<String> {
    let mut result = Vec::new();
    for chunk in chunks {
        if chunk.len() <= max_size {
            result.push(chunk.clone());
        } else {
            let bytes = chunk.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                let end = std::cmp::min(i + max_size, bytes.len());
                // Don't split in the middle of a UTF-8 character
                let actual_end = if end < bytes.len() {
                    let mut e = end;
                    while e > i && !chunk.is_char_boundary(e) {
                        e -= 1;
                    }
                    e
                } else {
                    end
                };
                result.push(chunk[i..actual_end].to_string());
                i = actual_end;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_merge_dedup() {
        let m1 = ExtractedMemories {
            identity: vec![IdentityFact {
                content: "Name is Gaurang".to_string(),
                category: "name".to_string(),
                confidence: Confidence::High,
                source: "source1".to_string(),
                date: "2026-02-14".to_string(),
            }],
            ..Default::default()
        };
        let m2 = ExtractedMemories {
            identity: vec![IdentityFact {
                content: "Name is Gaurang".to_string(), // duplicate
                category: "name".to_string(),
                confidence: Confidence::Medium,
                source: "source2".to_string(),
                date: "2026-02-14".to_string(),
            }],
            ..Default::default()
        };
        let merged = merge_all_memories(&[m1, m2]);
        assert_eq!(merged.identity.len(), 1); // deduped
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
                content: "lives in houston".to_string(), // same, different case
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
    fn test_chunk_text_small() {
        let text = "Hello world";
        let chunks = chunk_text(text, "test");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "Hello world");
        assert_eq!(chunks[0].index, 0);
        assert_eq!(chunks[0].total, 1);
    }

    #[test]
    fn test_chunk_text_preserves_source() {
        let text = "Short text";
        let chunks = chunk_text(text, "my-source");
        assert_eq!(chunks[0].source, "my-source");
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
}

//! Memory merge and deduplication logic.

use std::collections::HashMap;

use crate::types::ExtractedMemories;

/// Merges multiple extraction results into a single deduplicated set.
pub(crate) fn merge_all_memories(results: &[ExtractedMemories]) -> ExtractedMemories {
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

pub(crate) fn normalize(s: &str) -> String {
    s.to_lowercase()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

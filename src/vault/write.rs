//! Vault writing: daily digests, topic/people files, identity/preference appending.

use anyhow::Result;

use crate::types::ExtractedMemories;
use crate::vault::config::{identity_dir, people_dir, topics_dir};
use crate::vault::write_digest::write_daily_digest;
use crate::vault::write_entries::{append_entry, append_facts, FactRef};
use crate::vault::write_slug::slugify;

pub use crate::vault::write_types::WriteResult;

/// Writes extracted memories to all vault locations. Returns what was written.
pub fn write_memories_to_vault(memories: &ExtractedMemories, date: &str) -> Result<WriteResult> {
    let mut topics_written = Vec::new();
    let mut people_written = Vec::new();

    write_daily_digest(memories, date)?;

    for fact in &memories.topics {
        let slug = slugify(&fact.topic);
        if slug.is_empty() {
            continue;
        }

        append_entry(
            &topics_dir(),
            &slug,
            &fact.topic,
            &fact.content,
            &fact.date,
            &fact.confidence.to_string(),
            None,
        )?;
        if !topics_written.contains(&slug) {
            topics_written.push(slug);
        }
    }

    for fact in &memories.relationships {
        let slug = slugify(&fact.person);
        if slug.is_empty() {
            continue;
        }

        append_entry(
            &people_dir(),
            &slug,
            &fact.person,
            &fact.content,
            &fact.date,
            &fact.confidence.to_string(),
            fact.role.as_deref(),
        )?;
        if !people_written.contains(&slug) {
            people_written.push(slug);
        }
    }

    if !memories.identity.is_empty() {
        let confidence_strs: Vec<String> = memories
            .identity
            .iter()
            .map(|f| f.confidence.to_string())
            .collect();
        let facts: Vec<FactRef> = memories
            .identity
            .iter()
            .zip(confidence_strs.iter())
            .map(|(f, conf)| FactRef {
                content: &f.content,
                confidence: conf,
                meta: &f.category,
            })
            .collect();
        append_facts(&identity_dir().join("profile.md"), &facts)?;
    }

    if !memories.preferences.is_empty() {
        let confidence_strs: Vec<String> = memories
            .preferences
            .iter()
            .map(|f| f.confidence.to_string())
            .collect();
        let facts: Vec<FactRef> = memories
            .preferences
            .iter()
            .zip(confidence_strs.iter())
            .map(|(f, conf)| FactRef {
                content: &f.content,
                confidence: conf,
                meta: &f.pref_type,
            })
            .collect();
        append_facts(&identity_dir().join("preferences.md"), &facts)?;
    }

    Ok(WriteResult {
        topics_written,
        people_written,
    })
}

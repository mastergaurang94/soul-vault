//! Daily digest writing and digest section rendering.

use anyhow::Result;
use std::fs;

use crate::types::ExtractedMemories;
use crate::vault::config::memories_dir;

pub(crate) fn write_daily_digest(memories: &ExtractedMemories, date: &str) -> Result<()> {
    let dir = memories_dir();
    fs::create_dir_all(&dir)?;
    let file_path = dir.join(format!("{}.md", date));
    let new_content = build_digest_sections(memories);
    if new_content.is_empty() {
        return Ok(());
    }

    if file_path.exists() {
        let existing = fs::read_to_string(&file_path)?;
        fs::write(&file_path, format!("{}\n\n{}", existing, new_content))?;
    } else {
        let header = format!(
            "---\ndate: {}\nsources: [import]\n---\n\n# Daily Memories — {}\n\n",
            date, date
        );
        fs::write(&file_path, format!("{}{}", header, new_content))?;
    }

    Ok(())
}

pub(crate) fn build_digest_sections(m: &ExtractedMemories) -> String {
    let mut sections = Vec::new();

    if !m.decisions.is_empty() {
        let items: Vec<String> = m
            .decisions
            .iter()
            .map(|d| format!("- {}", d.content))
            .collect();
        sections.push(format!("## Decisions\n{}", items.join("\n")));
    }
    if !m.identity.is_empty() {
        let items: Vec<String> = m
            .identity
            .iter()
            .map(|i| format!("- {}", i.content))
            .collect();
        sections.push(format!("## Identity\n{}", items.join("\n")));
    }
    if !m.preferences.is_empty() {
        let items: Vec<String> = m
            .preferences
            .iter()
            .map(|p| format!("- {}", p.content))
            .collect();
        sections.push(format!("## Preferences\n{}", items.join("\n")));
    }
    if !m.topics.is_empty() {
        let items: Vec<String> = m
            .topics
            .iter()
            .map(|t| format!("- **{}**: {}", t.topic, t.content))
            .collect();
        sections.push(format!("## Topics\n{}", items.join("\n")));
    }
    if !m.relationships.is_empty() {
        let items: Vec<String> = m
            .relationships
            .iter()
            .map(|r| {
                let role_str = r
                    .role
                    .as_ref()
                    .map(|role| format!(" ({})", role))
                    .unwrap_or_default();
                format!("- **{}**{}: {}", r.person, role_str, r.content)
            })
            .collect();
        sections.push(format!("## People\n{}", items.join("\n")));
    }
    if !m.emotional_context.is_empty() {
        let items: Vec<String> = m
            .emotional_context
            .iter()
            .map(|e| format!("- {}: {}", e.mood, e.content))
            .collect();
        sections.push(format!("## Emotional Context\n{}", items.join("\n")));
    }

    sections.join("\n\n")
}

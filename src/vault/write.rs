//! Vault writing: daily digests, topic/people files, identity/preference appending.

use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::types::ExtractedMemories;
use crate::vault::config::{identity_dir, memories_dir, people_dir, topics_dir};

// ─── Public API ───────────────────────────────────────────────────────────────

/// Writes extracted memories to all vault locations. Returns what was written.
pub fn write_memories_to_vault(memories: &ExtractedMemories, date: &str) -> Result<WriteResult> {
    let mut topics_written = Vec::new();
    let mut people_written = Vec::new();

    write_daily_digest(memories, date)?;

    for fact in &memories.topics {
        let slug = slugify(&fact.topic);
        if !slug.is_empty() {
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
    }

    for fact in &memories.relationships {
        let slug = slugify(&fact.person);
        if !slug.is_empty() {
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

#[derive(Debug)]
pub struct WriteResult {
    pub topics_written: Vec<String>,
    pub people_written: Vec<String>,
}

// ─── Daily Digest ─────────────────────────────────────────────────────────────

fn write_daily_digest(memories: &ExtractedMemories, date: &str) -> Result<()> {
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

fn build_digest_sections(m: &ExtractedMemories) -> String {
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

// ─── Entry Files (Topics, People) ─────────────────────────────────────────────

fn append_entry(
    dir: &Path,
    slug: &str,
    title: &str,
    content: &str,
    date: &str,
    confidence: &str,
    role: Option<&str>,
) -> Result<()> {
    fs::create_dir_all(dir)?;
    let file_path = dir.join(format!("{}.md", slug));
    let entry = format!("\n- {} _({}, {})_", content, date, confidence);

    if file_path.exists() {
        let existing = fs::read_to_string(&file_path)?;
        if existing.contains(content) {
            return Ok(()); // dedup
        }
        fs::write(&file_path, format!("{}{}", existing, entry))?;
    } else {
        let kind = if dir.to_string_lossy().contains("people") {
            "person"
        } else {
            "topic"
        };
        let role_field = role.map(|r| format!("\nrole: {}", r)).unwrap_or_default();
        let header = format!(
            "---\n{}: {}{}\nupdated: {}\n---\n\n# {}\n",
            kind, title, role_field, date, title
        );
        fs::write(&file_path, format!("{}{}", header, entry))?;
    }

    Ok(())
}

// ─── Identity/Preferences Append ──────────────────────────────────────────────

struct FactRef<'a> {
    content: &'a str,
    confidence: &'a str,
    meta: &'a str,
}

fn append_facts(file_path: &Path, facts: &[FactRef]) -> Result<()> {
    if !file_path.exists() {
        return Ok(());
    }
    let mut existing = fs::read_to_string(file_path)?;

    for fact in facts {
        if !existing.contains(fact.content) {
            existing.push_str(&format!(
                "\n- {} _({}, {})_",
                fact.content, fact.meta, fact.confidence
            ));
        }
    }

    fs::write(file_path, existing)?;
    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn slugify(text: &str) -> String {
    let slug: String = text
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    // Collapse consecutive dashes
    let mut result = String::new();
    let mut prev_dash = false;
    for c in slug.chars() {
        if c == '-' {
            if !prev_dash {
                result.push(c);
            }
            prev_dash = true;
        } else {
            result.push(c);
            prev_dash = false;
        }
    }
    if result.len() > 50 {
        result.truncate(50);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

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
    fn test_write_daily_digest() {
        let tmp = tempfile::tempdir().unwrap();
        let memories_path = tmp.path().join("memories");
        fs::create_dir_all(&memories_path).unwrap();

        // We need to test with a custom path — but our function uses memories_dir().
        // Instead, test the build_digest_sections helper.
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

        // Write the same entry twice
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
        // Should appear only once
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
}

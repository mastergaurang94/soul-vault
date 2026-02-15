//! Context export rendering helpers.

use anyhow::Result;
use std::fs;

use crate::cli::export_types::{include, ExportSection};
use crate::types::{NamedContent, VaultContent};
use crate::ui::theme::*;

pub(crate) fn output_context(
    vault: &VaultContent,
    output_path: Option<&str>,
    topic_filter: Option<&str>,
    sections: &[ExportSection],
) -> Result<()> {
    let doc = build_context_document(vault, topic_filter, sections);
    if let Some(path) = output_path {
        fs::write(path, &doc)?;
        eprintln!("{}", banner());
        eprintln!("{}", check(&format!("Exported to {}", cyan(path))));
        eprintln!(
            "{}",
            dim(&format!(
                "  {} words, {} characters\n",
                count_words(&doc),
                doc.len()
            ))
        );
    } else {
        println!("{doc}");
    }
    Ok(())
}

pub(crate) fn build_context_document(
    vault: &VaultContent,
    topic_filter: Option<&str>,
    selected_sections: &[ExportSection],
) -> String {
    let mut blocks = Vec::new();

    blocks.push("# Soul Vault Memory — Context Export".to_string());
    blocks.push(format!(
        "> Generated: {}\n",
        chrono::Utc::now().format("%Y-%m-%d")
    ));

    if include(selected_sections, ExportSection::Identity) && !vault.identity.trim().is_empty() {
        blocks.push(format!(
            "---\n\n## Identity\n\n{}",
            strip_frontmatter(&vault.identity)
        ));
    }
    if include(selected_sections, ExportSection::Preferences)
        && !vault.preferences.trim().is_empty()
    {
        blocks.push(format!(
            "\n## Preferences\n\n{}",
            strip_frontmatter(&vault.preferences)
        ));
    }

    let topics = filtered_topics(vault, topic_filter);
    if include(selected_sections, ExportSection::Topics) && !topics.is_empty() {
        let mut topic_section = "\n---\n\n## Topics".to_string();
        for t in &topics {
            topic_section.push_str(&format!("\n\n{}", strip_frontmatter(&t.content)));
        }
        blocks.push(topic_section);
    }

    if include(selected_sections, ExportSection::People)
        && topic_filter.is_none()
        && !vault.people.is_empty()
    {
        let mut people_section = "\n---\n\n## People".to_string();
        for p in &vault.people {
            people_section.push_str(&format!("\n\n{}", strip_frontmatter(&p.content)));
        }
        blocks.push(people_section);
    }

    if include(selected_sections, ExportSection::Memories)
        && topic_filter.is_none()
        && !vault.memories.is_empty()
    {
        let mut mem_section = "\n---\n\n## Recent Memories".to_string();
        for m in vault.memories.iter().rev().take(7) {
            mem_section.push_str(&format!("\n\n{}", strip_frontmatter(&m.content)));
        }
        blocks.push(mem_section);
    }

    blocks.join("\n")
}

pub(crate) fn filtered_topics<'a>(
    vault: &'a VaultContent,
    topic_filter: Option<&str>,
) -> Vec<&'a NamedContent> {
    if let Some(filter) = topic_filter {
        let needle = filter.to_lowercase();
        vault
            .topics
            .iter()
            .filter(|t| t.name.to_lowercase().contains(&needle))
            .collect()
    } else {
        vault.topics.iter().collect()
    }
}

fn strip_frontmatter(content: &str) -> String {
    if let Ok(re) = regex::Regex::new(r"^---[\s\S]*?---\s*\n?") {
        re.replace(content, "").trim().to_string()
    } else {
        content.trim().to_string()
    }
}

fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_frontmatter() {
        let input = "---\ndate: 2026-02-14\n---\n\n# Title\n\nContent";
        let result = strip_frontmatter(input);
        assert_eq!(result, "# Title\n\nContent");
    }

    #[test]
    fn test_strip_frontmatter_no_frontmatter() {
        let input = "# Just content\n\nHello";
        let result = strip_frontmatter(input);
        assert_eq!(result, "# Just content\n\nHello");
    }

    #[test]
    fn test_count_words() {
        assert_eq!(count_words("hello world"), 2);
        assert_eq!(count_words("one"), 1);
        assert_eq!(count_words(""), 0);
    }

    #[test]
    fn test_build_context_document_empty() {
        let vault = VaultContent {
            identity: String::new(),
            preferences: String::new(),
            memories: vec![],
            topics: vec![],
            people: vec![],
        };
        let doc = build_context_document(&vault, None, &ExportSection::all());
        assert!(doc.contains("Soul Vault Memory"));
    }

    #[test]
    fn test_build_context_document_with_topic_filter() {
        let vault = VaultContent {
            identity: String::new(),
            preferences: String::new(),
            memories: vec![],
            topics: vec![
                NamedContent {
                    name: "rust".to_string(),
                    content: "# Rust\n\nLearning Rust".to_string(),
                },
                NamedContent {
                    name: "crypto".to_string(),
                    content: "# Crypto\n\nETH and SOL".to_string(),
                },
            ],
            people: vec![],
        };
        let doc = build_context_document(&vault, Some("rust"), &ExportSection::all());
        assert!(doc.contains("Rust"));
        assert!(!doc.contains("Crypto"));
    }

    #[test]
    fn test_build_context_document_with_section_filter() {
        let vault = VaultContent {
            identity: "# Profile\nAlice".to_string(),
            preferences: "# Preferences\nTea".to_string(),
            memories: vec![NamedContent {
                name: "m1".to_string(),
                content: "# M1\nMemory".to_string(),
            }],
            topics: vec![NamedContent {
                name: "rust".to_string(),
                content: "# Rust\nTopic".to_string(),
            }],
            people: vec![NamedContent {
                name: "bob".to_string(),
                content: "# Bob\nFriend".to_string(),
            }],
        };

        let doc = build_context_document(
            &vault,
            None,
            &[ExportSection::Identity, ExportSection::Topics],
        );
        assert!(doc.contains("## Identity"));
        assert!(doc.contains("## Topics"));
        assert!(!doc.contains("## Preferences"));
        assert!(!doc.contains("## People"));
        assert!(!doc.contains("## Recent Memories"));
    }
}

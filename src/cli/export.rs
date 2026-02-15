//! `soul export` — outputs vault as markdown or JSON context document.

use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::types::VaultContent;
use crate::ui::theme::*;
use crate::vault::config::assert_initialized;
use crate::vault::read::read_vault_content;

// ─── Export Command ───────────────────────────────────────────────────────────

pub fn run(output: Option<&str>, format: &str, topic: Option<&str>) -> Result<()> {
    assert_initialized()?;

    let vault = read_vault_content()?;

    if format == "json" {
        return output_json(&vault, output);
    }

    let doc = build_context_document(&vault, topic);

    if let Some(output_path) = output {
        fs::write(Path::new(output_path), &doc)?;
        eprintln!("{}", banner());
        eprintln!("{}", check(&format!("Exported to {}", cyan(output_path))));
        eprintln!(
            "{}",
            dim(&format!(
                "  {} words, {} characters\n",
                count_words(&doc),
                doc.len()
            ))
        );
    } else {
        println!("{}", doc);
    }

    Ok(())
}

// ─── JSON Output ──────────────────────────────────────────────────────────────

fn output_json(vault: &VaultContent, output_path: Option<&str>) -> Result<()> {
    let json = serde_json::to_string_pretty(vault)?;
    if let Some(path) = output_path {
        fs::write(path, &json)?;
        println!("{}", check(&format!("Exported JSON to {}", path)));
    } else {
        println!("{}", json);
    }
    Ok(())
}

// ─── Context Document Builder ─────────────────────────────────────────────────

fn build_context_document(vault: &VaultContent, topic_filter: Option<&str>) -> String {
    let mut sections = Vec::new();

    sections.push("# Soul Vault Memory — Context Export".to_string());
    sections.push(format!(
        "> Generated: {}\n",
        chrono::Utc::now().format("%Y-%m-%d")
    ));

    if !vault.identity.trim().is_empty() {
        sections.push(format!(
            "---\n\n## Identity\n\n{}",
            strip_frontmatter(&vault.identity)
        ));
    }
    if !vault.preferences.trim().is_empty() {
        sections.push(format!(
            "\n## Preferences\n\n{}",
            strip_frontmatter(&vault.preferences)
        ));
    }

    let topics: Vec<&crate::types::NamedContent> = if let Some(filter) = topic_filter {
        vault
            .topics
            .iter()
            .filter(|t| t.name.to_lowercase().contains(&filter.to_lowercase()))
            .collect()
    } else {
        vault.topics.iter().collect()
    };

    if !topics.is_empty() {
        let mut topic_section = "\n---\n\n## Topics".to_string();
        for t in &topics {
            topic_section.push_str(&format!("\n\n{}", strip_frontmatter(&t.content)));
        }
        sections.push(topic_section);
    }

    if topic_filter.is_none() && !vault.people.is_empty() {
        let mut people_section = "\n---\n\n## People".to_string();
        for p in &vault.people {
            people_section.push_str(&format!("\n\n{}", strip_frontmatter(&p.content)));
        }
        sections.push(people_section);
    }

    if topic_filter.is_none() && !vault.memories.is_empty() {
        let mut mem_section = "\n---\n\n## Recent Memories".to_string();
        for m in vault.memories.iter().rev().take(7) {
            mem_section.push_str(&format!("\n\n{}", strip_frontmatter(&m.content)));
        }
        sections.push(mem_section);
    }

    sections.join("\n")
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn strip_frontmatter(content: &str) -> String {
    let re = regex::Regex::new(r"^---[\s\S]*?---\s*\n?").unwrap();
    re.replace(content, "").trim().to_string()
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
        let doc = build_context_document(&vault, None);
        assert!(doc.contains("Soul Vault Memory"));
    }

    #[test]
    fn test_build_context_document_with_topic_filter() {
        use crate::types::NamedContent;
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
        let doc = build_context_document(&vault, Some("rust"));
        assert!(doc.contains("Rust"));
        assert!(!doc.contains("Crypto"));
    }
}

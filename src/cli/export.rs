//! `soul export` — outputs vault as context, JSON, or bundle directory.

use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::types::{NamedContent, VaultContent};
use crate::ui::theme::*;
use crate::vault::config::{
    assert_initialized, identity_dir, memories_dir, people_dir, topics_dir,
};
use crate::vault::read::read_vault_content;

// ─── Export Command ───────────────────────────────────────────────────────────

pub fn run(
    output: Option<&str>,
    format: &str,
    topic: Option<&str>,
    sections: Option<&str>,
) -> Result<()> {
    assert_initialized()?;

    let export_format = ExportFormat::parse(format)?;
    let selected_sections = parse_sections(sections)?;
    let vault = read_vault_content()?;

    match export_format {
        ExportFormat::Context => output_context(&vault, output, topic, &selected_sections),
        ExportFormat::Json => output_json(&vault, output, topic, &selected_sections),
        ExportFormat::Bundle => output_bundle(output, &selected_sections),
    }
}

pub fn smart_default_output_path(format: &str) -> Result<PathBuf> {
    Ok(default_output_path(ExportFormat::parse(format)?))
}

// ─── Export Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportFormat {
    Context,
    Json,
    Bundle,
}

impl ExportFormat {
    fn parse(value: &str) -> Result<Self> {
        match value.trim().to_lowercase().as_str() {
            "context" | "markdown" => Ok(Self::Context),
            "json" => Ok(Self::Json),
            "bundle" => Ok(Self::Bundle),
            other => bail!(
                "Unsupported export format: {other}\n      → Use one of: context, json, bundle."
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportSection {
    Identity,
    Preferences,
    Topics,
    People,
    Memories,
}

impl ExportSection {
    fn all() -> Vec<Self> {
        vec![
            Self::Identity,
            Self::Preferences,
            Self::Topics,
            Self::People,
            Self::Memories,
        ]
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "identity" => Some(Self::Identity),
            "preferences" => Some(Self::Preferences),
            "topics" => Some(Self::Topics),
            "people" => Some(Self::People),
            "memories" => Some(Self::Memories),
            _ => None,
        }
    }
}

fn parse_sections(raw: Option<&str>) -> Result<Vec<ExportSection>> {
    let Some(raw) = raw else {
        return Ok(ExportSection::all());
    };
    if raw.trim().is_empty() {
        return Ok(ExportSection::all());
    }

    let mut parsed = Vec::new();
    for token in raw.split(',') {
        let name = token.trim().to_lowercase();
        if name.is_empty() {
            continue;
        }
        let Some(section) = ExportSection::parse(&name) else {
            bail!(
                "Invalid section: {name}\n      → Use sections from: identity,preferences,topics,people,memories."
            );
        };
        if !parsed.contains(&section) {
            parsed.push(section);
        }
    }
    if parsed.is_empty() {
        return Ok(ExportSection::all());
    }
    Ok(parsed)
}

fn include(sections: &[ExportSection], section: ExportSection) -> bool {
    sections.contains(&section)
}

// ─── Context Output ───────────────────────────────────────────────────────────

fn output_context(
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

// ─── JSON Output ──────────────────────────────────────────────────────────────

fn output_json(
    vault: &VaultContent,
    output_path: Option<&str>,
    topic_filter: Option<&str>,
    sections: &[ExportSection],
) -> Result<()> {
    let mut doc = serde_json::Map::new();

    if include(sections, ExportSection::Identity) {
        doc.insert(
            "identity".to_string(),
            serde_json::Value::String(vault.identity.clone()),
        );
    }
    if include(sections, ExportSection::Preferences) {
        doc.insert(
            "preferences".to_string(),
            serde_json::Value::String(vault.preferences.clone()),
        );
    }
    if include(sections, ExportSection::Topics) {
        let topics = filtered_topics(vault, topic_filter);
        doc.insert("topics".to_string(), serde_json::to_value(topics)?);
    }
    if include(sections, ExportSection::People) && topic_filter.is_none() {
        doc.insert("people".to_string(), serde_json::to_value(&vault.people)?);
    }
    if include(sections, ExportSection::Memories) && topic_filter.is_none() {
        doc.insert(
            "memories".to_string(),
            serde_json::to_value(&vault.memories)?,
        );
    }

    let json = serde_json::to_string_pretty(&doc)?;
    if let Some(path) = output_path {
        fs::write(path, &json)?;
        eprintln!("{}", check(&format!("Exported JSON to {}", cyan(path))));
    } else {
        println!("{json}");
    }
    Ok(())
}

// ─── Bundle Output ────────────────────────────────────────────────────────────

struct BundlePaths {
    identity: PathBuf,
    topics: PathBuf,
    people: PathBuf,
    memories: PathBuf,
}

fn output_bundle(output_path: Option<&str>, sections: &[ExportSection]) -> Result<()> {
    let destination = output_path
        .map(PathBuf::from)
        .unwrap_or_else(|| default_output_path(ExportFormat::Bundle));
    let paths = BundlePaths {
        identity: identity_dir(),
        topics: topics_dir(),
        people: people_dir(),
        memories: memories_dir(),
    };
    create_bundle_dir(&paths, &destination, sections)?;
    eprintln!("{}", banner());
    eprintln!(
        "{}",
        check(&format!(
            "Bundle export created at {}",
            cyan(&destination.display().to_string())
        ))
    );
    eprintln!(
        "{}",
        dim("  Tip: zip this folder if you need a single archive.\n")
    );
    Ok(())
}

fn create_bundle_dir(
    paths: &BundlePaths,
    destination: &Path,
    sections: &[ExportSection],
) -> Result<()> {
    if destination.exists() {
        bail!(
            "Bundle destination already exists: {}\n      → Choose a different `--output` path.",
            destination.display()
        );
    }
    fs::create_dir_all(destination)?;

    if include(sections, ExportSection::Identity) {
        copy_file_if_exists(
            &paths.identity.join("profile.md"),
            &destination.join("identity").join("profile.md"),
        )?;
    }
    if include(sections, ExportSection::Preferences) {
        copy_file_if_exists(
            &paths.identity.join("preferences.md"),
            &destination.join("identity").join("preferences.md"),
        )?;
    }
    if include(sections, ExportSection::Topics) {
        copy_markdown_dir(&paths.topics, &destination.join("topics"))?;
    }
    if include(sections, ExportSection::People) {
        copy_markdown_dir(&paths.people, &destination.join("people"))?;
    }
    if include(sections, ExportSection::Memories) {
        copy_markdown_dir(&paths.memories, &destination.join("memories"))?;
    }

    Ok(())
}

fn copy_markdown_dir(source: &Path, destination: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|ext| ext == "md") {
            fs::copy(entry.path(), destination.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn copy_file_if_exists(source: &Path, destination: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(())
}

// ─── Context Document Builder ─────────────────────────────────────────────────

fn build_context_document(
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

fn filtered_topics<'a>(
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

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn strip_frontmatter(content: &str) -> String {
    let re = regex::Regex::new(r"^---[\s\S]*?---\s*\n?").unwrap();
    re.replace(content, "").trim().to_string()
}

fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

fn default_output_path(format: ExportFormat) -> PathBuf {
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    match format {
        ExportFormat::Context => base.join(format!("soul-vault-export-{date}.md")),
        ExportFormat::Json => base.join(format!("soul-vault-export-{date}.json")),
        ExportFormat::Bundle => base.join(format!("soul-vault-export-{date}")),
    }
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

    #[test]
    fn test_parse_sections_defaults_to_all() {
        let sections = parse_sections(None).unwrap();
        assert_eq!(sections, ExportSection::all());
    }

    #[test]
    fn test_parse_sections_csv() {
        let sections = parse_sections(Some("identity,topics,topics")).unwrap();
        assert_eq!(
            sections,
            vec![ExportSection::Identity, ExportSection::Topics]
        );
    }

    #[test]
    fn test_bundle_directory_creation() {
        let tmp = tempfile::tempdir().unwrap();
        let identity = tmp.path().join("identity");
        let topics = tmp.path().join("topics");
        let people = tmp.path().join("people");
        let memories = tmp.path().join("memories");
        fs::create_dir_all(&identity).unwrap();
        fs::create_dir_all(&topics).unwrap();
        fs::create_dir_all(&people).unwrap();
        fs::create_dir_all(&memories).unwrap();
        fs::write(identity.join("profile.md"), "profile").unwrap();
        fs::write(identity.join("preferences.md"), "prefs").unwrap();
        fs::write(topics.join("rust.md"), "rust").unwrap();
        fs::write(people.join("alice.md"), "alice").unwrap();
        fs::write(memories.join("today.md"), "today").unwrap();

        let destination = tmp.path().join("bundle");
        let paths = BundlePaths {
            identity,
            topics,
            people,
            memories,
        };
        create_bundle_dir(&paths, &destination, &ExportSection::all()).unwrap();

        assert!(destination.join("identity/profile.md").exists());
        assert!(destination.join("identity/preferences.md").exists());
        assert!(destination.join("topics/rust.md").exists());
        assert!(destination.join("people/alice.md").exists());
        assert!(destination.join("memories/today.md").exists());
    }
}

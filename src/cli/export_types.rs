//! Export command parsing and section/type selection.

use anyhow::{bail, Result};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExportFormat {
    Context,
    Json,
    Bundle,
}

impl ExportFormat {
    pub(crate) fn parse(value: &str) -> Result<Self> {
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
    pub(crate) fn all() -> Vec<Self> {
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

pub(crate) fn parse_sections(raw: Option<&str>) -> Result<Vec<ExportSection>> {
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

pub(crate) fn include(sections: &[ExportSection], section: ExportSection) -> bool {
    sections.contains(&section)
}

pub(crate) fn default_output_path(format: ExportFormat) -> PathBuf {
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let downloads = home.join("Downloads");
    let base = if downloads.is_dir() { downloads } else { home };
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
}

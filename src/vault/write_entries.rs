//! Topic/people entry append helpers and identity/preferences fact appends.

use anyhow::Result;
use std::fs;
use std::path::Path;

pub(crate) struct FactRef<'a> {
    pub content: &'a str,
    pub confidence: &'a str,
    pub meta: &'a str,
}

pub(crate) fn append_entry(
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
            return Ok(());
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

pub(crate) fn append_facts(file_path: &Path, facts: &[FactRef]) -> Result<()> {
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

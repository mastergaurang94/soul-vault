//! JSON export rendering helpers.

use anyhow::Result;
use std::fs;

use crate::cli::export_context::filtered_topics;
use crate::cli::export_types::{include, ExportSection};
use crate::types::VaultContent;
use crate::ui::theme::*;

pub(crate) fn output_json(
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

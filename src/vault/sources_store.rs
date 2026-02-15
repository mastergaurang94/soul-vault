//! Read/write and summary helpers for `sources.json`.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::vault::config::config_dir;
use crate::vault::sources_types::{SourceSummary, SourcesConfig};

pub fn sources_config_path() -> PathBuf {
    config_dir().join("sources.json")
}

/// Reads sources.json. Returns empty config if missing.
pub fn read_sources() -> Result<SourcesConfig> {
    let path = sources_config_path();
    if !path.exists() {
        return Ok(SourcesConfig {
            sources: Vec::new(),
        });
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let config: SourcesConfig =
        serde_json::from_str(&raw).with_context(|| "Failed to parse sources.json")?;
    Ok(config)
}

/// Writes sources.json with pretty formatting.
pub fn write_sources(config: &SourcesConfig) -> Result<()> {
    let path = sources_config_path();
    fs::create_dir_all(config_dir())?;
    let json = serde_json::to_string_pretty(config)?;
    fs::write(&path, json).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Returns summary info about tracked sources for the status command.
pub fn get_source_stats() -> Result<Vec<SourceSummary>> {
    let sources = read_sources()?;
    Ok(sources
        .sources
        .iter()
        .map(|s| SourceSummary {
            path: s.path.clone(),
            files_ingested: s.files_ingested,
            last_ingested: s.last_ingested.clone(),
        })
        .collect())
}

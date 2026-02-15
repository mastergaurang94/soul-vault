//! Source tracking and provider sync metadata helpers for pull/import.

use anyhow::Result;

use crate::adapters::SessionFile;
use crate::types::Provider;
use crate::vault::config::{read_config, write_config};
use crate::vault::sources::{compute_file_hash, read_sources, write_sources, SourceEntry};

const PULL_SOURCE_KEY: &str = "soul-pull";

pub(crate) fn filter_new_sessions(sessions: Vec<SessionFile>) -> Result<(Vec<SessionFile>, usize)> {
    let sources = read_sources()?;
    let pull_entry = sources.sources.iter().find(|s| s.path == PULL_SOURCE_KEY);

    let existing_hashes: std::collections::HashMap<String, String> = pull_entry
        .map(|e| e.file_hashes.clone())
        .unwrap_or_default();

    let mut to_import = Vec::new();
    let mut skipped = 0;

    for session in sessions {
        let path_key = session.path.to_string_lossy().to_string();
        match existing_hashes.get(&path_key) {
            Some(old_hash) => {
                if let Ok(current_hash) = compute_file_hash(&session.path) {
                    if current_hash == *old_hash {
                        skipped += 1;
                        continue;
                    }
                }
                to_import.push(session);
            }
            None => to_import.push(session),
        }
    }

    Ok((to_import, skipped))
}

pub(crate) fn update_pull_tracking(sessions: &[SessionFile]) -> Result<()> {
    let mut sources = read_sources()?;

    let mut file_hashes = sources
        .sources
        .iter()
        .find(|s| s.path == PULL_SOURCE_KEY)
        .map(|e| e.file_hashes.clone())
        .unwrap_or_default();

    for session in sessions {
        let path_key = session.path.to_string_lossy().to_string();
        if let Ok(hash) = compute_file_hash(&session.path) {
            file_hashes.insert(path_key, hash);
        }
    }

    let now = chrono::Utc::now().to_rfc3339();

    if let Some(entry) = sources
        .sources
        .iter_mut()
        .find(|s| s.path == PULL_SOURCE_KEY)
    {
        entry.files_ingested = file_hashes.len();
        entry.last_ingested = now;
        entry.file_hashes = file_hashes;
    } else {
        sources.sources.push(SourceEntry {
            path: PULL_SOURCE_KEY.to_string(),
            files_ingested: file_hashes.len(),
            last_ingested: now,
            file_hashes,
        });
    }

    write_sources(&sources)?;
    Ok(())
}

pub(crate) fn update_pull_config_timestamps(discovered_providers: &[Provider]) -> Result<()> {
    let mut config = read_config()?;
    let now = chrono::Utc::now().to_rfc3339();

    config.last_sync = Some(now.clone());
    for provider in &mut config.providers {
        if discovered_providers.contains(&provider.name) {
            provider.last_import = Some(now.clone());
        }
    }

    write_config(&config)
}

//! Provider discovery for gemini adapter.
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::adapters::SessionFile;

pub(super) fn discover_sessions() -> Result<Vec<SessionFile>> {
    let base = gemini_tmp_dir()?;
    if !base.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    for project_entry in fs::read_dir(&base)? {
        let project_entry = project_entry?;
        let project_dir = project_entry.path();
        if !project_dir.is_dir() {
            continue;
        }

        let project_hash = project_entry.file_name().to_string_lossy().to_string();
        let chats_dir = project_dir.join("chats");
        if !chats_dir.exists() {
            continue;
        }

        for file_entry in fs::read_dir(&chats_dir)? {
            let file_entry = file_entry?;
            let path = file_entry.path();
            if !is_session_file(&path) {
                continue;
            }

            let modified = file_entry
                .metadata()
                .map(|m| m.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH))
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

            sessions.push(SessionFile {
                path,
                provider: "gemini".to_string(),
                project: Some(project_hash.clone()),
                modified,
            });
        }
    }

    Ok(sessions)
}

fn gemini_tmp_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".gemini").join("tmp"))
}

pub(super) fn is_session_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    name.starts_with("session-") && name.ends_with(".json")
}

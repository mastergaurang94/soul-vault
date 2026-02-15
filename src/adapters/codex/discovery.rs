//! Provider discovery for codex adapter.
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::adapters::SessionFile;

pub(super) fn discover_sessions() -> Result<Vec<SessionFile>> {
    let base = codex_sessions_dir()?;
    if !base.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    discover_recursive(&base, &mut sessions)?;
    Ok(sessions)
}

fn codex_sessions_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".codex").join("sessions"))
}

pub(super) fn discover_recursive(dir: &Path, sessions: &mut Vec<SessionFile>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            discover_recursive(&path, sessions)?;
        } else if is_rollout_file(&path) {
            let modified = entry
                .metadata()
                .map(|m| m.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH))
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

            sessions.push(SessionFile {
                path,
                provider: "codex".to_string(),
                project: None,
                modified,
            });
        }
    }

    Ok(())
}

pub(super) fn is_rollout_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    name.starts_with("rollout-") && name.ends_with(".jsonl")
}

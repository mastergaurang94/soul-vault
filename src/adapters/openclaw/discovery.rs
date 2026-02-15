use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::adapters::SessionFile;

pub(super) fn discover_sessions() -> Result<Vec<SessionFile>> {
    let base = openclaw_agents_dir()?;
    if !base.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    for agent_entry in fs::read_dir(&base)? {
        let agent_entry = agent_entry?;
        let agent_dir = agent_entry.path();
        if !agent_dir.is_dir() {
            continue;
        }

        let agent_name = agent_entry.file_name().to_string_lossy().to_string();
        let sessions_dir = agent_dir.join("sessions");
        if !sessions_dir.exists() {
            continue;
        }

        for file_entry in fs::read_dir(&sessions_dir)? {
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
                provider: "openclaw".to_string(),
                project: Some(agent_name.clone()),
                modified,
            });
        }
    }

    Ok(sessions)
}

fn openclaw_agents_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".openclaw").join("agents"))
}

pub(super) fn is_session_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    name.ends_with(".jsonl") && !name.contains(".deleted.")
}

//! Provider discovery for claude adapter.
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::adapters::SessionFile;

pub(super) fn discover_sessions() -> Result<Vec<SessionFile>> {
    let base = claude_projects_dir()?;
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

        let project_name = decode_project_path(&project_entry.file_name().to_string_lossy());

        for file_entry in fs::read_dir(&project_dir)? {
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
                provider: "claude".to_string(),
                project: Some(project_name.clone()),
                modified,
            });
        }
    }

    Ok(sessions)
}

fn claude_projects_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".claude").join("projects"))
}

pub(super) fn is_session_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str());
    if ext != Some("jsonl") {
        return false;
    }

    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    !name.starts_with("agent-")
}

pub(super) fn decode_project_path(dir_name: &str) -> String {
    if let Some(stripped) = dir_name.strip_prefix('-') {
        format!("/{}", stripped.replace('-', "/"))
    } else {
        dir_name.replace('-', "/")
    }
}

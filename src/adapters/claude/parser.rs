//! Session parsing for claude adapter.
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;

use super::discovery::decode_project_path;
use crate::adapters::{Conversation, Message, Role};

pub(super) fn parse_claude_session(path: &Path) -> Result<Conversation> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let session_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut messages = Vec::new();
    let mut created_at: Option<DateTime<Utc>> = None;
    let mut cwd: Option<String> = None;
    let mut title: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let val: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let msg_type = val.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match msg_type {
            "user" | "human" => {
                if let Some(msg) = extract_message(&val, Role::User) {
                    if created_at.is_none() {
                        created_at = extract_timestamp(&val);
                    }
                    if cwd.is_none() {
                        cwd = val.get("cwd").and_then(|c| c.as_str()).map(String::from);
                    }
                    messages.push(msg);
                }
            }
            "assistant" => {
                if let Some(msg) = extract_message(&val, Role::Assistant) {
                    messages.push(msg);
                }
            }
            "system" => {
                if cwd.is_none() {
                    cwd = val.get("cwd").and_then(|c| c.as_str()).map(String::from);
                }
                if created_at.is_none() {
                    created_at = extract_timestamp(&val);
                }
            }
            "summary" => {
                if let Some(summary) = val.get("summary").and_then(|s| s.as_str()) {
                    title = Some(truncate(summary, 80));
                }
            }
            _ => {}
        }
    }

    if title.is_none() {
        title = messages
            .iter()
            .find(|m| m.role == Role::User)
            .map(|m| truncate(&m.content, 80));
    }

    let project = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(decode_project_path);

    Ok(Conversation {
        id: session_id,
        title,
        provider: format!("Claude Code{}", project_suffix(&project, &cwd)),
        created_at,
        messages,
    })
}

pub(super) fn extract_message(val: &serde_json::Value, role: Role) -> Option<Message> {
    let msg = val.get("message")?;
    let content = extract_content(msg)?;
    if content.trim().is_empty() {
        return None;
    }

    Some(Message {
        role,
        content,
        timestamp: extract_timestamp(val),
    })
}

pub(super) fn extract_content(msg: &serde_json::Value) -> Option<String> {
    let content = msg.get("content")?;

    if let Some(s) = content.as_str() {
        return Some(s.to_string());
    }

    if let Some(arr) = content.as_array() {
        let texts: Vec<String> = arr
            .iter()
            .filter_map(|block| {
                let block_type = block.get("type").and_then(|t| t.as_str())?;
                match block_type {
                    "text" => block.get("text").and_then(|t| t.as_str()).map(String::from),
                    _ => None,
                }
            })
            .collect();
        if texts.is_empty() {
            return None;
        }
        return Some(texts.join("\n"));
    }

    None
}

pub(super) fn extract_timestamp(val: &serde_json::Value) -> Option<DateTime<Utc>> {
    val.get("timestamp")
        .and_then(|t| t.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

pub(super) fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.min(s.len())])
    }
}

fn project_suffix(project: &Option<String>, cwd: &Option<String>) -> String {
    if let Some(p) = project {
        format!(" ({})", p)
    } else if let Some(c) = cwd {
        format!(" ({})", c)
    } else {
        String::new()
    }
}

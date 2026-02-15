//! Session parsing for openclaw adapter.
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;

use crate::adapters::{Conversation, Message, Role};

pub(super) fn parse_openclaw_session(path: &Path) -> Result<Conversation> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let session_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut messages = Vec::new();
    let mut created_at: Option<DateTime<Utc>> = None;

    let mut agent_name: Option<String> = None;
    if let Some(parent) = path.parent() {
        if let Some(agents_dir) = parent.parent() {
            agent_name = agents_dir
                .file_name()
                .and_then(|n| n.to_str())
                .map(String::from);
        }
    }

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
            "session" => {
                if created_at.is_none() {
                    created_at = extract_timestamp(&val);
                }
            }
            "message" => {
                if let Some(msg) = extract_openclaw_message(&val) {
                    if created_at.is_none() {
                        created_at = extract_timestamp(&val);
                    }
                    messages.push(msg);
                }
            }
            _ => {}
        }
    }

    let title = messages
        .iter()
        .find(|m| m.role == Role::User)
        .map(|m| truncate(&m.content, 80));

    let provider = match &agent_name {
        Some(name) => format!("OpenClaw ({})", name),
        None => "OpenClaw".to_string(),
    };

    Ok(Conversation {
        id: session_id,
        title,
        provider,
        created_at,
        messages,
    })
}

pub(super) fn extract_openclaw_message(val: &serde_json::Value) -> Option<Message> {
    let msg = val.get("message")?;
    let role_str = msg.get("role").and_then(|r| r.as_str())?;

    let role = match role_str {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "system" => Role::System,
        "toolResult" | "tool" => Role::Tool,
        _ => return None,
    };

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

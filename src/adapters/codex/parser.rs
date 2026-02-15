use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;

use crate::adapters::{Conversation, Message, Role};

pub(super) fn parse_codex_session(path: &Path) -> Result<Conversation> {
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

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let val: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let line_type = val.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let timestamp = extract_timestamp(&val);

        match line_type {
            "session_meta" => {
                if let Some(payload) = val.get("payload") {
                    if cwd.is_none() {
                        cwd = payload
                            .get("cwd")
                            .and_then(|c| c.as_str())
                            .map(String::from);
                    }
                    if created_at.is_none() {
                        created_at = payload
                            .get("timestamp")
                            .and_then(|t| t.as_str())
                            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.with_timezone(&Utc));
                    }
                }
                if created_at.is_none() {
                    created_at = timestamp;
                }
            }
            "event_msg" => {
                if let Some(payload) = val.get("payload") {
                    let event_type = payload.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    if event_type == "user_message" {
                        if let Some(msg_text) = payload.get("message").and_then(|m| m.as_str()) {
                            if !msg_text.trim().is_empty() {
                                messages.push(Message {
                                    role: Role::User,
                                    content: msg_text.to_string(),
                                    timestamp,
                                });
                            }
                        }
                    }
                }
            }
            "response_item" => {
                if let Some(msg) = extract_codex_response(&val) {
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

    let provider = match &cwd {
        Some(c) => format!("Codex ({})", c),
        None => "Codex".to_string(),
    };

    Ok(Conversation {
        id: session_id,
        title,
        provider,
        created_at,
        messages,
    })
}

pub(super) fn extract_codex_response(val: &serde_json::Value) -> Option<Message> {
    let payload = val.get("payload")?;
    let payload_type = payload.get("type").and_then(|t| t.as_str())?;
    if payload_type != "message" {
        return None;
    }

    let role_str = payload.get("role").and_then(|r| r.as_str())?;
    if role_str != "assistant" {
        return None;
    }

    let content = extract_response_content(payload)?;
    if content.trim().is_empty() {
        return None;
    }

    Some(Message {
        role: Role::Assistant,
        content,
        timestamp: extract_timestamp(val),
    })
}

pub(super) fn extract_response_content(payload: &serde_json::Value) -> Option<String> {
    let content = payload.get("content")?;

    if let Some(s) = content.as_str() {
        if s.is_empty() {
            return None;
        }
        return Some(s.to_string());
    }

    if let Some(arr) = content.as_array() {
        let texts: Vec<String> = arr
            .iter()
            .filter_map(|block| {
                let block_type = block.get("type").and_then(|t| t.as_str())?;
                match block_type {
                    "output_text" | "text" => {
                        block.get("text").and_then(|t| t.as_str()).map(String::from)
                    }
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

//! Session parsing for gemini adapter.
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;

use crate::adapters::{Conversation, Message, Role};

pub(super) fn parse_gemini_session(path: &Path) -> Result<Conversation> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let val: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON: {}", path.display()))?;

    let session_id = val
        .get("sessionId")
        .and_then(|s| s.as_str())
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
        })
        .to_string();

    let created_at = val
        .get("startTime")
        .and_then(|t| t.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let messages_arr = val
        .get("messages")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();

    let messages: Vec<Message> = messages_arr
        .iter()
        .filter_map(extract_gemini_message)
        .collect();

    let title = messages
        .iter()
        .find(|m| m.role == Role::User)
        .map(|m| truncate(&m.content, 80));

    Ok(Conversation {
        id: session_id,
        title,
        provider: "Gemini CLI".to_string(),
        created_at,
        messages,
    })
}

pub(super) fn extract_gemini_message(val: &serde_json::Value) -> Option<Message> {
    let msg_type = val.get("type").and_then(|t| t.as_str())?;

    let role = match msg_type {
        "user" => Role::User,
        "gemini" => Role::Assistant,
        "system" => Role::System,
        _ => return None,
    };

    let content = extract_content(val)?;
    if content.trim().is_empty() {
        return None;
    }

    let timestamp = val
        .get("timestamp")
        .and_then(|t| t.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    Some(Message {
        role,
        content,
        timestamp,
    })
}

pub(super) fn extract_content(val: &serde_json::Value) -> Option<String> {
    let content = val.get("content")?;

    if let Some(s) = content.as_str() {
        if s.is_empty() {
            return None;
        }
        return Some(s.to_string());
    }

    if let Some(arr) = content.as_array() {
        let texts: Vec<String> = arr
            .iter()
            .filter_map(|block| block.get("text").and_then(|t| t.as_str()).map(String::from))
            .collect();
        if texts.is_empty() {
            return None;
        }
        return Some(texts.join("\n"));
    }

    None
}

pub(super) fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.min(s.len())])
    }
}

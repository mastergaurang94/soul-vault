//! ChatGPT conversations.json parsing and tree traversal.

use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

use crate::vault::chatgpt_types::{timestamp_to_datetime, ParsedConversation, ParsedMessage};

/// Parses a ChatGPT export zip file into conversations.
pub fn parse_chatgpt_zip(path: &Path) -> Result<Vec<ParsedConversation>> {
    let file =
        fs::File::open(path).with_context(|| format!("Failed to open zip: {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Failed to read zip archive: {}", path.display()))?;

    let mut json_content = None;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name == "conversations.json" || name.ends_with("/conversations.json") {
            let mut buf = String::new();
            entry
                .read_to_string(&mut buf)
                .with_context(|| "Failed to read conversations.json from zip")?;
            json_content = Some(buf);
            break;
        }
    }

    let content = json_content
        .ok_or_else(|| anyhow::anyhow!("No conversations.json found in zip archive"))?;

    parse_conversations_json_str(&content)
}

/// Parses a `conversations.json` file from disk.
pub fn parse_chatgpt_json(path: &Path) -> Result<Vec<ParsedConversation>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read: {}", path.display()))?;
    parse_conversations_json_str(&content)
}

/// Parses the raw JSON string of a conversations.json export.
pub(crate) fn parse_conversations_json_str(json_str: &str) -> Result<Vec<ParsedConversation>> {
    let value: Value =
        serde_json::from_str(json_str).with_context(|| "Failed to parse conversations JSON")?;

    let arr = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Expected conversations.json to be a JSON array"))?;

    let mut conversations = Vec::with_capacity(arr.len());
    for item in arr {
        match parse_conversation(item) {
            Ok(conv) if !conv.messages.is_empty() => conversations.push(conv),
            Ok(_) => {}
            Err(e) => eprintln!("  [warn] Skipping conversation: {}", e),
        }
    }

    Ok(conversations)
}

/// Parses a single conversation object from the ChatGPT export format.
pub fn parse_conversation(value: &Value) -> Result<ParsedConversation> {
    let title = value
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("Untitled conversation")
        .to_string();

    let created_at = value
        .get("create_time")
        .and_then(|t| t.as_f64())
        .and_then(timestamp_to_datetime);

    let mapping = value
        .get("mapping")
        .and_then(|m| m.as_object())
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid mapping in conversation: {}", title))?;

    let nodes: HashMap<&str, &Value> = mapping.iter().map(|(k, v)| (k.as_str(), v)).collect();
    let root_ids: Vec<&str> = mapping
        .iter()
        .filter(|(_, node)| node.get("parent").is_none_or(|p| p.is_null()))
        .map(|(id, _)| id.as_str())
        .collect();

    let mut messages = Vec::new();
    for root_id in root_ids {
        collect_messages_dfs(root_id, &nodes, &mut messages);
    }

    Ok(ParsedConversation {
        title,
        created_at,
        messages,
    })
}

fn collect_messages_dfs(
    node_id: &str,
    nodes: &HashMap<&str, &Value>,
    messages: &mut Vec<ParsedMessage>,
) {
    let node = match nodes.get(node_id) {
        Some(n) => n,
        None => return,
    };

    if let Some(msg) = node.get("message") {
        if !msg.is_null() {
            if let Some(parsed) = extract_message(msg) {
                messages.push(parsed);
            }
        }
    }

    if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
        for child_id in children {
            if let Some(id) = child_id.as_str() {
                collect_messages_dfs(id, nodes, messages);
            }
        }
    }
}

fn extract_message(msg: &Value) -> Option<ParsedMessage> {
    let role = msg
        .pointer("/author/role")
        .and_then(|r| r.as_str())
        .unwrap_or("unknown")
        .to_string();

    if role == "system" {
        return None;
    }

    let parts = msg.pointer("/content/parts").and_then(|p| p.as_array())?;
    let text_parts: Vec<&str> = parts
        .iter()
        .filter_map(|p| p.as_str())
        .filter(|s| !s.is_empty())
        .collect();

    if text_parts.is_empty() {
        return None;
    }

    let timestamp = msg
        .get("create_time")
        .and_then(|t| t.as_f64())
        .and_then(timestamp_to_datetime);

    Some(ParsedMessage {
        role,
        content: text_parts.join("\n"),
        timestamp,
    })
}

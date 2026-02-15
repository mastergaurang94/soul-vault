//! Local file content extraction and normalization.

use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::types::FileInfo;
use crate::vault::chatgpt;

/// Reads and normalizes file content based on extension.
pub fn extract_file_content(file: &FileInfo) -> Result<String> {
    match file.extension.as_str() {
        ".zip" => {
            if chatgpt::is_chatgpt_zip(&file.path) {
                let convs = chatgpt::parse_chatgpt_zip(&file.path)?;
                Ok(chatgpt::format_conversations(&convs))
            } else {
                anyhow::bail!("Unsupported zip format (not a ChatGPT export)")
            }
        }
        ".json" => {
            if is_chatgpt_conversations_file(&file.path) {
                let convs = chatgpt::parse_chatgpt_json(&file.path)?;
                Ok(chatgpt::format_conversations(&convs))
            } else {
                extract_json(&file.path)
            }
        }
        ".jsonl" => extract_jsonl(&file.path),
        _ => Ok(fs::read_to_string(&file.path)?.trim().to_string()),
    }
}

pub(crate) fn is_chatgpt_conversations_file(path: &Path) -> bool {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if file_name != "conversations.json" {
        return false;
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return false,
    };

    match value.as_array() {
        Some(arr) => arr
            .first()
            .is_some_and(|item| item.get("mapping").is_some()),
        None => false,
    }
}

fn extract_jsonl(file_path: &Path) -> Result<String> {
    let content = fs::read_to_string(file_path)?;
    let lines: Vec<String> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(
            |line| match serde_json::from_str::<serde_json::Value>(line) {
                Ok(val) if val.is_string() => val.as_str().unwrap_or(line).to_string(),
                Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|_| line.to_string()),
                Err(_) => line.to_string(),
            },
        )
        .collect();
    Ok(lines.join("\n"))
}

fn extract_json(file_path: &Path) -> Result<String> {
    let content = fs::read_to_string(file_path)?;
    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Ok(content),
    };

    if let Some(arr) = parsed.as_array() {
        let formatted: Vec<String> = arr.iter().map(format_conversation).collect();
        Ok(formatted.join("\n\n---\n\n"))
    } else if is_conversation_object(&parsed) {
        Ok(format_conversation(&parsed))
    } else {
        Ok(serde_json::to_string_pretty(&parsed)?)
    }
}

fn is_conversation_object(obj: &serde_json::Value) -> bool {
    obj.is_object()
        && (obj.get("messages").is_some()
            || obj.get("conversation").is_some()
            || obj.get("mapping").is_some())
}

fn format_conversation(item: &serde_json::Value) -> String {
    let obj = match item.as_object() {
        Some(o) => o,
        None => return serde_json::to_string_pretty(item).unwrap_or_default(),
    };

    if let Some(messages) = obj.get("messages").and_then(|m| m.as_array()) {
        let title = obj
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or("Untitled conversation");
        let body: Vec<String> = messages
            .iter()
            .map(|m| {
                let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("unknown");
                let content = m.get("content").and_then(|c| c.as_str()).unwrap_or("");
                format!("{}: {}", role, content)
            })
            .collect();
        return format!("## {}\n\n{}", title, body.join("\n"));
    }

    if let (Some(title), Some(mapping)) = (
        obj.get("title").and_then(|t| t.as_str()),
        obj.get("mapping").and_then(|m| m.as_object()),
    ) {
        let extracted = extract_chatgpt_mapping(mapping);
        return format!("## {}\n\n{}", title, extracted);
    }

    serde_json::to_string_pretty(item).unwrap_or_default()
}

fn extract_chatgpt_mapping(mapping: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut messages = Vec::new();

    for node in mapping.values() {
        let msg = match node.get("message") {
            Some(m) if !m.is_null() => m,
            _ => continue,
        };
        let role = msg
            .pointer("/author/role")
            .and_then(|r| r.as_str())
            .unwrap_or("unknown");
        let parts = msg.pointer("/content/parts").and_then(|p| p.as_array());
        if let Some(parts) = parts {
            let text: Vec<&str> = parts
                .iter()
                .filter_map(|p| p.as_str())
                .filter(|s| !s.is_empty())
                .collect();
            if !text.is_empty() {
                messages.push(format!("{}: {}", role, text.join(" ")));
            }
        }
    }

    messages.join("\n")
}

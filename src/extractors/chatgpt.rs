//! ChatGPT export parser — handles the mapping-based conversation format.
//!
//! ChatGPT data exports contain a zip with `conversations.json` (plus other
//! files we ignore). The conversations use a tree-based `mapping` structure
//! where each node has an optional message and child pointers.
//!
//! This module provides:
//! - `parse_chatgpt_zip` — parse a ChatGPT export zip file
//! - `parse_chatgpt_json` — parse a `conversations.json` file directly
//! - `is_chatgpt_zip` — detect whether a zip is a ChatGPT export
//! - `is_chatgpt_export_dir` — detect an extracted ChatGPT export folder

use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

// ─── Types ────────────────────────────────────────────────────────────────────

/// A parsed ChatGPT conversation with ordered messages.
#[derive(Debug, Clone)]
pub struct ParsedConversation {
    pub title: String,
    /// When the conversation was created (from `create_time` field).
    /// Used by adapters and downstream processing.
    #[allow(dead_code)]
    pub created_at: Option<DateTime<Utc>>,
    pub messages: Vec<ParsedMessage>,
}

/// A single message extracted from a ChatGPT conversation tree.
#[derive(Debug, Clone)]
pub struct ParsedMessage {
    pub role: String,
    pub content: String,
    /// When the message was sent (from `create_time` field).
    /// Used by adapters and downstream processing.
    #[allow(dead_code)]
    pub timestamp: Option<DateTime<Utc>>,
}

// ─── Detection ────────────────────────────────────────────────────────────────

/// Checks if a zip file is a ChatGPT export (contains `conversations.json`).
pub fn is_chatgpt_zip(path: &Path) -> bool {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return false,
    };
    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            let name = entry.name().to_string();
            if name == "conversations.json" || name.ends_with("/conversations.json") {
                return true;
            }
        }
    }
    false
}

/// Checks if a directory is an extracted ChatGPT export (contains `conversations.json`).
pub fn is_chatgpt_export_dir(dir: &Path) -> bool {
    dir.is_dir() && dir.join("conversations.json").is_file()
}

// ─── Zip Parsing ──────────────────────────────────────────────────────────────

/// Parses a ChatGPT export zip file into conversations.
pub fn parse_chatgpt_zip(path: &Path) -> Result<Vec<ParsedConversation>> {
    let file =
        fs::File::open(path).with_context(|| format!("Failed to open zip: {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Failed to read zip archive: {}", path.display()))?;

    // Find conversations.json inside the zip
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

// ─── JSON Parsing ─────────────────────────────────────────────────────────────

/// Parses a `conversations.json` file from disk.
pub fn parse_chatgpt_json(path: &Path) -> Result<Vec<ParsedConversation>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read: {}", path.display()))?;
    parse_conversations_json_str(&content)
}

/// Parses the raw JSON string of a conversations.json export.
fn parse_conversations_json_str(json_str: &str) -> Result<Vec<ParsedConversation>> {
    let value: Value =
        serde_json::from_str(json_str).with_context(|| "Failed to parse conversations JSON")?;

    let arr = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Expected conversations.json to be a JSON array"))?;

    let mut conversations = Vec::with_capacity(arr.len());
    for item in arr {
        match parse_conversation(item) {
            Ok(conv) => {
                // Skip conversations with no meaningful messages
                if !conv.messages.is_empty() {
                    conversations.push(conv);
                }
            }
            Err(e) => {
                // Log but don't fail on individual conversation parse errors
                eprintln!("  [warn] Skipping conversation: {}", e);
            }
        }
    }

    Ok(conversations)
}

/// Parses a single conversation object from the ChatGPT export format.
///
/// The conversation uses a tree-based `mapping` where each node has:
/// - `id`: node identifier
/// - `parent`: parent node id (null for root)
/// - `children`: array of child node ids
/// - `message`: optional message object with author, content, timestamp
///
/// We find the root (no parent), then walk the tree depth-first following
/// children links to collect messages in order.
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

    // Build a lookup of node_id -> node value
    let nodes: HashMap<&str, &Value> = mapping
        .iter()
        .map(|(k, v)| (k.as_str(), v))
        .collect();

    // Find root node(s): nodes with null parent or no parent field
    let root_ids: Vec<&str> = mapping
        .iter()
        .filter(|(_, node)| {
            node.get("parent")
                .is_none_or(|p| p.is_null())
        })
        .map(|(id, _)| id.as_str())
        .collect();

    let mut messages = Vec::new();

    // Walk from each root, collecting messages in tree order
    for root_id in root_ids {
        collect_messages_dfs(root_id, &nodes, &mut messages);
    }

    Ok(ParsedConversation {
        title,
        created_at,
        messages,
    })
}

/// Depth-first traversal of the conversation tree, collecting messages in order.
fn collect_messages_dfs(
    node_id: &str,
    nodes: &HashMap<&str, &Value>,
    messages: &mut Vec<ParsedMessage>,
) {
    let node = match nodes.get(node_id) {
        Some(n) => n,
        None => return,
    };

    // Extract message from this node if present
    if let Some(msg) = node.get("message") {
        if !msg.is_null() {
            if let Some(parsed) = extract_message(msg) {
                messages.push(parsed);
            }
        }
    }

    // Walk children in order
    if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
        for child_id in children {
            if let Some(id) = child_id.as_str() {
                collect_messages_dfs(id, nodes, messages);
            }
        }
    }
}

/// Extracts a ParsedMessage from a ChatGPT message object.
fn extract_message(msg: &Value) -> Option<ParsedMessage> {
    let role = msg
        .pointer("/author/role")
        .and_then(|r| r.as_str())
        .unwrap_or("unknown")
        .to_string();

    // Skip system messages — they're typically empty or not useful
    if role == "system" {
        return None;
    }

    let parts = msg.pointer("/content/parts").and_then(|p| p.as_array())?;

    // Extract only text parts (parts can be strings or objects for images/code)
    let text_parts: Vec<&str> = parts
        .iter()
        .filter_map(|p| p.as_str())
        .filter(|s| !s.is_empty())
        .collect();

    if text_parts.is_empty() {
        return None;
    }

    let content = text_parts.join("\n");

    let timestamp = msg
        .get("create_time")
        .and_then(|t| t.as_f64())
        .and_then(timestamp_to_datetime);

    Some(ParsedMessage {
        role,
        content,
        timestamp,
    })
}

// ─── Formatting ───────────────────────────────────────────────────────────────

/// Formats parsed conversations into readable text matching the existing
/// `format_conversation` output style used by `local.rs`.
pub fn format_conversations(conversations: &[ParsedConversation]) -> String {
    conversations
        .iter()
        .map(format_single_conversation)
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

/// Formats a single conversation into the `## Title\n\nrole: content` format.
fn format_single_conversation(conv: &ParsedConversation) -> String {
    let mut lines = Vec::with_capacity(conv.messages.len() + 1);
    lines.push(format!("## {}", conv.title));
    lines.push(String::new()); // blank line after title

    for msg in &conv.messages {
        lines.push(format!("{}: {}", msg.role, msg.content));
    }

    lines.join("\n")
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Converts a UNIX timestamp (float) to a UTC DateTime.
fn timestamp_to_datetime(ts: f64) -> Option<DateTime<Utc>> {
    let secs = ts as i64;
    let nanos = ((ts - secs as f64) * 1_000_000_000.0) as u32;
    Utc.timestamp_opt(secs, nanos).single()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// Node definition for test conversations: (id, parent, children, message).
    type TestNode<'a> = (&'a str, Option<&'a str>, Vec<&'a str>, Option<(&'a str, &'a str)>);

    /// Helper: creates a minimal conversations.json value with the mapping tree.
    fn make_conversation_json(title: &str, nodes: Vec<TestNode<'_>>) -> Value {
        let mut mapping = serde_json::Map::new();
        for (id, parent, children, message) in nodes {
            let children_val: Vec<Value> = children.iter().map(|c| Value::String(c.to_string())).collect();
            let parent_val = match parent {
                Some(p) => Value::String(p.to_string()),
                None => Value::Null,
            };
            let message_val = match message {
                Some((role, content)) => serde_json::json!({
                    "id": format!("msg-{}", id),
                    "author": {"role": role},
                    "content": {"content_type": "text", "parts": [content]},
                    "create_time": 1700000000.0
                }),
                None => Value::Null,
            };
            mapping.insert(
                id.to_string(),
                serde_json::json!({
                    "id": id,
                    "parent": parent_val,
                    "children": children_val,
                    "message": message_val,
                }),
            );
        }
        serde_json::json!({
            "title": title,
            "create_time": 1700000000.0,
            "update_time": 1700001000.0,
            "mapping": mapping,
        })
    }

    #[test]
    fn test_parse_simple_conversation() {
        let conv_json = make_conversation_json(
            "Hello Chat",
            vec![
                ("root", None, vec!["n1"], None),
                ("n1", Some("root"), vec!["n2"], Some(("user", "Hello!"))),
                (
                    "n2",
                    Some("n1"),
                    vec![],
                    Some(("assistant", "Hi there!")),
                ),
            ],
        );

        let conv = parse_conversation(&conv_json).unwrap();
        assert_eq!(conv.title, "Hello Chat");
        assert!(conv.created_at.is_some());
        assert_eq!(conv.messages.len(), 2);
        assert_eq!(conv.messages[0].role, "user");
        assert_eq!(conv.messages[0].content, "Hello!");
        assert_eq!(conv.messages[1].role, "assistant");
        assert_eq!(conv.messages[1].content, "Hi there!");
    }

    #[test]
    fn test_parse_conversation_with_system_node() {
        // System messages should be skipped
        let conv_json = make_conversation_json(
            "System Chat",
            vec![
                ("root", None, vec!["sys"], None),
                (
                    "sys",
                    Some("root"),
                    vec!["n1"],
                    Some(("system", "You are helpful")),
                ),
                (
                    "n1",
                    Some("sys"),
                    vec!["n2"],
                    Some(("user", "What's up?")),
                ),
                (
                    "n2",
                    Some("n1"),
                    vec![],
                    Some(("assistant", "Not much!")),
                ),
            ],
        );

        let conv = parse_conversation(&conv_json).unwrap();
        assert_eq!(conv.messages.len(), 2);
        assert_eq!(conv.messages[0].role, "user");
        assert_eq!(conv.messages[1].role, "assistant");
    }

    #[test]
    fn test_parse_conversation_null_messages() {
        // Nodes with null message should be gracefully skipped
        let conv_json = make_conversation_json(
            "Null Messages",
            vec![
                ("root", None, vec!["n1"], None),
                ("n1", Some("root"), vec!["n2"], None), // null message
                (
                    "n2",
                    Some("n1"),
                    vec![],
                    Some(("user", "Only real message")),
                ),
            ],
        );

        let conv = parse_conversation(&conv_json).unwrap();
        assert_eq!(conv.messages.len(), 1);
        assert_eq!(conv.messages[0].content, "Only real message");
    }

    #[test]
    fn test_parse_conversation_empty_parts() {
        // Messages with empty parts array should be skipped
        let mut conv_json = make_conversation_json(
            "Empty Parts",
            vec![
                ("root", None, vec!["n1"], None),
                ("n1", Some("root"), vec![], Some(("user", "Real text"))),
            ],
        );

        // Manually add a node with empty parts
        if let Some(mapping) = conv_json.get_mut("mapping").and_then(|m| m.as_object_mut()) {
            mapping.insert(
                "n-empty".to_string(),
                serde_json::json!({
                    "id": "n-empty",
                    "parent": "root",
                    "children": ["n1"],
                    "message": {
                        "author": {"role": "user"},
                        "content": {"content_type": "text", "parts": []}
                    }
                }),
            );
            // Update root children
            if let Some(root) = mapping.get_mut("root") {
                root["children"] = serde_json::json!(["n-empty"]);
            }
        }

        let conv = parse_conversation(&conv_json).unwrap();
        // Should have 1 message (the empty-parts one gets skipped)
        assert_eq!(conv.messages.len(), 1);
        assert_eq!(conv.messages[0].content, "Real text");
    }

    #[test]
    fn test_parse_conversation_non_text_parts() {
        // Parts that are objects (images, etc.) should be filtered out
        let mut conv_json = make_conversation_json(
            "Mixed Parts",
            vec![
                ("root", None, vec!["n1"], None),
            ],
        );

        // Add a node with mixed parts (string + object)
        if let Some(mapping) = conv_json.get_mut("mapping").and_then(|m| m.as_object_mut()) {
            mapping.insert(
                "n1".to_string(),
                serde_json::json!({
                    "id": "n1",
                    "parent": "root",
                    "children": [],
                    "message": {
                        "author": {"role": "user"},
                        "content": {
                            "content_type": "multimodal_text",
                            "parts": [
                                "Here is an image:",
                                {"content_type": "image_asset_pointer", "asset_pointer": "file-xxx"},
                                "And some more text"
                            ]
                        },
                        "create_time": 1700000000.0
                    }
                }),
            );
        }

        let conv = parse_conversation(&conv_json).unwrap();
        assert_eq!(conv.messages.len(), 1);
        assert_eq!(
            conv.messages[0].content,
            "Here is an image:\nAnd some more text"
        );
    }

    #[test]
    fn test_parse_empty_conversation() {
        let conv_json = make_conversation_json(
            "Empty",
            vec![("root", None, vec![], None)],
        );

        let conv = parse_conversation(&conv_json).unwrap();
        assert_eq!(conv.title, "Empty");
        assert!(conv.messages.is_empty());
    }

    #[test]
    fn test_parse_missing_mapping() {
        let bad_json = serde_json::json!({"title": "No mapping"});
        let result = parse_conversation(&bad_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_conversations_json_str() {
        let json = serde_json::json!([
            {
                "title": "Chat 1",
                "create_time": 1700000000.0,
                "mapping": {
                    "root": {"id": "root", "parent": null, "children": ["n1"], "message": null},
                    "n1": {
                        "id": "n1", "parent": "root", "children": [],
                        "message": {
                            "author": {"role": "user"},
                            "content": {"content_type": "text", "parts": ["Hello"]},
                            "create_time": 1700000000.0
                        }
                    }
                }
            },
            {
                "title": "Chat 2",
                "create_time": 1700001000.0,
                "mapping": {
                    "root": {"id": "root", "parent": null, "children": ["n1"], "message": null},
                    "n1": {
                        "id": "n1", "parent": "root", "children": [],
                        "message": {
                            "author": {"role": "user"},
                            "content": {"content_type": "text", "parts": ["World"]},
                            "create_time": 1700001000.0
                        }
                    }
                }
            }
        ]);

        let convs = parse_conversations_json_str(&json.to_string()).unwrap();
        assert_eq!(convs.len(), 2);
        assert_eq!(convs[0].title, "Chat 1");
        assert_eq!(convs[1].title, "Chat 2");
    }

    #[test]
    fn test_parse_conversations_skips_empty() {
        // Conversations with no messages should be filtered out
        let json = serde_json::json!([
            {
                "title": "Empty Conv",
                "create_time": 1700000000.0,
                "mapping": {
                    "root": {"id": "root", "parent": null, "children": [], "message": null}
                }
            },
            {
                "title": "Real Conv",
                "create_time": 1700001000.0,
                "mapping": {
                    "root": {"id": "root", "parent": null, "children": ["n1"], "message": null},
                    "n1": {
                        "id": "n1", "parent": "root", "children": [],
                        "message": {
                            "author": {"role": "user"},
                            "content": {"content_type": "text", "parts": ["Hi"]},
                            "create_time": 1700001000.0
                        }
                    }
                }
            }
        ]);

        let convs = parse_conversations_json_str(&json.to_string()).unwrap();
        assert_eq!(convs.len(), 1);
        assert_eq!(convs[0].title, "Real Conv");
    }

    #[test]
    fn test_parse_chatgpt_json_file() {
        let tmp = TempDir::new().unwrap();
        let json_path = tmp.path().join("conversations.json");

        let json = serde_json::json!([{
            "title": "File Test",
            "create_time": 1700000000.0,
            "mapping": {
                "root": {"id": "root", "parent": null, "children": ["n1"], "message": null},
                "n1": {
                    "id": "n1", "parent": "root", "children": [],
                    "message": {
                        "author": {"role": "user"},
                        "content": {"content_type": "text", "parts": ["From file"]},
                        "create_time": 1700000000.0
                    }
                }
            }
        }]);

        fs::write(&json_path, json.to_string()).unwrap();

        let convs = parse_chatgpt_json(&json_path).unwrap();
        assert_eq!(convs.len(), 1);
        assert_eq!(convs[0].messages[0].content, "From file");
    }

    #[test]
    fn test_parse_chatgpt_zip() {
        let tmp = TempDir::new().unwrap();
        let zip_path = tmp.path().join("chatgpt-export.zip");

        let json = serde_json::json!([{
            "title": "Zip Test",
            "create_time": 1700000000.0,
            "mapping": {
                "root": {"id": "root", "parent": null, "children": ["n1"], "message": null},
                "n1": {
                    "id": "n1", "parent": "root", "children": ["n2"],
                    "message": {
                        "author": {"role": "user"},
                        "content": {"content_type": "text", "parts": ["From zip"]},
                        "create_time": 1700000000.0
                    }
                },
                "n2": {
                    "id": "n2", "parent": "n1", "children": [],
                    "message": {
                        "author": {"role": "assistant"},
                        "content": {"content_type": "text", "parts": ["Zip reply"]},
                        "create_time": 1700000001.0
                    }
                }
            }
        }]);

        // Create a zip with conversations.json
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip_writer
            .start_file("conversations.json", options)
            .unwrap();
        zip_writer
            .write_all(json.to_string().as_bytes())
            .unwrap();
        // Add a dummy file like ChatGPT exports include
        zip_writer.start_file("chat.html", options).unwrap();
        zip_writer
            .write_all(b"<html>not used</html>")
            .unwrap();
        zip_writer.finish().unwrap();

        let convs = parse_chatgpt_zip(&zip_path).unwrap();
        assert_eq!(convs.len(), 1);
        assert_eq!(convs[0].title, "Zip Test");
        assert_eq!(convs[0].messages.len(), 2);
        assert_eq!(convs[0].messages[0].content, "From zip");
        assert_eq!(convs[0].messages[1].content, "Zip reply");
    }

    #[test]
    fn test_is_chatgpt_zip_true() {
        let tmp = TempDir::new().unwrap();
        let zip_path = tmp.path().join("export.zip");

        let file = fs::File::create(&zip_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip_writer
            .start_file("conversations.json", options)
            .unwrap();
        zip_writer.write_all(b"[]").unwrap();
        zip_writer.finish().unwrap();

        assert!(is_chatgpt_zip(&zip_path));
    }

    #[test]
    fn test_is_chatgpt_zip_false() {
        let tmp = TempDir::new().unwrap();
        let zip_path = tmp.path().join("other.zip");

        let file = fs::File::create(&zip_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip_writer
            .start_file("something-else.txt", options)
            .unwrap();
        zip_writer.write_all(b"hello").unwrap();
        zip_writer.finish().unwrap();

        assert!(!is_chatgpt_zip(&zip_path));
    }

    #[test]
    fn test_is_chatgpt_zip_nonexistent() {
        assert!(!is_chatgpt_zip(Path::new("/nonexistent/file.zip")));
    }

    #[test]
    fn test_is_chatgpt_export_dir() {
        let tmp = TempDir::new().unwrap();
        assert!(!is_chatgpt_export_dir(tmp.path()));

        fs::write(tmp.path().join("conversations.json"), "[]").unwrap();
        assert!(is_chatgpt_export_dir(tmp.path()));
    }

    #[test]
    fn test_format_conversations() {
        let convs = vec![
            ParsedConversation {
                title: "Chat A".to_string(),
                created_at: None,
                messages: vec![
                    ParsedMessage {
                        role: "user".to_string(),
                        content: "Hello".to_string(),
                        timestamp: None,
                    },
                    ParsedMessage {
                        role: "assistant".to_string(),
                        content: "Hi!".to_string(),
                        timestamp: None,
                    },
                ],
            },
            ParsedConversation {
                title: "Chat B".to_string(),
                created_at: None,
                messages: vec![ParsedMessage {
                    role: "user".to_string(),
                    content: "Goodbye".to_string(),
                    timestamp: None,
                }],
            },
        ];

        let formatted = format_conversations(&convs);
        assert!(formatted.contains("## Chat A"));
        assert!(formatted.contains("user: Hello"));
        assert!(formatted.contains("assistant: Hi!"));
        assert!(formatted.contains("---"));
        assert!(formatted.contains("## Chat B"));
        assert!(formatted.contains("user: Goodbye"));
    }

    #[test]
    fn test_timestamp_parsing() {
        let ts = timestamp_to_datetime(1700000000.0);
        assert!(ts.is_some());
        let dt = ts.unwrap();
        assert_eq!(dt.year(), 2023);
    }

    #[test]
    fn test_deep_conversation_tree() {
        // Test a conversation with multiple branches (only first child path matters)
        let conv_json = serde_json::json!({
            "title": "Deep Tree",
            "create_time": 1700000000.0,
            "mapping": {
                "root": {"id": "root", "parent": null, "children": ["a"], "message": null},
                "a": {
                    "id": "a", "parent": "root", "children": ["b"],
                    "message": {
                        "author": {"role": "user"},
                        "content": {"content_type": "text", "parts": ["First"]},
                        "create_time": 1700000000.0
                    }
                },
                "b": {
                    "id": "b", "parent": "a", "children": ["c"],
                    "message": {
                        "author": {"role": "assistant"},
                        "content": {"content_type": "text", "parts": ["Second"]},
                        "create_time": 1700000001.0
                    }
                },
                "c": {
                    "id": "c", "parent": "b", "children": ["d"],
                    "message": {
                        "author": {"role": "user"},
                        "content": {"content_type": "text", "parts": ["Third"]},
                        "create_time": 1700000002.0
                    }
                },
                "d": {
                    "id": "d", "parent": "c", "children": [],
                    "message": {
                        "author": {"role": "assistant"},
                        "content": {"content_type": "text", "parts": ["Fourth"]},
                        "create_time": 1700000003.0
                    }
                }
            }
        });

        let conv = parse_conversation(&conv_json).unwrap();
        assert_eq!(conv.messages.len(), 4);
        assert_eq!(conv.messages[0].content, "First");
        assert_eq!(conv.messages[1].content, "Second");
        assert_eq!(conv.messages[2].content, "Third");
        assert_eq!(conv.messages[3].content, "Fourth");
    }

    #[test]
    fn test_untitled_conversation() {
        let conv_json = serde_json::json!({
            "mapping": {
                "root": {"id": "root", "parent": null, "children": ["n1"], "message": null},
                "n1": {
                    "id": "n1", "parent": "root", "children": [],
                    "message": {
                        "author": {"role": "user"},
                        "content": {"content_type": "text", "parts": ["test"]},
                        "create_time": 1700000000.0
                    }
                }
            }
        });

        let conv = parse_conversation(&conv_json).unwrap();
        assert_eq!(conv.title, "Untitled conversation");
        assert!(conv.created_at.is_none());
    }

    #[test]
    fn test_zip_without_conversations_json() {
        let tmp = TempDir::new().unwrap();
        let zip_path = tmp.path().join("bad-export.zip");

        let file = fs::File::create(&zip_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip_writer.start_file("readme.txt", options).unwrap();
        zip_writer.write_all(b"not a chatgpt export").unwrap();
        zip_writer.finish().unwrap();

        let result = parse_chatgpt_zip(&zip_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No conversations.json"));
    }

    use chrono::Datelike;

    #[test]
    fn test_parse_conversation_with_tool_role() {
        // Tool messages should pass through (not system)
        let conv_json = serde_json::json!({
            "title": "Tool Chat",
            "create_time": 1700000000.0,
            "mapping": {
                "root": {"id": "root", "parent": null, "children": ["n1"], "message": null},
                "n1": {
                    "id": "n1", "parent": "root", "children": ["n2"],
                    "message": {
                        "author": {"role": "user"},
                        "content": {"content_type": "text", "parts": ["search for X"]},
                        "create_time": 1700000000.0
                    }
                },
                "n2": {
                    "id": "n2", "parent": "n1", "children": ["n3"],
                    "message": {
                        "author": {"role": "tool"},
                        "content": {"content_type": "text", "parts": ["search results"]},
                        "create_time": 1700000001.0
                    }
                },
                "n3": {
                    "id": "n3", "parent": "n2", "children": [],
                    "message": {
                        "author": {"role": "assistant"},
                        "content": {"content_type": "text", "parts": ["Based on the results..."]},
                        "create_time": 1700000002.0
                    }
                }
            }
        });

        let conv = parse_conversation(&conv_json).unwrap();
        assert_eq!(conv.messages.len(), 3);
        assert_eq!(conv.messages[1].role, "tool");
    }
}

//! Gemini CLI session adapter — reads `~/.gemini/tmp/<hash>/chats/session-*.json`.
//!
//! Gemini CLI stores sessions as JSON files under project hash directories.
//! Each file contains `sessionId`, `projectHash`, `startTime`, `lastUpdated`,
//! and a `messages` array. Message types are `"user"` and `"gemini"`.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};

use super::{Conversation, Message, Role, SessionAdapter, SessionFile};

// ─── Gemini Adapter ───────────────────────────────────────────────────────────

pub struct GeminiAdapter;

impl SessionAdapter for GeminiAdapter {
    fn name(&self) -> &str {
        "gemini"
    }

    fn display_name(&self) -> &str {
        "Gemini CLI"
    }

    fn discover_sessions(&self) -> Result<Vec<SessionFile>> {
        let base = gemini_tmp_dir()?;
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

            let project_hash = project_entry.file_name().to_string_lossy().to_string();
            let chats_dir = project_dir.join("chats");
            if !chats_dir.exists() {
                continue;
            }

            for file_entry in fs::read_dir(&chats_dir)? {
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
                    provider: "gemini".to_string(),
                    project: Some(project_hash.clone()),
                    modified,
                });
            }
        }

        Ok(sessions)
    }

    fn parse_session(&self, path: &Path) -> Result<Conversation> {
        parse_gemini_session(path)
    }

    fn can_handle(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        path_str.contains(".gemini/tmp/")
            && path_str.contains("/chats/")
            && path_str.ends_with(".json")
    }
}

// ─── Discovery Helpers ────────────────────────────────────────────────────────

fn gemini_tmp_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".gemini").join("tmp"))
}

/// Checks if a file is a Gemini session file.
fn is_session_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    name.starts_with("session-") && name.ends_with(".json")
}

// ─── Parsing ──────────────────────────────────────────────────────────────────

fn parse_gemini_session(path: &Path) -> Result<Conversation> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

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

    let mut messages = Vec::new();
    for msg_val in &messages_arr {
        if let Some(msg) = extract_gemini_message(msg_val) {
            messages.push(msg);
        }
    }

    // Build title from first user message
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

/// Extracts a message from a Gemini session message object.
fn extract_gemini_message(val: &serde_json::Value) -> Option<Message> {
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

/// Extracts text content from a Gemini message.
///
/// Handles three formats:
/// - `"content": "string"` (gemini responses)
/// - `"content": [{"text": "..."}]` (user messages)
/// - Messages with only `toolCalls` and empty content (skipped)
fn extract_content(val: &serde_json::Value) -> Option<String> {
    let content = val.get("content")?;

    // String content (gemini responses)
    if let Some(s) = content.as_str() {
        if s.is_empty() {
            return None;
        }
        return Some(s.to_string());
    }

    // Array of content blocks (user messages)
    if let Some(arr) = content.as_array() {
        let texts: Vec<String> = arr
            .iter()
            .filter_map(|block| {
                block.get("text").and_then(|t| t.as_str()).map(String::from)
            })
            .collect();
        if texts.is_empty() {
            return None;
        }
        return Some(texts.join("\n"));
    }

    None
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.min(s.len())])
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_is_session_file() {
        assert!(is_session_file(Path::new("session-2026-01-19T20-58-abc.json")));
        assert!(!is_session_file(Path::new("settings.json")));
        assert!(!is_session_file(Path::new("session-abc.jsonl")));
        assert!(!is_session_file(Path::new("readme.md")));
    }

    #[test]
    fn test_extract_content_string() {
        let val: serde_json::Value = serde_json::json!({
            "type": "gemini",
            "content": "Hello from Gemini"
        });
        assert_eq!(extract_content(&val).unwrap(), "Hello from Gemini");
    }

    #[test]
    fn test_extract_content_array() {
        let val: serde_json::Value = serde_json::json!({
            "type": "user",
            "content": [
                {"text": "Part 1"},
                {"text": "Part 2"}
            ]
        });
        assert_eq!(extract_content(&val).unwrap(), "Part 1\nPart 2");
    }

    #[test]
    fn test_extract_content_empty_string() {
        let val: serde_json::Value = serde_json::json!({
            "type": "gemini",
            "content": ""
        });
        assert!(extract_content(&val).is_none());
    }

    #[test]
    fn test_extract_content_empty_array() {
        let val: serde_json::Value = serde_json::json!({
            "type": "user",
            "content": []
        });
        assert!(extract_content(&val).is_none());
    }

    #[test]
    fn test_extract_gemini_message_user() {
        let val: serde_json::Value = serde_json::json!({
            "id": "abc",
            "timestamp": "2026-02-14T10:00:00Z",
            "type": "user",
            "content": [{"text": "What is Rust?"}]
        });
        let msg = extract_gemini_message(&val).unwrap();
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "What is Rust?");
        assert!(msg.timestamp.is_some());
    }

    #[test]
    fn test_extract_gemini_message_assistant() {
        let val: serde_json::Value = serde_json::json!({
            "id": "def",
            "timestamp": "2026-02-14T10:00:05Z",
            "type": "gemini",
            "content": "Rust is a systems programming language.",
            "model": "gemini-2.5-pro"
        });
        let msg = extract_gemini_message(&val).unwrap();
        assert_eq!(msg.role, Role::Assistant);
        assert!(msg.content.contains("systems programming"));
    }

    #[test]
    fn test_extract_gemini_message_tool_only_skipped() {
        // Messages with toolCalls but empty content should be skipped
        let val: serde_json::Value = serde_json::json!({
            "id": "ghi",
            "type": "gemini",
            "content": "",
            "toolCalls": [{"name": "read_file"}]
        });
        assert!(extract_gemini_message(&val).is_none());
    }

    #[test]
    fn test_extract_gemini_message_unknown_type() {
        let val: serde_json::Value = serde_json::json!({
            "type": "tool_result",
            "content": "some output"
        });
        assert!(extract_gemini_message(&val).is_none());
    }

    #[test]
    fn test_parse_gemini_session() {
        let tmp = tempdir().unwrap();
        let chats_dir = tmp.path().join("abc123").join("chats");
        fs::create_dir_all(&chats_dir).unwrap();
        let session_file = chats_dir.join("session-2026-02-14T10-00-abc.json");

        let json = serde_json::json!({
            "sessionId": "sess-001",
            "projectHash": "abc123",
            "startTime": "2026-02-14T10:00:00Z",
            "lastUpdated": "2026-02-14T10:05:00Z",
            "messages": [
                {
                    "id": "m1",
                    "timestamp": "2026-02-14T10:00:01Z",
                    "type": "user",
                    "content": [{"text": "Hello Gemini!"}]
                },
                {
                    "id": "m2",
                    "timestamp": "2026-02-14T10:00:05Z",
                    "type": "gemini",
                    "content": "Hello! How can I help you today?",
                    "model": "gemini-2.5-pro",
                    "tokens": {"input": 10, "output": 8, "total": 18}
                },
                {
                    "id": "m3",
                    "timestamp": "2026-02-14T10:01:00Z",
                    "type": "user",
                    "content": [{"text": "Tell me about Rust."}]
                },
                {
                    "id": "m4",
                    "timestamp": "2026-02-14T10:01:10Z",
                    "type": "gemini",
                    "content": "",
                    "toolCalls": [{"name": "search"}]
                },
                {
                    "id": "m5",
                    "timestamp": "2026-02-14T10:01:15Z",
                    "type": "gemini",
                    "content": "Rust is a systems programming language focused on safety.",
                    "model": "gemini-2.5-pro"
                }
            ]
        });
        fs::write(&session_file, serde_json::to_string_pretty(&json).unwrap()).unwrap();

        let conv = parse_gemini_session(&session_file).unwrap();
        assert_eq!(conv.id, "sess-001");
        assert_eq!(conv.provider, "Gemini CLI");
        assert!(conv.created_at.is_some());
        // m4 (empty content + toolCalls) should be skipped
        assert_eq!(conv.messages.len(), 4);
        assert_eq!(conv.messages[0].role, Role::User);
        assert_eq!(conv.messages[0].content, "Hello Gemini!");
        assert_eq!(conv.messages[1].role, Role::Assistant);
        assert!(conv.messages[1].content.contains("How can I help"));
        assert_eq!(conv.messages[3].role, Role::Assistant);
        assert!(conv.messages[3].content.contains("systems programming"));
        assert!(conv.title.unwrap().contains("Hello Gemini!"));
    }

    #[test]
    fn test_parse_empty_session() {
        let tmp = tempdir().unwrap();
        let session_file = tmp.path().join("session-empty.json");
        let json = serde_json::json!({
            "sessionId": "empty",
            "messages": []
        });
        fs::write(&session_file, serde_json::to_string(&json).unwrap()).unwrap();

        let conv = parse_gemini_session(&session_file).unwrap();
        assert!(conv.messages.is_empty());
        assert!(conv.title.is_none());
    }

    #[test]
    fn test_parse_session_no_session_id() {
        let tmp = tempdir().unwrap();
        let session_file = tmp.path().join("session-fallback.json");
        let json = serde_json::json!({
            "messages": [
                {"type": "user", "content": [{"text": "Hi"}]}
            ]
        });
        fs::write(&session_file, serde_json::to_string(&json).unwrap()).unwrap();

        let conv = parse_gemini_session(&session_file).unwrap();
        // Falls back to filename stem
        assert_eq!(conv.id, "session-fallback");
        assert_eq!(conv.messages.len(), 1);
    }

    #[test]
    fn test_can_handle() {
        let adapter = GeminiAdapter;
        assert!(adapter.can_handle(Path::new(
            "/Users/test/.gemini/tmp/abc123/chats/session-2026-01-19.json"
        )));
        assert!(!adapter.can_handle(Path::new(
            "/Users/test/.gemini/settings.json"
        )));
        assert!(!adapter.can_handle(Path::new(
            "/Users/test/.claude/projects/-test/abc.jsonl"
        )));
        assert!(!adapter.can_handle(Path::new("/Users/test/readme.md")));
    }

    #[test]
    fn test_discover_sessions_with_fixture() {
        let tmp = tempdir().unwrap();
        let chats_dir = tmp.path().join("hash1").join("chats");
        fs::create_dir_all(&chats_dir).unwrap();

        // Valid session files
        fs::write(
            chats_dir.join("session-2026-01-01.json"),
            r#"{"messages":[]}"#,
        )
        .unwrap();
        fs::write(
            chats_dir.join("session-2026-01-02.json"),
            r#"{"messages":[]}"#,
        )
        .unwrap();

        // Non-session files (should be ignored)
        fs::write(chats_dir.join("settings.json"), "{}").unwrap();
        fs::write(chats_dir.join("notes.txt"), "hello").unwrap();

        // Can't test discover_sessions directly (hardcoded home path),
        // but we can test the filter logic
        let entries: Vec<_> = fs::read_dir(&chats_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| is_session_file(&e.path()))
            .collect();
        assert_eq!(entries.len(), 2);
    }
}

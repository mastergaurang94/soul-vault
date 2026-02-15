//! Tests for gemini module.
use super::{discovery, parser, GeminiAdapter};
use crate::adapters::{Role, SessionAdapter};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_is_session_file() {
    assert!(discovery::is_session_file(Path::new(
        "session-2026-01-19T20-58-abc.json"
    )));
    assert!(!discovery::is_session_file(Path::new("settings.json")));
    assert!(!discovery::is_session_file(Path::new("session-abc.jsonl")));
    assert!(!discovery::is_session_file(Path::new("readme.md")));
}

#[test]
fn test_extract_content_string() {
    let val: serde_json::Value = serde_json::json!({
        "type": "gemini",
        "content": "Hello from Gemini"
    });
    assert_eq!(
        parser::extract_content(&val).as_deref(),
        Some("Hello from Gemini")
    );
}

#[test]
fn test_extract_content_array() {
    let val: serde_json::Value = serde_json::json!({
        "type": "user",
        "content": [{"text": "Part 1"}, {"text": "Part 2"}]
    });
    assert_eq!(
        parser::extract_content(&val).as_deref(),
        Some("Part 1\nPart 2")
    );
}

#[test]
fn test_extract_gemini_message_tool_only_skipped() {
    let val: serde_json::Value = serde_json::json!({
        "id": "ghi",
        "type": "gemini",
        "content": "",
        "toolCalls": [{"name": "read_file"}]
    });
    assert!(parser::extract_gemini_message(&val).is_none());
}

#[test]
fn test_parse_gemini_session() {
    let tmp = tempdir().expect("tempdir");
    let chats_dir = tmp.path().join("abc123").join("chats");
    fs::create_dir_all(&chats_dir).expect("mkdir");
    let session_file = chats_dir.join("session-2026-02-14T10-00-abc.json");

    let json = serde_json::json!({
        "sessionId": "sess-001",
        "projectHash": "abc123",
        "startTime": "2026-02-14T10:00:00Z",
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
                "content": "Hello! How can I help you today?"
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
                "content": "Rust is a systems programming language focused on safety."
            }
        ]
    });
    fs::write(
        &session_file,
        serde_json::to_string_pretty(&json).expect("json"),
    )
    .expect("write");

    let conv = parser::parse_gemini_session(&session_file).expect("parse");
    assert_eq!(conv.id, "sess-001");
    assert_eq!(conv.provider, "Gemini CLI");
    assert!(conv.created_at.is_some());
    assert_eq!(conv.messages.len(), 4);
    assert_eq!(conv.messages[0].role, Role::User);
    assert!(conv.messages[3].content.contains("systems programming"));
    assert!(conv
        .title
        .as_deref()
        .unwrap_or_default()
        .contains("Hello Gemini!"));
}

#[test]
fn test_parse_empty_and_fallback_session_id() {
    let tmp = tempdir().expect("tempdir");
    let empty_file = tmp.path().join("session-empty.json");
    let json = serde_json::json!({"sessionId": "empty", "messages": []});
    fs::write(&empty_file, serde_json::to_string(&json).expect("json")).expect("write");

    let empty = parser::parse_gemini_session(&empty_file).expect("parse");
    assert!(empty.messages.is_empty());
    assert!(empty.title.is_none());

    let fallback_file = tmp.path().join("session-fallback.json");
    let fallback_json =
        serde_json::json!({"messages": [{"type": "user", "content": [{"text": "Hi"}]}]});
    fs::write(
        &fallback_file,
        serde_json::to_string(&fallback_json).expect("json"),
    )
    .expect("write");

    let fallback = parser::parse_gemini_session(&fallback_file).expect("parse");
    assert_eq!(fallback.id, "session-fallback");
    assert_eq!(fallback.messages.len(), 1);
}

#[test]
fn test_can_handle() {
    let adapter = GeminiAdapter;
    assert!(adapter.can_handle(Path::new(
        "/Users/test/.gemini/tmp/abc123/chats/session-2026-01-19.json"
    )));
    assert!(!adapter.can_handle(Path::new("/Users/test/.gemini/settings.json")));
    assert!(!adapter.can_handle(Path::new("/Users/test/.claude/projects/-test/abc.jsonl")));
    assert!(!adapter.can_handle(Path::new("/Users/test/readme.md")));
}

#[test]
fn test_discover_filter_logic() {
    let tmp = tempdir().expect("tempdir");
    let chats_dir = tmp.path().join("hash1").join("chats");
    fs::create_dir_all(&chats_dir).expect("mkdir");

    fs::write(
        chats_dir.join("session-2026-01-01.json"),
        r#"{"messages":[]}"#,
    )
    .expect("write");
    fs::write(
        chats_dir.join("session-2026-01-02.json"),
        r#"{"messages":[]}"#,
    )
    .expect("write");
    fs::write(chats_dir.join("settings.json"), "{}").expect("write");
    fs::write(chats_dir.join("notes.txt"), "hello").expect("write");

    let entries: Vec<_> = fs::read_dir(&chats_dir)
        .expect("readdir")
        .filter_map(|e| e.ok())
        .filter(|e| discovery::is_session_file(&e.path()))
        .collect();
    assert_eq!(entries.len(), 2);
}

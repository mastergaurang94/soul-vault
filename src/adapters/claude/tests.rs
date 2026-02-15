//! Tests for claude module.
use super::{discovery, parser, ClaudeAdapter};
use crate::adapters::{Role, SessionAdapter};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_decode_project_path() {
    assert_eq!(
        discovery::decode_project_path("-Users-gaurang-myproject"),
        "/Users/gaurang/myproject"
    );
    assert_eq!(
        discovery::decode_project_path("-Users-gaurang-Documents-dev"),
        "/Users/gaurang/Documents/dev"
    );
}

#[test]
fn test_decode_project_path_no_leading_dash() {
    assert_eq!(
        discovery::decode_project_path("some-project"),
        "some/project"
    );
}

#[test]
fn test_is_session_file() {
    assert!(discovery::is_session_file(Path::new("abc123.jsonl")));
    assert!(!discovery::is_session_file(Path::new("agent-task1.jsonl")));
    assert!(!discovery::is_session_file(Path::new("readme.md")));
}

#[test]
fn test_extract_content_string() {
    let msg: serde_json::Value = serde_json::json!({
        "role": "user",
        "content": "Hello world"
    });
    assert_eq!(
        parser::extract_content(&msg).as_deref(),
        Some("Hello world")
    );
}

#[test]
fn test_extract_content_array() {
    let msg: serde_json::Value = serde_json::json!({
        "role": "assistant",
        "content": [
            {"type": "thinking", "thinking": "Let me think..."},
            {"type": "text", "text": "Here is my answer."},
            {"type": "text", "text": "And more."}
        ]
    });
    let content = parser::extract_content(&msg).expect("content");
    assert_eq!(content, "Here is my answer.\nAnd more.");
    assert!(!content.contains("Let me think"));
}

#[test]
fn test_extract_content_empty_array() {
    let msg: serde_json::Value = serde_json::json!({
        "role": "assistant",
        "content": [
            {"type": "thinking", "thinking": "..."}
        ]
    });
    assert!(parser::extract_content(&msg).is_none());
}

#[test]
fn test_parse_claude_session() {
    let tmp = tempdir().expect("tempdir");
    let session_file = tmp.path().join("abc123.jsonl");
    let jsonl = concat!(
        r#"{"type":"system","cwd":"/Users/test/project","sessionId":"abc123","version":"2.1"}"#,
        "\n",
        r#"{"type":"user","message":{"role":"user","content":"What is Rust?"},"cwd":"/Users/test/project","timestamp":"2026-02-14T10:00:00Z"}"#,
        "\n",
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Rust is a systems programming language."}]},"timestamp":"2026-02-14T10:00:05Z"}"#,
        "\n",
        r#"{"type":"user","message":{"role":"user","content":"Tell me more."},"timestamp":"2026-02-14T10:01:00Z"}"#,
        "\n",
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"It focuses on safety and performance."}]},"timestamp":"2026-02-14T10:01:05Z"}"#,
        "\n",
        r#"{"type":"progress","content":"Running tests..."}"#,
        "\n",
    );
    fs::write(&session_file, jsonl).expect("write");

    let conv = parser::parse_claude_session(&session_file).expect("parse");
    assert_eq!(conv.id, "abc123");
    assert_eq!(conv.messages.len(), 4);
    assert_eq!(conv.messages[0].role, Role::User);
    assert_eq!(conv.messages[0].content, "What is Rust?");
    assert_eq!(conv.messages[1].role, Role::Assistant);
    assert!(conv.messages[1].content.contains("systems programming"));
    assert!(conv.created_at.is_some());
}

#[test]
fn test_parse_session_with_summary() {
    let tmp = tempdir().expect("tempdir");
    let session_file = tmp.path().join("def456.jsonl");
    let jsonl = concat!(
        r#"{"type":"user","message":{"role":"user","content":"Help me"},"timestamp":"2026-02-14T10:00:00Z"}"#,
        "\n",
        r#"{"type":"assistant","message":{"role":"assistant","content":"Sure!"},"timestamp":"2026-02-14T10:00:05Z"}"#,
        "\n",
        r#"{"type":"summary","summary":"User asked for help with a coding problem"}"#,
        "\n",
    );
    fs::write(&session_file, jsonl).expect("write");

    let conv = parser::parse_claude_session(&session_file).expect("parse");
    assert_eq!(
        conv.title.as_deref(),
        Some("User asked for help with a coding problem")
    );
}

#[test]
fn test_parse_empty_session() {
    let tmp = tempdir().expect("tempdir");
    let session_file = tmp.path().join("empty.jsonl");
    fs::write(&session_file, "").expect("write");

    let conv = parser::parse_claude_session(&session_file).expect("parse");
    assert!(conv.messages.is_empty());
}

#[test]
fn test_parse_session_malformed_lines() {
    let tmp = tempdir().expect("tempdir");
    let session_file = tmp.path().join("bad.jsonl");
    let jsonl = concat!(
        "not json at all\n",
        r#"{"type":"user","message":{"role":"user","content":"Hello"}}"#,
        "\n",
        "{invalid json}\n",
        r#"{"type":"assistant","message":{"role":"assistant","content":"World"}}"#,
        "\n",
    );
    fs::write(&session_file, jsonl).expect("write");

    let conv = parser::parse_claude_session(&session_file).expect("parse");
    assert_eq!(conv.messages.len(), 2);
}

#[test]
fn test_can_handle() {
    let adapter = ClaudeAdapter;
    assert!(adapter.can_handle(Path::new(
        "/Users/test/.claude/projects/-Users-test/abc.jsonl"
    )));
    assert!(!adapter.can_handle(Path::new(
        "/Users/test/.openclaw/agents/main/sessions/abc.jsonl"
    )));
    assert!(!adapter.can_handle(Path::new("/Users/test/readme.md")));
}

#[test]
fn test_truncate() {
    assert_eq!(parser::truncate("short", 80), "short");
    let long = "a".repeat(100);
    let result = parser::truncate(&long, 80);
    assert!(result.len() <= 84);
    assert!(result.ends_with("..."));
}

//! Tests for openclaw module.
use super::{discovery, parser, OpenClawAdapter};
use crate::adapters::{Role, SessionAdapter};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_is_session_file() {
    assert!(discovery::is_session_file(Path::new("abc123.jsonl")));
    assert!(!discovery::is_session_file(Path::new(
        "abc.jsonl.deleted.2026-02-09T18-28-45.247Z"
    )));
    assert!(!discovery::is_session_file(Path::new("readme.md")));
}

#[test]
fn test_extract_content_string() {
    let msg: serde_json::Value = serde_json::json!({"role": "user", "content": "Hello"});
    assert_eq!(parser::extract_content(&msg).as_deref(), Some("Hello"));
}

#[test]
fn test_extract_content_array_non_text() {
    let msg: serde_json::Value = serde_json::json!({
        "role": "assistant",
        "content": [{"type": "tool_use", "tool": "exec"}]
    });
    assert!(parser::extract_content(&msg).is_none());
}

#[test]
fn test_parse_openclaw_session() {
    let tmp = tempdir().expect("tempdir");
    let agents_dir = tmp.path().join("main").join("sessions");
    fs::create_dir_all(&agents_dir).expect("mkdir");
    let session_file = agents_dir.join("session1.jsonl");

    let jsonl = concat!(
        r#"{"type":"session","version":3,"id":"session1","timestamp":"2026-02-14T10:00:00Z","cwd":"/test"}"#,
        "\n",
        r#"{"type":"message","id":"m1","timestamp":"2026-02-14T10:00:01Z","message":{"role":"user","content":"What is 2+2?"}}"#,
        "\n",
        r#"{"type":"message","id":"m2","timestamp":"2026-02-14T10:00:02Z","message":{"role":"assistant","content":[{"type":"text","text":"2+2 equals 4."}]}}"#,
        "\n",
        r#"{"type":"message","id":"m3","timestamp":"2026-02-14T10:00:03Z","message":{"role":"user","content":"Thanks!"}}"#,
        "\n",
    );
    fs::write(&session_file, jsonl).expect("write");

    let conv = parser::parse_openclaw_session(&session_file).expect("parse");
    assert_eq!(conv.id, "session1");
    assert_eq!(conv.messages.len(), 3);
    assert_eq!(conv.messages[0].role, Role::User);
    assert_eq!(conv.messages[1].content, "2+2 equals 4.");
    assert!(conv.created_at.is_some());
    assert!(conv
        .title
        .as_deref()
        .unwrap_or_default()
        .contains("What is 2+2?"));
}

#[test]
fn test_parse_session_malformed_lines() {
    let tmp = tempdir().expect("tempdir");
    let session_file = tmp.path().join("bad.jsonl");
    let jsonl = concat!(
        "not json\n",
        r#"{"type":"message","id":"m1","message":{"role":"user","content":"Hello"}}"#,
        "\n",
        "{broken}\n",
        r#"{"type":"message","id":"m2","message":{"role":"assistant","content":"World"}}"#,
        "\n",
    );
    fs::write(&session_file, jsonl).expect("write");

    let conv = parser::parse_openclaw_session(&session_file).expect("parse");
    assert_eq!(conv.messages.len(), 2);
}

#[test]
fn test_parse_session_keeps_tool_results() {
    let tmp = tempdir().expect("tempdir");
    let session_file = tmp.path().join("tools.jsonl");
    let jsonl = concat!(
        r#"{"type":"message","id":"m1","message":{"role":"user","content":"Run a command"}}"#,
        "\n",
        r#"{"type":"message","id":"m2","message":{"role":"assistant","content":[{"type":"text","text":"Running..."}]}}"#,
        "\n",
        r#"{"type":"message","id":"m3","message":{"role":"toolResult","content":"Command output"}}"#,
        "\n",
        r#"{"type":"message","id":"m4","message":{"role":"assistant","content":[{"type":"text","text":"Done!"}]}}"#,
        "\n",
    );
    fs::write(&session_file, jsonl).expect("write");

    let conv = parser::parse_openclaw_session(&session_file).expect("parse");
    assert_eq!(conv.messages.len(), 4);
    assert_eq!(conv.messages[2].role, Role::Tool);
}

#[test]
fn test_can_handle() {
    let adapter = OpenClawAdapter;
    assert!(adapter.can_handle(Path::new(
        "/Users/test/.openclaw/agents/main/sessions/abc.jsonl"
    )));
    assert!(!adapter.can_handle(Path::new(
        "/Users/test/.claude/projects/-Users-test/abc.jsonl"
    )));
    assert!(!adapter.can_handle(Path::new("/Users/test/readme.md")));
}

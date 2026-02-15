use super::parser;
use crate::adapters::Role;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_extract_response_content_string() {
    let payload: serde_json::Value = serde_json::json!({
        "type": "message",
        "role": "assistant",
        "content": "Hello world"
    });
    assert_eq!(
        parser::extract_response_content(&payload).as_deref(),
        Some("Hello world")
    );
}

#[test]
fn test_extract_response_content_array() {
    let payload: serde_json::Value = serde_json::json!({
        "type": "message",
        "role": "assistant",
        "content": [
            {"type": "output_text", "text": "Part 1"},
            {"type": "output_text", "text": "Part 2"}
        ]
    });
    assert_eq!(
        parser::extract_response_content(&payload).as_deref(),
        Some("Part 1\nPart 2")
    );
}

#[test]
fn test_extract_response_content_skips_input_text() {
    let payload: serde_json::Value = serde_json::json!({
        "type": "message",
        "role": "assistant",
        "content": [
            {"type": "input_text", "text": "system prompt"},
            {"type": "output_text", "text": "actual answer"}
        ]
    });
    assert_eq!(
        parser::extract_response_content(&payload).as_deref(),
        Some("actual answer")
    );
}

#[test]
fn test_extract_codex_response_filters_roles_and_types() {
    let assistant: serde_json::Value = serde_json::json!({
        "timestamp": "2026-02-14T10:00:05Z",
        "type": "response_item",
        "payload": {
            "type": "message",
            "role": "assistant",
            "content": [{"type": "output_text", "text": "Here is the answer."}]
        }
    });
    let msg = parser::extract_codex_response(&assistant).expect("assistant message");
    assert_eq!(msg.role, Role::Assistant);
    assert_eq!(msg.content, "Here is the answer.");

    let developer: serde_json::Value = serde_json::json!({
        "type": "response_item",
        "payload": {
            "type": "message",
            "role": "developer",
            "content": [{"type": "input_text", "text": "system prompt"}]
        }
    });
    assert!(parser::extract_codex_response(&developer).is_none());

    let function_call: serde_json::Value = serde_json::json!({
        "type": "response_item",
        "payload": {
            "type": "function_call",
            "name": "exec_command",
            "arguments": "{\"cmd\":\"ls\"}"
        }
    });
    assert!(parser::extract_codex_response(&function_call).is_none());
}

#[test]
fn test_parse_codex_session() {
    let tmp = tempdir().expect("tempdir");
    let session_dir = tmp.path().join("2026").join("02").join("14");
    fs::create_dir_all(&session_dir).expect("mkdir");
    let session_file = session_dir.join("rollout-2026-02-14T10-00-00-abc123.jsonl");

    let jsonl = [
        r#"{"timestamp":"2026-02-14T10:00:00Z","type":"session_meta","payload":{"id":"abc123","timestamp":"2026-02-14T10:00:00Z","cwd":"/Users/test/project"}}"#,
        r#"{"timestamp":"2026-02-14T10:00:03Z","type":"event_msg","payload":{"type":"user_message","message":"What is Rust?"}}"#,
        r#"{"timestamp":"2026-02-14T10:00:05Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Looking into that for you."}]}}"#,
        r#"{"timestamp":"2026-02-14T10:00:15Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Rust is a systems programming language focused on safety and performance."}]}}"#,
        r#"{"timestamp":"2026-02-14T10:01:00Z","type":"event_msg","payload":{"type":"user_message","message":"Thanks!"}}"#,
        r#"{"timestamp":"2026-02-14T10:01:05Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"You're welcome!"}]}}"#,
    ]
    .join("\n");
    fs::write(&session_file, jsonl).expect("write");

    let conv = parser::parse_codex_session(&session_file).expect("parse");
    assert_eq!(conv.id, "rollout-2026-02-14T10-00-00-abc123");
    assert_eq!(conv.provider, "Codex (/Users/test/project)");
    assert!(conv.created_at.is_some());
    assert_eq!(conv.messages.len(), 5);
    assert_eq!(conv.messages[0].role, Role::User);
    assert_eq!(conv.messages[4].content, "You're welcome!");
    assert!(conv
        .title
        .as_deref()
        .unwrap_or_default()
        .contains("What is Rust?"));
}

#[test]
fn test_parse_empty_session_and_malformed_lines() {
    let tmp = tempdir().expect("tempdir");
    let empty_file = tmp.path().join("rollout-empty.jsonl");
    fs::write(&empty_file, "").expect("write");
    let empty = parser::parse_codex_session(&empty_file).expect("parse");
    assert!(empty.messages.is_empty());
    assert!(empty.title.is_none());

    let bad_file = tmp.path().join("rollout-bad.jsonl");
    let jsonl = concat!(
        "not json at all\n",
        r#"{"timestamp":"2026-02-14T10:00:00Z","type":"event_msg","payload":{"type":"user_message","message":"Hello"}}"#,
        "\n",
        "{broken}\n",
        r#"{"timestamp":"2026-02-14T10:00:05Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"World"}]}}"#,
        "\n",
    );
    fs::write(&bad_file, jsonl).expect("write");
    let bad = parser::parse_codex_session(&bad_file).expect("parse");
    assert_eq!(bad.messages.len(), 2);
}

#[test]
fn test_truncate() {
    assert_eq!(parser::truncate("short", 80), "short");
    let long = "a".repeat(100);
    assert!(parser::truncate(&long, 80).ends_with("..."));
}

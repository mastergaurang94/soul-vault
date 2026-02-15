//! Tests for codex module.
use super::{discovery, CodexAdapter};
use crate::adapters::SessionAdapter;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_is_rollout_file() {
    assert!(discovery::is_rollout_file(Path::new(
        "rollout-2026-02-11T08-15-20-abc123.jsonl"
    )));
    assert!(!discovery::is_rollout_file(Path::new("session-abc.jsonl")));
    assert!(!discovery::is_rollout_file(Path::new("rollout-abc.json")));
    assert!(!discovery::is_rollout_file(Path::new("readme.md")));
}

#[test]
fn test_discover_recursive_with_fixture() {
    let tmp = tempdir().expect("tempdir");
    let day_dir = tmp.path().join("2026").join("02").join("14");
    fs::create_dir_all(&day_dir).expect("mkdir");

    fs::write(
        day_dir.join("rollout-2026-02-14T10-00-abc.jsonl"),
        r#"{"type":"session_meta","payload":{}}"#,
    )
    .expect("write");
    fs::write(
        day_dir.join("rollout-2026-02-14T11-00-def.jsonl"),
        r#"{"type":"session_meta","payload":{}}"#,
    )
    .expect("write");

    fs::write(day_dir.join("session-abc.jsonl"), "{}").expect("write");
    fs::write(day_dir.join("notes.txt"), "hello").expect("write");

    let mut sessions = Vec::new();
    discovery::discover_recursive(tmp.path(), &mut sessions).expect("discover");
    assert_eq!(sessions.len(), 2);
    assert!(sessions.iter().all(|s| s.provider == "codex"));
}

#[test]
fn test_can_handle() {
    let adapter = CodexAdapter;
    assert!(adapter.can_handle(Path::new(
        "/Users/test/.codex/sessions/2026/02/11/rollout-2026-02-11T08-15-abc.jsonl"
    )));
    assert!(!adapter.can_handle(Path::new(
        "/Users/test/.codex/sessions/2026/02/11/session-abc.jsonl"
    )));
    assert!(!adapter.can_handle(Path::new("/Users/test/.claude/projects/-test/abc.jsonl")));
    assert!(!adapter.can_handle(Path::new("/Users/test/readme.md")));
}

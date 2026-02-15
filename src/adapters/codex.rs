//! Codex CLI session adapter — reads `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`.
//!
//! Codex CLI stores sessions as JSONL files organized by date.
//! Each line has a `type` field:
//! - `session_meta`: session metadata (cwd, model, cli version)
//! - `event_msg` with `user_message`: actual user messages
//! - `response_item` with `role=assistant`: assistant responses
//!   (phase: "commentary" for intermediate, "final_answer" for final)

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};

use super::{Conversation, Message, Role, SessionAdapter, SessionFile};

// ─── Codex Adapter ────────────────────────────────────────────────────────────

pub struct CodexAdapter;

impl SessionAdapter for CodexAdapter {
    fn name(&self) -> &str {
        "codex"
    }

    fn display_name(&self) -> &str {
        "Codex"
    }

    fn discover_sessions(&self) -> Result<Vec<SessionFile>> {
        let base = codex_sessions_dir()?;
        if !base.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();
        discover_recursive(&base, &mut sessions)?;
        Ok(sessions)
    }

    fn parse_session(&self, path: &Path) -> Result<Conversation> {
        parse_codex_session(path)
    }

    fn can_handle(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        path_str.contains(".codex/sessions/") && is_rollout_file(path)
    }
}

// ─── Discovery Helpers ────────────────────────────────────────────────────────

fn codex_sessions_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".codex").join("sessions"))
}

/// Recursively discovers rollout JSONL files under the sessions directory.
fn discover_recursive(dir: &Path, sessions: &mut Vec<SessionFile>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            discover_recursive(&path, sessions)?;
        } else if is_rollout_file(&path) {
            let modified = entry
                .metadata()
                .map(|m| m.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH))
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

            // Extract project from session_meta cwd (deferred to parse time)
            sessions.push(SessionFile {
                path,
                provider: "codex".to_string(),
                project: None,
                modified,
            });
        }
    }

    Ok(())
}

/// Checks if a file is a Codex rollout session file.
fn is_rollout_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    name.starts_with("rollout-") && name.ends_with(".jsonl")
}

// ─── Parsing ──────────────────────────────────────────────────────────────────

fn parse_codex_session(path: &Path) -> Result<Conversation> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let session_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut messages = Vec::new();
    let mut created_at: Option<DateTime<Utc>> = None;
    let mut cwd: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let val: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let line_type = val.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let timestamp = extract_timestamp(&val);

        match line_type {
            "session_meta" => {
                // Extract metadata from session_meta payload
                if let Some(payload) = val.get("payload") {
                    if cwd.is_none() {
                        cwd = payload.get("cwd").and_then(|c| c.as_str()).map(String::from);
                    }
                    if created_at.is_none() {
                        created_at = payload
                            .get("timestamp")
                            .and_then(|t| t.as_str())
                            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.with_timezone(&Utc));
                    }
                }
                if created_at.is_none() {
                    created_at = timestamp;
                }
            }
            "event_msg" => {
                // User messages come as event_msg with type=user_message
                if let Some(payload) = val.get("payload") {
                    let event_type = payload.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    if event_type == "user_message" {
                        if let Some(msg_text) = payload.get("message").and_then(|m| m.as_str()) {
                            if !msg_text.trim().is_empty() {
                                messages.push(Message {
                                    role: Role::User,
                                    content: msg_text.to_string(),
                                    timestamp,
                                });
                            }
                        }
                    }
                }
            }
            "response_item" => {
                // Assistant messages come as response_item with role=assistant
                if let Some(msg) = extract_codex_response(&val) {
                    messages.push(msg);
                }
            }
            _ => {} // Skip turn_context, etc.
        }
    }

    // Build title from first user message
    let title = messages
        .iter()
        .find(|m| m.role == Role::User)
        .map(|m| truncate(&m.content, 80));

    let provider = match &cwd {
        Some(c) => format!("Codex ({})", c),
        None => "Codex".to_string(),
    };

    Ok(Conversation {
        id: session_id,
        title,
        provider,
        created_at,
        messages,
    })
}

/// Extracts an assistant message from a response_item line.
fn extract_codex_response(val: &serde_json::Value) -> Option<Message> {
    let payload = val.get("payload")?;
    let payload_type = payload.get("type").and_then(|t| t.as_str())?;

    // Only extract actual messages, not function calls or reasoning
    if payload_type != "message" {
        return None;
    }

    let role_str = payload.get("role").and_then(|r| r.as_str())?;

    // Only capture assistant messages — skip developer/user context injections
    if role_str != "assistant" {
        return None;
    }

    let content = extract_response_content(payload)?;
    if content.trim().is_empty() {
        return None;
    }

    let timestamp = extract_timestamp(val);

    Some(Message {
        role: Role::Assistant,
        content,
        timestamp,
    })
}

/// Extracts text content from a Codex response_item payload.
///
/// Content is an array of blocks: `[{"type": "output_text", "text": "..."}]`
fn extract_response_content(payload: &serde_json::Value) -> Option<String> {
    let content = payload.get("content")?;

    // Simple string content
    if let Some(s) = content.as_str() {
        if s.is_empty() {
            return None;
        }
        return Some(s.to_string());
    }

    // Array of content blocks
    if let Some(arr) = content.as_array() {
        let texts: Vec<String> = arr
            .iter()
            .filter_map(|block| {
                let block_type = block.get("type").and_then(|t| t.as_str())?;
                match block_type {
                    "output_text" | "text" => {
                        block.get("text").and_then(|t| t.as_str()).map(String::from)
                    }
                    _ => None, // Skip input_text, thinking, etc.
                }
            })
            .collect();
        if texts.is_empty() {
            return None;
        }
        return Some(texts.join("\n"));
    }

    None
}

fn extract_timestamp(val: &serde_json::Value) -> Option<DateTime<Utc>> {
    val.get("timestamp")
        .and_then(|t| t.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
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
    fn test_is_rollout_file() {
        assert!(is_rollout_file(Path::new(
            "rollout-2026-02-11T08-15-20-abc123.jsonl"
        )));
        assert!(!is_rollout_file(Path::new("session-abc.jsonl")));
        assert!(!is_rollout_file(Path::new("rollout-abc.json")));
        assert!(!is_rollout_file(Path::new("readme.md")));
    }

    #[test]
    fn test_extract_response_content_string() {
        let payload: serde_json::Value = serde_json::json!({
            "type": "message",
            "role": "assistant",
            "content": "Hello world"
        });
        assert_eq!(extract_response_content(&payload).unwrap(), "Hello world");
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
            extract_response_content(&payload).unwrap(),
            "Part 1\nPart 2"
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
            extract_response_content(&payload).unwrap(),
            "actual answer"
        );
    }

    #[test]
    fn test_extract_response_content_empty() {
        let payload: serde_json::Value = serde_json::json!({
            "type": "message",
            "role": "assistant",
            "content": ""
        });
        assert!(extract_response_content(&payload).is_none());
    }

    #[test]
    fn test_extract_response_content_empty_array() {
        let payload: serde_json::Value = serde_json::json!({
            "type": "message",
            "role": "assistant",
            "content": [
                {"type": "input_text", "text": "system stuff"}
            ]
        });
        assert!(extract_response_content(&payload).is_none());
    }

    #[test]
    fn test_extract_codex_response_assistant() {
        let val: serde_json::Value = serde_json::json!({
            "timestamp": "2026-02-14T10:00:05Z",
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "output_text", "text": "Here is the answer."}
                ],
                "phase": "final_answer"
            }
        });
        let msg = extract_codex_response(&val).unwrap();
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content, "Here is the answer.");
        assert!(msg.timestamp.is_some());
    }

    #[test]
    fn test_extract_codex_response_skips_developer() {
        let val: serde_json::Value = serde_json::json!({
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "developer",
                "content": [{"type": "input_text", "text": "system prompt"}]
            }
        });
        assert!(extract_codex_response(&val).is_none());
    }

    #[test]
    fn test_extract_codex_response_skips_user_context() {
        let val: serde_json::Value = serde_json::json!({
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "AGENTS.md content"}]
            }
        });
        assert!(extract_codex_response(&val).is_none());
    }

    #[test]
    fn test_extract_codex_response_skips_function_calls() {
        let val: serde_json::Value = serde_json::json!({
            "type": "response_item",
            "payload": {
                "type": "function_call",
                "name": "exec_command",
                "arguments": "{\"cmd\":\"ls\"}"
            }
        });
        assert!(extract_codex_response(&val).is_none());
    }

    #[test]
    fn test_extract_codex_response_skips_reasoning() {
        let val: serde_json::Value = serde_json::json!({
            "type": "response_item",
            "payload": {
                "type": "reasoning",
                "summary": [{"type": "summary_text", "text": "thinking..."}]
            }
        });
        assert!(extract_codex_response(&val).is_none());
    }

    #[test]
    fn test_parse_codex_session() {
        let tmp = tempdir().unwrap();
        let session_dir = tmp.path().join("2026").join("02").join("14");
        fs::create_dir_all(&session_dir).unwrap();
        let session_file = session_dir.join("rollout-2026-02-14T10-00-00-abc123.jsonl");

        let jsonl = [
            r#"{"timestamp":"2026-02-14T10:00:00Z","type":"session_meta","payload":{"id":"abc123","timestamp":"2026-02-14T10:00:00Z","cwd":"/Users/test/project","model_provider":"openai","cli_version":"0.98.0"}}"#,
            r#"{"timestamp":"2026-02-14T10:00:01Z","type":"response_item","payload":{"type":"message","role":"developer","content":[{"type":"input_text","text":"system prompt"}]}}"#,
            r#"{"timestamp":"2026-02-14T10:00:02Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"AGENTS.md content"}]}}"#,
            r#"{"timestamp":"2026-02-14T10:00:03Z","type":"event_msg","payload":{"type":"user_message","message":"What is Rust?"}}"#,
            r#"{"timestamp":"2026-02-14T10:00:04Z","type":"turn_context","payload":{"cwd":"/Users/test/project"}}"#,
            r#"{"timestamp":"2026-02-14T10:00:05Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Looking into that for you."}],"phase":"commentary"}}"#,
            r#"{"timestamp":"2026-02-14T10:00:10Z","type":"response_item","payload":{"type":"function_call","name":"exec_command","arguments":"{\"cmd\":\"rustc --version\"}"}}"#,
            r#"{"timestamp":"2026-02-14T10:00:11Z","type":"response_item","payload":{"type":"function_call_output","output":"rustc 1.82.0"}}"#,
            r#"{"timestamp":"2026-02-14T10:00:15Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Rust is a systems programming language focused on safety and performance."}],"phase":"final_answer"}}"#,
            r#"{"timestamp":"2026-02-14T10:01:00Z","type":"event_msg","payload":{"type":"user_message","message":"Thanks!"}}"#,
            r#"{"timestamp":"2026-02-14T10:01:05Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"You're welcome!"}],"phase":"final_answer"}}"#,
        ].join("\n");
        fs::write(&session_file, jsonl).unwrap();

        let conv = parse_codex_session(&session_file).unwrap();
        assert_eq!(conv.id, "rollout-2026-02-14T10-00-00-abc123");
        assert_eq!(conv.provider, "Codex (/Users/test/project)");
        assert!(conv.created_at.is_some());
        // Should have: 2 user messages + 3 assistant messages (commentary + 2 final)
        // Developer and user context messages are skipped
        assert_eq!(conv.messages.len(), 5);
        assert_eq!(conv.messages[0].role, Role::User);
        assert_eq!(conv.messages[0].content, "What is Rust?");
        assert_eq!(conv.messages[1].role, Role::Assistant);
        assert!(conv.messages[1].content.contains("Looking into that"));
        assert_eq!(conv.messages[2].role, Role::Assistant);
        assert!(conv.messages[2].content.contains("systems programming"));
        assert_eq!(conv.messages[3].role, Role::User);
        assert_eq!(conv.messages[3].content, "Thanks!");
        assert_eq!(conv.messages[4].role, Role::Assistant);
        assert_eq!(conv.messages[4].content, "You're welcome!");
        assert!(conv.title.unwrap().contains("What is Rust?"));
    }

    #[test]
    fn test_parse_empty_session() {
        let tmp = tempdir().unwrap();
        let session_file = tmp.path().join("rollout-empty.jsonl");
        fs::write(&session_file, "").unwrap();

        let conv = parse_codex_session(&session_file).unwrap();
        assert!(conv.messages.is_empty());
        assert!(conv.title.is_none());
    }

    #[test]
    fn test_parse_session_meta_only() {
        let tmp = tempdir().unwrap();
        let session_file = tmp.path().join("rollout-meta.jsonl");
        let jsonl = r#"{"timestamp":"2026-02-14T10:00:00Z","type":"session_meta","payload":{"id":"meta-only","timestamp":"2026-02-14T10:00:00Z","cwd":"/test"}}"#;
        fs::write(&session_file, jsonl).unwrap();

        let conv = parse_codex_session(&session_file).unwrap();
        assert!(conv.messages.is_empty());
        assert!(conv.created_at.is_some());
        assert!(conv.provider.contains("/test"));
    }

    #[test]
    fn test_parse_session_malformed_lines() {
        let tmp = tempdir().unwrap();
        let session_file = tmp.path().join("rollout-bad.jsonl");
        let jsonl = concat!(
            "not json at all\n",
            r#"{"timestamp":"2026-02-14T10:00:00Z","type":"event_msg","payload":{"type":"user_message","message":"Hello"}}"#,
            "\n",
            "{broken}\n",
            r#"{"timestamp":"2026-02-14T10:00:05Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"World"}]}}"#,
            "\n",
        );
        fs::write(&session_file, jsonl).unwrap();

        let conv = parse_codex_session(&session_file).unwrap();
        assert_eq!(conv.messages.len(), 2);
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
        assert!(!adapter.can_handle(Path::new(
            "/Users/test/.claude/projects/-test/abc.jsonl"
        )));
        assert!(!adapter.can_handle(Path::new("/Users/test/readme.md")));
    }

    #[test]
    fn test_discover_recursive_with_fixture() {
        let tmp = tempdir().unwrap();
        let day_dir = tmp.path().join("2026").join("02").join("14");
        fs::create_dir_all(&day_dir).unwrap();

        // Valid rollout files
        fs::write(
            day_dir.join("rollout-2026-02-14T10-00-abc.jsonl"),
            r#"{"type":"session_meta","payload":{}}"#,
        )
        .unwrap();
        fs::write(
            day_dir.join("rollout-2026-02-14T11-00-def.jsonl"),
            r#"{"type":"session_meta","payload":{}}"#,
        )
        .unwrap();

        // Non-rollout files (should be ignored)
        fs::write(day_dir.join("session-abc.jsonl"), "{}").unwrap();
        fs::write(day_dir.join("notes.txt"), "hello").unwrap();

        let mut sessions = Vec::new();
        discover_recursive(tmp.path(), &mut sessions).unwrap();
        assert_eq!(sessions.len(), 2);
        assert!(sessions.iter().all(|s| s.provider == "codex"));
    }
}

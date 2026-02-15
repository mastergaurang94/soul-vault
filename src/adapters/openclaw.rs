//! OpenClaw session adapter — reads `~/.openclaw/agents/*/sessions/*.jsonl`.
//!
//! OpenClaw stores sessions as JSONL files organized by agent name.
//! Each line has a `type` field; messages have `role`, `content`, and `model`.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};

use super::{Conversation, Message, Role, SessionAdapter, SessionFile};

// ─── OpenClaw Adapter ─────────────────────────────────────────────────────────

pub struct OpenClawAdapter;

impl SessionAdapter for OpenClawAdapter {
    fn name(&self) -> &str {
        "openclaw"
    }

    fn display_name(&self) -> &str {
        "OpenClaw"
    }

    fn discover_sessions(&self) -> Result<Vec<SessionFile>> {
        let base = openclaw_agents_dir()?;
        if !base.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();
        for agent_entry in fs::read_dir(&base)? {
            let agent_entry = agent_entry?;
            let agent_dir = agent_entry.path();
            if !agent_dir.is_dir() {
                continue;
            }

            let agent_name = agent_entry.file_name().to_string_lossy().to_string();
            let sessions_dir = agent_dir.join("sessions");
            if !sessions_dir.exists() {
                continue;
            }

            for file_entry in fs::read_dir(&sessions_dir)? {
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
                    provider: "openclaw".to_string(),
                    project: Some(agent_name.clone()),
                    modified,
                });
            }
        }

        Ok(sessions)
    }

    fn parse_session(&self, path: &Path) -> Result<Conversation> {
        parse_openclaw_session(path)
    }

    fn can_handle(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        path_str.contains(".openclaw/agents/")
            && path_str.contains("/sessions/")
            && path_str.ends_with(".jsonl")
    }
}

// ─── Discovery Helpers ────────────────────────────────────────────────────────

fn openclaw_agents_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".openclaw").join("agents"))
}

/// Checks if a file is a valid session (not deleted/backup).
fn is_session_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    name.ends_with(".jsonl") && !name.contains(".deleted.")
}

// ─── Parsing ──────────────────────────────────────────────────────────────────

fn parse_openclaw_session(path: &Path) -> Result<Conversation> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let session_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut messages = Vec::new();
    let mut created_at: Option<DateTime<Utc>> = None;
    let mut agent_name: Option<String> = None;

    // Derive agent name from path: .openclaw/agents/<name>/sessions/
    if let Some(parent) = path.parent() {
        if let Some(agents_dir) = parent.parent() {
            agent_name = agents_dir
                .file_name()
                .and_then(|n| n.to_str())
                .map(String::from);
        }
    }

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let val: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let msg_type = val.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match msg_type {
            "session" => {
                if created_at.is_none() {
                    created_at = extract_timestamp(&val);
                }
            }
            "message" => {
                if let Some(msg) = extract_openclaw_message(&val) {
                    if created_at.is_none() {
                        created_at = extract_timestamp(&val);
                    }
                    messages.push(msg);
                }
            }
            _ => {} // Skip model_change, thinking_level_change, compaction, custom
        }
    }

    // Build title from first user message
    let title = messages
        .iter()
        .find(|m| m.role == Role::User)
        .map(|m| truncate(&m.content, 80));

    let provider = match &agent_name {
        Some(name) => format!("OpenClaw ({})", name),
        None => "OpenClaw".to_string(),
    };

    Ok(Conversation {
        id: session_id,
        title,
        provider,
        created_at,
        messages,
    })
}

/// Extracts a message from an OpenClaw JSONL message line.
fn extract_openclaw_message(val: &serde_json::Value) -> Option<Message> {
    let msg = val.get("message")?;
    let role_str = msg.get("role").and_then(|r| r.as_str())?;

    let role = match role_str {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "system" => Role::System,
        "toolResult" | "tool" => Role::Tool,
        _ => return None,
    };

    let content = extract_content(msg)?;
    if content.trim().is_empty() {
        return None;
    }

    let timestamp = extract_timestamp(val);

    Some(Message {
        role,
        content,
        timestamp,
    })
}

/// Extracts text content from a message — handles both string and array formats.
fn extract_content(msg: &serde_json::Value) -> Option<String> {
    let content = msg.get("content")?;

    // Simple string content
    if let Some(s) = content.as_str() {
        return Some(s.to_string());
    }

    // Array of content blocks
    if let Some(arr) = content.as_array() {
        let texts: Vec<String> = arr
            .iter()
            .filter_map(|block| {
                let block_type = block.get("type").and_then(|t| t.as_str())?;
                match block_type {
                    "text" => block.get("text").and_then(|t| t.as_str()).map(String::from),
                    _ => None,
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
    fn test_is_session_file() {
        assert!(is_session_file(Path::new("abc123.jsonl")));
        assert!(!is_session_file(Path::new(
            "abc.jsonl.deleted.2026-02-09T18-28-45.247Z"
        )));
        assert!(!is_session_file(Path::new("readme.md")));
    }

    #[test]
    fn test_extract_content_string() {
        let msg: serde_json::Value = serde_json::json!({
            "role": "user",
            "content": "Hello"
        });
        assert_eq!(extract_content(&msg).unwrap(), "Hello");
    }

    #[test]
    fn test_extract_content_array() {
        let msg: serde_json::Value = serde_json::json!({
            "role": "assistant",
            "content": [
                {"type": "text", "text": "Part 1"},
                {"type": "text", "text": "Part 2"}
            ]
        });
        assert_eq!(extract_content(&msg).unwrap(), "Part 1\nPart 2");
    }

    #[test]
    fn test_extract_content_array_non_text() {
        let msg: serde_json::Value = serde_json::json!({
            "role": "assistant",
            "content": [
                {"type": "tool_use", "tool": "exec"}
            ]
        });
        assert!(extract_content(&msg).is_none());
    }

    #[test]
    fn test_parse_openclaw_session() {
        let tmp = tempdir().unwrap();
        let agents_dir = tmp.path().join("main").join("sessions");
        fs::create_dir_all(&agents_dir).unwrap();
        let session_file = agents_dir.join("session1.jsonl");

        let jsonl = concat!(
            r#"{"type":"session","version":3,"id":"session1","timestamp":"2026-02-14T10:00:00Z","cwd":"/test"}"#,
            "\n",
            r#"{"type":"model_change","id":"mc1","timestamp":"2026-02-14T10:00:00Z","provider":"anthropic","modelId":"claude-opus-4-5"}"#,
            "\n",
            r#"{"type":"message","id":"m1","timestamp":"2026-02-14T10:00:01Z","message":{"role":"user","content":"What is 2+2?"}}"#,
            "\n",
            r#"{"type":"message","id":"m2","timestamp":"2026-02-14T10:00:02Z","message":{"role":"assistant","content":[{"type":"text","text":"2+2 equals 4."}],"model":"claude-opus-4-5"}}"#,
            "\n",
            r#"{"type":"message","id":"m3","timestamp":"2026-02-14T10:00:03Z","message":{"role":"user","content":"Thanks!"}}"#,
            "\n",
        );
        fs::write(&session_file, jsonl).unwrap();

        let conv = parse_openclaw_session(&session_file).unwrap();
        assert_eq!(conv.id, "session1");
        assert_eq!(conv.messages.len(), 3);
        assert_eq!(conv.messages[0].role, Role::User);
        assert_eq!(conv.messages[0].content, "What is 2+2?");
        assert_eq!(conv.messages[1].role, Role::Assistant);
        assert_eq!(conv.messages[1].content, "2+2 equals 4.");
        assert!(conv.created_at.is_some());
        assert!(conv.title.unwrap().contains("What is 2+2?"));
    }

    #[test]
    fn test_parse_empty_session() {
        let tmp = tempdir().unwrap();
        let session_file = tmp.path().join("empty.jsonl");
        fs::write(&session_file, "").unwrap();

        let conv = parse_openclaw_session(&session_file).unwrap();
        assert!(conv.messages.is_empty());
    }

    #[test]
    fn test_parse_session_skips_tool_results() {
        let tmp = tempdir().unwrap();
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
        fs::write(&session_file, jsonl).unwrap();

        let conv = parse_openclaw_session(&session_file).unwrap();
        assert_eq!(conv.messages.len(), 4);
        assert_eq!(conv.messages[2].role, Role::Tool);
    }

    #[test]
    fn test_parse_session_malformed_lines() {
        let tmp = tempdir().unwrap();
        let session_file = tmp.path().join("bad.jsonl");
        let jsonl = concat!(
            "not json\n",
            r#"{"type":"message","id":"m1","message":{"role":"user","content":"Hello"}}"#,
            "\n",
            "{broken}\n",
            r#"{"type":"message","id":"m2","message":{"role":"assistant","content":"World"}}"#,
            "\n",
        );
        fs::write(&session_file, jsonl).unwrap();

        let conv = parse_openclaw_session(&session_file).unwrap();
        assert_eq!(conv.messages.len(), 2);
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
}

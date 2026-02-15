//! Claude Code session adapter — reads `~/.claude/projects/**/*.jsonl`.
//!
//! Claude Code stores sessions as JSONL files under project directories.
//! Directory names encode the project path: leading `-` maps to `/`,
//! subsequent `-` also map to `/`.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};

use super::{Conversation, Message, Role, SessionAdapter, SessionFile};

// ─── Claude Adapter ───────────────────────────────────────────────────────────

pub struct ClaudeAdapter;

impl SessionAdapter for ClaudeAdapter {
    fn name(&self) -> &str {
        "claude"
    }

    fn display_name(&self) -> &str {
        "Claude Code"
    }

    fn discover_sessions(&self) -> Result<Vec<SessionFile>> {
        let base = claude_projects_dir()?;
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

            let project_name = decode_project_path(&project_entry.file_name().to_string_lossy());

            for file_entry in fs::read_dir(&project_dir)? {
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
                    provider: "claude".to_string(),
                    project: Some(project_name.clone()),
                    modified,
                });
            }
        }

        Ok(sessions)
    }

    fn parse_session(&self, path: &Path) -> Result<Conversation> {
        parse_claude_session(path)
    }

    fn can_handle(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        path_str.contains(".claude/projects/") && path_str.ends_with(".jsonl")
    }
}

// ─── Discovery Helpers ────────────────────────────────────────────────────────

fn claude_projects_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".claude").join("projects"))
}

/// Checks if a file is a valid session (not a subtask file).
fn is_session_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str());
    if ext != Some("jsonl") {
        return false;
    }
    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    // Filter out agent subtask files
    !name.starts_with("agent-")
}

/// Decodes project path from Claude's directory naming convention.
///
/// Claude encodes paths by replacing `/` with `-`:
/// `-Users-gaurang-myproject` → `/Users/gaurang/myproject`
fn decode_project_path(dir_name: &str) -> String {
    if let Some(stripped) = dir_name.strip_prefix('-') {
        // Leading `-` represents root `/`, rest are path separators
        format!("/{}", stripped.replace('-', "/"))
    } else {
        dir_name.replace('-', "/")
    }
}

// ─── Parsing ──────────────────────────────────────────────────────────────────

fn parse_claude_session(path: &Path) -> Result<Conversation> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let session_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut messages = Vec::new();
    let mut created_at: Option<DateTime<Utc>> = None;
    let mut cwd: Option<String> = None;
    let mut title: Option<String> = None;

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
            "user" | "human" => {
                if let Some(msg) = extract_message(&val, Role::User) {
                    if created_at.is_none() {
                        created_at = extract_timestamp(&val);
                    }
                    if cwd.is_none() {
                        cwd = val.get("cwd").and_then(|c| c.as_str()).map(String::from);
                    }
                    messages.push(msg);
                }
            }
            "assistant" => {
                if let Some(msg) = extract_message(&val, Role::Assistant) {
                    messages.push(msg);
                }
            }
            "system" => {
                // System messages often contain cwd
                if cwd.is_none() {
                    cwd = val.get("cwd").and_then(|c| c.as_str()).map(String::from);
                }
                if created_at.is_none() {
                    created_at = extract_timestamp(&val);
                }
            }
            "summary" => {
                // Use summary as title if available
                if let Some(summary) = val.get("summary").and_then(|s| s.as_str()) {
                    title = Some(truncate(summary, 80));
                }
            }
            _ => {} // Skip progress, file-history-snapshot, etc.
        }
    }

    // Build title from first user message if no summary
    if title.is_none() {
        title = messages
            .iter()
            .find(|m| m.role == Role::User)
            .map(|m| truncate(&m.content, 80));
    }

    // Derive project from parent directory name
    let project = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(decode_project_path);

    Ok(Conversation {
        id: session_id,
        title,
        provider: format!("Claude Code{}", project_suffix(&project, &cwd)),
        created_at,
        messages,
    })
}

/// Extracts a message from a Claude JSONL line.
fn extract_message(val: &serde_json::Value, role: Role) -> Option<Message> {
    let msg = val.get("message")?;
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

/// Extracts text content from a message object.
///
/// Handles both `"content": "string"` and `"content": [{"type":"text","text":"..."}]`.
fn extract_content(msg: &serde_json::Value) -> Option<String> {
    let content = msg.get("content")?;

    // Simple string content
    if let Some(s) = content.as_str() {
        return Some(s.to_string());
    }

    // Array of content blocks (Claude's format)
    if let Some(arr) = content.as_array() {
        let texts: Vec<String> = arr
            .iter()
            .filter_map(|block| {
                let block_type = block.get("type").and_then(|t| t.as_str())?;
                match block_type {
                    "text" => block.get("text").and_then(|t| t.as_str()).map(String::from),
                    _ => None, // Skip thinking, tool_use, etc.
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

fn project_suffix(project: &Option<String>, cwd: &Option<String>) -> String {
    if let Some(p) = project {
        format!(" ({})", p)
    } else if let Some(c) = cwd {
        format!(" ({})", c)
    } else {
        String::new()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_decode_project_path() {
        assert_eq!(
            decode_project_path("-Users-gaurang-myproject"),
            "/Users/gaurang/myproject"
        );
        assert_eq!(
            decode_project_path("-Users-gaurang-Documents-dev"),
            "/Users/gaurang/Documents/dev"
        );
    }

    #[test]
    fn test_decode_project_path_no_leading_dash() {
        assert_eq!(decode_project_path("some-project"), "some/project");
    }

    #[test]
    fn test_is_session_file() {
        assert!(is_session_file(Path::new("abc123.jsonl")));
        assert!(!is_session_file(Path::new("agent-task1.jsonl")));
        assert!(!is_session_file(Path::new("readme.md")));
    }

    #[test]
    fn test_extract_content_string() {
        let msg: serde_json::Value = serde_json::json!({
            "role": "user",
            "content": "Hello world"
        });
        assert_eq!(extract_content(&msg).unwrap(), "Hello world");
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
        let content = extract_content(&msg).unwrap();
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
        assert!(extract_content(&msg).is_none());
    }

    #[test]
    fn test_parse_claude_session() {
        let tmp = tempdir().unwrap();
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
        fs::write(&session_file, jsonl).unwrap();

        let conv = parse_claude_session(&session_file).unwrap();
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
        let tmp = tempdir().unwrap();
        let session_file = tmp.path().join("def456.jsonl");
        let jsonl = concat!(
            r#"{"type":"user","message":{"role":"user","content":"Help me"},"timestamp":"2026-02-14T10:00:00Z"}"#,
            "\n",
            r#"{"type":"assistant","message":{"role":"assistant","content":"Sure!"},"timestamp":"2026-02-14T10:00:05Z"}"#,
            "\n",
            r#"{"type":"summary","summary":"User asked for help with a coding problem"}"#,
            "\n",
        );
        fs::write(&session_file, jsonl).unwrap();

        let conv = parse_claude_session(&session_file).unwrap();
        assert_eq!(
            conv.title.unwrap(),
            "User asked for help with a coding problem"
        );
    }

    #[test]
    fn test_parse_empty_session() {
        let tmp = tempdir().unwrap();
        let session_file = tmp.path().join("empty.jsonl");
        fs::write(&session_file, "").unwrap();

        let conv = parse_claude_session(&session_file).unwrap();
        assert!(conv.messages.is_empty());
    }

    #[test]
    fn test_parse_session_malformed_lines() {
        let tmp = tempdir().unwrap();
        let session_file = tmp.path().join("bad.jsonl");
        let jsonl = concat!(
            "not json at all\n",
            r#"{"type":"user","message":{"role":"user","content":"Hello"}}"#,
            "\n",
            "{invalid json}\n",
            r#"{"type":"assistant","message":{"role":"assistant","content":"World"}}"#,
            "\n",
        );
        fs::write(&session_file, jsonl).unwrap();

        let conv = parse_claude_session(&session_file).unwrap();
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
        assert_eq!(truncate("short", 80), "short");
        let long = "a".repeat(100);
        let result = truncate(&long, 80);
        assert!(result.len() <= 84); // 80 + "..."
        assert!(result.ends_with("..."));
    }
}

//! Pluggable adapter system for reading AI session files from different providers.
//!
//! Each provider (Claude Code, OpenClaw, etc.) gets its own adapter that handles
//! discovery, parsing, and normalization of session data.

pub mod claude;
pub mod codex;
pub mod gemini;
pub mod openclaw;

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// ─── Normalized Types ─────────────────────────────────────────────────────────

/// A discovered session file on disk.
#[derive(Debug, Clone)]
pub struct SessionFile {
    pub path: PathBuf,
    #[allow(dead_code)]
    pub provider: String,
    #[allow(dead_code)]
    pub project: Option<String>,
    #[allow(dead_code)]
    pub modified: SystemTime,
}

/// A normalized conversation parsed from a session file.
#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: String,
    pub title: Option<String>,
    pub provider: String,
    pub created_at: Option<DateTime<Utc>>,
    pub messages: Vec<Message>,
}

/// A single message in a conversation.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
    #[allow(dead_code)]
    pub timestamp: Option<DateTime<Utc>>,
}

/// Message author role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::System => write!(f, "system"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

// ─── Session Adapter Trait ────────────────────────────────────────────────────

/// Each AI provider implements this trait for session discovery and parsing.
pub trait SessionAdapter: Send + Sync {
    /// Unique identifier: "claude", "openclaw", etc.
    #[allow(dead_code)]
    fn name(&self) -> &str;

    /// Human-readable display name: "Claude Code", "OpenClaw", etc.
    fn display_name(&self) -> &str;

    /// Discover session files on disk for this provider.
    fn discover_sessions(&self) -> Result<Vec<SessionFile>>;

    /// Parse a session file into normalized conversations.
    fn parse_session(&self, path: &Path) -> Result<Conversation>;

    /// Check if a file path belongs to this adapter.
    fn can_handle(&self, path: &Path) -> bool;
}

// ─── Adapter Registry ─────────────────────────────────────────────────────────

/// Holds all registered adapters and provides unified discovery.
pub struct AdapterRegistry {
    adapters: Vec<Box<dyn SessionAdapter>>,
}

impl AdapterRegistry {
    /// Creates a new registry with all built-in adapters.
    pub fn new() -> Self {
        Self {
            adapters: vec![
                Box::new(claude::ClaudeAdapter),
                Box::new(openclaw::OpenClawAdapter),
                Box::new(gemini::GeminiAdapter),
                Box::new(codex::CodexAdapter),
            ],
        }
    }

    /// Discover sessions from all registered adapters.
    pub fn discover_all(&self) -> Vec<(String, Vec<SessionFile>)> {
        let mut results = Vec::new();
        for adapter in &self.adapters {
            let sessions = adapter.discover_sessions().unwrap_or_default();
            results.push((adapter.display_name().to_string(), sessions));
        }
        results
    }

    /// Find which adapter can handle a given file path.
    pub fn auto_detect(&self, path: &Path) -> Option<&dyn SessionAdapter> {
        self.adapters
            .iter()
            .find(|a| a.can_handle(path))
            .map(|a| a.as_ref())
    }

    /// Returns base directories for all adapters (top-level provider dirs).
    pub fn base_dirs(&self) -> Vec<(String, PathBuf)> {
        let mut result = Vec::new();
        if let Some(home) = dirs::home_dir() {
            let claude_dir = home.join(".claude").join("projects");
            if claude_dir.exists() {
                result.push(("Claude Code".to_string(), claude_dir));
            }
            let openclaw_dir = home.join(".openclaw").join("agents");
            if openclaw_dir.exists() {
                result.push(("OpenClaw".to_string(), openclaw_dir));
            }
            let gemini_dir = home.join(".gemini").join("tmp");
            if gemini_dir.exists() {
                result.push(("Gemini CLI".to_string(), gemini_dir));
            }
            let codex_dir = home.join(".codex").join("sessions");
            if codex_dir.exists() {
                result.push(("Codex".to_string(), codex_dir));
            }
        }
        result
    }
}

// ─── Conversation to Text ─────────────────────────────────────────────────────

/// Converts a conversation to human-readable text for the LLM pipeline.
pub fn conversation_to_text(conv: &Conversation) -> String {
    let mut lines = Vec::new();

    if let Some(title) = &conv.title {
        lines.push(format!("## {}", title));
    } else {
        lines.push(format!("## Session {}", conv.id));
    }

    lines.push(format!("Provider: {}", conv.provider));
    if let Some(created) = &conv.created_at {
        lines.push(format!("Date: {}", created.format("%Y-%m-%d %H:%M")));
    }
    lines.push(String::new());

    for msg in &conv.messages {
        lines.push(format!("{}: {}", msg.role, msg.content));
        lines.push(String::new());
    }

    lines.join("\n")
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_display() {
        assert_eq!(Role::User.to_string(), "user");
        assert_eq!(Role::Assistant.to_string(), "assistant");
        assert_eq!(Role::System.to_string(), "system");
        assert_eq!(Role::Tool.to_string(), "tool");
    }

    #[test]
    fn test_conversation_to_text() {
        let conv = Conversation {
            id: "abc123".to_string(),
            title: Some("Test Chat".to_string()),
            provider: "test".to_string(),
            created_at: None,
            messages: vec![
                Message {
                    role: Role::User,
                    content: "Hello".to_string(),
                    timestamp: None,
                },
                Message {
                    role: Role::Assistant,
                    content: "Hi there!".to_string(),
                    timestamp: None,
                },
            ],
        };
        let text = conversation_to_text(&conv);
        assert!(text.contains("## Test Chat"));
        assert!(text.contains("user: Hello"));
        assert!(text.contains("assistant: Hi there!"));
    }

    #[test]
    fn test_conversation_to_text_no_title() {
        let conv = Conversation {
            id: "xyz".to_string(),
            title: None,
            provider: "test".to_string(),
            created_at: None,
            messages: vec![],
        };
        let text = conversation_to_text(&conv);
        assert!(text.contains("## Session xyz"));
    }

    #[test]
    fn test_registry_creation() {
        let registry = AdapterRegistry::new();
        let results = registry.discover_all();
        // Should have entries for each adapter (even if 0 sessions)
        assert_eq!(results.len(), 4);
        assert_eq!(results[0].0, "Claude Code");
        assert_eq!(results[1].0, "OpenClaw");
        assert_eq!(results[2].0, "Gemini CLI");
        assert_eq!(results[3].0, "Codex");
    }
}

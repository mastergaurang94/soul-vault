//! Provider-agnostic cloud import domain models and progress events.

use chrono::{DateTime, Utc};

use crate::types::Provider;

#[derive(Debug, Clone)]
pub(crate) struct CloudConversationStub {
    pub conversation_id: String,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub(crate) struct CloudMessage {
    pub role: String,
    pub content: String,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub(crate) struct CloudConversation {
    pub provider: Provider,
    pub conversation_id: String,
    pub title: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub messages: Vec<CloudMessage>,
}

impl CloudConversation {
    pub fn content_hash_material(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.provider.to_string());
        out.push('|');
        out.push_str(&self.conversation_id);
        out.push('|');
        if let Some(ts) = &self.updated_at {
            out.push_str(&ts.to_rfc3339());
        }
        for msg in &self.messages {
            out.push('|');
            out.push_str(&msg.role);
            out.push(':');
            out.push_str(&msg.content);
        }
        out
    }

    pub fn to_text(&self) -> String {
        let mut lines = vec![format!(
            "## {}",
            self.title
                .clone()
                .unwrap_or_else(|| format!("Session {}", self.conversation_id))
        )];
        lines.push(format!("Provider: {}", self.provider.display_name()));
        if let Some(updated_at) = &self.updated_at {
            lines.push(format!("Date: {}", updated_at.format("%Y-%m-%d %H:%M")));
        }
        lines.push(String::new());

        for msg in &self.messages {
            let ts = msg
                .timestamp
                .as_ref()
                .map(|v| format!(" [{}]", v.format("%Y-%m-%d %H:%M")))
                .unwrap_or_default();
            lines.push(format!("{}{}: {}", msg.role, ts, msg.content));
            lines.push(String::new());
        }

        lines.join("\n")
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CloudFetchPage {
    pub items: Vec<CloudConversationStub>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImportJobState {
    Queued,
    Fetching,
    Normalizing,
    Processing,
    Writing,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub(crate) struct CloudImportEvent {
    pub provider: Provider,
    pub state: ImportJobState,
    pub current: usize,
    pub total: usize,
    pub conversation_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub(crate) struct CloudImportSummary {
    pub provider: Provider,
    pub fetched: usize,
    pub imported: usize,
    pub skipped_unchanged: usize,
    pub processed_chunks: usize,
    pub memories: usize,
    pub topics: usize,
    pub people: usize,
    pub errors: Vec<String>,
    pub cancelled: bool,
}

impl CloudImportSummary {
    pub fn to_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("Provider: {}", self.provider.display_name()),
            format!(
                "Fetched {} conversations ({} imported, {} unchanged)",
                self.fetched, self.imported, self.skipped_unchanged
            ),
            format!(
                "Processed {} chunks and extracted {} memories",
                self.processed_chunks, self.memories
            ),
            format!("Topics: {} | People: {}", self.topics, self.people),
        ];

        if self.cancelled {
            lines.push("Import cancelled by user. Partial progress kept safely.".to_string());
        }
        if !self.errors.is_empty() {
            lines.push(format!("Warnings: {}", self.errors.len()));
        }

        lines
    }
}

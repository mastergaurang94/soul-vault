//! Shared types and helpers for ChatGPT export parsing.

use chrono::{DateTime, TimeZone, Utc};

/// A parsed ChatGPT conversation with ordered messages.
#[derive(Debug, Clone)]
pub struct ParsedConversation {
    pub title: String,
    /// When the conversation was created (from `create_time` field).
    /// Used by adapters and downstream processing.
    #[allow(dead_code)]
    pub created_at: Option<DateTime<Utc>>,
    pub messages: Vec<ParsedMessage>,
}

/// A single message extracted from a ChatGPT conversation tree.
#[derive(Debug, Clone)]
pub struct ParsedMessage {
    pub role: String,
    pub content: String,
    /// When the message was sent (from `create_time` field).
    /// Used by adapters and downstream processing.
    #[allow(dead_code)]
    pub timestamp: Option<DateTime<Utc>>,
}

/// Converts a UNIX timestamp (float) to a UTC DateTime.
pub(crate) fn timestamp_to_datetime(ts: f64) -> Option<DateTime<Utc>> {
    let secs = ts as i64;
    let nanos = ((ts - secs as f64) * 1_000_000_000.0) as u32;
    Utc.timestamp_opt(secs, nanos).single()
}

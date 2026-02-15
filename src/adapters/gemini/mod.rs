//! Gemini CLI session adapter — reads `~/.gemini/tmp/<hash>/chats/session-*.json`.
//!
//! Gemini CLI stores sessions as JSON files under project hash directories.
//! Each file contains `sessionId`, `projectHash`, `startTime`, `lastUpdated`,
//! and a `messages` array. Message types are `"user"` and `"gemini"`.

mod discovery;
mod parser;

use anyhow::Result;
use std::path::Path;

use super::{Conversation, SessionAdapter, SessionFile};

pub struct GeminiAdapter;

impl SessionAdapter for GeminiAdapter {
    fn name(&self) -> &str {
        "gemini"
    }

    fn display_name(&self) -> &str {
        "Gemini CLI"
    }

    fn discover_sessions(&self) -> Result<Vec<SessionFile>> {
        discovery::discover_sessions()
    }

    fn parse_session(&self, path: &Path) -> Result<Conversation> {
        parser::parse_gemini_session(path)
    }

    fn can_handle(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        path_str.contains(".gemini/tmp/")
            && path_str.contains("/chats/")
            && path_str.ends_with(".json")
    }
}

#[cfg(test)]
mod tests;

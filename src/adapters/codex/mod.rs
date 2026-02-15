//! Codex CLI session adapter — reads `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`.
//!
//! Codex CLI stores sessions as JSONL files organized by date.
//! Each line has a `type` field:
//! - `session_meta`: session metadata (cwd, model, cli version)
//! - `event_msg` with `user_message`: actual user messages
//! - `response_item` with `role=assistant`: assistant responses
//!   (phase: "commentary" for intermediate, "final_answer" for final)

mod discovery;
mod parser;

use anyhow::Result;
use std::path::Path;

use super::{Conversation, SessionAdapter, SessionFile};

pub struct CodexAdapter;

impl SessionAdapter for CodexAdapter {
    fn name(&self) -> &str {
        "codex"
    }

    fn display_name(&self) -> &str {
        "Codex"
    }

    fn discover_sessions(&self) -> Result<Vec<SessionFile>> {
        discovery::discover_sessions()
    }

    fn parse_session(&self, path: &Path) -> Result<Conversation> {
        parser::parse_codex_session(path)
    }

    fn can_handle(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        path_str.contains(".codex/sessions/") && discovery::is_rollout_file(path)
    }
}

#[cfg(test)]
mod tests_discovery;
#[cfg(test)]
mod tests_parser;

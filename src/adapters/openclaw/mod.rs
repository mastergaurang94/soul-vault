//! OpenClaw session adapter — reads `~/.openclaw/agents/*/sessions/*.jsonl`.
//!
//! OpenClaw stores sessions as JSONL files organized by agent name.
//! Each line has a `type` field; messages have `role`, `content`, and `model`.

mod discovery;
mod parser;

use anyhow::Result;
use std::path::Path;

use super::{Conversation, SessionAdapter, SessionFile};

pub struct OpenClawAdapter;

impl SessionAdapter for OpenClawAdapter {
    fn name(&self) -> &str {
        "openclaw"
    }

    fn display_name(&self) -> &str {
        "OpenClaw"
    }

    fn discover_sessions(&self) -> Result<Vec<SessionFile>> {
        discovery::discover_sessions()
    }

    fn parse_session(&self, path: &Path) -> Result<Conversation> {
        parser::parse_openclaw_session(path)
    }

    fn can_handle(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        path_str.contains(".openclaw/agents/")
            && path_str.contains("/sessions/")
            && path_str.ends_with(".jsonl")
    }
}

#[cfg(test)]
mod tests;

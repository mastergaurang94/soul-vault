//! Claude Code session adapter — reads `~/.claude/projects/**/*.jsonl`.
//!
//! Claude Code stores sessions as JSONL files under project directories.
//! Directory names encode the project path: leading `-` maps to `/`,
//! subsequent `-` also map to `/`.

mod discovery;
mod parser;

use anyhow::Result;
use std::path::Path;

use super::{Conversation, SessionAdapter, SessionFile};

pub struct ClaudeAdapter;

impl SessionAdapter for ClaudeAdapter {
    fn name(&self) -> &str {
        "claude"
    }

    fn display_name(&self) -> &str {
        "Claude Code"
    }

    fn discover_sessions(&self) -> Result<Vec<SessionFile>> {
        discovery::discover_sessions()
    }

    fn parse_session(&self, path: &Path) -> Result<Conversation> {
        parser::parse_claude_session(path)
    }

    fn can_handle(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        path_str.contains(".claude/projects/") && path_str.ends_with(".jsonl")
    }
}

#[cfg(test)]
mod tests;

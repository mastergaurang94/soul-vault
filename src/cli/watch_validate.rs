//! Validation checks for auto-watch mode.

use anyhow::Result;
use std::path::PathBuf;

pub(crate) fn validate_auto_watch_prereqs(
    is_tty: bool,
    base_dirs: &[(String, PathBuf)],
) -> Result<()> {
    if !is_tty {
        anyhow::bail!(
            "Auto-watch requires a terminal.\n      \
             → Usage: soul watch <folder>\n      \
             → Or run `soul import` for one-time provider import."
        );
    }

    if base_dirs.is_empty() {
        anyhow::bail!(
            "No provider directories found.\n      \
             → Looked for: ~/.claude/projects/, ~/.openclaw/agents/, ~/.gemini/tmp/, ~/.codex/sessions/\n      \
             → You can also specify a folder: soul watch <folder>"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_auto_watch_requires_tty() {
        let dirs = vec![("Claude Code".to_string(), PathBuf::from("/tmp"))];
        let err = validate_auto_watch_prereqs(false, &dirs).unwrap_err();
        assert!(err.to_string().contains("Auto-watch requires a terminal"));
    }

    #[test]
    fn validate_auto_watch_requires_provider_dirs() {
        let err = validate_auto_watch_prereqs(true, &[]).unwrap_err();
        assert!(err.to_string().contains("No provider directories found"));
    }

    #[test]
    fn validate_auto_watch_ok_when_tty_and_dirs_present() {
        let dirs = vec![("Codex".to_string(), PathBuf::from("/tmp"))];
        assert!(validate_auto_watch_prereqs(true, &dirs).is_ok());
    }
}

//! `soul reset` — wipe vault and config, returning to pre-init state.

use anyhow::{bail, Result};
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::Path;

use crate::ui::theme::*;
use crate::vault::config;

// ─── Safety ───────────────────────────────────────────────────────────────────

/// Validates that a path is safe to delete. Returns true only if:
/// - Path is not `/`, `~`, or the home directory itself
/// - Path contains "soul-vault"
/// - Path is inside the user's home directory
pub fn is_safe_to_delete(path: &Path) -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };

    let path_str = path.to_string_lossy();

    // Reject root, home, or tilde
    if path_str == "/" || path_str == "~" || path == home {
        return false;
    }

    // Must contain "soul-vault" somewhere in the path
    if !path_str.contains("soul-vault") {
        return false;
    }

    // Must be inside the home directory (starts_with handles both exact and prefix)
    if !path.starts_with(&home) {
        return false;
    }

    // Must not BE the home directory (already checked above, but be explicit)
    if path == home {
        return false;
    }

    true
}

// ─── Count Helpers ────────────────────────────────────────────────────────────

fn count_md_files(dir: &Path) -> usize {
    if !dir.exists() {
        return 0;
    }
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ext == "md")
                })
                .count()
        })
        .unwrap_or(0)
}

// ─── Reset Command ───────────────────────────────────────────────────────────

pub fn run(force: bool) -> Result<()> {
    let vault_root = config::vault_root();

    // Check if vault exists
    if !vault_root.exists() || !config::is_initialized() {
        println!("\n  {} Nothing to reset — vault not initialized.\n", dim(ICON_DOT));
        return Ok(());
    }

    let config_dir = config::config_dir();

    // Validate safety before anything else
    if !is_safe_to_delete(&vault_root) {
        bail!(
            "Refusing to delete path: {}\n      \
             → Path failed safety validation. It must be inside your home directory and contain \"soul-vault\".",
            vault_root.display()
        );
    }

    // Gather stats for display
    let memory_count = count_md_files(&config::memories_dir());
    let topic_count = count_md_files(&config::topics_dir());
    let people_count = count_md_files(&config::people_dir());

    if !force {
        // Check TTY — if not a terminal, require --force
        if !io::stdin().is_terminal() {
            bail!(
                "Cannot confirm reset in non-interactive mode.\n      \
                 → Use `soul reset --force` to skip confirmation."
            );
        }

        // Show warning
        println!();
        println!(
            "  {} {}",
            red("⚠"),
            red("This will delete your entire Soul Vault and all configuration.")
        );
        println!();
        println!("  {}", bold_white("What will be deleted:"));
        println!(
            "    {} Vault path:    {}",
            ICON_FOLDER,
            cyan(&vault_root.display().to_string())
        );
        println!(
            "    {} Config dir:    {}",
            dim("⚙"),
            cyan(&config_dir.display().to_string())
        );
        println!(
            "    {} {} memories, {} topics, {} people files",
            ICON_BRAIN,
            bold_white(&memory_count.to_string()),
            bold_white(&topic_count.to_string()),
            bold_white(&people_count.to_string()),
        );
        println!();

        // Require typing "reset"
        print!("  Type '{}' to confirm: ", bold_white("reset"));
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();

        if trimmed != "reset" {
            println!("{}", dim("\n  Cancelled.\n"));
            return Ok(());
        }
    }

    // Perform deletion
    fs::remove_dir_all(&vault_root).map_err(|e| {
        anyhow::anyhow!(
            "Failed to delete vault at {}: {}\n      → Check file permissions.",
            vault_root.display(),
            e
        )
    })?;

    println!(
        "\n{}\n",
        check("Vault reset. Run `soul init` to start fresh.")
    );

    Ok(())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_safe_to_delete_valid_soul_vault_path() {
        let home = dirs::home_dir().unwrap();
        let path = home.join("soul-vault");
        assert!(is_safe_to_delete(&path));
    }

    #[test]
    fn test_safe_to_delete_nested_soul_vault_path() {
        let home = dirs::home_dir().unwrap();
        let path = home.join("projects").join("soul-vault");
        assert!(is_safe_to_delete(&path));
    }

    #[test]
    fn test_reject_root() {
        assert!(!is_safe_to_delete(Path::new("/")));
    }

    #[test]
    fn test_reject_home_dir() {
        let home = dirs::home_dir().unwrap();
        assert!(!is_safe_to_delete(&home));
    }

    #[test]
    fn test_reject_tilde() {
        assert!(!is_safe_to_delete(Path::new("~")));
    }

    #[test]
    fn test_reject_path_without_soul_vault() {
        let home = dirs::home_dir().unwrap();
        let path = home.join("Documents");
        assert!(!is_safe_to_delete(&path));
    }

    #[test]
    fn test_reject_path_outside_home() {
        assert!(!is_safe_to_delete(Path::new("/tmp/soul-vault")));
    }

    #[test]
    fn test_reject_etc_path_with_soul_vault() {
        assert!(!is_safe_to_delete(Path::new("/etc/soul-vault")));
    }

    #[test]
    fn test_count_md_files_nonexistent() {
        assert_eq!(count_md_files(Path::new("/nonexistent/path")), 0);
    }

    #[test]
    fn test_count_md_files_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(count_md_files(tmp.path()), 0);
    }

    #[test]
    fn test_count_md_files_with_files() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("one.md"), "# One").unwrap();
        fs::write(tmp.path().join("two.md"), "# Two").unwrap();
        fs::write(tmp.path().join("three.txt"), "Not markdown").unwrap();
        assert_eq!(count_md_files(tmp.path()), 2);
    }

    #[test]
    fn test_safe_to_delete_rejects_slash_soul_vault_outside_home() {
        // /soul-vault is outside home directory
        let path = PathBuf::from("/soul-vault");
        assert!(!is_safe_to_delete(&path));
    }
}

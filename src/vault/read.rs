//! Vault reading: stats, content, markdown file access.

use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::types::{NamedContent, ProviderStatus, VaultContent, VaultStats};
use crate::vault::config::{
    identity_dir, memories_dir, people_dir, read_config, topics_dir,
};

// ─── Vault Stats ──────────────────────────────────────────────────────────────

/// Computes vault statistics from the filesystem.
pub fn get_vault_stats() -> Result<VaultStats> {
    let config = read_config()?;

    let memory_count = count_md_files(&memories_dir());
    let topic_count = count_md_files(&topics_dir());
    let people_count = count_md_files(&people_dir());

    let providers: Vec<ProviderStatus> = config
        .providers
        .iter()
        .map(|p| ProviderStatus {
            name: p.name.clone(),
            connected: p.enabled,
            last_pull: p.last_pull.clone(),
        })
        .collect();

    Ok(VaultStats {
        memory_count,
        topic_count,
        people_count,
        last_sync: config.last_sync,
        providers,
        vault_path: config.vault_path,
    })
}

// ─── Read All Vault Content ───────────────────────────────────────────────────

/// Reads entire vault into memory for export.
pub fn read_vault_content() -> Result<VaultContent> {
    let identity = safe_read_file(&identity_dir().join("profile.md"));
    let preferences = safe_read_file(&identity_dir().join("preferences.md"));
    let memories = read_dir_markdown(&memories_dir());
    let topics = read_dir_markdown(&topics_dir());
    let people = read_dir_markdown(&people_dir());

    Ok(VaultContent {
        identity,
        preferences,
        memories: memories
            .into_iter()
            .map(|(name, content)| NamedContent { name, content })
            .collect(),
        topics: topics
            .into_iter()
            .map(|(name, content)| NamedContent { name, content })
            .collect(),
        people: people
            .into_iter()
            .map(|(name, content)| NamedContent { name, content })
            .collect(),
    })
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

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

fn safe_read_file(path: &Path) -> String {
    if !path.exists() {
        return String::new();
    }
    fs::read_to_string(path).unwrap_or_default()
}

fn read_dir_markdown(dir: &Path) -> Vec<(String, String)> {
    if !dir.exists() {
        return Vec::new();
    }
    let mut entries: Vec<(String, String)> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .filter_map(|e| {
            let path = e.path();
            let name = path.file_stem()?.to_str()?.to_string();
            let content = fs::read_to_string(&path).ok()?;
            Some((name, content))
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

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
        fs::write(tmp.path().join("test1.md"), "content").unwrap();
        fs::write(tmp.path().join("test2.md"), "content").unwrap();
        fs::write(tmp.path().join("test3.txt"), "content").unwrap();
        assert_eq!(count_md_files(tmp.path()), 2);
    }

    #[test]
    fn test_safe_read_file_missing() {
        assert_eq!(safe_read_file(Path::new("/nonexistent")), "");
    }

    #[test]
    fn test_safe_read_file_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.md");
        fs::write(&path, "hello world").unwrap();
        assert_eq!(safe_read_file(&path), "hello world");
    }

    #[test]
    fn test_read_dir_markdown() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("beta.md"), "beta content").unwrap();
        fs::write(tmp.path().join("alpha.md"), "alpha content").unwrap();
        fs::write(tmp.path().join("gamma.txt"), "ignored").unwrap();

        let entries = read_dir_markdown(tmp.path());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, "alpha");
        assert_eq!(entries[1].0, "beta");
    }
}

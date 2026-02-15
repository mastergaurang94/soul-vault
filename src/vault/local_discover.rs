//! Local file discovery helpers.

use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::types::FileInfo;
use crate::vault::chatgpt;

const SUPPORTED_EXTENSIONS: &[&str] = &["md", "txt", "json", "jsonl", "zip"];
const IGNORED_DIRS: &[&str] = &[".git", ".config", "node_modules", ".DS_Store"];

/// Recursively discovers supported files in a directory.
pub fn discover_files(dir_path: &Path) -> Result<Vec<FileInfo>> {
    if chatgpt::is_chatgpt_export_dir(dir_path) {
        let conv_path = dir_path.join("conversations.json");
        let metadata = fs::metadata(&conv_path)?;
        return Ok(vec![FileInfo {
            path: conv_path,
            name: "conversations".to_string(),
            extension: ".json".to_string(),
            size: metadata.len(),
        }]);
    }

    let supported: HashSet<&str> = SUPPORTED_EXTENSIONS.iter().copied().collect();
    let ignored: HashSet<&str> = IGNORED_DIRS.iter().copied().collect();
    let mut files = Vec::new();
    walk_dir(dir_path, &supported, &ignored, &mut files)?;
    Ok(files)
}

fn walk_dir(
    dir_path: &Path,
    supported: &HashSet<&str>,
    ignored: &HashSet<&str>,
    results: &mut Vec<FileInfo>,
) -> Result<()> {
    let entries = fs::read_dir(dir_path)?;

    for entry in entries.flatten() {
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || ignored.contains(name_str.as_ref()) {
                continue;
            }
            walk_dir(&path, supported, ignored, results)?;
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !supported.contains(ext.as_str()) {
            continue;
        }

        let metadata = fs::metadata(&path)?;
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        results.push(FileInfo {
            path: path.clone(),
            name,
            extension: format!(".{}", ext),
            size: metadata.len(),
        });
    }

    Ok(())
}

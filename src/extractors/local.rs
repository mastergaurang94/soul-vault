//! Local file reader: discovers and reads .md, .txt, .json, .jsonl files.

use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::types::FileInfo;

// ─── Constants ────────────────────────────────────────────────────────────────

const SUPPORTED_EXTENSIONS: &[&str] = &["md", "txt", "json", "jsonl"];
const IGNORED_DIRS: &[&str] = &[".git", ".config", "node_modules", ".DS_Store"];

// ─── File Discovery ───────────────────────────────────────────────────────────

/// Recursively discovers supported files in a directory.
pub fn discover_files(dir_path: &Path) -> Result<Vec<FileInfo>> {
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
        } else if file_type.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if supported.contains(ext.as_str()) {
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
        }
    }

    Ok(())
}

// ─── File Content Extraction ──────────────────────────────────────────────────

/// Reads and normalizes file content based on extension.
pub fn extract_file_content(file: &FileInfo) -> Result<String> {
    match file.extension.as_str() {
        ".jsonl" => extract_jsonl(&file.path),
        ".json" => extract_json(&file.path),
        _ => {
            let content = fs::read_to_string(&file.path)?;
            Ok(content.trim().to_string())
        }
    }
}

// ─── JSONL Parsing ────────────────────────────────────────────────────────────

fn extract_jsonl(file_path: &Path) -> Result<String> {
    let content = fs::read_to_string(file_path)?;
    let lines: Vec<String> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(val) => {
                    if val.is_string() {
                        val.as_str().unwrap_or(line).to_string()
                    } else {
                        serde_json::to_string_pretty(&val).unwrap_or_else(|_| line.to_string())
                    }
                }
                Err(_) => line.to_string(),
            }
        })
        .collect();
    Ok(lines.join("\n"))
}

// ─── JSON Parsing ─────────────────────────────────────────────────────────────

fn extract_json(file_path: &Path) -> Result<String> {
    let content = fs::read_to_string(file_path)?;
    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Ok(content), // treat as text if invalid JSON
    };

    if let Some(arr) = parsed.as_array() {
        let formatted: Vec<String> = arr.iter().map(format_conversation).collect();
        Ok(formatted.join("\n\n---\n\n"))
    } else if is_conversation_object(&parsed) {
        Ok(format_conversation(&parsed))
    } else {
        Ok(serde_json::to_string_pretty(&parsed)?)
    }
}

// ─── Conversation Formatting ──────────────────────────────────────────────────

fn is_conversation_object(obj: &serde_json::Value) -> bool {
    obj.is_object()
        && (obj.get("messages").is_some()
            || obj.get("conversation").is_some()
            || obj.get("mapping").is_some())
}

fn format_conversation(item: &serde_json::Value) -> String {
    let obj = match item.as_object() {
        Some(o) => o,
        None => return serde_json::to_string_pretty(item).unwrap_or_default(),
    };

    // Standard messages format
    if let Some(messages) = obj.get("messages").and_then(|m| m.as_array()) {
        let title = obj
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or("Untitled conversation");
        let body: Vec<String> = messages
            .iter()
            .map(|m| {
                let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("unknown");
                let content = m.get("content").and_then(|c| c.as_str()).unwrap_or("");
                format!("{}: {}", role, content)
            })
            .collect();
        return format!("## {}\n\n{}", title, body.join("\n"));
    }

    // ChatGPT export format (mapping-based)
    if let (Some(title), Some(mapping)) = (
        obj.get("title").and_then(|t| t.as_str()),
        obj.get("mapping").and_then(|m| m.as_object()),
    ) {
        let extracted = extract_chatgpt_mapping(mapping);
        return format!("## {}\n\n{}", title, extracted);
    }

    serde_json::to_string_pretty(item).unwrap_or_default()
}

fn extract_chatgpt_mapping(
    mapping: &serde_json::Map<String, serde_json::Value>,
) -> String {
    let mut messages = Vec::new();
    for node in mapping.values() {
        let msg = match node.get("message") {
            Some(m) if !m.is_null() => m,
            _ => continue,
        };
        let role = msg
            .pointer("/author/role")
            .and_then(|r| r.as_str())
            .unwrap_or("unknown");
        let parts = msg.pointer("/content/parts").and_then(|p| p.as_array());
        if let Some(parts) = parts {
            let text: Vec<&str> = parts
                .iter()
                .filter_map(|p| p.as_str())
                .filter(|s| !s.is_empty())
                .collect();
            if !text.is_empty() {
                messages.push(format!("{}: {}", role, text.join(" ")));
            }
        }
    }
    messages.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_files() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("test.md"), "# Hello").unwrap();
        fs::write(tmp.path().join("notes.txt"), "Some notes").unwrap();
        fs::write(tmp.path().join("data.json"), "{}").unwrap();
        fs::write(tmp.path().join("log.jsonl"), "{}").unwrap();
        fs::write(tmp.path().join("image.png"), "binary").unwrap();

        let files = discover_files(tmp.path()).unwrap();
        assert_eq!(files.len(), 4);
    }

    #[test]
    fn test_discover_files_ignores_hidden() {
        let tmp = tempfile::tempdir().unwrap();
        let hidden = tmp.path().join(".hidden");
        fs::create_dir_all(&hidden).unwrap();
        fs::write(hidden.join("secret.md"), "secret").unwrap();
        fs::write(tmp.path().join("visible.md"), "visible").unwrap();

        let files = discover_files(tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "visible");
    }

    #[test]
    fn test_discover_files_recursive() {
        let tmp = tempfile::tempdir().unwrap();
        let subdir = tmp.path().join("subdir");
        fs::create_dir_all(&subdir).unwrap();
        fs::write(tmp.path().join("root.md"), "root").unwrap();
        fs::write(subdir.join("nested.md"), "nested").unwrap();

        let files = discover_files(tmp.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_extract_text_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("notes.txt");
        fs::write(&path, "  Hello world  \n").unwrap();

        let file = FileInfo {
            path,
            name: "notes".to_string(),
            extension: ".txt".to_string(),
            size: 15,
        };
        let content = extract_file_content(&file).unwrap();
        assert_eq!(content, "Hello world");
    }

    #[test]
    fn test_extract_json_messages() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("chat.json");
        let json = r#"[{"title": "Test", "messages": [{"role": "user", "content": "Hello"}]}]"#;
        fs::write(&path, json).unwrap();

        let file = FileInfo {
            path,
            name: "chat".to_string(),
            extension: ".json".to_string(),
            size: json.len() as u64,
        };
        let content = extract_file_content(&file).unwrap();
        assert!(content.contains("## Test"));
        assert!(content.contains("user: Hello"));
    }

    #[test]
    fn test_extract_jsonl() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("session.jsonl");
        let jsonl = r#"{"role": "user", "content": "Hello"}
{"role": "assistant", "content": "Hi there"}"#;
        fs::write(&path, jsonl).unwrap();

        let file = FileInfo {
            path,
            name: "session".to_string(),
            extension: ".jsonl".to_string(),
            size: jsonl.len() as u64,
        };
        let content = extract_file_content(&file).unwrap();
        assert!(content.contains("user"));
        assert!(content.contains("Hello"));
    }

    #[test]
    fn test_extract_chatgpt_mapping() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("export.json");
        let json = r#"{
            "title": "Test Chat",
            "mapping": {
                "node-1": {
                    "message": {
                        "author": {"role": "user"},
                        "content": {"parts": ["Hello from ChatGPT export"]}
                    }
                },
                "node-empty": { "message": null }
            }
        }"#;
        fs::write(&path, json).unwrap();

        let file = FileInfo {
            path,
            name: "export".to_string(),
            extension: ".json".to_string(),
            size: json.len() as u64,
        };
        let content = extract_file_content(&file).unwrap();
        assert!(content.contains("## Test Chat"));
        assert!(content.contains("Hello from ChatGPT export"));
    }
}

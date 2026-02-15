//! ChatGPT export detection helpers.

use std::fs;
use std::path::Path;

/// Checks if a zip file is a ChatGPT export (contains `conversations.json`).
pub fn is_chatgpt_zip(path: &Path) -> bool {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return false,
    };

    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            let name = entry.name().to_string();
            if name == "conversations.json" || name.ends_with("/conversations.json") {
                return true;
            }
        }
    }

    false
}

/// Checks if a directory is an extracted ChatGPT export (contains `conversations.json`).
pub fn is_chatgpt_export_dir(dir: &Path) -> bool {
    dir.is_dir() && dir.join("conversations.json").is_file()
}

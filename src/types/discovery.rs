//! Local file discovery and chunking metadata.

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name: String,
    pub extension: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub content: String,
    pub source: String,
    pub index: usize,
    pub total: usize,
}

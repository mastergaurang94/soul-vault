//! Data types for source tracking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesConfig {
    pub sources: Vec<SourceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceEntry {
    pub path: String,
    pub files_ingested: usize,
    pub last_ingested: String,
    pub file_hashes: HashMap<String, String>,
}

/// Result of classifying files against previous ingestion.
#[derive(Debug, Default)]
pub struct IngestClassification {
    pub new_files: Vec<PathBuf>,
    pub modified_files: Vec<PathBuf>,
    pub skipped_files: Vec<PathBuf>,
}

impl IngestClassification {
    pub fn all_to_ingest(&self) -> Vec<PathBuf> {
        let mut result = self.new_files.clone();
        result.extend(self.modified_files.iter().cloned());
        result
    }
}

#[derive(Debug)]
pub struct SourceSummary {
    pub path: String,
    pub files_ingested: usize,
    pub last_ingested: String,
}

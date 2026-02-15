//! Memory merging and deduplication public API.

use crate::types::{ChunkInfo, ExtractedMemories};

pub fn merge_all_memories(results: &[ExtractedMemories]) -> ExtractedMemories {
    crate::core::merger_dedup::merge_all_memories(results)
}

pub fn chunk_text(text: &str, source: &str) -> Vec<ChunkInfo> {
    crate::core::merger_chunk::chunk_text(text, source)
}

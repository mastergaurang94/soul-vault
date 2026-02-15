//! Text chunking helpers for LLM processing.

use crate::types::ChunkInfo;

/// Max characters per chunk (~20K tokens, safe for Claude).
const MAX_CHUNK_CHARS: usize = 80_000;

/// Splits text into LLM-friendly chunks, breaking at paragraph boundaries.
pub(crate) fn chunk_text(text: &str, source: &str) -> Vec<ChunkInfo> {
    if text.len() <= MAX_CHUNK_CHARS {
        return vec![ChunkInfo {
            content: text.to_string(),
            source: source.to_string(),
            index: 0,
            total: 1,
        }];
    }

    let raw_chunks = split_at_paragraphs(text, MAX_CHUNK_CHARS);
    let final_chunks = force_split_oversized(&raw_chunks, MAX_CHUNK_CHARS);
    let total = final_chunks.len();

    final_chunks
        .into_iter()
        .enumerate()
        .map(|(index, content)| ChunkInfo {
            content,
            source: source.to_string(),
            index,
            total,
        })
        .collect()
}

fn split_at_paragraphs(text: &str, max_size: usize) -> Vec<String> {
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let mut chunks = Vec::new();
    let mut current = String::new();

    for para in paragraphs {
        let combined = if current.is_empty() {
            para.to_string()
        } else {
            format!("{}\n\n{}", current, para)
        };

        if combined.len() > max_size && !current.is_empty() {
            chunks.push(current.trim().to_string());
            current = para.to_string();
        } else {
            current = combined;
        }
    }

    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }

    chunks
}

fn force_split_oversized(chunks: &[String], max_size: usize) -> Vec<String> {
    let mut result = Vec::new();

    for chunk in chunks {
        if chunk.len() <= max_size {
            result.push(chunk.clone());
            continue;
        }

        let bytes = chunk.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let end = std::cmp::min(i + max_size, bytes.len());
            let actual_end = if end < bytes.len() {
                let mut e = end;
                while e > i && !chunk.is_char_boundary(e) {
                    e -= 1;
                }
                e
            } else {
                end
            };
            result.push(chunk[i..actual_end].to_string());
            i = actual_end;
        }
    }

    result
}

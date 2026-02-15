//! LLM memory extraction — sends chunks to Claude via reqwest.

use anyhow::{Context, Result};
use serde_json::json;

use crate::core::parser::parse_extraction_response;
use crate::core::prompt::EXTRACTION_PROMPT;
use crate::types::{ChunkInfo, ExtractedMemories, SoulVaultError};
use crate::vault::config::get_api_key;

// ─── API Constants ────────────────────────────────────────────────────────────

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MODEL: &str = "claude-sonnet-4-20250514";
const MAX_TOKENS: u32 = 4096;

// ─── Process a Single Chunk ───────────────────────────────────────────────────

/// Sends a text chunk to Claude for memory extraction.
/// Returns structured memories parsed and validated.
pub async fn process_chunk(
    client: &reqwest::Client,
    chunk: &ChunkInfo,
) -> Result<ExtractedMemories> {
    let api_key = get_api_key("claude")?
        .ok_or(SoulVaultError::MissingApiKey {
            provider: "Claude".to_string(),
        })?;

    let chunk_label = if chunk.total > 1 {
        format!(" (Part {} of {})", chunk.index + 1, chunk.total)
    } else {
        String::new()
    };

    let user_content = format!(
        "{}\n\nSource: {}{}\n\n---\n\n{}",
        EXTRACTION_PROMPT, chunk.source, chunk_label, chunk.content
    );

    let request_body = json!({
        "model": MODEL,
        "max_tokens": MAX_TOKENS,
        "messages": [
            {
                "role": "user",
                "content": user_content
            }
        ]
    });

    let response = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", &api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .context("Failed to connect to Anthropic API")?;

    let status = response.status();

    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(SoulVaultError::RateLimited.into());
    }

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(SoulVaultError::MissingApiKey {
            provider: "Claude".to_string(),
        }
        .into());
    }

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(SoulVaultError::ApiError {
            status: status.as_u16(),
            message: body,
        }
        .into());
    }

    let body: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse API response")?;

    let response_text = body
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| {
            arr.iter()
                .filter(|block| block.get("type").and_then(|t| t.as_str()) == Some("text"))
                .filter_map(|block| block.get("text").and_then(|t| t.as_str()))
                .next()
        })
        .unwrap_or("");

    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    Ok(parse_extraction_response(response_text, &chunk.source, &date))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_url() {
        assert_eq!(ANTHROPIC_API_URL, "https://api.anthropic.com/v1/messages");
    }

    #[test]
    fn test_model_name() {
        assert!(MODEL.contains("claude"));
    }

    #[test]
    fn test_chunk_label_single() {
        let chunk = ChunkInfo {
            content: "test".to_string(),
            source: "test".to_string(),
            index: 0,
            total: 1,
        };
        let label = if chunk.total > 1 {
            format!(" (Part {} of {})", chunk.index + 1, chunk.total)
        } else {
            String::new()
        };
        assert_eq!(label, "");
    }

    #[test]
    fn test_chunk_label_multi() {
        let chunk = ChunkInfo {
            content: "test".to_string(),
            source: "test".to_string(),
            index: 2,
            total: 5,
        };
        let label = if chunk.total > 1 {
            format!(" (Part {} of {})", chunk.index + 1, chunk.total)
        } else {
            String::new()
        };
        assert_eq!(label, " (Part 3 of 5)");
    }
}

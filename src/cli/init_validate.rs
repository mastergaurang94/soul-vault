//! API key validation helpers used by `soul init`.

use reqwest::StatusCode;
use serde_json::json;

use crate::types::Provider;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const ANTHROPIC_MODEL: &str = "claude-sonnet-4-20250514";
const OPENAI_MODELS_URL: &str = "https://api.openai.com/v1/models";
const GEMINI_MODELS_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

pub enum ApiKeyValidation {
    Verified,
    Invalid(String),
    Unverified(String),
}

pub async fn validate_api_key(provider: &Provider, key: &str) -> ApiKeyValidation {
    match provider {
        Provider::Claude => validate_claude_key(key).await,
        Provider::ChatGpt => validate_chatgpt_key(key).await,
        Provider::Gemini => validate_gemini_key(key).await,
    }
}

async fn validate_claude_key(key: &str) -> ApiKeyValidation {
    if !key.starts_with("sk-ant-") {
        return ApiKeyValidation::Invalid(
            "Expected key format to start with `sk-ant-`.".to_string(),
        );
    }

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            return ApiKeyValidation::Unverified(format!("Could not build HTTP client: {}", e));
        }
    };

    let request_body = json!({
        "model": ANTHROPIC_MODEL,
        "max_tokens": 1,
        "messages": [{ "role": "user", "content": "ping" }]
    });

    let response = match client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            return ApiKeyValidation::Unverified(format!(
                "Network check failed (saved anyway): {}",
                e
            ));
        }
    };

    match response.status() {
        StatusCode::OK | StatusCode::TOO_MANY_REQUESTS => ApiKeyValidation::Verified,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            ApiKeyValidation::Invalid("Key was rejected by Anthropic (401/403).".to_string())
        }
        status => unverified_from_status(response, status, "Anthropic").await,
    }
}

async fn validate_chatgpt_key(key: &str) -> ApiKeyValidation {
    if !key.starts_with("sk-") {
        return ApiKeyValidation::Invalid("Expected key format to start with `sk-`.".to_string());
    }

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            return ApiKeyValidation::Unverified(format!("Could not build HTTP client: {}", e));
        }
    };

    let response = match client
        .get(OPENAI_MODELS_URL)
        .header("authorization", format!("Bearer {}", key))
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            return ApiKeyValidation::Unverified(format!(
                "Network check failed (saved anyway): {}",
                e
            ));
        }
    };

    match response.status() {
        StatusCode::OK | StatusCode::TOO_MANY_REQUESTS => ApiKeyValidation::Verified,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            ApiKeyValidation::Invalid("Key was rejected by OpenAI (401/403).".to_string())
        }
        status => unverified_from_status(response, status, "OpenAI").await,
    }
}

async fn validate_gemini_key(key: &str) -> ApiKeyValidation {
    if !key.starts_with("AIza") {
        return ApiKeyValidation::Invalid(
            "Expected Gemini key format to start with `AIza`.".to_string(),
        );
    }

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            return ApiKeyValidation::Unverified(format!("Could not build HTTP client: {}", e));
        }
    };

    let response = match client
        .get(GEMINI_MODELS_URL)
        .query(&[("key", key)])
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            return ApiKeyValidation::Unverified(format!(
                "Network check failed (saved anyway): {}",
                e
            ));
        }
    };

    match response.status() {
        StatusCode::OK | StatusCode::TOO_MANY_REQUESTS => ApiKeyValidation::Verified,
        StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            ApiKeyValidation::Invalid("Key was rejected by Gemini API.".to_string())
        }
        status => unverified_from_status(response, status, "Gemini").await,
    }
}

async fn unverified_from_status(
    response: reqwest::Response,
    status: StatusCode,
    provider: &str,
) -> ApiKeyValidation {
    let body = response.text().await.unwrap_or_default();
    ApiKeyValidation::Unverified(format!(
        "{} returned {} during validation{}",
        provider,
        status.as_u16(),
        if body.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", trim_for_display(&body, 180))
        }
    ))
}

fn trim_for_display(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let mut s = trimmed.chars().take(max_chars).collect::<String>();
    s.push_str("...");
    s
}

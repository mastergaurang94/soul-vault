//! Cloud provider clients and resilient HTTP helpers.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use serde_json::Value;

use crate::auth::ensure_valid_credentials;
use crate::types::Provider;

use super::cloud_types::{CloudConversation, CloudConversationStub, CloudFetchPage, CloudMessage};

pub(crate) trait CloudProviderClient: Send + Sync {
    fn list_conversations(
        &self,
        cursor: Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<CloudFetchPage>> + Send + '_>>;

    fn fetch_conversation<'a>(
        &'a self,
        conversation_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<CloudConversation>> + Send + 'a>>;
}

pub(crate) fn build_cloud_client(provider: Provider) -> Box<dyn CloudProviderClient> {
    Box::new(HttpCloudClient::new(provider))
}

struct HttpCloudClient {
    provider: Provider,
    base_url: String,
    list_path: String,
    detail_path_template: String,
    items_path_template: Option<String>,
    cursor_param: Option<String>,
    client: Client,
}

impl HttpCloudClient {
    fn new(provider: Provider) -> Self {
        let (base_url, list_path, detail_path_template, items_path_template, cursor_param) =
            match provider {
            Provider::Claude => (
                std::env::var("SOUL_CLOUD_CLAUDE_BASE_URL")
                    .unwrap_or_else(|_| "https://api.anthropic.com/v1".to_string()),
                "/conversations".to_string(),
                "/conversations/{id}".to_string(),
                None,
                Some("cursor".to_string()),
            ),
            Provider::ChatGpt => (
                std::env::var("SOUL_CLOUD_CHATGPT_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
                "/conversations".to_string(),
                "/conversations/{id}".to_string(),
                Some("/conversations/{id}/items".to_string()),
                Some("cursor".to_string()),
            ),
            Provider::Gemini => (
                std::env::var("SOUL_CLOUD_GEMINI_BASE_URL")
                    .unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta".to_string()),
                "/interactions".to_string(),
                "/interactions/{id}".to_string(),
                None,
                Some("pageToken".to_string()),
            ),
            };

        Self {
            provider,
            base_url,
            list_path,
            detail_path_template,
            items_path_template,
            cursor_param,
            client: Client::new(),
        }
    }

    async fn bearer_token(&self) -> Result<String> {
        let creds = ensure_valid_credentials(&self.provider).await?;
        if let Some(creds) = creds {
            if !creds.access_token.trim().is_empty() {
                return Ok(creds.access_token);
            }
        }

        if let Some(token) = env_access_token(&self.provider) {
            return Ok(token);
        }

        bail!(
            "{} cloud import requires OAuth. Run `soul login {}` or set {} and retry.",
            self.provider.display_name(),
            self.provider,
            provider_token_env(&self.provider)
        )
    }

    async fn get_json(&self, path: &str, query: &[(&str, String)]) -> Result<Value> {
        let token = self.bearer_token().await?;
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);

        retry_with_backoff(async || {
            let mut request = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {token}"))
                .header("Accept", "application/json");

            if matches!(self.provider, Provider::Claude) {
                request = request.header("anthropic-version", "2023-06-01");
            }

            if !query.is_empty() {
                request = request.query(query);
            }

            let response = request.send().await.context("Request failed")?;
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            if status.is_success() {
                let json: Value = serde_json::from_str(&body)
                    .with_context(|| format!("{} returned non-JSON response", self.provider))?;
                return Ok(json);
            }

            Err(classify_http_error(self.provider.clone(), status, &body))
        })
        .await
    }
}

impl CloudProviderClient for HttpCloudClient {
    fn list_conversations(
        &self,
        cursor: Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<CloudFetchPage>> + Send + '_>> {
        Box::pin(async move {
            let mut query = Vec::new();
            if let Some(cursor) = cursor {
                if let Some(param) = &self.cursor_param {
                    query.push((param.as_str(), cursor));
                }
            }

            let value = self.get_json(&self.list_path, &query).await?;
            parse_list_page(&self.provider, value)
        })
    }

    fn fetch_conversation<'a>(
        &'a self,
        conversation_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<CloudConversation>> + Send + 'a>> {
        Box::pin(async move {
            let path = self
                .detail_path_template
                .replace("{id}", &percent_encode(conversation_id));
            let value = self.get_json(&path, &[]).await?;
            let mut conversation = parse_conversation(self.provider.clone(), conversation_id, value)?;

            // OpenAI conversation bodies may omit full message items; fetch them when needed.
            if conversation.messages.is_empty() {
                if let Some(template) = &self.items_path_template {
                    let items_path = template.replace("{id}", &percent_encode(conversation_id));
                    let items_value = self.get_json(&items_path, &[]).await?;
                    conversation.messages = parse_items_messages(items_value);
                }
            }

            if conversation.messages.is_empty() {
                bail!(
                    "{} conversation {} has no parseable messages",
                    self.provider.display_name(),
                    conversation.conversation_id
                );
            }

            Ok(conversation)
        })
    }
}

async fn retry_with_backoff<F, Fut, T>(mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut delay = Duration::from_millis(500);
    let attempts = 5;

    for attempt in 1..=attempts {
        match f().await {
            Ok(value) => return Ok(value),
            Err(e) if is_retryable_error(&e) && attempt < attempts => {
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(8));
            }
            Err(e) => return Err(e),
        }
    }

    bail!("Unexpected retry loop termination")
}

fn is_retryable_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("rate limited")
        || msg.contains("temporary")
        || msg.contains("service unavailable")
        || msg.contains("timed out")
}

fn classify_http_error(provider: Provider, status: StatusCode, body: &str) -> anyhow::Error {
    let trimmed = body.trim();
    let excerpt = if trimmed.len() > 220 {
        format!("{}...", &trimmed[..220])
    } else {
        trimmed.to_string()
    };

    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => anyhow::anyhow!(
            "{} OAuth token was rejected ({}). Run `soul login {}` to reconnect. {}",
            provider.display_name(),
            status,
            provider,
            excerpt
        ),
        StatusCode::TOO_MANY_REQUESTS => anyhow::anyhow!(
            "{} cloud API rate limited (429). Retrying with backoff. {}",
            provider.display_name(),
            excerpt
        ),
        StatusCode::PAYMENT_REQUIRED => anyhow::anyhow!(
            "{} cloud quota/billing issue (402). {}",
            provider.display_name(),
            excerpt
        ),
        StatusCode::BAD_GATEWAY
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::GATEWAY_TIMEOUT => anyhow::anyhow!(
            "{} cloud service is temporarily unavailable ({}). {}",
            provider.display_name(),
            status,
            excerpt
        ),
        _ => anyhow::anyhow!(
            "{} cloud API error ({}). {}",
            provider.display_name(),
            status,
            excerpt
        ),
    }
}

fn parse_list_page(provider: &Provider, value: Value) -> Result<CloudFetchPage> {
    let next_cursor = value
        .get("next_cursor")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            value
                .get("nextPageToken")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| {
            value
                .get("paging")
                .and_then(|v| v.get("next"))
                .and_then(Value::as_str)
                .map(str::to_string)
        });

    let items_value = value
        .get("items")
        .or_else(|| value.get("data"))
        .or_else(|| value.get("conversations"))
        .or_else(|| value.get("interactions"));
    let raw_items = items_value.and_then(Value::as_array);

    let mut items = Vec::new();
    for raw in raw_items.into_iter().flatten() {
        let conversation_id = raw
            .get("id")
            .or_else(|| raw.get("conversation_id"))
            .or_else(|| raw.get("name"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();

        if conversation_id.is_empty() {
            continue;
        }

        let updated_at = parse_datetime(
            raw.get("updated_at")
                .or_else(|| raw.get("update_time"))
                .or_else(|| raw.get("last_message_at"))
                .or_else(|| raw.get("lastUpdateTime"))
                .or_else(|| raw.get("create_time")),
        );

        items.push(CloudConversationStub {
            conversation_id,
            updated_at,
        });
    }

    if matches!(raw_items, Some(arr) if arr.is_empty()) {
        return Ok(CloudFetchPage { items, next_cursor });
    }

    if items.is_empty() {
        bail!(
            "{} returned no parseable conversation entries from list response",
            provider.display_name()
        );
    }

    Ok(CloudFetchPage { items, next_cursor })
}

fn parse_conversation(provider: Provider, fallback_id: &str, value: Value) -> Result<CloudConversation> {
    let conversation_id = value
        .get("id")
        .or_else(|| value.get("conversation_id"))
        .or_else(|| value.get("name"))
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(fallback_id)
        .to_string();

    let title = value
        .get("title")
        .or_else(|| value.get("name"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let updated_at = parse_datetime(
        value
            .get("updated_at")
            .or_else(|| value.get("update_time"))
            .or_else(|| value.get("last_message_at"))
            .or_else(|| value.get("lastUpdateTime"))
            .or_else(|| value.get("create_time")),
    );

    let mut messages = Vec::new();

    if let Some(arr) = value.get("messages").and_then(Value::as_array) {
        messages.extend(arr.iter().filter_map(parse_message));
    } else if let Some(arr) = value.get("items").and_then(Value::as_array) {
        messages.extend(arr.iter().filter_map(parse_message));
    } else if let Some(arr) = value.get("events").and_then(Value::as_array) {
        messages.extend(arr.iter().filter_map(parse_message));
    } else if let Some(mapping) = value.get("mapping").and_then(Value::as_object) {
        for node in mapping.values() {
            if let Some(message) = node.get("message").and_then(parse_message) {
                messages.push(message);
            }
        }
    }

    Ok(CloudConversation {
        provider,
        conversation_id,
        title,
        updated_at,
        messages,
    })
}

fn parse_message(raw: &Value) -> Option<CloudMessage> {
    let role = raw
        .get("role")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            raw.get("author")
                .and_then(Value::as_str)
                .map(gemini_author_role)
        })
        .or_else(|| {
            raw.get("author")
                .and_then(|a| a.get("role"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })?;

    let content = raw
        .get("content")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            raw.get("content")
                .and_then(|v| v.get("text"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| {
            raw.get("content")
                .and_then(|v| v.get("parts"))
                .and_then(Value::as_array)
                .map(|parts| {
                    parts
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join("\n")
                })
        })
        .or_else(|| {
            raw.get("content")
                .and_then(Value::as_array)
                .map(|parts| {
                    parts
                        .iter()
                        .filter_map(|part| part.get("text").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
        })
        .or_else(|| {
            raw.get("output")
                .and_then(|v| v.get("text"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default();

    if content.trim().is_empty() {
        return None;
    }

    let timestamp = parse_datetime(
        raw.get("timestamp")
            .or_else(|| raw.get("created_at"))
            .or_else(|| raw.get("create_time")),
    );

    Some(CloudMessage {
        role,
        content,
        timestamp,
    })
}

fn parse_items_messages(value: Value) -> Vec<CloudMessage> {
    value
        .get("items")
        .or_else(|| value.get("data"))
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(parse_message).collect::<Vec<_>>())
        .unwrap_or_default()
}

fn parse_datetime(value: Option<&Value>) -> Option<DateTime<Utc>> {
    let value = value?;
    if let Some(s) = value.as_str() {
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Some(dt.with_timezone(&Utc));
        }
    }
    if let Some(seconds) = value.as_i64() {
        return DateTime::<Utc>::from_timestamp(seconds, 0);
    }
    None
}

fn env_access_token(provider: &Provider) -> Option<String> {
    std::env::var(provider_token_env(provider))
        .ok()
        .filter(|v| !v.trim().is_empty())
}

fn provider_token_env(provider: &Provider) -> &'static str {
    match provider {
        Provider::Claude => "SOUL_CLOUD_CLAUDE_ACCESS_TOKEN",
        Provider::ChatGpt => "SOUL_CLOUD_CHATGPT_ACCESS_TOKEN",
        Provider::Gemini => "SOUL_CLOUD_GEMINI_ACCESS_TOKEN",
    }
}

fn gemini_author_role(author: &str) -> String {
    match author.to_lowercase().as_str() {
        "user" => "user".to_string(),
        "model" | "assistant" => "assistant".to_string(),
        other => other.to_string(),
    }
}

fn percent_encode(raw: &str) -> String {
    let mut out = String::new();
    for b in raw.bytes() {
        let is_unreserved =
            matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~');
        if is_unreserved {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    use crate::auth::{load_credentials, save_credentials, AuthCredentials};
    use tokio::sync::Mutex;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvSnapshot {
        key: &'static str,
        value: Option<String>,
    }

    struct Retry429ThenOk {
        calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl Respond for Retry429ThenOk {
        fn respond(&self, _request: &Request) -> ResponseTemplate {
            let call = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if call == 0 {
                ResponseTemplate::new(429).set_body_string("rate limited")
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "conversations": [],
                    "next_cursor": null
                }))
            }
        }
    }

    struct PaginationResponder {
        calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl Respond for PaginationResponder {
        fn respond(&self, _request: &Request) -> ResponseTemplate {
            let call = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if call == 0 {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "conversations": [
                        {"id": "c1", "updated_at": "2026-01-01T00:00:00Z"}
                    ],
                    "next_cursor": "cursor-2"
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "conversations": [
                        {"id": "c2", "updated_at": "2026-01-02T00:00:00Z"}
                    ],
                    "next_cursor": null
                }))
            }
        }
    }

    impl EnvSnapshot {
        fn capture(key: &'static str) -> Self {
            Self {
                key,
                value: std::env::var(key).ok(),
            }
        }

        fn restore(self) {
            match self.value {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn parse_list_page_accepts_multiple_shapes() {
        let payload = serde_json::json!({
            "data": [
                {"id": "a", "title": "A", "updated_at": "2026-01-01T00:00:00Z"},
                {"conversation_id": "b", "name": "B", "version": "2"}
            ],
            "next_cursor": "nxt"
        });

        let page = parse_list_page(&Provider::Claude, payload).expect("list parse should succeed");
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].conversation_id, "a");
        assert_eq!(page.items[1].conversation_id, "b");
        assert_eq!(page.next_cursor.as_deref(), Some("nxt"));
    }

    #[test]
    fn parse_conversation_handles_messages_array() {
        let payload = serde_json::json!({
            "id": "conv-1",
            "messages": [
                {"role": "user", "content": "hello"},
                {"role": "assistant", "content": {"parts": ["hi"]}}
            ]
        });

        let conv = parse_conversation(Provider::Claude, "fallback", payload)
            .expect("conversation parse should succeed");
        assert_eq!(conv.conversation_id, "conv-1");
        assert_eq!(conv.messages.len(), 2);
    }

    #[test]
    fn parse_list_page_accepts_gemini_interactions_shape() {
        let payload = serde_json::json!({
            "interactions": [
                {"name": "interactions/abc", "title": "Session A", "lastUpdateTime": "2026-01-01T00:00:00Z"}
            ],
            "nextPageToken": "tok-2"
        });
        let page = parse_list_page(&Provider::Gemini, payload).expect("gemini list parse should succeed");
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].conversation_id, "interactions/abc");
        assert_eq!(page.next_cursor.as_deref(), Some("tok-2"));
    }

    #[test]
    fn parse_message_accepts_gemini_event_shape() {
        let raw = serde_json::json!({
            "author": "model",
            "content": [{"text": "hello from gemini"}],
            "timestamp": "2026-01-01T00:00:00Z"
        });
        let msg = parse_message(&raw).expect("gemini event should parse");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "hello from gemini");
    }

    #[test]
    fn parse_list_page_accepts_empty_list() {
        let payload = serde_json::json!({
            "data": [],
            "next_cursor": null
        });
        let page = parse_list_page(&Provider::ChatGpt, payload)
            .expect("empty list should be valid");
        assert!(page.items.is_empty());
        assert!(page.next_cursor.is_none());
    }

    #[test]
    fn retryable_error_detection_covers_rate_limit_and_timeout() {
        let rate_limited = anyhow::anyhow!("Rate limited by provider API");
        let timeout = anyhow::anyhow!("request timed out");
        let fatal = anyhow::anyhow!("invalid schema");

        assert!(is_retryable_error(&rate_limited));
        assert!(is_retryable_error(&timeout));
        assert!(!is_retryable_error(&fatal));
    }

    #[test]
    fn classify_http_error_is_actionable_per_provider() {
        let auth_err = classify_http_error(
            Provider::Claude,
            StatusCode::UNAUTHORIZED,
            "token rejected",
        );
        assert!(auth_err.to_string().contains("soul login claude"));

        let rate_err = classify_http_error(Provider::ChatGpt, StatusCode::TOO_MANY_REQUESTS, "");
        assert!(rate_err.to_string().contains("rate limited"));

        let quota_err = classify_http_error(Provider::Gemini, StatusCode::PAYMENT_REQUIRED, "");
        assert!(quota_err.to_string().contains("quota"));
    }

    #[test]
    fn provider_token_env_names_are_provider_scoped() {
        assert_eq!(
            provider_token_env(&Provider::Claude),
            "SOUL_CLOUD_CLAUDE_ACCESS_TOKEN"
        );
        assert_eq!(
            provider_token_env(&Provider::ChatGpt),
            "SOUL_CLOUD_CHATGPT_ACCESS_TOKEN"
        );
        assert_eq!(
            provider_token_env(&Provider::Gemini),
            "SOUL_CLOUD_GEMINI_ACCESS_TOKEN"
        );
    }

    #[tokio::test]
    #[ignore = "requires local TCP bind for mock HTTP server"]
    async fn cloud_list_supports_pagination_with_mocked_api() {
        let _guard = env_lock().lock().await;
        let base = EnvSnapshot::capture("SOUL_CLOUD_CHATGPT_BASE_URL");
        let token = EnvSnapshot::capture("SOUL_CLOUD_CHATGPT_ACCESS_TOKEN");
        let home = EnvSnapshot::capture("HOME");
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var("HOME", tmp.path());

        let server = MockServer::start().await;
        std::env::set_var("SOUL_CLOUD_CHATGPT_BASE_URL", server.uri());
        std::env::set_var("SOUL_CLOUD_CHATGPT_ACCESS_TOKEN", "test-token");

        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        Mock::given(method("GET"))
            .and(path("/conversations"))
            .and(header("authorization", "Bearer test-token"))
            .respond_with(PaginationResponder {
                calls: calls.clone(),
            })
            .expect(2)
            .mount(&server)
            .await;

        let client = build_cloud_client(Provider::ChatGpt);
        let first = client
            .list_conversations(None)
            .await
            .expect("first page should succeed");
        assert_eq!(first.items.len(), 1);
        assert_eq!(first.items[0].conversation_id, "c1");
        assert_eq!(first.next_cursor.as_deref(), Some("cursor-2"));

        let second = client
            .list_conversations(first.next_cursor.clone())
            .await
            .expect("second page should succeed");
        assert_eq!(second.items.len(), 1);
        assert_eq!(second.items[0].conversation_id, "c2");
        assert!(second.next_cursor.is_none());
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);

        token.restore();
        base.restore();
        home.restore();
    }

    #[tokio::test]
    #[ignore = "requires local TCP bind for mock HTTP server"]
    async fn cloud_list_retries_after_429_with_mocked_api() {
        let _guard = env_lock().lock().await;
        let base = EnvSnapshot::capture("SOUL_CLOUD_CHATGPT_BASE_URL");
        let token = EnvSnapshot::capture("SOUL_CLOUD_CHATGPT_ACCESS_TOKEN");
        let home = EnvSnapshot::capture("HOME");
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var("HOME", tmp.path());

        let server = MockServer::start().await;
        std::env::set_var("SOUL_CLOUD_CHATGPT_BASE_URL", server.uri());
        std::env::set_var("SOUL_CLOUD_CHATGPT_ACCESS_TOKEN", "retry-token");

        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        Mock::given(method("GET"))
            .and(path("/conversations"))
            .and(header("authorization", "Bearer retry-token"))
            .respond_with(Retry429ThenOk {
                calls: calls.clone(),
            })
            .expect(2)
            .mount(&server)
            .await;

        let client = build_cloud_client(Provider::ChatGpt);
        let page = client
            .list_conversations(None)
            .await
            .expect("request should retry and eventually succeed");
        assert!(page.items.is_empty());
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);

        token.restore();
        base.restore();
        home.restore();
    }

    #[tokio::test]
    #[ignore = "requires local TCP bind for mock HTTP server"]
    async fn cloud_list_refreshes_expired_oauth_token_and_succeeds() {
        let _guard = env_lock().lock().await;
        let home = EnvSnapshot::capture("HOME");
        let cloud_base = EnvSnapshot::capture("SOUL_CLOUD_CHATGPT_BASE_URL");
        let cloud_env_token = EnvSnapshot::capture("SOUL_CLOUD_CHATGPT_ACCESS_TOKEN");
        let oauth_client_id = EnvSnapshot::capture("SOUL_OAUTH_CHATGPT_CLIENT_ID");
        let oauth_token_url = EnvSnapshot::capture("SOUL_OAUTH_CHATGPT_TOKEN_URL");

        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var("HOME", tmp.path());
        std::env::remove_var("SOUL_CLOUD_CHATGPT_ACCESS_TOKEN");

        let server = MockServer::start().await;
        std::env::set_var("SOUL_CLOUD_CHATGPT_BASE_URL", server.uri());
        std::env::set_var("SOUL_OAUTH_CHATGPT_CLIENT_ID", "test-client-id");
        std::env::set_var(
            "SOUL_OAUTH_CHATGPT_TOKEN_URL",
            format!("{}/oauth/token", server.uri()),
        );

        save_credentials(&AuthCredentials {
            provider: Provider::ChatGpt,
            access_token: "expired-token".to_string(),
            refresh_token: Some("refresh-123".to_string()),
            expires_at: Some((chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339()),
        })
        .expect("seed credentials");

        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "new-access-token",
                "refresh_token": "new-refresh-token",
                "expires_in": 3600
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/conversations"))
            .and(header("authorization", "Bearer new-access-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "conversations": [],
                "next_cursor": null
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = build_cloud_client(Provider::ChatGpt);
        let page = client
            .list_conversations(None)
            .await
            .expect("refresh + list should succeed");
        assert!(page.items.is_empty());

        let saved = load_credentials(&Provider::ChatGpt)
            .expect("load credentials")
            .expect("credential exists");
        assert_eq!(saved.access_token, "new-access-token");
        assert_eq!(saved.refresh_token.as_deref(), Some("new-refresh-token"));

        oauth_token_url.restore();
        oauth_client_id.restore();
        cloud_env_token.restore();
        cloud_base.restore();
        home.restore();
    }
}

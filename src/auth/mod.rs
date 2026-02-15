//! OAuth credential storage and token lifecycle management.

use anyhow::{bail, Context, Result};
use chrono::{Duration, Utc};
use reqwest::Client;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::types::Provider;
use crate::vault::config::vault_root;

// ─── Stored Credentials ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AuthCredentials {
    pub provider: Provider,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Default)]
struct AuthStore {
    credentials: Vec<AuthCredentials>,
}

#[derive(Debug, serde::Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    expires_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub provider: Provider,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub scope: String,
}

// ─── Paths ────────────────────────────────────────────────────────────────────

pub fn auth_path() -> PathBuf {
    vault_root().join("auth.yaml")
}

// ─── Storage ──────────────────────────────────────────────────────────────────

pub fn save_credentials(credentials: &AuthCredentials) -> Result<()> {
    let path = auth_path();
    fs::create_dir_all(vault_root())
        .with_context(|| format!("Failed to create {}", vault_root().display()))?;

    let mut store = read_auth_store()?;
    store
        .credentials
        .retain(|c| c.provider != credentials.provider);
    store.credentials.push(credentials.clone());

    let yaml = serialize_auth_store(&store);
    fs::write(&path, yaml).with_context(|| format!("Failed to write {}", path.display()))?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("Failed to set permissions on {}", path.display()))?;
    Ok(())
}

pub fn load_credentials(provider: &Provider) -> Result<Option<AuthCredentials>> {
    let store = read_auth_store()?;
    Ok(store
        .credentials
        .into_iter()
        .find(|c| &c.provider == provider))
}

pub fn remove_credentials(provider: &Provider) -> Result<bool> {
    let path = auth_path();
    if !path.exists() {
        return Ok(false);
    }

    let mut store = read_auth_store()?;
    let before = store.credentials.len();
    store.credentials.retain(|c| &c.provider != provider);

    if before == store.credentials.len() {
        return Ok(false);
    }

    if store.credentials.is_empty() {
        fs::remove_file(&path).with_context(|| format!("Failed to remove {}", path.display()))?;
    } else {
        let yaml = serialize_auth_store(&store);
        fs::write(&path, yaml)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(true)
}

pub fn clear_credentials() -> Result<bool> {
    let path = auth_path();
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(&path).with_context(|| format!("Failed to remove {}", path.display()))?;
    Ok(true)
}

pub fn is_logged_in(provider: &Provider) -> Result<bool> {
    Ok(load_credentials(provider)?.is_some())
}

pub async fn ensure_valid_credentials(provider: &Provider) -> Result<Option<AuthCredentials>> {
    let Some(mut creds) = load_credentials(provider)? else {
        return Ok(None);
    };

    if !is_expired(&creds) {
        return Ok(Some(creds));
    }

    let Some(refresh_token) = creds.refresh_token.clone() else {
        return Ok(Some(creds));
    };

    let oauth = oauth_config(provider);
    let refreshed = refresh_access_token(&oauth, &refresh_token).await?;
    creds.access_token = refreshed.access_token;
    creds.refresh_token = refreshed.refresh_token.or(Some(refresh_token));
    creds.expires_at = refreshed.expires_at;
    save_credentials(&creds)?;
    Ok(Some(creds))
}

fn read_auth_store() -> Result<AuthStore> {
    let path = auth_path();
    if !path.exists() {
        return Ok(AuthStore::default());
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    parse_auth_store(&raw)
}

fn parse_auth_store(raw: &str) -> Result<AuthStore> {
    let mut store = AuthStore::default();

    let mut provider: Option<Provider> = None;
    let mut access_token: Option<String> = None;
    let mut refresh_token: Option<String> = None;
    let mut expires_at: Option<String> = None;

    for line in raw.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("- provider:") {
            if let (Some(p), Some(token)) = (provider.take(), access_token.take()) {
                store.credentials.push(AuthCredentials {
                    provider: p,
                    access_token: token,
                    refresh_token: refresh_token.take(),
                    expires_at: expires_at.take(),
                });
            }

            let value = parse_yaml_value(trimmed.trim_start_matches("- provider:").trim());
            provider = Some(value.parse::<Provider>().map_err(anyhow::Error::msg)?);
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("access_token:") {
            access_token = Some(parse_yaml_value(value.trim()));
        } else if let Some(value) = trimmed.strip_prefix("refresh_token:") {
            refresh_token = Some(parse_yaml_value(value.trim()));
        } else if let Some(value) = trimmed.strip_prefix("expires_at:") {
            expires_at = Some(parse_yaml_value(value.trim()));
        }
    }

    if let (Some(p), Some(token)) = (provider, access_token) {
        store.credentials.push(AuthCredentials {
            provider: p,
            access_token: token,
            refresh_token,
            expires_at,
        });
    }

    Ok(store)
}

fn parse_yaml_value(raw: &str) -> String {
    let mut value = raw.trim().to_string();
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        value = value[1..value.len() - 1].to_string();
    }
    value.replace("\\\"", "\"").replace("\\\\", "\\")
}

fn serialize_auth_store(store: &AuthStore) -> String {
    let mut out = String::from("credentials:\n");

    for creds in &store.credentials {
        out.push_str(&format!("  - provider: {}\n", creds.provider));
        out.push_str(&format!(
            "    access_token: \"{}\"\n",
            yaml_escape(&creds.access_token)
        ));
        if let Some(refresh) = &creds.refresh_token {
            out.push_str(&format!(
                "    refresh_token: \"{}\"\n",
                yaml_escape(refresh)
            ));
        }
        if let Some(expires_at) = &creds.expires_at {
            out.push_str(&format!(
                "    expires_at: \"{}\"\n",
                yaml_escape(expires_at)
            ));
        }
    }

    out
}

fn yaml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn is_expired(credentials: &AuthCredentials) -> bool {
    let Some(expires_at) = &credentials.expires_at else {
        return false;
    };
    chrono::DateTime::parse_from_rfc3339(expires_at)
        .map(|dt| dt.with_timezone(&Utc) <= Utc::now() + Duration::minutes(2))
        .unwrap_or(false)
}

// ─── OAuth Endpoints ─────────────────────────────────────────────────────────

pub fn oauth_config(provider: &Provider) -> OAuthConfig {
    match provider {
        Provider::Claude => OAuthConfig {
            provider: Provider::Claude,
            client_id: std::env::var("SOUL_OAUTH_CLAUDE_CLIENT_ID")
                .unwrap_or_else(|_| "anthropic-cli-placeholder-client-id".to_string()),
            client_secret: std::env::var("SOUL_OAUTH_CLAUDE_CLIENT_SECRET").ok(),
            auth_url: std::env::var("SOUL_OAUTH_CLAUDE_AUTH_URL")
                .unwrap_or_else(|_| "https://console.anthropic.com/oauth/authorize".to_string()),
            token_url: std::env::var("SOUL_OAUTH_CLAUDE_TOKEN_URL")
                .unwrap_or_else(|_| "https://console.anthropic.com/oauth/token".to_string()),
            scope: "conversations:read offline_access".to_string(),
        },
        Provider::ChatGpt => OAuthConfig {
            provider: Provider::ChatGpt,
            client_id: "openai-placeholder-client-id".to_string(),
            client_secret: None,
            auth_url: "https://auth.openai.com/oauth/authorize".to_string(),
            token_url: "https://auth.openai.com/oauth/token".to_string(),
            scope: "conversations.read offline_access".to_string(),
        },
        Provider::Gemini => OAuthConfig {
            provider: Provider::Gemini,
            client_id: "google-placeholder-client-id".to_string(),
            client_secret: None,
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            scope: "https://www.googleapis.com/auth/userinfo.email".to_string(),
        },
    }
}

pub async fn exchange_code_for_token(
    oauth: &OAuthConfig,
    code: &str,
    redirect_uri: &str,
) -> Result<AuthCredentials> {
    let client = Client::new();
    let mut params = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", code.to_string()),
        ("redirect_uri", redirect_uri.to_string()),
        ("client_id", oauth.client_id.clone()),
    ];
    if let Some(secret) = &oauth.client_secret {
        params.push(("client_secret", secret.clone()));
    }

    let response = client
        .post(&oauth.token_url)
        .form(&params)
        .send()
        .await
        .with_context(|| {
            format!(
                "Failed to reach OAuth token endpoint for {}",
                oauth.provider
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("OAuth token exchange failed ({status}). Check client ID/secret and redirect URI. Response: {body}");
    }

    let payload: TokenResponse = response
        .json()
        .await
        .context("OAuth token response was not valid JSON")?;

    let access_token = payload
        .access_token
        .filter(|t| !t.is_empty())
        .ok_or_else(|| anyhow::anyhow!("OAuth response missing access_token"))?;

    Ok(AuthCredentials {
        provider: oauth.provider.clone(),
        access_token,
        refresh_token: payload.refresh_token,
        expires_at: resolve_expiry(payload.expires_at, payload.expires_in),
    })
}

pub async fn refresh_access_token(
    oauth: &OAuthConfig,
    refresh_token: &str,
) -> Result<AuthCredentials> {
    let client = Client::new();
    let mut params = vec![
        ("grant_type", "refresh_token".to_string()),
        ("refresh_token", refresh_token.to_string()),
        ("client_id", oauth.client_id.clone()),
    ];
    if let Some(secret) = &oauth.client_secret {
        params.push(("client_secret", secret.clone()));
    }

    let response = client
        .post(&oauth.token_url)
        .form(&params)
        .send()
        .await
        .with_context(|| format!("Failed to refresh OAuth token for {}", oauth.provider))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!(
            "OAuth refresh failed ({status}). Run `soul login {}` again. Response: {body}",
            oauth.provider
        );
    }

    let payload: TokenResponse = response
        .json()
        .await
        .context("OAuth refresh response was not valid JSON")?;

    let access_token = payload
        .access_token
        .filter(|t| !t.is_empty())
        .ok_or_else(|| anyhow::anyhow!("OAuth refresh response missing access_token"))?;

    Ok(AuthCredentials {
        provider: oauth.provider.clone(),
        access_token,
        refresh_token: payload.refresh_token,
        expires_at: resolve_expiry(payload.expires_at, payload.expires_in),
    })
}

fn resolve_expiry(expires_at: Option<String>, expires_in: Option<i64>) -> Option<String> {
    match (expires_at, expires_in) {
        (Some(ts), _) => Some(ts),
        (None, Some(seconds)) if seconds > 0 => {
            Some((Utc::now() + Duration::seconds(seconds)).to_rfc3339())
        }
        _ => None,
    }
}

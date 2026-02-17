//! OAuth flow helpers.
use anyhow::{bail, Context, Result};
use chrono::{Duration, Utc};
use reqwest::Client;

use crate::auth::types::{AuthCredentials, OAuthConfig, TokenResponse};
use crate::types::Provider;

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
            client_id: std::env::var("SOUL_OAUTH_CHATGPT_CLIENT_ID")
                .unwrap_or_else(|_| "openai-placeholder-client-id".to_string()),
            client_secret: std::env::var("SOUL_OAUTH_CHATGPT_CLIENT_SECRET").ok(),
            auth_url: std::env::var("SOUL_OAUTH_CHATGPT_AUTH_URL")
                .unwrap_or_else(|_| "https://auth.openai.com/oauth/authorize".to_string()),
            token_url: std::env::var("SOUL_OAUTH_CHATGPT_TOKEN_URL")
                .unwrap_or_else(|_| "https://auth.openai.com/oauth/token".to_string()),
            scope: std::env::var("SOUL_OAUTH_CHATGPT_SCOPE")
                .unwrap_or_else(|_| "conversations.read offline_access".to_string()),
        },
        Provider::Gemini => OAuthConfig {
            provider: Provider::Gemini,
            client_id: std::env::var("SOUL_OAUTH_GEMINI_CLIENT_ID")
                .unwrap_or_else(|_| "google-placeholder-client-id".to_string()),
            client_secret: std::env::var("SOUL_OAUTH_GEMINI_CLIENT_SECRET").ok(),
            auth_url: std::env::var("SOUL_OAUTH_GEMINI_AUTH_URL")
                .unwrap_or_else(|_| "https://accounts.google.com/o/oauth2/v2/auth".to_string()),
            token_url: std::env::var("SOUL_OAUTH_GEMINI_TOKEN_URL")
                .unwrap_or_else(|_| "https://oauth2.googleapis.com/token".to_string()),
            scope: std::env::var("SOUL_OAUTH_GEMINI_SCOPE")
                .unwrap_or_else(|_| "https://www.googleapis.com/auth/userinfo.email".to_string()),
        },
    }
}

pub fn oauth_is_configured(provider: &Provider) -> bool {
    let config = oauth_config(provider);
    !config.client_id.to_lowercase().contains("placeholder")
        && !config.auth_url.trim().is_empty()
        && !config.token_url.trim().is_empty()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth_is_configured_is_false_for_placeholder_defaults() {
        std::env::remove_var("SOUL_OAUTH_CHATGPT_CLIENT_ID");
        std::env::remove_var("SOUL_OAUTH_GEMINI_CLIENT_ID");
        assert!(!oauth_is_configured(&Provider::ChatGpt));
        assert!(!oauth_is_configured(&Provider::Gemini));
    }

    #[test]
    fn oauth_is_configured_is_true_when_provider_client_id_is_set() {
        std::env::set_var("SOUL_OAUTH_CHATGPT_CLIENT_ID", "real-client-id");
        assert!(oauth_is_configured(&Provider::ChatGpt));
        std::env::remove_var("SOUL_OAUTH_CHATGPT_CLIENT_ID");
    }
}

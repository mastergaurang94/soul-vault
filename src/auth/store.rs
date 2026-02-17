//! Token storage and credential management.
use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::auth::oauth::{oauth_config, refresh_access_token};
use crate::auth::types::{AuthCredentials, AuthStore};
use crate::types::Provider;
use crate::vault::config::vault_root;

pub fn auth_path() -> PathBuf {
    vault_root().join("auth.yaml")
}

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

pub fn save_setup_token(provider: &Provider, token: &str) -> Result<()> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Setup-token cannot be empty");
    }
    save_credentials(&AuthCredentials {
        provider: provider.clone(),
        access_token: trimmed.to_string(),
        refresh_token: None,
        expires_at: None,
    })
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

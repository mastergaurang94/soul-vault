//! Vault configuration: paths, config.json, keys.json management.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::types::{KeysConfig, ProcessingMode, Provider, SoulVaultConfig, SoulVaultError};

/// Returns the vault root directory: ~/soul-vault/
pub fn vault_root() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("soul-vault")
}

pub fn config_dir() -> PathBuf {
    vault_root().join(".config")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn keys_path() -> PathBuf {
    config_dir().join("keys.json")
}

pub fn key_status_path() -> PathBuf {
    config_dir().join("key_status.json")
}

pub fn identity_dir() -> PathBuf {
    vault_root().join("identity")
}

pub fn memories_dir() -> PathBuf {
    vault_root().join("memories")
}

pub fn topics_dir() -> PathBuf {
    vault_root().join("topics")
}

pub fn people_dir() -> PathBuf {
    vault_root().join("people")
}

pub fn sources_dir() -> PathBuf {
    vault_root().join("sources")
}

/// Returns true if config.json exists (vault has been initialized).
pub fn is_initialized() -> bool {
    config_path().exists()
}

/// Creates all vault directories. Idempotent.
pub fn create_vault_structure() -> Result<()> {
    let dirs = vec![
        config_dir(),
        identity_dir(),
        memories_dir(),
        topics_dir(),
        people_dir(),
        sources_dir(),
    ];

    for dir in dirs {
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
    }

    Ok(())
}

/// Reads and validates config.json. Returns NotInitialized if missing.
pub fn read_config() -> Result<SoulVaultConfig> {
    let path = config_path();
    if !path.exists() {
        return Err(SoulVaultError::NotInitialized.into());
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let config: SoulVaultConfig =
        serde_json::from_str(&raw).with_context(|| "Failed to parse config.json")?;
    Ok(config)
}

/// Writes config.json with pretty formatting.
pub fn write_config(config: &SoulVaultConfig) -> Result<()> {
    let path = config_path();
    fs::create_dir_all(config_dir())?;
    let json = serde_json::to_string_pretty(config)?;
    fs::write(&path, json).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Reads keys.json. Returns empty map if missing.
pub fn read_keys() -> Result<KeysConfig> {
    let path = keys_path();
    if !path.exists() {
        return Ok(KeysConfig::new());
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let keys: KeysConfig =
        serde_json::from_str(&raw).with_context(|| "Failed to parse keys.json")?;
    Ok(keys)
}

/// Writes keys.json with restricted permissions (0o600).
pub fn write_keys(keys: &KeysConfig) -> Result<()> {
    let path = keys_path();
    fs::create_dir_all(config_dir())?;
    let json = serde_json::to_string_pretty(keys)?;
    fs::write(&path, &json).with_context(|| format!("Failed to write {}", path.display()))?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyHealth {
    Verified,
    Unverified,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyHealthRecord {
    pub status: ApiKeyHealth,
    pub checked_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub type ApiKeyHealthConfig = HashMap<String, ApiKeyHealthRecord>;

/// Reads key_status.json. Returns empty map if missing.
pub fn read_key_health() -> Result<ApiKeyHealthConfig> {
    let path = key_status_path();
    if !path.exists() {
        return Ok(ApiKeyHealthConfig::new());
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let health: ApiKeyHealthConfig =
        serde_json::from_str(&raw).with_context(|| "Failed to parse key_status.json")?;
    Ok(health)
}

/// Writes key_status.json with pretty formatting.
pub fn write_key_health(health: &ApiKeyHealthConfig) -> Result<()> {
    let path = key_status_path();
    fs::create_dir_all(config_dir())?;
    let json = serde_json::to_string_pretty(health)?;
    fs::write(&path, json).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Updates one provider key validation state.
pub fn set_key_health(
    provider: &Provider,
    status: ApiKeyHealth,
    message: Option<String>,
) -> Result<()> {
    let mut health = read_key_health()?;
    health.insert(
        provider.to_string(),
        ApiKeyHealthRecord {
            status,
            checked_at: chrono::Utc::now().to_rfc3339(),
            message,
        },
    );
    write_key_health(&health)
}

/// Returns one provider key validation state, if present.
pub fn get_key_health(provider: &Provider) -> Result<Option<ApiKeyHealthRecord>> {
    let health = read_key_health()?;
    Ok(health.get(&provider.to_string()).cloned())
}

/// Returns a single provider's API key, or None.
pub fn get_api_key(provider: &str) -> Result<Option<String>> {
    let keys = read_keys()?;
    Ok(keys.get(provider).cloned())
}

/// Stores a single provider's API key.
pub fn set_api_key(provider: &str, key: &str) -> Result<()> {
    let mut keys = read_keys()?;
    keys.insert(provider.to_string(), key.to_string());
    write_keys(&keys)
}

/// Creates .gitignore to protect keys.json.
pub fn create_gitignore() -> Result<()> {
    fs::write(vault_root().join(".gitignore"), ".config/keys.json\n")?;
    Ok(())
}

/// Creates default profile and preferences files if they don't exist.
pub fn create_default_files() -> Result<()> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let profile_path = identity_dir().join("profile.md");
    if !profile_path.exists() {
        let content = format!(
            "---\nupdated: {}\n---\n\n# Profile\n\n<!-- Core identity facts extracted from your conversations -->\n",
            today
        );
        fs::write(&profile_path, content)?;
    }

    let prefs_path = identity_dir().join("preferences.md");
    if !prefs_path.exists() {
        let content = format!(
            "---\nupdated: {}\n---\n\n# Preferences\n\n<!-- Communication style, interests, and values -->\n",
            today
        );
        fs::write(&prefs_path, content)?;
    }

    Ok(())
}

/// Asserts that the vault is initialized. Returns NotInitialized if not.
pub fn assert_initialized() -> Result<()> {
    if !is_initialized() {
        return Err(SoulVaultError::NotInitialized.into());
    }
    Ok(())
}

/// Asserts that a path exists.
pub fn assert_path_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(SoulVaultError::PathNotFound {
            path: path.display().to_string(),
        }
        .into());
    }
    Ok(())
}

pub fn read_processing_mode() -> Result<ProcessingMode> {
    Ok(read_config()?.processing_mode)
}

pub fn processing_enabled() -> Result<bool> {
    Ok(read_processing_mode()?.is_enabled())
}

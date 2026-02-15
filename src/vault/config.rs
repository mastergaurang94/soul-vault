//! Vault configuration: paths, config.json, keys.json management.

use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::types::{KeysConfig, SoulVaultConfig, SoulVaultError};

// ─── Paths ────────────────────────────────────────────────────────────────────

/// Returns the vault root directory: ~/soul-vault/
pub fn vault_root() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
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

// ─── Vault Lifecycle ──────────────────────────────────────────────────────────

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
        sources_dir().join("chatgpt"),
        sources_dir().join("claude"),
        sources_dir().join("gemini"),
    ];
    for dir in dirs {
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
    }
    Ok(())
}

// ─── Config Read/Write ────────────────────────────────────────────────────────

/// Reads and validates config.json. Returns SoulVaultError::NotInitialized if missing.
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

// ─── Keys Read/Write ──────────────────────────────────────────────────────────

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
    // Set restrictive permissions
    let perms = fs::Permissions::from_mode(0o600);
    fs::set_permissions(&path, perms)?;
    Ok(())
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

// ─── Scaffolding ──────────────────────────────────────────────────────────────

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

/// Asserts that the vault is initialized. Returns SoulVaultError::NotInitialized if not.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_root_is_home_soul_vault() {
        let root = vault_root();
        assert!(root.ends_with("soul-vault"));
    }

    #[test]
    fn test_config_paths() {
        assert!(config_dir().ends_with(".config"));
        assert!(config_path().ends_with("config.json"));
        assert!(keys_path().ends_with("keys.json"));
    }

    #[test]
    fn test_vault_dirs() {
        assert!(identity_dir().ends_with("identity"));
        assert!(memories_dir().ends_with("memories"));
        assert!(topics_dir().ends_with("topics"));
        assert!(people_dir().ends_with("people"));
    }

    #[test]
    fn test_config_serde_roundtrip() {
        use crate::types::{Provider, ProviderConfig};

        let config = SoulVaultConfig {
            providers: vec![ProviderConfig {
                name: Provider::Claude,
                enabled: true,
                last_pull: None,
            }],
            processing_llm: Provider::Claude,
            vault_path: "/tmp/soul-vault".to_string(),
            created_at: "2026-02-14T00:00:00Z".to_string(),
            last_sync: None,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: SoulVaultConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.processing_llm, Provider::Claude);
        assert_eq!(parsed.providers.len(), 1);
    }

    #[test]
    fn test_keys_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let keys_file = tmp.path().join("keys.json");
        let mut keys = KeysConfig::new();
        keys.insert("claude".to_string(), "sk-ant-test123".to_string());
        let json = serde_json::to_string_pretty(&keys).unwrap();
        fs::write(&keys_file, &json).unwrap();
        let raw = fs::read_to_string(&keys_file).unwrap();
        let loaded: KeysConfig = serde_json::from_str(&raw).unwrap();
        assert_eq!(loaded.get("claude").unwrap(), "sk-ant-test123");
    }
}

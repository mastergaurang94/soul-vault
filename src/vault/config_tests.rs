//! Tests for vault module.
use std::fs;

use crate::types::{KeysConfig, Provider, ProviderConfig, SoulVaultConfig};
use crate::vault::config::{
    config_dir, config_path, identity_dir, keys_path, memories_dir, people_dir, topics_dir,
    vault_root,
};

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
    let config = SoulVaultConfig {
        providers: vec![ProviderConfig {
            name: Provider::Claude,
            enabled: true,
            last_import: None,
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

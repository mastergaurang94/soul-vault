//! Config models persisted in `.config`.

use serde::{Deserialize, Serialize};

use super::Provider;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoulVaultConfig {
    pub providers: Vec<ProviderConfig>,
    pub processing_llm: Provider,
    pub vault_path: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub name: Provider,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_pull: Option<String>,
}

/// API keys stored as provider_name -> key.
pub type KeysConfig = std::collections::HashMap<String, String>;

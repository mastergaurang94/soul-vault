//! Config models persisted in `.config`.

use serde::{Deserialize, Serialize};

use super::Provider;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoulVaultConfig {
    pub providers: Vec<ProviderConfig>,
    pub processing_mode: ProcessingMode,
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
    #[serde(skip_serializing_if = "Option::is_none", alias = "last_pull")]
    pub last_import: Option<String>,
}

/// API keys stored as provider_name -> key.
pub type KeysConfig = std::collections::HashMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingMode {
    Disabled,
    Claude,
    ChatGpt,
    Gemini,
}

impl ProcessingMode {
    pub fn display_name(&self) -> &str {
        match self {
            ProcessingMode::Disabled => "Disabled (raw sessions only)",
            ProcessingMode::Claude => "Claude",
            ProcessingMode::ChatGpt => "ChatGPT",
            ProcessingMode::Gemini => "Gemini",
        }
    }

    pub fn as_provider(&self) -> Option<Provider> {
        match self {
            ProcessingMode::Disabled => None,
            ProcessingMode::Claude => Some(Provider::Claude),
            ProcessingMode::ChatGpt => Some(Provider::ChatGpt),
            ProcessingMode::Gemini => Some(Provider::Gemini),
        }
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self, ProcessingMode::Disabled)
    }

    pub fn from_provider(provider: &Provider) -> Self {
        match provider {
            Provider::Claude => ProcessingMode::Claude,
            Provider::ChatGpt => ProcessingMode::ChatGpt,
            Provider::Gemini => ProcessingMode::Gemini,
        }
    }
}

//! Core types for Soul Vault — structs, enums, and serde derives.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─── Provider & Confidence ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    #[serde(alias = "chatgpt")]
    ChatGpt,
    Gemini,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::Claude => write!(f, "claude"),
            Provider::ChatGpt => write!(f, "chatgpt"),
            Provider::Gemini => write!(f, "gemini"),
        }
    }
}

impl Provider {
    pub fn display_name(&self) -> &str {
        match self {
            Provider::Claude => "Claude",
            Provider::ChatGpt => "ChatGPT",
            Provider::Gemini => "Gemini",
        }
    }

    pub fn api_key_hint(&self) -> &str {
        match self {
            Provider::Claude => "sk-ant-...",
            Provider::ChatGpt => "sk-...",
            Provider::Gemini => "AI...",
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    #[default]
    Medium,
    Low,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::High => write!(f, "high"),
            Confidence::Medium => write!(f, "medium"),
            Confidence::Low => write!(f, "low"),
        }
    }
}

// ─── Config ───────────────────────────────────────────────────────────────────

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

// ─── Memory Facts ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityFact {
    pub content: String,
    pub category: String,
    pub confidence: Confidence,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceFact {
    pub content: String,
    #[serde(rename = "type")]
    pub pref_type: String,
    pub confidence: Confidence,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionFact {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    pub confidence: Confidence,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipFact {
    pub person: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub confidence: Confidence,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicFact {
    pub topic: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opinion: Option<String>,
    pub confidence: Confidence,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalContext {
    pub mood: String,
    pub content: String,
    pub confidence: Confidence,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub date: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ExtractedMemories {
    #[serde(default)]
    pub identity: Vec<IdentityFact>,
    #[serde(default)]
    pub preferences: Vec<PreferenceFact>,
    #[serde(default)]
    pub decisions: Vec<DecisionFact>,
    #[serde(default)]
    pub relationships: Vec<RelationshipFact>,
    #[serde(default)]
    pub topics: Vec<TopicFact>,
    #[serde(default)]
    pub emotional_context: Vec<EmotionalContext>,
}

impl ExtractedMemories {
    /// Total number of facts across all categories.
    pub fn fact_count(&self) -> usize {
        self.identity.len()
            + self.preferences.len()
            + self.decisions.len()
            + self.relationships.len()
            + self.topics.len()
    }

    /// Returns true if all categories are empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.identity.is_empty()
            && self.preferences.is_empty()
            && self.decisions.is_empty()
            && self.relationships.is_empty()
            && self.topics.is_empty()
            && self.emotional_context.is_empty()
    }
}

// ─── File Discovery ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name: String,
    pub extension: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub content: String,
    pub source: String,
    pub index: usize,
    pub total: usize,
}

// ─── Vault Stats ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct VaultStats {
    pub memory_count: usize,
    pub topic_count: usize,
    pub people_count: usize,
    pub last_sync: Option<String>,
    pub providers: Vec<ProviderStatus>,
    pub vault_path: String,
}

#[derive(Debug)]
pub struct ProviderStatus {
    pub name: Provider,
    pub connected: bool,
    pub last_pull: Option<String>,
}

// ─── Vault Content (for export) ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct VaultContent {
    pub identity: String,
    pub preferences: String,
    pub memories: Vec<NamedContent>,
    pub topics: Vec<NamedContent>,
    pub people: Vec<NamedContent>,
}

#[derive(Debug, Serialize)]
pub struct NamedContent {
    pub name: String,
    pub content: String,
}

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum SoulVaultError {
    #[error("Soul Vault not initialized.\n      → Run `soul init` to create your vault.")]
    NotInitialized,

    #[error("No API key found for {provider}.\n      → Run `soul init` to configure your API key.")]
    MissingApiKey { provider: String },

    #[error("Failed to parse LLM response: {reason}")]
    #[allow(dead_code)]
    ParseError { reason: String },

    #[error("Path not found: {path}\n      → Check the path and try again.")]
    PathNotFound { path: String },

    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Rate limited. Waiting before retry...")]
    RateLimited,

    #[error("{0}")]
    #[allow(dead_code)]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_display() {
        assert_eq!(Provider::Claude.to_string(), "claude");
        assert_eq!(Provider::ChatGpt.to_string(), "chatgpt");
        assert_eq!(Provider::Gemini.to_string(), "gemini");
    }

    #[test]
    fn test_provider_display_name() {
        assert_eq!(Provider::Claude.display_name(), "Claude");
        assert_eq!(Provider::ChatGpt.display_name(), "ChatGPT");
        assert_eq!(Provider::Gemini.display_name(), "Gemini");
    }

    #[test]
    fn test_confidence_default() {
        assert_eq!(Confidence::default(), Confidence::Medium);
    }

    #[test]
    fn test_extracted_memories_empty() {
        let m = ExtractedMemories::default();
        assert!(m.is_empty());
        assert_eq!(m.fact_count(), 0);
    }

    #[test]
    fn test_config_serde() {
        let config = SoulVaultConfig {
            providers: vec![ProviderConfig {
                name: Provider::Claude,
                enabled: true,
                last_pull: None,
            }],
            processing_llm: Provider::Claude,
            vault_path: "/home/user/soul-vault".to_string(),
            created_at: "2026-02-14T00:00:00Z".to_string(),
            last_sync: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: SoulVaultConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.processing_llm, Provider::Claude);
    }

    #[test]
    fn test_extracted_memories_serde() {
        let json = r#"{
            "identity": [{"content": "Test", "category": "name", "confidence": "high"}],
            "preferences": [],
            "decisions": [],
            "relationships": [],
            "topics": [],
            "emotional_context": []
        }"#;
        let m: ExtractedMemories = serde_json::from_str(json).unwrap();
        assert_eq!(m.identity.len(), 1);
        assert_eq!(m.fact_count(), 1);
    }

    #[test]
    fn test_extracted_memories_missing_fields() {
        let json = r#"{}"#;
        let m: ExtractedMemories = serde_json::from_str(json).unwrap();
        assert!(m.is_empty());
    }
}

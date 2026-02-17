//! Tests for types module.
use super::{
    Confidence, ExtractedMemories, ProcessingMode, Provider, ProviderConfig, SoulVaultConfig,
};

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
    let memories = ExtractedMemories::default();
    assert!(memories.is_empty());
    assert_eq!(memories.fact_count(), 0);
}

#[test]
fn test_config_serde() {
    let config = SoulVaultConfig {
        providers: vec![ProviderConfig {
            name: Provider::Claude,
            enabled: true,
            last_import: None,
        }],
        processing_mode: ProcessingMode::Claude,
        vault_path: "/home/user/soul-vault".to_string(),
        created_at: "2026-02-14T00:00:00Z".to_string(),
        last_sync: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    let parsed: SoulVaultConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.processing_mode, ProcessingMode::Claude);
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
    let memories: ExtractedMemories = serde_json::from_str(json).unwrap();
    assert_eq!(memories.identity.len(), 1);
    assert_eq!(memories.fact_count(), 1);
}

#[test]
fn test_extracted_memories_missing_fields() {
    let memories: ExtractedMemories = serde_json::from_str("{}").unwrap();
    assert!(memories.is_empty());
}

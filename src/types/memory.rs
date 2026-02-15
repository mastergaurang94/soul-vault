//! Extracted memory fact types produced by parsing and processing.

use serde::{Deserialize, Serialize};

use super::Confidence;

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

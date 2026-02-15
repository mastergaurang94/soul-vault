//! Core types for Soul Vault — structs, enums, and serde derives.

mod config;
mod discovery;
mod error;
mod memory;
mod provider;
mod vault;

pub use config::{KeysConfig, ProviderConfig, SoulVaultConfig};
pub use discovery::{ChunkInfo, FileInfo};
pub use error::SoulVaultError;
#[allow(unused_imports)]
pub use memory::{
    DecisionFact, EmotionalContext, ExtractedMemories, IdentityFact, PreferenceFact,
    RelationshipFact, TopicFact,
};
pub use provider::{Confidence, Provider};
pub use vault::{NamedContent, ProviderStatus, VaultContent, VaultStats};

#[cfg(test)]
mod tests;

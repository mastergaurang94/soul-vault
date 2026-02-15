//! View models used when reading/exporting vault data.

use serde::Serialize;

use super::Provider;

#[derive(Debug)]
pub struct VaultStats {
    pub memory_count: usize,
    pub topic_count: usize,
    pub people_count: usize,
    pub last_sync: Option<String>,
    pub providers: Vec<ProviderStatus>,
}

#[derive(Debug)]
pub struct ProviderStatus {
    pub name: Provider,
    pub connected: bool,
    pub last_pull: Option<String>,
}

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

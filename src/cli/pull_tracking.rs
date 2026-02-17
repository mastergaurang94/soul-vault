//! Source tracking and provider sync metadata helpers for pull/import.

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use crate::adapters::SessionFile;
use crate::cli::cloud_types::{CloudConversation, CloudConversationStub};
use crate::types::Provider;
use crate::vault::config::{read_config, write_config};
use crate::vault::sources::{compute_file_hash, read_sources, write_sources, SourceEntry};

const PULL_SOURCE_KEY: &str = "soul-pull";
const CLOUD_SOURCE_KEY: &str = "soul-cloud";

pub(crate) fn filter_new_sessions(sessions: Vec<SessionFile>) -> Result<(Vec<SessionFile>, usize)> {
    let existing_hashes = source_hashes(PULL_SOURCE_KEY)?;

    let mut to_import = Vec::new();
    let mut skipped = 0;

    for session in sessions {
        let path_key = session.path.to_string_lossy().to_string();
        match existing_hashes.get(&path_key) {
            Some(old_hash) => {
                if let Ok(current_hash) = compute_file_hash(&session.path) {
                    if current_hash == *old_hash {
                        skipped += 1;
                        continue;
                    }
                }
                to_import.push(session);
            }
            None => to_import.push(session),
        }
    }

    Ok((to_import, skipped))
}

pub(crate) fn update_pull_tracking(sessions: &[SessionFile]) -> Result<()> {
    let mut sources = read_sources()?;
    let mut file_hashes = source_hashes_from_entries(&sources.sources, PULL_SOURCE_KEY);

    for session in sessions {
        let path_key = session.path.to_string_lossy().to_string();
        if let Ok(hash) = compute_file_hash(&session.path) {
            file_hashes.insert(path_key, hash);
        }
    }

    upsert_source_entry(&mut sources.sources, PULL_SOURCE_KEY, file_hashes);

    write_sources(&sources)?;
    Ok(())
}

pub(crate) fn update_pull_config_timestamps(discovered_providers: &[Provider]) -> Result<()> {
    let mut config = read_config()?;
    let now = chrono::Utc::now().to_rfc3339();

    config.last_sync = Some(now.clone());
    for provider in &mut config.providers {
        if discovered_providers.contains(&provider.name) {
            provider.last_import = Some(now.clone());
        }
    }

    write_config(&config)
}

pub(crate) fn filter_cloud_stubs(
    provider: &Provider,
    stubs: Vec<CloudConversationStub>,
) -> Result<(Vec<CloudConversationStub>, usize)> {
    let hashes = cloud_hashes()?;
    let mut kept = Vec::new();
    let mut skipped = 0;

    for stub in stubs {
        let key = cloud_meta_key(provider, &stub.conversation_id);
        let marker = hash_string(&cloud_meta_marker(
            &stub.conversation_id,
            stub.updated_at.as_ref(),
        ));
        if hashes.get(&key).map(|v| v == &marker).unwrap_or(false) {
            skipped += 1;
        } else {
            kept.push(stub);
        }
    }

    Ok((kept, skipped))
}

pub(crate) fn filter_new_cloud_conversations(
    provider: &Provider,
    conversations: Vec<CloudConversation>,
) -> Result<(Vec<CloudConversation>, usize)> {
    let hashes = cloud_hashes()?;
    let mut kept = Vec::new();
    let mut skipped = 0;

    for conv in conversations {
        let key = cloud_body_key(provider, &conv.conversation_id);
        let material = conv.content_hash_material();
        let current = hash_string(&material);
        if hashes.get(&key).map(|v| v == &current).unwrap_or(false) {
            skipped += 1;
        } else {
            kept.push(conv);
        }
    }

    Ok((kept, skipped))
}

pub(crate) fn update_cloud_tracking(
    provider: &Provider,
    conversations: &[CloudConversation],
) -> Result<()> {
    let mut sources = read_sources()?;
    let mut file_hashes = source_hashes_from_entries(&sources.sources, CLOUD_SOURCE_KEY);

    for conv in conversations {
        file_hashes.insert(
            cloud_meta_key(provider, &conv.conversation_id),
            hash_string(&cloud_meta_marker(
                &conv.conversation_id,
                conv.updated_at.as_ref(),
            )),
        );
        file_hashes.insert(
            cloud_body_key(provider, &conv.conversation_id),
            hash_string(&conv.content_hash_material()),
        );
    }

    upsert_source_entry(&mut sources.sources, CLOUD_SOURCE_KEY, file_hashes);

    write_sources(&sources)?;
    update_pull_config_timestamps(std::slice::from_ref(provider))
}

fn cloud_hashes() -> Result<HashMap<String, String>> {
    source_hashes(CLOUD_SOURCE_KEY)
}

fn source_hashes(source_key: &str) -> Result<HashMap<String, String>> {
    let sources = read_sources()?;
    Ok(source_hashes_from_entries(&sources.sources, source_key))
}

fn source_hashes_from_entries(
    entries: &[SourceEntry],
    source_key: &str,
) -> HashMap<String, String> {
    entries
        .iter()
        .find(|s| s.path == source_key)
        .map(|e| e.file_hashes.clone())
        .unwrap_or_default()
}

fn upsert_source_entry(
    entries: &mut Vec<SourceEntry>,
    source_key: &str,
    file_hashes: HashMap<String, String>,
) {
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(entry) = entries.iter_mut().find(|s| s.path == source_key) {
        entry.files_ingested = file_hashes.len();
        entry.last_ingested = now;
        entry.file_hashes = file_hashes;
    } else {
        entries.push(SourceEntry {
            path: source_key.to_string(),
            files_ingested: file_hashes.len(),
            last_ingested: now,
            file_hashes,
        });
    }
}

fn cloud_meta_key(provider: &Provider, conversation_id: &str) -> String {
    format!("meta:{}:{}", provider, conversation_id)
}

fn cloud_body_key(provider: &Provider, conversation_id: &str) -> String {
    format!("body:{}:{}", provider, conversation_id)
}

fn hash_string(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn cloud_meta_marker(
    conversation_id: &str,
    updated_at: Option<&chrono::DateTime<chrono::Utc>>,
) -> String {
    let marker = updated_at.map(|v| v.to_rfc3339()).unwrap_or_default();
    format!("{}|{}", conversation_id, marker)
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    use chrono::Utc;

    use super::*;
    use crate::cli::cloud_types::{CloudConversation, CloudConversationStub, CloudMessage};
    use crate::types::{ProcessingMode, ProviderConfig, SoulVaultConfig};
    use crate::vault::config::write_config;

    #[test]
    fn cloud_tracking_skips_unchanged_stub_on_next_run() {
        let _guard = env_lock();
        let tmp_home = tempfile::tempdir().expect("temp home");
        std::env::set_var("HOME", tmp_home.path());

        seed_config();

        let updated_at = Utc::now();
        let conversation = CloudConversation {
            provider: Provider::ChatGpt,
            conversation_id: "conv-1".to_string(),
            title: Some("A chat".to_string()),
            updated_at: Some(updated_at),
            messages: vec![CloudMessage {
                role: "user".to_string(),
                content: "hello".to_string(),
                timestamp: None,
            }],
        };

        update_cloud_tracking(&Provider::ChatGpt, &[conversation]).expect("tracking write");

        let stub = CloudConversationStub {
            conversation_id: "conv-1".to_string(),
            updated_at: Some(updated_at),
        };

        let (kept, skipped) =
            filter_cloud_stubs(&Provider::ChatGpt, vec![stub]).expect("stub filter");
        assert_eq!(kept.len(), 0, "unchanged stub should be skipped");
        assert_eq!(skipped, 1);
    }

    fn seed_config() {
        let config = SoulVaultConfig {
            providers: vec![
                ProviderConfig {
                    name: Provider::Claude,
                    enabled: true,
                    last_import: None,
                },
                ProviderConfig {
                    name: Provider::ChatGpt,
                    enabled: true,
                    last_import: None,
                },
                ProviderConfig {
                    name: Provider::Gemini,
                    enabled: true,
                    last_import: None,
                },
            ],
            processing_mode: ProcessingMode::Disabled,
            vault_path: "~/soul-vault".to_string(),
            created_at: Utc::now().to_rfc3339(),
            last_sync: None,
        };
        write_config(&config).expect("seed config");
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

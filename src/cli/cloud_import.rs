//! Cloud import orchestrator shared by CLI and TUI.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;

use crate::cli::cloud_client::build_cloud_client;
use crate::cli::cloud_types::{CloudImportEvent, CloudImportSummary, ImportJobState};
use crate::cli::pull_tracking::{
    filter_cloud_stubs, filter_new_cloud_conversations, update_cloud_tracking,
};
use crate::core::merger::{chunk_text, merge_all_memories};
use crate::core::processor::process_chunk;
use crate::types::Provider;
use crate::vault::config::{assert_initialized, processing_enabled};
use crate::vault::write::write_memories_to_vault;

pub(crate) async fn run_cloud_import<F>(
    provider: Provider,
    force: bool,
    cancel_flag: Option<Arc<AtomicBool>>,
    mut emit: F,
) -> Result<CloudImportSummary>
where
    F: FnMut(CloudImportEvent),
{
    assert_initialized()?;
    if matches!(provider, Provider::Claude) {
        anyhow::bail!(
            "Anthropic does not publish a documented cloud conversation-history API.\n      \
             → Use Claude export import (`soul import <export-folder>`) for Claude history."
        );
    }

    let client = build_cloud_client(provider.clone());
    emit(event(
        provider.clone(),
        ImportJobState::Queued,
        0,
        0,
        None,
        "Queued",
    ));

    let mut cursor = None;
    let mut stubs = Vec::new();

    loop {
        if is_cancelled(&cancel_flag) {
            emit(event(
                provider.clone(),
                ImportJobState::Cancelled,
                0,
                0,
                None,
                "Cancelled",
            ));
            return Ok(cancelled_summary(provider));
        }
        emit(event(
            provider.clone(),
            ImportJobState::Fetching,
            stubs.len(),
            stubs.len() + 1,
            None,
            "Fetching conversation list...",
        ));
        let page = client.list_conversations(cursor.clone()).await?;
        if page.items.is_empty() && page.next_cursor.is_none() {
            break;
        }
        stubs.extend(page.items);
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }

    let fetched = stubs.len();
    if fetched == 0 {
        return Ok(CloudImportSummary {
            provider,
            fetched: 0,
            imported: 0,
            skipped_unchanged: 0,
            processed_chunks: 0,
            memories: 0,
            topics: 0,
            people: 0,
            errors: Vec::new(),
            cancelled: false,
        });
    }

    let (to_fetch, skipped_unchanged) = if force {
        (stubs, 0)
    } else {
        filter_cloud_stubs(&provider, stubs)?
    };

    let mut conversations = Vec::new();
    let mut errors = Vec::new();

    for (idx, stub) in to_fetch.iter().enumerate() {
        if is_cancelled(&cancel_flag) {
            emit(event(
                provider.clone(),
                ImportJobState::Cancelled,
                idx,
                to_fetch.len(),
                None,
                "Cancelled",
            ));
            return Ok(cancelled_summary(provider));
        }
        emit(event(
            provider.clone(),
            ImportJobState::Fetching,
            idx + 1,
            to_fetch.len(),
            Some(stub.conversation_id.clone()),
            "Fetching conversation details...",
        ));
        match client.fetch_conversation(&stub.conversation_id).await {
            Ok(conv) => conversations.push(conv),
            Err(e) => errors.push(format!("{}: {}", stub.conversation_id, e)),
        }
    }

    let (to_import, body_skipped) = if force {
        (conversations, 0)
    } else {
        filter_new_cloud_conversations(&provider, conversations)?
    };

    let skipped_unchanged = skipped_unchanged + body_skipped;

    emit(event(
        provider.clone(),
        ImportJobState::Normalizing,
        to_import.len(),
        fetched,
        None,
        "Normalizing conversations...",
    ));

    let mut chunks = Vec::new();
    for conv in &to_import {
        if is_cancelled(&cancel_flag) {
            emit(event(
                provider.clone(),
                ImportJobState::Cancelled,
                0,
                0,
                None,
                "Cancelled",
            ));
            return Ok(cancelled_summary(provider));
        }
        let text = conv.to_text();
        if text.trim().is_empty() {
            continue;
        }
        chunks.extend(chunk_text(&text, &conv.conversation_id));
    }

    if !processing_enabled()? {
        update_cloud_tracking(&provider, &to_import)?;
        return Ok(CloudImportSummary {
            provider,
            fetched,
            imported: to_import.len(),
            skipped_unchanged,
            processed_chunks: chunks.len(),
            memories: 0,
            topics: 0,
            people: 0,
            errors,
            cancelled: false,
        });
    }

    let client_http = reqwest::Client::new();
    let mut all_memories = Vec::new();

    for (idx, chunk) in chunks.iter().enumerate() {
        if is_cancelled(&cancel_flag) {
            emit(event(
                provider.clone(),
                ImportJobState::Cancelled,
                idx,
                chunks.len(),
                None,
                "Cancelled",
            ));
            return Ok(cancelled_summary(provider));
        }
        emit(event(
            provider.clone(),
            ImportJobState::Processing,
            idx + 1,
            chunks.len(),
            None,
            "Processing through configured model...",
        ));

        match process_chunk(&client_http, chunk).await {
            Ok(memories) => all_memories.push(memories),
            Err(e) => errors.push(format!("{}: {}", chunk.source, e)),
        }
    }

    emit(event(
        provider.clone(),
        ImportJobState::Writing,
        1,
        1,
        None,
        "Writing vault updates...",
    ));

    let merged = merge_all_memories(&all_memories);
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let write_result = write_memories_to_vault(&merged, &today)?;
    update_cloud_tracking(&provider, &to_import)?;

    let summary = CloudImportSummary {
        provider: provider.clone(),
        fetched,
        imported: to_import.len(),
        skipped_unchanged,
        processed_chunks: chunks.len(),
        memories: merged.fact_count(),
        topics: write_result.topics_written.len(),
        people: write_result.people_written.len(),
        errors,
        cancelled: false,
    };

    emit(event(
        provider,
        ImportJobState::Done,
        1,
        1,
        None,
        "Completed",
    ));
    Ok(summary)
}

fn is_cancelled(cancel_flag: &Option<Arc<AtomicBool>>) -> bool {
    cancel_flag
        .as_ref()
        .map(|f| f.load(Ordering::Relaxed))
        .unwrap_or(false)
}

fn cancelled_summary(provider: Provider) -> CloudImportSummary {
    CloudImportSummary {
        provider,
        fetched: 0,
        imported: 0,
        skipped_unchanged: 0,
        processed_chunks: 0,
        memories: 0,
        topics: 0,
        people: 0,
        errors: Vec::new(),
        cancelled: true,
    }
}

fn event(
    provider: Provider,
    state: ImportJobState,
    current: usize,
    total: usize,
    conversation_id: Option<String>,
    message: &str,
) -> CloudImportEvent {
    CloudImportEvent {
        provider,
        state,
        current,
        total,
        conversation_id,
        message: message.to_string(),
    }
}

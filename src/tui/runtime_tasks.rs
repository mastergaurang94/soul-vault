//! Async channel draining and task spawners used by the TUI runtime loop.

use tokio::sync::mpsc;

use super::pages::import::ImportPage;
use super::pages::watch::{WatchEvent, WatchPage};
use super::watcher;
use crate::core::pipeline::ImportProgress;

// ─── Channels ─────────────────────────────────────────────────────────────────

#[derive(Default)]
pub(super) struct Channels {
    pub(super) import_rx: Option<mpsc::Receiver<ImportProgress>>,
    pub(super) watch_event_rx: Option<mpsc::Receiver<WatchEvent>>,
    pub(super) watch_stop_tx: Option<mpsc::Sender<()>>,
    pub(super) pull_rx: Option<mpsc::Receiver<String>>,
}

pub(super) fn shutdown_watcher(channels: &mut Channels) {
    if let Some(tx) = channels.watch_stop_tx.take() {
        let _ = tx.try_send(());
    }
}

pub(super) fn drain_folder_import_progress(import: &mut ImportPage, channels: &mut Channels) {
    if let Some(rx) = &mut channels.import_rx {
        while let Ok(progress) = rx.try_recv() {
            let is_terminal = matches!(
                progress,
                ImportProgress::Done(_)
                    | ImportProgress::Error(_)
                    | ImportProgress::NothingToImport { .. }
            );
            import.on_folder_progress(progress);
            if is_terminal {
                channels.import_rx = None;
                break;
            }
        }
    }
}

pub(super) fn drain_watch_events(watch: &mut WatchPage, channels: &mut Channels) {
    if let Some(rx) = &mut channels.watch_event_rx {
        while let Ok(event) = rx.try_recv() {
            watch.on_event(event);
        }
    }
}

pub(super) fn drain_provider_import_progress(import: &mut ImportPage, channels: &mut Channels) {
    if let Some(rx) = &mut channels.pull_rx {
        while let Ok(msg) = rx.try_recv() {
            if let Some(summary_str) = msg.strip_prefix("DONE:") {
                let summary: Vec<String> = summary_str.split('\n').map(String::from).collect();
                import.on_provider_done(summary);
                channels.pull_rx = None;
                return;
            } else if let Some(error_str) = msg.strip_prefix("ERROR:") {
                import.on_provider_error(error_str.to_string());
                channels.pull_rx = None;
                return;
            } else if let Some(progress_str) = msg.strip_prefix("PROGRESS:") {
                if let Some((cur, tot)) = progress_str.split_once('/') {
                    let current = cur.parse().unwrap_or(0);
                    let total = tot.parse().unwrap_or(0);
                    import.on_provider_processing(current, total);
                }
            } else {
                import.on_provider_progress(msg);
            }
        }
    }
}

// ─── Async Task Spawners ──────────────────────────────────────────────────────

pub(super) fn start_import(folder: &str, import_page: &mut ImportPage, channels: &mut Channels) {
    let (tx, rx) = mpsc::channel(64);
    channels.import_rx = Some(rx);
    import_page.on_folder_progress(ImportProgress::Scanning);

    let folder = folder.to_string();
    tokio::spawn(async move {
        crate::core::pipeline::run_import(folder, tx).await;
    });
}

pub(super) fn start_watch(folder: &str, watch_page: &mut WatchPage, channels: &mut Channels) {
    shutdown_watcher(channels);

    let (event_tx, event_rx) = mpsc::channel(256);
    let (stop_tx, stop_rx) = mpsc::channel(1);

    channels.watch_event_rx = Some(event_rx);
    channels.watch_stop_tx = Some(stop_tx);
    watch_page.start_watching(folder);

    let folder = folder.to_string();
    watcher::start_watcher(folder, event_tx, stop_rx);
}

pub(super) fn start_provider_import(import_page: &mut ImportPage, channels: &mut Channels) {
    let (tx, rx) = mpsc::channel(64);
    channels.pull_rx = Some(rx);
    import_page.on_provider_progress("Discovering AI sessions...".to_string());

    tokio::spawn(async move {
        use crate::adapters::{conversation_to_text, AdapterRegistry};
        use crate::core::merger::{chunk_text, merge_all_memories};
        use crate::core::processor::process_chunk;
        use crate::vault::write::write_memories_to_vault;

        let registry = AdapterRegistry::new();
        let discovered = registry.discover_all();

        let mut total = 0;
        for (name, sessions) in &discovered {
            total += sessions.len();
            let _ = tx
                .send(format!("{}: {} sessions", name, sessions.len()))
                .await;
        }

        if total == 0 {
            let _ = tx.send("ERROR:No AI sessions found.".to_string()).await;
            return;
        }

        // Validate API key upfront
        let api_key = crate::vault::config::get_api_key("claude")
            .ok()
            .flatten()
            .unwrap_or_default();
        if api_key.trim().is_empty() {
            let _ = tx
                .send("ERROR:No API key configured. Run `soul init` to set up your Claude API key.".to_string())
                .await;
            return;
        }

        // Quick validation: try a minimal API call
        let client = reqwest::Client::new();
        let check = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": "claude-sonnet-4-20250514",
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "hi"}]
            }))
            .send()
            .await;

        match check {
            Ok(resp) if resp.status() == reqwest::StatusCode::UNAUTHORIZED => {
                let _ = tx
                    .send("ERROR:API key is invalid or expired. Run `soul init` to reconfigure.".to_string())
                    .await;
                return;
            }
            Err(e) => {
                let _ = tx
                    .send(format!("ERROR:Cannot reach Anthropic API: {e}"))
                    .await;
                return;
            }
            _ => {}
        }

        let _ = tx.send("API key verified ✓".to_string()).await;

        let all_sessions: Vec<_> = discovered.into_iter().flat_map(|(_, s)| s).collect();
        let mut all_chunks = Vec::new();
        for session in &all_sessions {
            if let Some(adapter) = registry.auto_detect(&session.path) {
                if let Ok(conv) = adapter.parse_session(&session.path) {
                    if !conv.messages.is_empty() {
                        let text = conversation_to_text(&conv);
                        if !text.trim().is_empty() {
                            all_chunks.extend(chunk_text(&text, &conv.id));
                        }
                    }
                }
            }
        }

        let _ = tx.send(format!("Parsed {} chunks", all_chunks.len())).await;
        if all_chunks.is_empty() {
            let _ = tx
                .send("DONE:No meaningful content found.".to_string())
                .await;
            return;
        }

        let mut all_memories = Vec::new();
        let mut errors = 0usize;
        let chunk_count = all_chunks.len();
        for (i, chunk) in all_chunks.iter().enumerate() {
            let _ = tx
                .send(format!("PROGRESS:{}/{}", i + 1, chunk_count))
                .await;
            match process_chunk(&client, chunk).await {
                Ok(memories) => all_memories.push(memories),
                Err(e) => {
                    errors += 1;
                    let msg = e.to_string();
                    if msg.contains("API key") || msg.contains("401") {
                        let _ = tx
                            .send("ERROR:API key rejected during processing. Run `soul init`.".to_string())
                            .await;
                        return;
                    }
                    let _ = tx.send(format!("Warning: {}", msg)).await;
                }
            }
        }

        let merged = merge_all_memories(&all_memories);
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        match write_memories_to_vault(&merged, &today) {
            Ok(result) => {
                let mut summary = format!(
                    "DONE:{} sessions processed\n{} memories extracted\n{} topics\n{} people",
                    all_sessions.len(),
                    merged.fact_count(),
                    result.topics_written.len(),
                    result.people_written.len()
                );
                if errors > 0 {
                    summary.push_str(&format!("\n{} chunks had errors", errors));
                }
                let _ = tx.send(summary).await;
            }
            Err(e) => {
                let _ = tx.send(format!("ERROR:{}", e)).await;
            }
        }
    });
}

pub(super) fn stop_watch(watch_page: &mut WatchPage, channels: &mut Channels) {
    shutdown_watcher(channels);
    channels.watch_event_rx = None;
    watch_page.stop_watching();
}

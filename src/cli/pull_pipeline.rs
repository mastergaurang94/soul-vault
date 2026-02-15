//! Provider import discovery, parsing, and LLM processing pipeline helpers.

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

use crate::adapters::{conversation_to_text, AdapterRegistry, SessionFile};
use crate::core::merger::chunk_text;
use crate::core::processor::process_chunk;
use crate::types::{ChunkInfo, ExtractedMemories, Provider};
use crate::ui::theme::*;

pub(crate) struct DiscoveryOutcome {
    pub(crate) all_sessions: Vec<SessionFile>,
    pub(crate) discovered_providers: Vec<Provider>,
    pub(crate) total_sessions: usize,
}

pub(crate) fn discover_sessions(registry: &AdapterRegistry) -> DiscoveryOutcome {
    let discovered = registry.discover_all();

    let mut total_sessions = 0;
    for (name, sessions) in &discovered {
        let count = sessions.len();
        total_sessions += count;
        if count > 0 {
            println!(
                "  {} {}: {} sessions",
                emerald(ICON_CHECK),
                bold_white(name),
                bold_white(&count.to_string())
            );
        } else {
            println!("  {} {}: {}", dim(ICON_DOT), name, dim("no sessions found"));
        }
    }

    let mut discovered_providers: Vec<Provider> = discovered
        .iter()
        .filter_map(|(name, sessions)| {
            if sessions.is_empty() {
                return None;
            }
            provider_from_display_name(name)
        })
        .collect();
    discovered_providers.sort_by_key(|p| p.to_string());
    discovered_providers.dedup();

    let all_sessions = discovered
        .into_iter()
        .flat_map(|(_, sessions)| sessions)
        .collect();

    DiscoveryOutcome {
        all_sessions,
        discovered_providers,
        total_sessions,
    }
}

pub(crate) fn parse_sessions_to_chunks(
    registry: &AdapterRegistry,
    to_import: &[SessionFile],
) -> (Vec<ChunkInfo>, Vec<String>) {
    let pb = spinner("Parsing sessions...");
    let mut all_chunks = Vec::new();
    let mut parse_errors = Vec::new();

    for session in to_import {
        let adapter = registry.auto_detect(&session.path);
        match adapter {
            Some(a) => match a.parse_session(&session.path) {
                Ok(conv) => {
                    if conv.messages.is_empty() {
                        continue;
                    }
                    let text = conversation_to_text(&conv);
                    if !text.trim().is_empty() {
                        all_chunks.extend(chunk_text(&text, &conv.id));
                    }
                }
                Err(e) => parse_errors.push(format!("{}: {}", session.path.display(), e)),
            },
            None => parse_errors.push(format!("{}: no adapter found", session.path.display())),
        }
    }

    pb.finish_with_message(check(&format!(
        "Parsed into {} chunks from {} sessions",
        bold_white(&all_chunks.len().to_string()),
        to_import.len()
    )));

    (all_chunks, parse_errors)
}

pub(crate) async fn process_chunks(
    all_chunks: &[ChunkInfo],
) -> Result<(Vec<ExtractedMemories>, Vec<String>)> {
    println!("  {} {}", ICON_BRAIN, gold("Processing through LLM..."));

    let client = reqwest::Client::new();
    let mut all_memories = Vec::new();
    let mut errors = Vec::new();

    let progress = ProgressBar::new(all_chunks.len() as u64);
    let progress_style = ProgressStyle::with_template(
        "  {spinner:.yellow} [{bar:20.yellow/dark_gray}] {pos}/{len} {elapsed_precise} {msg}",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar())
    .progress_chars("█░░");
    progress.set_style(progress_style);

    for (i, chunk) in all_chunks.iter().enumerate() {
        let label = if chunk.total > 1 {
            format!("{} ({}/{})", chunk.source, chunk.index + 1, chunk.total)
        } else {
            chunk.source.clone()
        };

        progress.set_message(cyan(&label));

        match process_chunk(&client, chunk).await {
            Ok(memories) => {
                all_memories.push(memories);
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("API key") || msg.contains("401") {
                    progress.finish_and_clear();
                    anyhow::bail!("API key error. Run `soul init` to reconfigure.");
                }
                if msg.contains("Rate limited") || msg.contains("429") {
                    for waited in 1..=30 {
                        progress.set_message(amber(&format!(
                            "Rate limited. Waiting {}/30s | {}",
                            waited, label
                        )));
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                    if let Ok(memories) = process_chunk(&client, chunk).await {
                        all_memories.push(memories);
                        progress.set_position((i + 1) as u64);
                        continue;
                    }
                }
                errors.push(format!("{}: {}", label, msg));
            }
        }

        progress.set_position((i + 1) as u64);
    }

    progress.finish_and_clear();
    println!(
        "{}",
        check(&format!("Processed {} chunks", all_chunks.len()))
    );

    Ok((all_memories, errors))
}

fn provider_from_display_name(name: &str) -> Option<Provider> {
    match name {
        "Claude Code" => Some(Provider::Claude),
        "Gemini CLI" => Some(Provider::Gemini),
        _ => None,
    }
}

fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    let style = ProgressStyle::with_template("  {spinner:.yellow} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]);
    pb.set_style(style);
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

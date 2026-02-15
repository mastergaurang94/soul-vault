//! Provider auto-discovery import implementation for `soul import`.
//!
//! Discovers sessions from Claude Code, OpenClaw, and other providers,
//! then runs them through the import pipeline.

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

use crate::adapters::{conversation_to_text, AdapterRegistry, SessionFile};
use crate::auth::{ensure_valid_credentials, is_logged_in};
use crate::core::merger::{chunk_text, merge_all_memories};
use crate::core::processor::process_chunk;
use crate::types::{ExtractedMemories, Provider};
use crate::ui::theme::*;
use crate::vault::config::{assert_initialized, get_api_key, read_config, write_config};
use crate::vault::sources::{compute_file_hash, read_sources, write_sources, SourceEntry};
use crate::vault::write::write_memories_to_vault;

// ─── Provider Import ──────────────────────────────────────────────────────────

pub async fn run(force: bool, cloud: bool, provider: Option<&str>) -> Result<()> {
    if cloud {
        return run_cloud(provider).await;
    }

    println!("{}", banner());
    assert_initialized()?;
    let api_key = get_api_key("claude")?;
    if api_key.as_deref().map(str::trim).unwrap_or("").is_empty() {
        anyhow::bail!("No API key configured. Run `soul init` to set up your Claude API key.");
    }

    println!("  {} {}", ICON_BRAIN, gold("Discovering AI sessions..."));
    println!("{}", line());

    // Phase 1: Discover
    let registry = AdapterRegistry::new();
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
    if total_sessions == 0 {
        println!("{}", dim("  No AI sessions found on this machine."));
        println!(
            "  {} Supported: Claude Code, OpenClaw, Gemini CLI, Codex",
            dim(ICON_DOT)
        );
        println!();
        return Ok(());
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

    // Phase 2: Filter (skip already-imported sessions)
    let all_sessions: Vec<SessionFile> = discovered
        .into_iter()
        .flat_map(|(_, sessions)| sessions)
        .collect();

    let (to_import, skipped) = if force {
        println!("{}", amber("  ! Force mode: re-importing all sessions"));
        (all_sessions, 0)
    } else {
        filter_new_sessions(all_sessions)?
    };

    if to_import.is_empty() {
        println!(
            "{}",
            check(&format!(
                "All {} sessions already imported. Nothing to do.",
                skipped
            ))
        );
        println!(
            "  {} {} {}",
            dim("Use"),
            cyan("soul import --force"),
            dim("to re-import everything.")
        );
        println!();
        return Ok(());
    }

    println!(
        "  {} {} to import, {} already imported",
        ICON_FOLDER,
        bold_white(&to_import.len().to_string()),
        dim(&skipped.to_string())
    );

    // Phase 3: Parse & Chunk
    let pb = spinner("Parsing sessions...");
    let mut all_chunks = Vec::new();
    let mut parse_errors = Vec::new();

    for session in &to_import {
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
                Err(e) => {
                    parse_errors.push(format!("{}: {}", session.path.display(), e));
                }
            },
            None => {
                parse_errors.push(format!("{}: no adapter found", session.path.display()));
            }
        }
    }

    pb.finish_with_message(check(&format!(
        "Parsed into {} chunks from {} sessions",
        bold_white(&all_chunks.len().to_string()),
        to_import.len()
    )));

    if all_chunks.is_empty() {
        println!("{}", dim("  No meaningful content found in sessions."));
        println!();
        return Ok(());
    }

    // Phase 4: LLM Processing
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

    // Phase 5: Merge & Write
    let merged = merge_all_memories(&all_memories);

    let pb = spinner("Writing to vault...");
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let write_result = write_memories_to_vault(&merged, &today)?;
    pb.finish_with_message(check("Vault updated"));

    // Phase 6: Update source tracking
    let pb = spinner("Updating source tracking...");
    match update_pull_tracking(&to_import) {
        Ok(()) => pb.finish_with_message(check("Source tracking updated")),
        Err(e) => {
            pb.finish_with_message(amber("Source tracking skipped"));
            eprintln!(
                "{}",
                amber(&format!(
                    "  ⚠ Could not update provider import source tracking: {}",
                    e
                ))
            );
        }
    }

    // Phase 7: Update config sync metadata
    if let Err(e) = update_pull_config_timestamps(&discovered_providers) {
        eprintln!(
            "{}",
            amber(&format!(
                "  ⚠ Could not update provider import sync timestamps: {}",
                e
            ))
        );
    }

    // Summary
    print_summary(
        to_import.len(),
        skipped,
        &merged,
        &write_result.topics_written,
        &write_result.people_written,
        &parse_errors,
        &errors,
    );

    Ok(())
}

async fn run_cloud(provider: Option<&str>) -> Result<()> {
    println!("{}", banner());
    assert_initialized()?;

    let provider = match provider {
        Some(raw) => raw
            .parse::<crate::types::Provider>()
            .map_err(anyhow::Error::msg)?,
        None => crate::types::Provider::Claude,
    };

    if !is_logged_in(&provider)? {
        anyhow::bail!(
            "Not logged in to {} cloud.\n      → Run `soul login {}` and try again.",
            provider.display_name(),
            provider
        );
    }

    let creds = ensure_valid_credentials(&provider).await?;
    if creds.is_none() {
        anyhow::bail!(
            "No valid OAuth credentials for {}.\n      → Run `soul login {}` and try again.",
            provider.display_name(),
            provider
        );
    }

    println!(
        "  {} Authenticated with {} cloud.",
        emerald(ICON_CHECK),
        bold_white(provider.display_name())
    );
    println!(
        "  {} Coming soon — use {} with your exported data.\n",
        amber(ICON_STAR),
        cyan("soul import")
    );
    Ok(())
}

// ─── Source Tracking for Pull ─────────────────────────────────────────────────

const PULL_SOURCE_KEY: &str = "soul-pull";

/// Filters out sessions that have already been imported (by file hash).
fn filter_new_sessions(sessions: Vec<SessionFile>) -> Result<(Vec<SessionFile>, usize)> {
    let sources = read_sources()?;
    let pull_entry = sources.sources.iter().find(|s| s.path == PULL_SOURCE_KEY);

    let existing_hashes: std::collections::HashMap<String, String> = pull_entry
        .map(|e| e.file_hashes.clone())
        .unwrap_or_default();

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
            None => {
                to_import.push(session);
            }
        }
    }

    Ok((to_import, skipped))
}

/// Records all imported sessions in source tracking.
fn update_pull_tracking(sessions: &[SessionFile]) -> Result<()> {
    let mut sources = read_sources()?;

    // Preserve existing hashes
    let mut file_hashes = sources
        .sources
        .iter()
        .find(|s| s.path == PULL_SOURCE_KEY)
        .map(|e| e.file_hashes.clone())
        .unwrap_or_default();

    // Add/update hashes for newly imported sessions
    for session in sessions {
        let path_key = session.path.to_string_lossy().to_string();
        if let Ok(hash) = compute_file_hash(&session.path) {
            file_hashes.insert(path_key, hash);
        }
    }

    let now = chrono::Utc::now().to_rfc3339();

    if let Some(entry) = sources
        .sources
        .iter_mut()
        .find(|s| s.path == PULL_SOURCE_KEY)
    {
        entry.files_ingested = file_hashes.len();
        entry.last_ingested = now;
        entry.file_hashes = file_hashes;
    } else {
        sources.sources.push(SourceEntry {
            path: PULL_SOURCE_KEY.to_string(),
            files_ingested: file_hashes.len(),
            last_ingested: now,
            file_hashes,
        });
    }

    write_sources(&sources)?;
    Ok(())
}

fn update_pull_config_timestamps(discovered_providers: &[Provider]) -> Result<()> {
    let mut config = read_config()?;
    let now = chrono::Utc::now().to_rfc3339();

    config.last_sync = Some(now.clone());
    for provider in &mut config.providers {
        if discovered_providers.contains(&provider.name) {
            provider.last_pull = Some(now.clone());
        }
    }

    write_config(&config)
}

fn provider_from_display_name(name: &str) -> Option<Provider> {
    match name {
        "Claude Code" => Some(Provider::Claude),
        "Gemini CLI" => Some(Provider::Gemini),
        _ => None,
    }
}

// ─── Summary ──────────────────────────────────────────────────────────────────

fn print_summary(
    imported: usize,
    skipped: usize,
    merged: &ExtractedMemories,
    topics: &[String],
    people: &[String],
    parse_errors: &[String],
    processing_errors: &[String],
) {
    let total = merged.fact_count();
    println!("{}", line());
    println!(
        "  {} {}",
        amber(ICON_STAR),
        bold_gold("Provider import complete")
    );

    println!(
        "  {} {} imported, {} skipped",
        dim("Sessions"),
        bold_white(&imported.to_string()),
        dim(&skipped.to_string())
    );
    println!("  {} {}", dim("Memories"), bold_white(&total.to_string()));
    println!(
        "  {} {}{}",
        dim("Topics"),
        bold_white(&topics.len().to_string()),
        summarize_list(topics)
    );
    println!(
        "  {} {}{}",
        dim("People"),
        bold_white(&people.len().to_string()),
        summarize_list(people)
    );

    print_error_group("Parse errors", parse_errors);
    print_error_group("Processing errors", processing_errors);

    println!(
        "  {} {} {}",
        dim("Run"),
        cyan("soul status"),
        dim("to see your vault.")
    );
    println!();
}

fn print_error_group(title: &str, errors: &[String]) {
    if errors.is_empty() {
        return;
    }

    println!(
        "  {} {}",
        amber("!"),
        amber(&format!("{} ({})", title, errors.len()))
    );
    for err in errors.iter().take(8) {
        println!("    {} {}", dim("-"), dim(err));
    }
    if errors.len() > 8 {
        println!(
            "    {} {}",
            dim("-"),
            dim(&format!("+{} more", errors.len() - 8))
        );
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

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

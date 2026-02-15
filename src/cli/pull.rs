//! `soul pull` — auto-discover and import AI sessions from all providers.
//!
//! Discovers sessions from Claude Code, OpenClaw, and other providers,
//! then runs them through the import pipeline.

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

use crate::adapters::{conversation_to_text, AdapterRegistry, SessionFile};
use crate::core::merger::{chunk_text, merge_all_memories};
use crate::core::processor::process_chunk;
use crate::types::ExtractedMemories;
use crate::ui::theme::*;
use crate::vault::config::assert_initialized;
use crate::vault::sources::{
    compute_file_hash, read_sources, write_sources, SourceEntry,
};
use crate::vault::write::write_memories_to_vault;

// ─── Pull Command ─────────────────────────────────────────────────────────────

pub async fn run(force: bool) -> Result<()> {
    println!("{}", banner());
    assert_initialized()?;

    println!("  {} {}\n", ICON_BRAIN, gold("Discovering AI sessions..."));
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
            println!(
                "  {} {}: {}",
                dim(ICON_DOT),
                name,
                dim("no sessions found")
            );
        }
    }
    println!();

    if total_sessions == 0 {
        println!("{}", dim("  No AI sessions found on this machine.\n"));
        println!(
            "  {} Supported: Claude Code, OpenClaw, Gemini CLI, Codex",
            dim(ICON_DOT)
        );
        println!();
        return Ok(());
    }

    // Phase 2: Filter (skip already-imported sessions)
    let all_sessions: Vec<SessionFile> = discovered
        .into_iter()
        .flat_map(|(_, sessions)| sessions)
        .collect();

    let (to_import, skipped) = if force {
        println!("{}", amber("  ⚠ Force mode: re-importing all sessions\n"));
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
            "\n  {} {} {}",
            dim("Use"),
            cyan("soul pull --force"),
            dim("to re-import everything.")
        );
        println!();
        return Ok(());
    }

    println!(
        "  {} {} to import, {} already imported\n",
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

    if !parse_errors.is_empty() && parse_errors.len() <= 5 {
        for err in &parse_errors {
            println!("    {}", dim(err));
        }
    }

    if all_chunks.is_empty() {
        println!("\n{}", dim("  No meaningful content found in sessions."));
        println!();
        return Ok(());
    }

    // Phase 4: LLM Processing
    println!("\n  {} {}\n", ICON_BRAIN, gold("Processing through LLM..."));

    let client = reqwest::Client::new();
    let mut all_memories = Vec::new();
    let mut errors = Vec::new();

    let progress = ProgressBar::new(all_chunks.len() as u64);
    progress.set_style(
        ProgressStyle::with_template(
            "  {spinner:.yellow} [{bar:20.yellow/dark_gray}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("█░░"),
    );

    for (i, chunk) in all_chunks.iter().enumerate() {
        let label = if chunk.total > 1 {
            format!("{} ({}/{})", chunk.source, chunk.index + 1, chunk.total)
        } else {
            chunk.source.clone()
        };

        progress.set_message(dim(&format!("Processing {}", label)));

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
                    progress.set_message(amber("Rate limited. Waiting 30s..."));
                    tokio::time::sleep(Duration::from_secs(30)).await;
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
        check(&format!(
            "Processed {} chunks",
            bold_white(&all_chunks.len().to_string())
        ))
    );

    // Phase 5: Merge & Write
    let merged = merge_all_memories(&all_memories);

    let pb = spinner("Writing to vault...");
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let write_result = write_memories_to_vault(&merged, &today)?;
    pb.finish_with_message(check("Vault updated"));

    // Phase 6: Update source tracking
    let pb = spinner("Updating source tracking...");
    update_pull_tracking(&to_import)?;
    pb.finish_with_message(check("Source tracking updated"));

    // Summary
    print_summary(
        to_import.len(),
        skipped,
        &merged,
        &write_result.topics_written,
        &write_result.people_written,
        &errors,
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

// ─── Summary ──────────────────────────────────────────────────────────────────

fn print_summary(
    imported: usize,
    skipped: usize,
    merged: &ExtractedMemories,
    topics: &[String],
    people: &[String],
    errors: &[String],
) {
    let total = merged.fact_count();
    println!("\n{}", line());
    println!("\n  {} {}\n", amber(ICON_STAR), bold_gold("Pull complete!"));

    println!(
        "  {} {} imported, {} skipped",
        dim(&format!("{:<18}", "Sessions")),
        bold_white(&imported.to_string()),
        dim(&skipped.to_string())
    );
    println!(
        "  {} {}",
        dim(&format!("{:<18}", "Memories extracted")),
        bold_white(&total.to_string())
    );
    println!(
        "  {} {}{}",
        dim(&format!("{:<18}", "Topics found")),
        bold_white(&topics.len().to_string()),
        summarize_list(topics)
    );
    println!(
        "  {} {}{}",
        dim(&format!("{:<18}", "People found")),
        bold_white(&people.len().to_string()),
        summarize_list(people)
    );
    if !errors.is_empty() {
        println!(
            "  {} {}",
            dim(&format!("{:<18}", "Errors")),
            amber(&errors.len().to_string())
        );
    }
    println!(
        "\n  {} {} {}",
        dim("Run"),
        cyan("soul status"),
        dim("to see your vault.")
    );
    println!();
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("  {spinner:.yellow} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

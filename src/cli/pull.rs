//! Provider auto-discovery import implementation for `soul import`.
//!
//! Discovers sessions from Claude Code, OpenClaw, and other providers,
//! then runs them through the import pipeline.

use anyhow::Result;

use crate::adapters::AdapterRegistry;
use crate::auth::{ensure_valid_credentials, is_logged_in};
use crate::cli::pull_pipeline::{discover_sessions, parse_sessions_to_chunks, process_chunks};
use crate::cli::pull_summary::print_summary;
use crate::cli::pull_tracking::{
    filter_new_sessions, update_pull_config_timestamps, update_pull_tracking,
};
use crate::core::merger::merge_all_memories;
use crate::types::Provider;
use crate::ui::theme::*;
use crate::vault::config::{assert_initialized, get_api_key};
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

    let registry = AdapterRegistry::new();
    let discovery = discover_sessions(&registry);

    if discovery.total_sessions == 0 {
        println!("{}", dim("  No AI sessions found on this machine."));
        println!(
            "  {} Supported: Claude Code, OpenClaw, Gemini CLI, Codex",
            dim(ICON_DOT)
        );
        println!();
        return Ok(());
    }

    let (to_import, skipped) = if force {
        println!("{}", amber("  ! Force mode: re-importing all sessions"));
        (discovery.all_sessions, 0)
    } else {
        filter_new_sessions(discovery.all_sessions)?
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

    let (all_chunks, parse_errors) = parse_sessions_to_chunks(&registry, &to_import);
    if all_chunks.is_empty() {
        println!("{}", dim("  No meaningful content found in sessions."));
        println!();
        return Ok(());
    }

    let (all_memories, errors) = process_chunks(&all_chunks).await?;
    let merged = merge_all_memories(&all_memories);

    let pb = crate::cli::ingest_process::spinner("Writing to vault...");
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let write_result = write_memories_to_vault(&merged, &today)?;
    pb.finish_with_message(check("Vault updated"));

    let pb = crate::cli::ingest_process::spinner("Updating source tracking...");
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

    if let Err(e) = update_pull_config_timestamps(&discovery.discovered_providers) {
        eprintln!(
            "{}",
            amber(&format!(
                "  ⚠ Could not update provider import sync timestamps: {}",
                e
            ))
        );
    }

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
        Some(raw) => raw.parse::<Provider>().map_err(anyhow::Error::msg)?,
        None => Provider::Claude,
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

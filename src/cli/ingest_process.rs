//! Chunk processing and progress UI for ingest.

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

use crate::core::processor::process_chunk;
use crate::types::{ChunkInfo, ExtractedMemories};
use crate::ui::theme::*;

pub(crate) async fn process_all_chunks(
    all_chunks: &[ChunkInfo],
) -> Result<(Vec<ExtractedMemories>, Vec<String>)> {
    println!(
        "\n  {} {}\n",
        ICON_BRAIN,
        gold("Processing through Claude...")
    );

    let client = reqwest::Client::new();
    let mut all_memories = Vec::new();
    let mut errors = Vec::new();

    let pb = ProgressBar::new(all_chunks.len() as u64);
    let style = ProgressStyle::with_template(
        "  {spinner:.yellow} [{bar:20.yellow/dark_gray}] {pos}/{len} {msg}",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar())
    .progress_chars("█░░");
    pb.set_style(style);

    for (i, chunk) in all_chunks.iter().enumerate() {
        let label = if chunk.total > 1 {
            format!("{} ({}/{})", chunk.source, chunk.index + 1, chunk.total)
        } else {
            chunk.source.clone()
        };

        pb.set_message(dim(&format!("Processing {}", label)));

        match process_chunk(&client, chunk).await {
            Ok(memories) => {
                let fact_count = memories.fact_count();
                all_memories.push(memories);
                pb.set_message(format!(
                    "{} → {} facts",
                    dim(&chunk.source),
                    bold_white(&fact_count.to_string())
                ));
            }
            Err(e) => {
                let msg = e.to_string();
                pb.set_message(format!("{} → {}", dim(&chunk.source), red(&msg)));
                errors.push(format!("Processing {}: {}", chunk.source, msg));

                if msg.contains("API key") || msg.contains("401") {
                    pb.finish_and_clear();
                    anyhow::bail!("API key error. Run `soul init` to reconfigure.");
                }

                if msg.contains("Rate limited") || msg.contains("429") {
                    pb.set_message(amber("Rate limited. Waiting 30s..."));
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    if let Ok(memories) = process_chunk(&client, chunk).await {
                        all_memories.push(memories);
                    }
                }
            }
        }

        pb.set_position((i + 1) as u64);
    }

    pb.finish_and_clear();
    println!(
        "{}",
        check(&format!(
            "Processed {} chunks",
            bold_white(&all_chunks.len().to_string())
        ))
    );

    Ok((all_memories, errors))
}

pub(crate) fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    let style = ProgressStyle::with_template("  {spinner:.yellow} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]);
    pb.set_style(style);
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

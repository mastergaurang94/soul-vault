//! Summary rendering for ingest.

use crate::types::ExtractedMemories;
use crate::ui::theme::*;

pub(crate) fn print_summary(
    new_count: usize,
    modified_count: usize,
    skipped_count: usize,
    merged: &ExtractedMemories,
    topics_written: &[String],
    people_written: &[String],
    errors: &[String],
) {
    let total = merged.fact_count();
    println!("\n{}", line());
    println!(
        "\n  {} {}\n",
        amber(ICON_STAR),
        bold_gold("Import complete!")
    );

    println!(
        "  {} {} new, {} updated, {} skipped",
        dim(&format!("{:<18}", "Imported")),
        bold_white(&new_count.to_string()),
        amber(&modified_count.to_string()),
        dim(&skipped_count.to_string())
    );
    println!(
        "  {} {}",
        dim(&format!("{:<18}", "Memories extracted")),
        bold_white(&total.to_string())
    );
    println!(
        "  {} {}{}",
        dim(&format!("{:<18}", "Topics found")),
        bold_white(&topics_written.len().to_string()),
        summarize_list(topics_written)
    );
    println!(
        "  {} {}{}",
        dim(&format!("{:<18}", "People found")),
        bold_white(&people_written.len().to_string()),
        summarize_list(people_written)
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

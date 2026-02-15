//! Summary rendering for provider import.

use crate::types::ExtractedMemories;
use crate::ui::theme::*;

pub(crate) fn print_summary(
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

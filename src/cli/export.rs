//! `soul export` — outputs vault as context, JSON, or bundle directory.

use anyhow::Result;
use std::path::PathBuf;

use crate::cli::export_bundle::output_bundle;
use crate::cli::export_context::output_context;
use crate::cli::export_json::output_json;
use crate::cli::export_types::{default_output_path, parse_sections, ExportFormat};
use crate::vault::config::assert_initialized;
use crate::vault::read::read_vault_content;

pub use crate::cli::export_types::ExportSection;

// ─── Export Command ───────────────────────────────────────────────────────────

pub fn run(
    output: Option<&str>,
    format: &str,
    topic: Option<&str>,
    sections: Option<&str>,
) -> Result<()> {
    assert_initialized()?;

    let export_format = ExportFormat::parse(format)?;
    let selected_sections: Vec<ExportSection> = parse_sections(sections)?;
    let vault = read_vault_content()?;

    match export_format {
        ExportFormat::Context => output_context(&vault, output, topic, &selected_sections)?,
        ExportFormat::Json => output_json(&vault, output, topic, &selected_sections)?,
        ExportFormat::Bundle => output_bundle(output, &selected_sections)?,
    }

    // Show CLI confirmation (skipped when called from TUI)
    if let Some(path) = output {
        use crate::ui::theme::*;
        eprintln!("{}", check(&format!("Exported to {}", cyan(path))));
    }

    Ok(())
}

/// Silent export — used by TUI to avoid stderr output.
pub fn run_quiet(
    output: Option<&str>,
    format: &str,
    topic: Option<&str>,
    sections: Option<&str>,
) -> Result<()> {
    assert_initialized()?;

    let export_format = ExportFormat::parse(format)?;
    let selected_sections: Vec<ExportSection> = parse_sections(sections)?;
    let vault = read_vault_content()?;

    match export_format {
        ExportFormat::Context => output_context(&vault, output, topic, &selected_sections),
        ExportFormat::Json => output_json(&vault, output, topic, &selected_sections),
        ExportFormat::Bundle => output_bundle(output, &selected_sections),
    }
}

pub fn smart_default_output_path(format: &str) -> Result<PathBuf> {
    Ok(default_output_path(ExportFormat::parse(format)?))
}

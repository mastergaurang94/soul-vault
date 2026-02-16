//! Soul Vault — Your AI memory, unified.
//!
//! A CLI tool that distills AI conversations into a structured local vault.

mod adapters;
mod auth;
mod cli;
mod core;
mod extractors;
mod tui;
mod types;
mod ui;
mod vault;

use clap::{Parser, Subcommand};
use std::io::{self, IsTerminal, Write};

// ─── CLI Definition ───────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "soul",
    about = "Your AI memory, unified. Distills AI conversations into a structured local vault.",
    version,
    disable_version_flag = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Print version
    #[arg(short = 'v', short_alias = 'V', long = "version", action = clap::ArgAction::Version)]
    version: (),
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize your Soul Vault (first-time setup)
    Init,

    /// Import sessions from AI providers or process local files into your vault
    Import {
        /// Path to folder containing files to import (omit for provider auto-discovery)
        folder: Option<String>,

        /// Force re-import, ignoring source tracking
        #[arg(short, long)]
        force: bool,

        /// Import from provider cloud APIs instead of local session files
        #[arg(long)]
        cloud: bool,

        /// Cloud provider to use: claude, chatgpt, gemini (default: claude)
        #[arg(long)]
        provider: Option<String>,
    },

    /// Watch a folder and auto-import on changes
    Watch {
        /// Path to folder to watch
        folder: Option<String>,
    },

    /// Export vault as context document
    Export {
        /// Write to file instead of stdout
        #[arg(short, long)]
        output: Option<String>,

        /// Output format: context (default), json, or bundle
        #[arg(short, long, default_value = "context")]
        format: String,

        /// Filter by topic
        #[arg(short, long)]
        topic: Option<String>,

        /// Sections to include: identity,preferences,topics,people,memories
        #[arg(long)]
        sections: Option<String>,
    },

    /// Show vault summary and imported sources
    Status,

    /// Login to a cloud provider via OAuth (default: claude)
    Login {
        /// Provider to authenticate: claude, chatgpt, gemini
        provider: Option<String>,
    },

    /// Logout and remove stored OAuth credentials
    Logout {
        /// Provider to logout from (omit to clear all saved credentials)
        provider: Option<String>,
    },

    /// Reset vault — delete all data and return to pre-init state
    Reset {
        /// Skip confirmation prompt (for scripting/testing)
        #[arg(short, long)]
        force: bool,

        /// Permanently delete vault instead of moving to Trash
        #[arg(long)]
        permanent: bool,
    },
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        None => run_no_subcommand().await,
        Some(Commands::Init) => cli::init::run().await,
        Some(Commands::Import {
            folder,
            force,
            cloud,
            provider,
        }) => cli::import::run(folder.as_deref(), force, cloud, provider.as_deref()).await,
        Some(Commands::Watch { folder }) => match folder {
            Some(f) => cli::watch::run(&f).await,
            None => cli::watch::run_auto().await,
        },
        Some(Commands::Login { provider }) => cli::login::run(provider.as_deref()).await,
        Some(Commands::Logout { provider }) => cli::logout::run(provider.as_deref()),
        Some(Commands::Export {
            output,
            format,
            topic,
            sections,
        }) => cli::export::run(
            output.as_deref(),
            &format,
            topic.as_deref(),
            sections.as_deref(),
        ),
        Some(Commands::Status) => cli::status::run(),
        Some(Commands::Reset { force, permanent }) => cli::reset::run(force, permanent),
    };

    if let Err(e) = result {
        eprintln!("  {} {}\n", ui::theme::red("✗"), e);
        std::process::exit(1);
    }
}

async fn run_no_subcommand() -> anyhow::Result<()> {
    if io::stdin().is_terminal() && !vault::config::is_initialized() {
        print!(
            "  Vault not initialized. Run setup now with {}? {} ",
            ui::theme::cyan("soul init"),
            ui::theme::dim("(Y/n)")
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !should_run_init(&input) {
            println!(
                "\n  {} Run {} when you're ready.\n",
                ui::theme::dim("Setup skipped."),
                ui::theme::cyan("soul")
            );
            return Ok(());
        }

        println!();
        cli::init::run().await?;
    }

    // No subcommand → full-screen TUI
    tui::run()
}

fn should_run_init(input: &str) -> bool {
    input.trim().to_lowercase() != "n"
}

#[cfg(test)]
mod tests {
    use super::should_run_init;

    #[test]
    fn should_run_init_defaults_to_yes() {
        assert!(should_run_init(""));
        assert!(should_run_init("y"));
        assert!(should_run_init("Y"));
        assert!(should_run_init("yes"));
    }

    #[test]
    fn should_run_init_respects_no() {
        assert!(!should_run_init("n"));
        assert!(!should_run_init("N"));
        assert!(!should_run_init(" n "));
    }
}

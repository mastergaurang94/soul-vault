//! Soma — Your AI memory, unified.
//!
//! A CLI tool that distills AI conversations into a structured local vault.

mod cli;
mod core;
mod extractors;
mod types;
mod ui;
mod vault;

use clap::{Parser, Subcommand};

// ─── CLI Definition ───────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "soma",
    about = "Your AI memory, unified. Distills AI conversations into a structured local vault.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize your Soma vault (first-time setup)
    Init,

    /// Import and process local files into your vault
    Ingest {
        /// Path to folder containing files to ingest
        folder: String,

        /// Force re-ingestion of all files, ignoring source tracking
        #[arg(short, long)]
        force: bool,
    },

    /// Watch a folder and auto-ingest on changes
    Watch {
        /// Path to folder to watch
        folder: String,
    },

    /// Export vault as context document
    Export {
        /// Write to file instead of stdout
        #[arg(short, long)]
        output: Option<String>,

        /// Output format: markdown (default) or json
        #[arg(short, long, default_value = "markdown")]
        format: String,

        /// Filter by topic
        #[arg(short, long)]
        topic: Option<String>,
    },

    /// Show vault summary and ingested sources
    Status,
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        None => {
            // No subcommand → interactive menu
            cli::interactive::run()
        }
        Some(Commands::Init) => cli::init::run(),
        Some(Commands::Ingest { folder, force }) => cli::ingest::run(&folder, force).await,
        Some(Commands::Watch { folder }) => cli::watch::run(&folder).await,
        Some(Commands::Export {
            output,
            format,
            topic,
        }) => cli::export::run(output.as_deref(), &format, topic.as_deref()),
        Some(Commands::Status) => cli::status::run(),
    };

    if let Err(e) = result {
        eprintln!("\n  {} {}\n", ui::theme::red("✗"), e);
        std::process::exit(1);
    }
}

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
    Import {
        /// Path to folder containing files to import
        folder: Option<String>,

        /// Force re-import of all files, ignoring source tracking
        #[arg(short, long)]
        force: bool,
    },

    /// Import and process local files into your vault (alias for import)
    #[command(hide = true)]
    Ingest {
        /// Path to folder containing files to import
        folder: Option<String>,

        /// Force re-import of all files, ignoring source tracking
        #[arg(short, long)]
        force: bool,
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

        /// Output format: markdown (default) or json
        #[arg(short, long, default_value = "markdown")]
        format: String,

        /// Filter by topic
        #[arg(short, long)]
        topic: Option<String>,
    },

    /// Show vault summary and imported sources
    Status,

    /// Reset vault — delete all data and return to pre-init state
    Reset {
        /// Skip confirmation prompt (for scripting/testing)
        #[arg(short, long)]
        force: bool,
    },
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
        Some(Commands::Import { folder, force }) | Some(Commands::Ingest { folder, force }) => {
            match folder {
                Some(f) => cli::ingest::run(&f, force).await,
                None => {
                    eprintln!("\n  {} Missing folder path.\n", ui::theme::red("✗"));
                    eprintln!("  Usage: soma import <folder>\n");
                    eprintln!("  Example: soma import ~/Documents/chatgpt-exports\n");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Watch { folder }) => match folder {
            Some(f) => cli::watch::run(&f).await,
            None => {
                eprintln!("\n  {} Missing folder path.\n", ui::theme::red("✗"));
                eprintln!("  Usage: soma watch <folder>\n");
                eprintln!("  Example: soma watch ~/Documents/chatgpt-exports\n");
                std::process::exit(1);
            }
        },
        Some(Commands::Export {
            output,
            format,
            topic,
        }) => cli::export::run(output.as_deref(), &format, topic.as_deref()),
        Some(Commands::Status) => cli::status::run(),
        Some(Commands::Reset { force }) => cli::reset::run(force),
    };

    if let Err(e) = result {
        eprintln!("  {} {}\n", ui::theme::red("✗"), e);
        std::process::exit(1);
    }
}

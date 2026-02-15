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

// ─── CLI Definition ───────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "soul",
    about = "Your AI memory, unified. Distills AI conversations into a structured local vault.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize your Soul Vault (first-time setup)
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

    /// Pull AI sessions from all providers (Claude Code, OpenClaw, etc.)
    Pull {
        /// Force re-import of all sessions, ignoring source tracking
        #[arg(short, long)]
        force: bool,

        /// Pull from provider cloud APIs instead of local session files
        #[arg(long)]
        cloud: bool,

        /// Cloud provider to use: claude, chatgpt, gemini (default: claude)
        #[arg(long)]
        provider: Option<String>,
    },

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
    },
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        None => {
            // No subcommand → full-screen TUI
            tui::run()
        }
        Some(Commands::Init) => cli::init::run(),
        Some(Commands::Import { folder, force }) | Some(Commands::Ingest { folder, force }) => {
            match folder {
                Some(f) => cli::ingest::run(&f, force).await,
                None => {
                    eprintln!("\n  {} Missing folder path.\n", ui::theme::red("✗"));
                    eprintln!("  Usage: soul import <folder>\n");
                    eprintln!("  Example: soul import ~/Documents/chatgpt-exports\n");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Watch { folder }) => match folder {
            Some(f) => cli::watch::run(&f).await,
            None => cli::watch::run_auto().await,
        },
        Some(Commands::Pull {
            force,
            cloud,
            provider,
        }) => cli::pull::run(force, cloud, provider.as_deref()).await,
        Some(Commands::Login { provider }) => cli::login::run(provider.as_deref()).await,
        Some(Commands::Logout { provider }) => cli::logout::run(provider.as_deref()),
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

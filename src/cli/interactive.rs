//! `soul` (no args) — legacy inline interactive menu.
//! Replaced by `tui::run()` for the full-screen TUI experience.
//! Kept for reference; may be removed in a future release.

#![allow(dead_code)]

use anyhow::Result;
use std::io::{self, Write};

use crate::cli::interactive_menu::{run_inline_menu, Action};
use crate::ui::theme::*;
use crate::vault::config::is_initialized;

// ─── Run Interactive Mode ─────────────────────────────────────────────────────

pub fn run() -> Result<()> {
    if !atty_check() {
        print_non_tty_help();
        return Ok(());
    }

    println!("{}", banner());
    if is_initialized() {
        println!("  {}", dim("Vault ready. Select an action:"));
    } else {
        println!(
            "  {}",
            dim("Vault not initialized. Select Init to get started.")
        );
    }
    println!();

    let selected = run_inline_menu()?;

    if let Some(ref action) = selected {
        println!();
        match action {
            Action::Init => {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(crate::cli::init::run())?;
            }
            Action::Import => run_import_prompt()?,
            Action::Login => {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(crate::cli::login::run(None))?;
            }
            Action::Logout => crate::cli::logout::run(None)?,
            Action::Watch => run_watch_prompt()?,
            Action::Export => crate::cli::export::run(None, "context", None, None)?,
            Action::Status => crate::cli::status::run()?,
            Action::Reset => crate::cli::reset::run(false)?,
            Action::Quit => {}
        }
    }

    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn run_import_prompt() -> Result<()> {
    print!("  Enter folder path (leave blank for providers mode): ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let folder = input.trim();

    let rt = tokio::runtime::Handle::current();
    if folder.is_empty() {
        rt.block_on(crate::cli::import::run(None, false, false, None))?;
    } else {
        rt.block_on(crate::cli::import::run(Some(folder), false, false, None))?;
    }

    Ok(())
}

fn run_watch_prompt() -> Result<()> {
    print!("  Enter folder path to watch: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let folder = input.trim();

    if folder.is_empty() {
        println!("\n  No folder path provided.\n");
        println!("  Usage: soul watch <folder>");
        println!("  Example: soul watch ~/Documents/chatgpt-exports\n");
    } else {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(crate::cli::watch::run(folder))?;
    }

    Ok(())
}

fn atty_check() -> bool {
    use std::io::IsTerminal;
    io::stdin().is_terminal()
}

fn print_non_tty_help() {
    println!("{}", banner());
    println!("  Interactive mode requires a terminal (TTY).");
    println!("  Use a subcommand instead:\n");
    println!("    {}              Initialize vault", cyan("soul init"));
    println!(
        "    {}          Import from AI providers",
        cyan("soul import")
    );
    println!(
        "    {}  Import files from a folder",
        cyan("soul import <folder>")
    );
    println!(
        "    {}   Watch folder for changes",
        cyan("soul watch <folder>")
    );
    println!(
        "    {}      Login to cloud provider via OAuth",
        cyan("soul login [provider]")
    );
    println!(
        "    {}     Logout and clear OAuth credentials",
        cyan("soul logout [provider]")
    );
    println!(
        "    {}            Export vault context",
        cyan("soul export")
    );
    println!("    {}            Show vault summary", cyan("soul status"));
    println!(
        "    {}            Delete vault and start over",
        cyan("soul reset")
    );
    println!("    {}            Show all commands", dim("soul --help"));
    println!();
}

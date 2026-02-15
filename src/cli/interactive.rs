//! `soul` (no args) — legacy inline interactive menu.
//! Replaced by `tui::run()` for the full-screen TUI experience.
//! Kept for reference; may be removed in a future release.

#![allow(dead_code)]

use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    style::Print,
    terminal::{self, Clear, ClearType},
    ExecutableCommand,
};
use std::io::{self, Write};

use crate::ui::theme::*;
use crate::vault::config::is_initialized;

// ─── Menu Items ───────────────────────────────────────────────────────────────

struct MenuItem {
    label: &'static str,
    description: &'static str,
    action: Action,
}

#[derive(Clone)]
enum Action {
    Init,
    Status,
    Import,
    Pull,
    Login,
    Logout,
    Export,
    Watch,
    Reset,
    Quit,
}

const MENU_ITEMS: &[MenuItem] = &[
    MenuItem {
        label: "Init",
        description: "Setup or reconfigure",
        action: Action::Init,
    },
    MenuItem {
        label: "Status",
        description: "What's in your vault",
        action: Action::Status,
    },
    MenuItem {
        label: "Import",
        description: "Import local files & transcripts",
        action: Action::Import,
    },
    MenuItem {
        label: "Pull",
        description: "Import local AI app sessions",
        action: Action::Pull,
    },
    MenuItem {
        label: "Login",
        description: "OAuth login for cloud pull",
        action: Action::Login,
    },
    MenuItem {
        label: "Logout",
        description: "Clear saved OAuth credentials",
        action: Action::Logout,
    },
    MenuItem {
        label: "Export",
        description: "Output context for any AI",
        action: Action::Export,
    },
    MenuItem {
        label: "Watch",
        description: "Auto-import on file changes",
        action: Action::Watch,
    },
    MenuItem {
        label: "Reset",
        description: "Delete vault and start over",
        action: Action::Reset,
    },
    MenuItem {
        label: "Quit",
        description: "",
        action: Action::Quit,
    },
];

// ─── Run Interactive Mode ─────────────────────────────────────────────────────

pub fn run() -> Result<()> {
    // Check TTY
    if !atty_check() {
        print_non_tty_help();
        return Ok(());
    }

    // Print banner and status (these stay in scrollback)
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

    // Run inline menu
    let selected = run_inline_menu()?;

    // Handle the chosen action
    if let Some(ref action) = selected {
        println!(); // blank line before command output
        match action {
            Action::Init => {
                crate::cli::init::run()?;
            }
            Action::Import => {
                print!("  Enter folder path: ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let folder = input.trim();
                if folder.is_empty() {
                    println!("\n  No folder path provided.\n");
                    println!("  Usage: soul import <folder>");
                    println!("  Example: soul import ~/Documents/chatgpt-exports\n");
                } else {
                    let rt = tokio::runtime::Handle::current();
                    rt.block_on(crate::cli::ingest::run(folder, false))?;
                }
            }
            Action::Pull => {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(crate::cli::pull::run(false, false, None))?;
            }
            Action::Login => {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(crate::cli::login::run(None))?;
            }
            Action::Logout => {
                crate::cli::logout::run(None)?;
            }
            Action::Watch => {
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
            }
            Action::Export => {
                crate::cli::export::run(None, "context", None, None)?;
            }
            Action::Status => {
                crate::cli::status::run()?;
            }
            Action::Reset => {
                crate::cli::reset::run(false)?;
            }
            Action::Quit => {}
        }
    }

    Ok(())
}

// ─── Inline Menu ──────────────────────────────────────────────────────────────

fn run_inline_menu() -> Result<Option<Action>> {
    let mut selected: usize = 0;
    let menu_len = MENU_ITEMS.len();

    // Print initial menu
    print_menu(selected);

    // Enable raw mode for key input
    terminal::enable_raw_mode()?;

    let result = loop {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if selected > 0 {
                        selected -= 1;
                    } else {
                        selected = menu_len - 1;
                    }
                    reprint_menu(selected, menu_len)?;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if selected < menu_len - 1 {
                        selected += 1;
                    } else {
                        selected = 0;
                    }
                    reprint_menu(selected, menu_len)?;
                }
                KeyCode::Enter => {
                    break Some(MENU_ITEMS[selected].action.clone());
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    break None;
                }
                _ => {}
            }
        }
    };

    // Restore terminal
    terminal::disable_raw_mode()?;

    // Move cursor past the menu and clear the menu lines
    // (so the selected action's output appears cleanly)
    let stdout = io::stdout();
    let mut out = stdout.lock();
    // Move to end of menu area
    out.execute(cursor::MoveDown((menu_len - selected) as u16))?;
    out.execute(Print("\n"))?;
    out.flush()?;

    Ok(result)
}

fn print_menu(selected: usize) {
    for (i, item) in MENU_ITEMS.iter().enumerate() {
        print_menu_item(i, item, i == selected);
    }
    // Footer
    println!();
    println!("  {}", dim("  up/down/jk navigate  enter select  q quit"));
}

fn print_menu_item(index: usize, item: &MenuItem, is_selected: bool) {
    let num = format!("{}.", index + 1);
    if is_selected {
        let label = format!("  > {} {}", num, item.label);
        if item.description.is_empty() {
            println!("{}", bold_gold(&label));
        } else {
            println!("{}  {}", bold_gold(&label), dim(item.description));
        }
    } else {
        let label = format!("    {} {}", num, item.label);
        if item.description.is_empty() {
            println!("{}", label);
        } else {
            println!("{}  {}", label, dim(item.description));
        }
    }
}

fn reprint_menu(selected: usize, menu_len: usize) -> Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    // Move cursor up to the start of the menu (menu lines + 1 blank + 1 footer)
    let lines_to_move = menu_len + 2;
    out.execute(cursor::MoveUp(lines_to_move as u16))?;

    // Clear and reprint each line
    for (i, item) in MENU_ITEMS.iter().enumerate() {
        out.execute(Clear(ClearType::CurrentLine))?;
        // We need to print without the println adding \r\n in raw mode
        // So we disable raw mode briefly... actually let's just write directly
        let num = format!("{}.", i + 1);
        if i == selected {
            let label = format!("  > {} {}", num, item.label);
            if item.description.is_empty() {
                let line = bold_gold(&label);
                out.execute(Print(format!("{}\r\n", line)))?;
            } else {
                let line = format!("{}  {}", bold_gold(&label), dim(item.description));
                out.execute(Print(format!("{}\r\n", line)))?;
            }
        } else {
            let label = format!("    {} {}", num, item.label);
            if item.description.is_empty() {
                out.execute(Print(format!("{}\r\n", label)))?;
            } else {
                let line = format!("{}  {}", label, dim(item.description));
                out.execute(Print(format!("{}\r\n", line)))?;
            }
        }
    }

    // Blank line + footer
    out.execute(Clear(ClearType::CurrentLine))?;
    out.execute(Print("\r\n"))?;
    out.execute(Clear(ClearType::CurrentLine))?;
    out.execute(Print(format!(
        "  {}\r\n",
        dim("  up/down/jk navigate  enter select  q quit")
    )))?;

    out.flush()?;
    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn atty_check() -> bool {
    use std::io::IsTerminal;
    io::stdin().is_terminal()
}

fn print_non_tty_help() {
    println!("{}", banner());
    println!("  Interactive mode requires a terminal (TTY).");
    println!("  Use a subcommand instead:\n");
    println!("    {}              Initialize vault", cyan("soul init"));
    println!("    {}  Import files", cyan("soul import <folder>"));
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

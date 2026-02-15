//! Inline menu rendering and keyboard handling for legacy interactive mode.

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

struct MenuItem {
    label: &'static str,
    description: &'static str,
    action: Action,
}

#[derive(Clone)]
pub(crate) enum Action {
    Init,
    Status,
    Import,
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
        description: "Providers auto-discovery or local folder import",
        action: Action::Import,
    },
    MenuItem {
        label: "Login",
        description: "OAuth login for cloud import",
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

pub(crate) fn run_inline_menu() -> Result<Option<Action>> {
    let mut selected: usize = 0;
    let menu_len = MENU_ITEMS.len();

    print_menu(selected);
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
                KeyCode::Enter => break Some(MENU_ITEMS[selected].action.clone()),
                KeyCode::Char('q') | KeyCode::Esc => break None,
                _ => {}
            }
        }
    };

    terminal::disable_raw_mode()?;

    let stdout = io::stdout();
    let mut out = stdout.lock();
    out.execute(cursor::MoveDown((menu_len - selected) as u16))?;
    out.execute(Print("\n"))?;
    out.flush()?;

    Ok(result)
}

fn print_menu(selected: usize) {
    for (i, item) in MENU_ITEMS.iter().enumerate() {
        print_menu_item(i, item, i == selected);
    }
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

    let lines_to_move = menu_len + 2;
    out.execute(cursor::MoveUp(lines_to_move as u16))?;

    for (i, item) in MENU_ITEMS.iter().enumerate() {
        out.execute(Clear(ClearType::CurrentLine))?;
        let num = format!("{}.", i + 1);
        if i == selected {
            let label = format!("  > {} {}", num, item.label);
            if item.description.is_empty() {
                out.execute(Print(format!("{}\r\n", bold_gold(&label))))?;
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

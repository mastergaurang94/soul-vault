//! `soma` (no args) — ratatui interactive menu with arrow key / vim bindings.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame, Terminal,
};
use std::io;

use crate::ui::theme::rat;
use crate::vault::config::is_initialized;

// ─── Menu Items ───────────────────────────────────────────────────────────────

struct MenuItem {
    label: &'static str,
    description: &'static str,
    icon: &'static str,
    action: Action,
}

#[derive(Clone)]
enum Action {
    Ingest,
    Watch,
    Export,
    Status,
    Init,
    Quit,
}

const MENU_ITEMS: &[MenuItem] = &[
    MenuItem {
        label: "Ingest",
        description: "Import local files & transcripts",
        icon: "📥",
        action: Action::Ingest,
    },
    MenuItem {
        label: "Watch",
        description: "Auto-ingest on file changes",
        icon: "👁 ",
        action: Action::Watch,
    },
    MenuItem {
        label: "Export",
        description: "Output context for any AI",
        icon: "📤",
        action: Action::Export,
    },
    MenuItem {
        label: "Status",
        description: "What's in your vault",
        icon: "📊",
        action: Action::Status,
    },
    MenuItem {
        label: "Init",
        description: "Setup or reconfigure",
        icon: "⚙️ ",
        action: Action::Init,
    },
    MenuItem {
        label: "Quit",
        description: "",
        icon: "👋",
        action: Action::Quit,
    },
];

// ─── App State ────────────────────────────────────────────────────────────────

struct App {
    selected: usize,
    should_quit: bool,
    chosen_action: Option<Action>,
}

impl App {
    fn new() -> Self {
        Self {
            selected: 0,
            should_quit: false,
            chosen_action: None,
        }
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = MENU_ITEMS.len() - 1;
        }
    }

    fn move_down(&mut self) {
        if self.selected < MENU_ITEMS.len() - 1 {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
    }

    fn select(&mut self) {
        self.chosen_action = Some(MENU_ITEMS[self.selected].action.clone());
        self.should_quit = true;
    }
}

// ─── Run Interactive Mode ─────────────────────────────────────────────────────

pub fn run() -> Result<()> {
    // Check TTY
    if !atty_check() {
        print_non_tty_help();
        return Ok(());
    }

    let mut app = App::new();

    // Setup terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    loop {
        terminal.draw(|f| draw(f, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                KeyCode::Enter => app.select(),
                KeyCode::Char('q') | KeyCode::Esc => {
                    app.should_quit = true;
                }
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    // Handle the chosen action
    if let Some(ref action) = app.chosen_action {
        match action {
            Action::Init => {
                crate::cli::init::run()?;
            }
            Action::Ingest => {
                println!("\n  Usage: soma ingest <folder>\n");
                println!("  Example: soma ingest ~/Documents/chatgpt-exports\n");
            }
            Action::Watch => {
                println!("\n  Usage: soma watch <folder>\n");
                println!("  Example: soma watch ~/Documents/chatgpt-exports\n");
            }
            Action::Export => {
                crate::cli::export::run(None, "markdown", None)?;
            }
            Action::Status => {
                crate::cli::status::run()?;
            }
            Action::Quit => {}
        }
    }

    Ok(())
}

// ─── Draw ─────────────────────────────────────────────────────────────────────

fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Center the menu vertically
    let total_height = 12; // header + items + footer
    let y_offset = if area.height > total_height as u16 {
        (area.height - total_height as u16) / 3
    } else {
        0
    };

    let chunks = Layout::vertical([
        Constraint::Length(y_offset),
        Constraint::Length(1), // blank
        Constraint::Length(1), // title
        Constraint::Length(1), // blank
        Constraint::Length(1), // status
        Constraint::Length(1), // blank
        Constraint::Length(MENU_ITEMS.len() as u16),
        Constraint::Length(1), // blank
        Constraint::Length(1), // footer
        Constraint::Min(0),
    ])
    .split(area);

    // Title: "  Soma ✦ Your AI memory, unified."
    let title = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "Soma",
            Style::default()
                .fg(rat::PURPLE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled("✦", Style::default().fg(rat::AMBER)),
        Span::raw(" "),
        Span::styled("Your AI memory, unified.", Style::default().fg(rat::DIM)),
    ]);
    f.render_widget(Paragraph::new(title), chunks[2]);

    // Status line
    let status_text = if is_initialized() {
        "  Vault ready. Select an action:"
    } else {
        "  Vault not initialized. Select Init to get started."
    };
    let status = Line::from(Span::styled(status_text, Style::default().fg(rat::DIM)));
    f.render_widget(Paragraph::new(status), chunks[4]);

    // Menu items
    let menu_area = chunks[6];
    for (i, item) in MENU_ITEMS.iter().enumerate() {
        let is_selected = i == app.selected;
        let item_area = Rect::new(menu_area.x, menu_area.y + i as u16, menu_area.width, 1);

        let (prefix, label_style) = if is_selected {
            (
                "  ▶ ",
                Style::default()
                    .fg(rat::PURPLE)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ("    ", Style::default().fg(ratatui::style::Color::White))
        };

        let mut spans = vec![
            Span::styled(prefix, label_style),
            Span::styled(format!("{} {}", item.icon, item.label), label_style),
        ];

        if !item.description.is_empty() {
            spans.push(Span::styled(
                format!("  {}", item.description),
                Style::default().fg(rat::DIM),
            ));
        }

        f.render_widget(Paragraph::new(Line::from(spans)), item_area);
    }

    // Footer
    let footer = Line::from(Span::styled(
        "  ↑↓/jk navigate  ↵ select  q quit",
        Style::default().fg(rat::DIM),
    ));
    f.render_widget(Paragraph::new(footer), chunks[8]);
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn atty_check() -> bool {
    use std::io::IsTerminal;
    io::stdin().is_terminal()
}

fn print_non_tty_help() {
    use crate::ui::theme::{banner, cyan, dim};
    println!("{}", banner());
    println!("  Interactive mode requires a terminal (TTY).");
    println!("  Use a subcommand instead:\n");
    println!("    {}              Initialize vault", cyan("soma init"));
    println!("    {}   Import files", cyan("soma ingest <folder>"));
    println!("    {}    Watch folder for changes", cyan("soma watch <folder>"));
    println!("    {}            Export vault context", cyan("soma export"));
    println!("    {}            Show vault summary", cyan("soma status"));
    println!(
        "    {}            Show all commands",
        dim("soma --help")
    );
    println!();
}

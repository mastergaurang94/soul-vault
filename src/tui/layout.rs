//! TUI layout rendering — header, body (sidebar + content), footer.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use super::app::{App, Focus};
use super::pages::PageWidget;
use super::sidebar::Sidebar;
use crate::ui::theme::rat;

// ─── Page Set ─────────────────────────────────────────────────────────────────

use super::app::Page;
use super::pages::browse::BrowsePage;
use super::pages::export::ExportPage;
use super::pages::import::ImportPage;
use super::pages::pull::PullPage;
use super::pages::settings::SettingsPage;
use super::pages::status::StatusPage;
use super::pages::watch::WatchPage;

pub struct PageSet {
    pub status: StatusPage,
    pub pull: PullPage,
    pub import: ImportPage,
    pub browse: BrowsePage,
    pub export: ExportPage,
    pub watch: WatchPage,
    pub settings: SettingsPage,
}

impl PageSet {
    pub fn new() -> Self {
        Self {
            status: StatusPage::default(),
            pull: PullPage::default(),
            import: ImportPage::default(),
            browse: BrowsePage::default(),
            export: ExportPage::default(),
            watch: WatchPage::default(),
            settings: SettingsPage::default(),
        }
    }

    pub fn current(&self, page: Page) -> &dyn PageWidget {
        match page {
            Page::Status => &self.status,
            Page::Pull => &self.pull,
            Page::Import => &self.import,
            Page::Browse => &self.browse,
            Page::Export => &self.export,
            Page::Watch => &self.watch,
            Page::Settings => &self.settings,
        }
    }

    pub fn current_mut(&mut self, page: Page) -> &mut dyn PageWidget {
        match page {
            Page::Status => &mut self.status,
            Page::Pull => &mut self.pull,
            Page::Import => &mut self.import,
            Page::Browse => &mut self.browse,
            Page::Export => &mut self.export,
            Page::Watch => &mut self.watch,
            Page::Settings => &mut self.settings,
        }
    }
}

// ─── Layout Rendering ─────────────────────────────────────────────────────────

pub fn render_layout(area: Rect, buf: &mut Buffer, app: &App, pages: &PageSet) {
    let vertical = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Min(1),    // body
        Constraint::Length(1), // footer
    ])
    .split(area);

    render_header(vertical[0], buf);
    render_body(vertical[1], buf, app, pages);
    render_footer(vertical[2], buf, app);
}

fn render_header(area: Rect, buf: &mut Buffer) {
    let line = Line::from(vec![
        Span::styled(
            " Soul Vault ",
            Style::default().fg(rat::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled("* ", Style::default().fg(rat::AMBER)),
        Span::styled("Your AI memory, unified.", Style::default().fg(rat::DIM)),
    ]);
    Paragraph::new(line).render(area, buf);
}

fn render_body(area: Rect, buf: &mut Buffer, app: &App, pages: &PageSet) {
    let horizontal = Layout::horizontal([Constraint::Length(20), Constraint::Min(1)]).split(area);

    Sidebar::new(app).render(horizontal[0], buf);
    pages
        .current(app.current_page)
        .render(horizontal[1], buf, app);
}

fn render_footer(area: Rect, buf: &mut Buffer, app: &App) {
    let hints = match app.focus {
        Focus::Sidebar => "  j/k navigate  enter select  tab content  q quit  ? help",
        Focus::Content => "  esc back  tab sidebar  q quit",
    };
    let line = Line::from(Span::styled(hints, Style::default().fg(rat::DIM)));
    Paragraph::new(line).render(area, buf);
}

// ─── Non-TTY Help ─────────────────────────────────────────────────────────────

pub fn print_non_tty_help() {
    use crate::ui::theme::*;
    println!("{}", banner());
    println!("  Interactive mode requires a terminal (TTY).");
    println!("  Use a subcommand instead:\n");
    println!("    {}              Initialize vault", cyan("soul init"));
    println!(
        "    {}              Pull sessions from AI providers",
        cyan("soul pull")
    );
    println!(
        "    {}      Login to cloud provider via OAuth",
        cyan("soul login [provider]")
    );
    println!(
        "    {}     Logout and clear OAuth credentials",
        cyan("soul logout [provider]")
    );
    println!("    {}  Import files", cyan("soul import <folder>"));
    println!(
        "    {}   Watch folder for changes",
        cyan("soul watch <folder>")
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

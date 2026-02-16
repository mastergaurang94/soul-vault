//! TUI application state — tracks current page, focus, sidebar, and vault status.

use crate::vault::config::is_initialized;

// ─── Page Enum ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Status,
    Import,
    Browse,
    Export,
    Watch,
    Reset,
    Settings,
}

impl Page {
    pub const ALL: &[Page] = &[
        Page::Status,
        Page::Import,
        Page::Browse,
        Page::Export,
        Page::Watch,
        Page::Reset,
        Page::Settings,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Page::Status => "Status",
            Page::Import => "Import",
            Page::Browse => "Browse",
            Page::Export => "Export",
            Page::Watch => "Watch",
            Page::Reset => "Reset",
            Page::Settings => "Settings",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Page::Status => "",
            Page::Import => "",
            Page::Browse => "",
            Page::Export => "",
            Page::Watch => "",
            Page::Reset => "",
            Page::Settings => "",
        }
    }
}

// ─── Focus ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Content,
}

// ─── App State ────────────────────────────────────────────────────────────────

pub struct App {
    pub current_page: Page,
    pub sidebar_selected: usize,
    pub focus: Focus,
    pub should_quit: bool,
    pub vault_initialized: bool,
    pub show_help: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_page: Page::Status,
            sidebar_selected: 0,
            focus: Focus::Sidebar,
            should_quit: false,
            vault_initialized: is_initialized(),
            show_help: false,
        }
    }

    pub fn select_page(&mut self, index: usize) {
        if let Some(&page) = Page::ALL.get(index) {
            self.sidebar_selected = index;
            self.current_page = page;
        }
    }

    pub fn sidebar_down(&mut self) {
        if self.sidebar_selected < Page::ALL.len() - 1 {
            self.sidebar_selected += 1;
        } else {
            self.sidebar_selected = 0;
        }
    }

    pub fn sidebar_up(&mut self) {
        if self.sidebar_selected > 0 {
            self.sidebar_selected -= 1;
        } else {
            self.sidebar_selected = Page::ALL.len() - 1;
        }
    }

    pub fn confirm_sidebar(&mut self) {
        self.select_page(self.sidebar_selected);
        self.focus = Focus::Content;
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Sidebar => Focus::Content,
            Focus::Content => Focus::Sidebar,
        };
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_labels() {
        assert_eq!(Page::Status.label(), "Status");
        assert_eq!(Page::Import.label(), "Import");
        assert_eq!(Page::Browse.label(), "Browse");
        assert_eq!(Page::Export.label(), "Export");
        assert_eq!(Page::Watch.label(), "Watch");
        assert_eq!(Page::Reset.label(), "Reset");
        assert_eq!(Page::Settings.label(), "Settings");
    }

    #[test]
    fn test_sidebar_navigation() {
        let mut app = App::new();
        assert_eq!(app.sidebar_selected, 0);
        app.sidebar_down();
        assert_eq!(app.sidebar_selected, 1);
        app.sidebar_up();
        assert_eq!(app.sidebar_selected, 0);
    }

    #[test]
    fn test_sidebar_wraps() {
        let mut app = App::new();
        app.sidebar_up(); // wraps to last
        assert_eq!(app.sidebar_selected, Page::ALL.len() - 1);
        app.sidebar_down(); // wraps to first
        assert_eq!(app.sidebar_selected, 0);
    }

    #[test]
    fn test_confirm_sidebar() {
        let mut app = App::new();
        app.sidebar_selected = 2;
        app.confirm_sidebar();
        assert_eq!(app.current_page, Page::Browse);
        assert_eq!(app.focus, Focus::Content);
    }

    #[test]
    fn test_all_pages_reachable_by_index() {
        let mut app = App::new();
        let expected = [
            Page::Status,
            Page::Import,
            Page::Browse,
            Page::Export,
            Page::Watch,
            Page::Reset,
            Page::Settings,
        ];

        for (i, page) in expected.into_iter().enumerate() {
            app.select_page(i);
            assert_eq!(app.current_page, page);
        }
    }

    #[test]
    fn test_toggle_focus() {
        let mut app = App::new();
        assert_eq!(app.focus, Focus::Sidebar);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::Content);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::Sidebar);
    }
}

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::export::ExportPage;
use super::export_state::ExportField;
use super::PageWidget;
use crate::tui::app::App;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn tui_navigation_wraps() {
    let mut page = ExportPage::default();
    let mut app = App::new();

    page.handle_key(key(KeyCode::Char('k')), &mut app);
    assert_eq!(page.active_field, ExportField::Execute);

    page.handle_key(key(KeyCode::Char('j')), &mut app);
    assert_eq!(page.active_field, ExportField::Format);
}

#[test]
fn tui_space_toggles_sections() {
    let mut page = ExportPage::default();
    let mut app = App::new();

    page.active_field = ExportField::Identity;
    page.handle_key(key(KeyCode::Char(' ')), &mut app);
    assert!(!page.include_identity);
}

#[test]
fn tui_prevents_disabling_last_section() {
    let mut page = ExportPage::default();
    let mut app = App::new();

    page.include_identity = true;
    page.include_preferences = false;
    page.include_topics = false;
    page.include_people = false;
    page.include_memories = false;
    page.active_field = ExportField::Identity;

    page.handle_key(key(KeyCode::Char(' ')), &mut app);
    assert!(page.include_identity);
}

#[test]
fn smart_default_paths_match_format() {
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let context = crate::cli::export::smart_default_output_path("context").unwrap();
    let json = crate::cli::export::smart_default_output_path("json").unwrap();
    let bundle = crate::cli::export::smart_default_output_path("bundle").unwrap();

    assert!(context.ends_with(format!("soul-vault-export-{date}.md")));
    assert!(json.ends_with(format!("soul-vault-export-{date}.json")));
    assert!(bundle.ends_with(format!("soul-vault-export-{date}")));
}

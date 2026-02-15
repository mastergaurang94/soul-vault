//! TUI entry point — alternate screen, event loop, async channel integration.

pub mod app;
pub mod layout;
pub mod pages;
mod runtime;
mod runtime_tasks;
pub mod sidebar;
pub mod watcher;

pub use runtime::run;

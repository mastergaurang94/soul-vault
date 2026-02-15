//! File watcher task — runs notify in a background thread, sends events to TUI.

use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::extractors::local::discover_files;
use crate::tui::pages::watch::{EventKind, WatchEvent};
use crate::vault::sources::{classify_files, update_source_tracking};

const SUPPORTED_EXTENSIONS: &[&str] = &["md", "txt", "json", "jsonl"];

/// Starts a file watcher in a background thread. Returns a handle to stop it.
///
/// Sends `WatchEvent`s over `event_tx`. The `stop_rx` channel is used
/// to signal shutdown from the TUI.
pub fn start_watcher(
    folder: String,
    event_tx: mpsc::Sender<WatchEvent>,
    mut stop_rx: mpsc::Receiver<()>,
) {
    std::thread::spawn(move || {
        let abs_path = match std::fs::canonicalize(&folder) {
            Ok(p) => p,
            Err(e) => {
                let _ = event_tx.blocking_send(WatchEvent {
                    timestamp: now(),
                    message: format!("Failed to resolve path: {}", e),
                    kind: EventKind::Error,
                });
                return;
            }
        };

        let (fs_tx, fs_rx) = std::sync::mpsc::channel();

        let mut debouncer = match new_debouncer(Duration::from_secs(2), fs_tx) {
            Ok(d) => d,
            Err(e) => {
                let _ = event_tx.blocking_send(WatchEvent {
                    timestamp: now(),
                    message: format!("Failed to create watcher: {}", e),
                    kind: EventKind::Error,
                });
                return;
            }
        };

        if let Err(e) = debouncer
            .watcher()
            .watch(&abs_path, RecursiveMode::Recursive)
        {
            let _ = event_tx.blocking_send(WatchEvent {
                timestamp: now(),
                message: format!("Failed to watch path: {}", e),
                kind: EventKind::Error,
            });
            return;
        }

        let _ = event_tx.blocking_send(WatchEvent {
            timestamp: now(),
            message: "Watcher active — waiting for file changes...".into(),
            kind: EventKind::Info,
        });

        loop {
            // Check for stop signal (non-blocking)
            if stop_rx.try_recv().is_ok() {
                let _ = event_tx.blocking_send(WatchEvent {
                    timestamp: now(),
                    message: "Watcher stopped.".into(),
                    kind: EventKind::Info,
                });
                return;
            }

            // Check for file system events (with timeout so we can check stop)
            match fs_rx.recv_timeout(Duration::from_millis(200)) {
                Ok(Ok(events)) => {
                    handle_fs_events(&abs_path, &events, &event_tx);
                }
                Ok(Err(err)) => {
                    let _ = event_tx.blocking_send(WatchEvent {
                        timestamp: now(),
                        message: format!("Watch error: {}", err),
                        kind: EventKind::Error,
                    });
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return;
                }
            }
        }
    });
}

fn handle_fs_events(
    abs_path: &Path,
    events: &[notify_debouncer_mini::DebouncedEvent],
    event_tx: &mpsc::Sender<WatchEvent>,
) {
    let changed_files: HashSet<PathBuf> = events
        .iter()
        .filter_map(|event| {
            let path = &event.path;
            if !path.is_file() {
                return None;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                return None;
            }
            let path_str = path.to_string_lossy();
            if path_str.contains("/.") || path_str.contains("\\.") {
                return None;
            }
            Some(path.clone())
        })
        .collect();

    if changed_files.is_empty() {
        return;
    }

    let _ = event_tx.blocking_send(WatchEvent {
        timestamp: now(),
        message: format!("{} file(s) changed", changed_files.len()),
        kind: EventKind::Info,
    });

    let file_paths: Vec<PathBuf> = changed_files.into_iter().collect();
    match classify_files(abs_path, &file_paths) {
        Ok(classification) => {
            let to_ingest = classification.all_to_ingest();
            if to_ingest.is_empty() {
                let _ = event_tx.blocking_send(WatchEvent {
                    timestamp: now(),
                    message: "All files unchanged — skipped".into(),
                    kind: EventKind::Info,
                });
                return;
            }

            // Build FileInfo for changed files
            let all_files = match discover_files(abs_path) {
                Ok(f) => f,
                Err(e) => {
                    let _ = event_tx.blocking_send(WatchEvent {
                        timestamp: now(),
                        message: format!("Scan error: {}", e),
                        kind: EventKind::Error,
                    });
                    return;
                }
            };

            let files_to_ingest: Vec<_> = all_files
                .into_iter()
                .filter(|f| to_ingest.contains(&f.path))
                .collect();

            if files_to_ingest.is_empty() {
                return;
            }

            // Show which files are being imported
            for f in &files_to_ingest {
                let rel = f.path.strip_prefix(abs_path).unwrap_or(&f.path);
                let _ = event_tx.blocking_send(WatchEvent {
                    timestamp: now(),
                    message: format!("Importing {}", rel.display()),
                    kind: EventKind::Info,
                });
            }

            // Run import synchronously via blocking tokio runtime
            let abs_owned = abs_path.to_owned();
            match run_import_blocking(&abs_owned, &files_to_ingest) {
                Ok(()) => {
                    let _ = event_tx.blocking_send(WatchEvent {
                        timestamp: now(),
                        message: format!("{} file(s) imported successfully", files_to_ingest.len()),
                        kind: EventKind::Success,
                    });
                }
                Err(e) => {
                    let _ = event_tx.blocking_send(WatchEvent {
                        timestamp: now(),
                        message: format!("Import error: {}", e),
                        kind: EventKind::Error,
                    });
                }
            }

            // Update source tracking
            if let Ok(all_files) = discover_files(&abs_owned) {
                let all_paths: Vec<_> = all_files.iter().map(|f| f.path.clone()).collect();
                let _ = update_source_tracking(&abs_owned, &all_paths);
            }
        }
        Err(e) => {
            let _ = event_tx.blocking_send(WatchEvent {
                timestamp: now(),
                message: format!("Classification error: {}", e),
                kind: EventKind::Warning,
            });
        }
    }
}

/// Run the import pipeline in a blocking context (inside the watcher thread).
fn run_import_blocking(base_path: &Path, files: &[crate::types::FileInfo]) -> anyhow::Result<()> {
    // Create a new tokio runtime for this blocking import
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(crate::cli::ingest::run_for_files(base_path, files))
}

fn now() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

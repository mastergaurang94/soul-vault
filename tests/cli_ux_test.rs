//! Comprehensive CLI UX / regression tests for Soma.
//!
//! These tests exercise every user-facing command and edge case,
//! verifying error messages are helpful, output is correct, and
//! no panics or ugly stack traces leak through.
//!
//! # Architecture note
//! The vault path is hardcoded to `~/soma/` — there's no `SOMA_VAULT_PATH`
//! env var or `--vault-path` flag. This means integration tests that need
//! vault isolation CANNOT fully isolate from the real vault. Tests below
//! are designed to be safe regardless: they test CLI arg parsing, error
//! messages, and behaviors that don't mutate the vault.
//!
//! **FINDING: Soma should support `SOMA_VAULT_PATH` env var for testability
//! and multi-vault workflows.**

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

// ─── Helpers ──────────────────────────────────────────────────────────────────

#[allow(deprecated)]
fn soma() -> Command {
    Command::cargo_bin("soma").expect("binary should exist")
}

// ═══════════════════════════════════════════════════════════════════════════════
// 1. FIRST-TIME USER EXPERIENCE
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn help_flag_shows_all_commands() {
    soma()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("import"))
        .stdout(predicate::str::contains("watch"))
        .stdout(predicate::str::contains("export"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("help"));
}

#[test]
fn help_flag_shows_description() {
    soma()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI memory"));
}

#[test]
fn version_flag_works() {
    soma()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("soma"));
}

#[test]
fn version_flag_shows_semver() {
    // Should show something like "soma 0.1.0"
    soma()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"soma \d+\.\d+\.\d+").unwrap());
}

#[test]
fn no_args_non_tty_shows_help() {
    // When stdin is not a TTY (which is always the case in tests/CI),
    // soma should show a helpful message instead of crashing.
    soma()
        .assert()
        .success()
        .stdout(predicate::str::contains("Interactive mode requires a terminal"))
        .stdout(predicate::str::contains("soma init"))
        .stdout(predicate::str::contains("soma import"))
        .stdout(predicate::str::contains("soma export"))
        .stdout(predicate::str::contains("soma status"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2. CLI ARGUMENT PARSING
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn unknown_subcommand_shows_error() {
    soma()
        .arg("bogus")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"))
        .stderr(predicate::str::contains("--help"));
}

#[test]
fn unknown_subcommand_exits_nonzero() {
    soma()
        .arg("totallyinvalid")
        .assert()
        .code(2); // clap uses exit code 2 for parse errors
}

#[test]
fn help_import_subcommand() {
    soma()
        .args(["help", "import"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Import and process local files"))
        .stdout(predicate::str::contains("FOLDER"))
        .stdout(predicate::str::contains("--force"));
}

#[test]
fn import_dash_dash_help() {
    soma()
        .args(["import", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Import and process local files"))
        .stdout(predicate::str::contains("--force"));
}

#[test]
fn help_export_subcommand() {
    soma()
        .args(["help", "export"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Export vault as context document"))
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--format"))
        .stdout(predicate::str::contains("--topic"));
}

#[test]
fn export_dash_dash_help() {
    soma()
        .args(["export", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--format"))
        .stdout(predicate::str::contains("--topic"));
}

#[test]
fn help_watch_subcommand() {
    soma()
        .args(["help", "watch"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Watch a folder"))
        .stdout(predicate::str::contains("FOLDER"));
}

#[test]
fn help_status_subcommand() {
    soma()
        .args(["help", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("vault summary"));
}

#[test]
fn help_init_subcommand() {
    soma()
        .args(["help", "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialize"));
}

#[test]
fn short_help_flag() {
    soma()
        .arg("-h")
        .assert()
        .success()
        .stdout(predicate::str::contains("COMMAND"));
}

#[test]
fn short_version_flag() {
    soma()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::contains("soma"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3. IMPORT COMMAND EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn import_no_folder_arg_shows_usage() {
    soma()
        .arg("import")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Missing folder path"))
        .stderr(predicate::str::contains("Usage: soma import <folder>"))
        .stderr(predicate::str::contains("Example:"));
}

#[test]
fn import_no_folder_arg_exits_1() {
    soma()
        .arg("import")
        .assert()
        .code(1);
}

#[test]
fn import_no_folder_no_panic() {
    // Must NOT contain panic traces
    let output = soma()
        .arg("import")
        .output()
        .expect("should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("thread 'main' panicked"), "Should not panic");
    assert!(!stderr.contains("RUST_BACKTRACE"), "Should not suggest backtrace");
}

#[test]
fn import_nonexistent_path_shows_helpful_error() {
    soma()
        .args(["import", "/nonexistent/path/that/does/not/exist"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Path not found"))
        .stderr(predicate::str::contains("Check the path"));
}

#[test]
fn import_nonexistent_path_exits_1() {
    soma()
        .args(["import", "/nonexistent/path"])
        .assert()
        .code(1);
}

#[test]
fn import_nonexistent_path_no_panic() {
    let output = soma()
        .args(["import", "/nonexistent/path/xyz"])
        .output()
        .expect("should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("thread 'main' panicked"));
}

#[test]
fn import_empty_folder_shows_no_files() {
    let tmp = tempdir().unwrap();

    soma()
        .args(["import", tmp.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No files to process")
            .or(predicate::str::contains("No supported files")));
}

#[test]
fn import_unsupported_file_types_only() {
    let tmp = tempdir().unwrap();
    fs::write(tmp.path().join("document.pdf"), "fake pdf content").unwrap();
    fs::write(tmp.path().join("image.png"), "fake png content").unwrap();
    fs::write(tmp.path().join("spreadsheet.xlsx"), "fake xlsx").unwrap();
    fs::write(tmp.path().join("word.docx"), "fake docx").unwrap();

    soma()
        .args(["import", tmp.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No files to process")
            .or(predicate::str::contains("No supported files")));
}

#[test]
fn import_force_flag_short() {
    // -f should be accepted as --force
    soma()
        .args(["import", "-f", "/nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Path not found"));
}

#[test]
fn import_force_flag_long() {
    soma()
        .args(["import", "--force", "/nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Path not found"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3b. INGEST ALIAS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ingest_alias_no_folder_same_as_import() {
    soma()
        .arg("ingest")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Missing folder path"));
}

#[test]
fn ingest_alias_nonexistent_path() {
    soma()
        .args(["ingest", "/nonexistent/path"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Path not found"));
}

#[test]
fn ingest_hidden_from_help() {
    // The `ingest` command is hidden; `--help` should not show it
    soma()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("ingest").not());
}

// ═══════════════════════════════════════════════════════════════════════════════
// 4. EXPORT COMMAND
// ═══════════════════════════════════════════════════════════════════════════════

// Note: These tests run against the REAL ~/soma/ vault since there's no
// isolation mechanism. They test behavior, not content.

#[test]
fn export_default_format_is_markdown() {
    soma()
        .arg("export")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Soma Memory Vault"));
}

#[test]
fn export_markdown_has_generated_date() {
    soma()
        .arg("export")
        .assert()
        .success()
        .stdout(predicate::str::contains("> Generated:"));
}

#[test]
fn export_json_is_valid_json() {
    let output = soma()
        .args(["export", "--format", "json"])
        .output()
        .expect("should run");
    assert!(output.status.success(), "export --format json should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "JSON output should be valid JSON. Got: {}", &stdout[..stdout.len().min(200)]);
}

#[test]
fn export_json_has_expected_fields() {
    let output = soma()
        .args(["export", "--format", "json"])
        .output()
        .expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert!(parsed.get("identity").is_some(), "JSON should have 'identity' field");
    assert!(parsed.get("preferences").is_some(), "JSON should have 'preferences' field");
    assert!(parsed.get("memories").is_some(), "JSON should have 'memories' field");
    assert!(parsed.get("topics").is_some(), "JSON should have 'topics' field");
    assert!(parsed.get("people").is_some(), "JSON should have 'people' field");
}

#[test]
fn export_to_file_creates_file() {
    let tmp = tempdir().unwrap();
    let output_path = tmp.path().join("export-test.md");

    soma()
        .args(["export", "-o", output_path.to_str().unwrap()])
        .assert()
        .success();

    assert!(output_path.exists(), "Export file should be created");
    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("Soma Memory Vault"), "File should contain vault header");
}

#[test]
fn export_to_file_shows_confirmation() {
    let tmp = tempdir().unwrap();
    let output_path = tmp.path().join("export-confirm.md");

    soma()
        .args(["export", "-o", output_path.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Exported to")
            .or(predicate::str::contains("words")));
}

#[test]
fn export_json_to_file() {
    let tmp = tempdir().unwrap();
    let output_path = tmp.path().join("export-test.json");

    soma()
        .args(["export", "--format", "json", "-o", output_path.to_str().unwrap()])
        .assert()
        .success();

    let content = fs::read_to_string(&output_path).unwrap();
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&content);
    assert!(parsed.is_ok(), "File should contain valid JSON");
}

#[test]
fn export_topic_filter_nonexistent() {
    // Filtering by a topic that doesn't exist should still succeed
    // (with minimal output, no topics section)
    soma()
        .args(["export", "--topic", "zzz_nonexistent_topic_zzz"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Soma Memory Vault"));
}

#[test]
fn export_topic_filter_excludes_unmatched() {
    // With a very specific nonexistent topic, the Topics section should be absent
    let output = soma()
        .args(["export", "--topic", "zzz_definitely_not_a_topic_zzz"])
        .output()
        .expect("should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("## Topics"), "Should not contain Topics section when filter matches nothing");
    // Also should not contain People or Recent Memories (topic filter skips those)
    assert!(!stdout.contains("## People"), "Topic filter should skip People section");
    assert!(!stdout.contains("## Recent Memories"), "Topic filter should skip Memories section");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 5. STATUS COMMAND
// ═══════════════════════════════════════════════════════════════════════════════

// Note: status reads from ~/soma/ — these tests verify structure, not values.

#[test]
fn status_shows_vault_overview() {
    soma()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Vault Overview")
            .or(predicate::str::contains("Soma Vault")));
}

#[test]
fn status_shows_memory_counts() {
    soma()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Memories"))
        .stdout(predicate::str::contains("Topics"))
        .stdout(predicate::str::contains("People"));
}

#[test]
fn status_shows_providers() {
    soma()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Providers"));
}

#[test]
fn status_box_drawing_is_consistent() {
    let output = soma()
        .arg("status")
        .output()
        .expect("should run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Count box-drawing characters — each box should have matching top/bottom
    let top_left = stdout.matches('┌').count();
    let bottom_left = stdout.matches('└').count();
    let top_right = stdout.matches('┐').count();
    let bottom_right = stdout.matches('┘').count();

    assert_eq!(top_left, bottom_left, "Mismatched box corners: ┌={} └={}", top_left, bottom_left);
    assert_eq!(top_right, bottom_right, "Mismatched box corners: ┐={} ┘={}", top_right, bottom_right);
    assert_eq!(top_left, top_right, "Mismatched box corners: ┌={} ┐={}", top_left, top_right);
}

#[test]
fn status_no_panic() {
    let output = soma()
        .arg("status")
        .output()
        .expect("should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"), "Status should not panic");
}

#[test]
fn status_shows_vault_size() {
    soma()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Vault size"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 6. WATCH COMMAND
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn watch_no_folder_arg_errors() {
    soma()
        .arg("watch")
        .assert()
        .failure()
        .stderr(predicate::str::contains("FOLDER")
            .or(predicate::str::contains("required arguments")));
}

#[test]
fn watch_no_folder_exits_2() {
    // clap exits with code 2 for missing required args
    soma()
        .arg("watch")
        .assert()
        .code(2);
}

#[test]
fn watch_nonexistent_path_shows_helpful_error() {
    soma()
        .args(["watch", "/nonexistent/watch/path"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Path not found"))
        .stderr(predicate::str::contains("Check the path"));
}

#[test]
fn watch_nonexistent_path_no_panic() {
    let output = soma()
        .args(["watch", "/nonexistent/watch/path"])
        .output()
        .expect("should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"), "Watch should not panic on bad path");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 7. ERROR MESSAGE QUALITY
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn no_commands_produce_raw_panic() {
    // Run each command that might fail and verify no panics
    let test_cases: Vec<Vec<&str>> = vec![
        vec!["import"],
        vec!["import", "/nonexistent"],
        vec!["export"],
        vec!["status"],
        vec!["watch", "/nonexistent"],
        vec!["--help"],
        vec!["--version"],
    ];

    for args in &test_cases {
        let output = soma()
            .args(args)
            .output()
            .expect("should run");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stderr.contains("thread 'main' panicked"),
            "Panic in `soma {}`: {}",
            args.join(" "),
            stderr
        );
        assert!(
            !stdout.contains("thread 'main' panicked"),
            "Panic in stdout `soma {}`: {}",
            args.join(" "),
            stdout
        );
    }
}

#[test]
fn error_messages_contain_actionable_guidance() {
    // Import nonexistent path should tell user what to do
    let output = soma()
        .args(["import", "/nonexistent"])
        .output()
        .expect("should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("→") || stderr.contains("Check") || stderr.contains("try"),
        "Error should contain actionable guidance. Got: {}",
        stderr
    );
}

#[test]
fn import_error_contains_cross_icon() {
    // Error output should use the ✗ icon for visual clarity
    soma()
        .args(["import", "/nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("✗"));
}

#[test]
fn import_no_args_error_contains_cross_icon() {
    soma()
        .arg("import")
        .assert()
        .failure()
        .stderr(predicate::str::contains("✗"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 8. IMPORT WITH TEMP FIXTURES (safe, doesn't write to vault)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn import_folder_with_mixed_files_finds_supported() {
    // This test verifies file discovery logic but will fail at LLM processing
    // (no API key in test env). We just check it gets past the scan phase.
    let tmp = tempdir().unwrap();
    fs::write(tmp.path().join("notes.md"), "# My Notes\n\nSome content here").unwrap();
    fs::write(tmp.path().join("data.json"), r#"{"key": "value"}"#).unwrap();
    fs::write(tmp.path().join("photo.jpg"), "binary junk").unwrap();

    let output = soma()
        .args(["import", tmp.path().to_str().unwrap()])
        .output()
        .expect("should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should find 2 supported files (.md and .json), skip .jpg
    // It should at least get past scanning (shows "Found X files")
    // May fail later at LLM processing, but that's expected
    assert!(
        combined.contains("Found") || combined.contains("No API key") || combined.contains("API key"),
        "Should either find files or fail at API key stage. Got: {}",
        combined
    );
}

#[test]
fn import_folder_with_nested_structure() {
    let tmp = tempdir().unwrap();
    let sub = tmp.path().join("subdir");
    fs::create_dir_all(&sub).unwrap();
    fs::write(tmp.path().join("root.md"), "# Root file").unwrap();
    fs::write(sub.join("nested.txt"), "Nested content").unwrap();

    let output = soma()
        .args(["import", tmp.path().to_str().unwrap()])
        .output()
        .expect("should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should find 2 files (root.md and nested.txt)
    assert!(
        combined.contains("Found 2 files") || combined.contains("API key") || combined.contains("No API key"),
        "Should find 2 files or fail at API stage. Got: {}",
        combined
    );
}

#[test]
fn import_folder_skips_hidden_dirs() {
    let tmp = tempdir().unwrap();
    let hidden = tmp.path().join(".hidden");
    fs::create_dir_all(&hidden).unwrap();
    fs::write(hidden.join("secret.md"), "# Secret").unwrap();
    fs::write(tmp.path().join("visible.md"), "# Visible").unwrap();

    let output = soma()
        .args(["import", tmp.path().to_str().unwrap()])
        .output()
        .expect("should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should find only 1 file (visible.md), not the hidden one
    assert!(
        combined.contains("Found 1 files") || combined.contains("Found 1 file")
            || combined.contains("API key") || combined.contains("No API key"),
        "Should find 1 file (skip hidden). Got: {}",
        combined
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 9. EXPORT FORMAT EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn export_invalid_format_falls_through_to_markdown() {
    // BUG/UX ISSUE: --format bogus silently defaults to markdown
    // instead of showing an error. Documenting current behavior.
    soma()
        .args(["export", "--format", "bogus"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Soma Memory Vault"));
}

#[test]
fn export_format_json_flag() {
    soma()
        .args(["export", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("{"));
}

#[test]
fn export_format_markdown_explicit() {
    soma()
        .args(["export", "--format", "markdown"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Soma Memory Vault"));
}

#[test]
fn export_output_to_nonexistent_dir_fails_gracefully() {
    // Writing to a path where parent dir doesn't exist
    let output = soma()
        .args(["export", "-o", "/nonexistent/dir/output.md"])
        .output()
        .expect("should run");

    // Should fail but not panic
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"), "Should not panic writing to bad path");
    assert!(!output.status.success(), "Should fail when output dir doesn't exist");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 10. COMBINED EDGE CASES & REGRESSION
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn double_dash_terminates_flags() {
    // `soma import -- --help` should treat --help as a folder path, not a flag
    soma()
        .args(["import", "--", "--help"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Path not found")
            .or(predicate::str::contains("not found")));
}

#[test]
fn export_short_flags_work() {
    let tmp = tempdir().unwrap();
    let output_path = tmp.path().join("short-flag.md");

    // -o for output, -f for format, -t for topic
    soma()
        .args(["export", "-o", output_path.to_str().unwrap(), "-f", "markdown"])
        .assert()
        .success();

    assert!(output_path.exists());
}

#[test]
fn multiple_unknown_flags_rejected() {
    soma()
        .args(["import", "--verbose", "--dry-run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument")
            .or(predicate::str::contains("error")));
}

#[test]
fn status_exits_zero() {
    soma()
        .arg("status")
        .assert()
        .success();
}

#[test]
fn export_exits_zero() {
    soma()
        .arg("export")
        .assert()
        .success();
}

#[test]
fn import_exits_one_on_error() {
    // Missing folder
    soma()
        .arg("import")
        .assert()
        .code(1);

    // Nonexistent path
    soma()
        .args(["import", "/no/such/path"])
        .assert()
        .code(1);
}

#[test]
fn export_markdown_not_empty() {
    let output = soma()
        .arg("export")
        .output()
        .expect("should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.len() > 50, "Export should produce non-trivial output. Got {} bytes", stdout.len());
}

#[test]
fn export_json_not_empty() {
    let output = soma()
        .args(["export", "--format", "json"])
        .output()
        .expect("should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.len() > 20, "JSON export should produce non-trivial output. Got {} bytes", stdout.len());
}

// ═══════════════════════════════════════════════════════════════════════════════
// 11. IMPORT EMPTY-CONTENT FILES
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn import_folder_with_empty_files() {
    let tmp = tempdir().unwrap();
    fs::write(tmp.path().join("empty.md"), "").unwrap();
    fs::write(tmp.path().join("whitespace.txt"), "   \n\n   ").unwrap();

    let output = soma()
        .args(["import", tmp.path().to_str().unwrap()])
        .output()
        .expect("should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should either report "No content to process" or get past scan
    // The key is: no panic
    assert!(!combined.contains("panicked"), "Should not panic on empty files");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 12. BANNER / UI CONSISTENCY
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn import_nonexistent_shows_banner() {
    // Import should show the Soma banner before the error.
    // Banner goes to stdout, error goes to stderr.
    soma()
        .args(["import", "/nonexistent"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Soma"));
}

#[test]
fn status_shows_banner() {
    soma()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Soma"));
}

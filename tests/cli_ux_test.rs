//! Comprehensive CLI UX / regression tests for Soul Vault.
//!
//! These tests exercise every user-facing command and edge case,
//! verifying error messages are helpful, output is correct, and
//! no panics or ugly stack traces leak through.
//!
//! # Architecture note
//! The vault path is hardcoded to `~/soul-vault/` — there's no `SOUL_VAULT_PATH`
//! env var or `--vault-path` flag. This means integration tests that need
//! vault isolation CANNOT fully isolate from the real vault. Tests below
//! are designed to be safe regardless: they test CLI arg parsing, error
//! messages, and behaviors that don't mutate the vault.
//!
//! **FINDING: Soul Vault should support `SOUL_VAULT_PATH` env var for testability
//! and multi-vault workflows.**

use assert_cmd::Command;
use predicates::prelude::*;
use regex::Regex;
use std::fs;
use tempfile::tempdir;
use unicode_width::UnicodeWidthStr;

// ─── Helpers ──────────────────────────────────────────────────────────────────

#[allow(deprecated)]
fn soul() -> Command {
    Command::cargo_bin("soul").expect("binary should exist")
}

// ═══════════════════════════════════════════════════════════════════════════════
// 1. FIRST-TIME USER EXPERIENCE
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn help_flag_shows_all_commands() {
    soul()
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
    soul()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI memory"));
}

#[test]
fn version_flag_works() {
    soul()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("soul"));
}

#[test]
fn version_flag_shows_semver() {
    // Should show something like "soul 0.1.0"
    soul()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"soul \d+\.\d+\.\d+").unwrap());
}

#[test]
fn no_args_non_tty_shows_help() {
    // When stdin is not a TTY (which is always the case in tests/CI),
    // soul should show a helpful message instead of crashing.
    soul()
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Interactive mode requires a terminal",
        ))
        .stdout(predicate::str::contains("soul init"))
        .stdout(predicate::str::contains("soul import"))
        .stdout(predicate::str::contains("soul export"))
        .stdout(predicate::str::contains("soul status"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2. CLI ARGUMENT PARSING
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn unknown_subcommand_shows_error() {
    soul()
        .arg("bogus")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"))
        .stderr(predicate::str::contains("--help"));
}

#[test]
fn unknown_subcommand_exits_nonzero() {
    soul().arg("totallyinvalid").assert().code(2); // clap uses exit code 2 for parse errors
}

#[test]
fn help_import_subcommand() {
    soul()
        .args(["help", "import"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Import sessions from AI providers"))
        .stdout(predicate::str::contains("FOLDER"))
        .stdout(predicate::str::contains("--force"));
}

#[test]
fn import_dash_dash_help() {
    soul()
        .args(["import", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Import sessions from AI providers"))
        .stdout(predicate::str::contains("--force"));
}

#[test]
fn help_export_subcommand() {
    soul()
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
    soul()
        .args(["export", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--format"))
        .stdout(predicate::str::contains("--topic"));
}

#[test]
fn help_watch_subcommand() {
    soul()
        .args(["help", "watch"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Watch a folder"))
        .stdout(predicate::str::contains("FOLDER"))
        .stdout(predicate::str::contains("Path to folder to watch"));
}

#[test]
fn help_status_subcommand() {
    soul()
        .args(["help", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("vault summary"));
}

#[test]
fn help_init_subcommand() {
    soul()
        .args(["help", "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialize"));
}

#[test]
fn short_help_flag() {
    soul()
        .arg("-h")
        .assert()
        .success()
        .stdout(predicate::str::contains("COMMAND"));
}

#[test]
fn short_version_flag() {
    soul()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::contains("soul"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3. IMPORT COMMAND EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn import_no_folder_arg_shows_usage() {
    let tmp = tempdir().unwrap();
    soul()
        .env("HOME", tmp.path().to_str().unwrap())
        .arg("import")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Run `soul init`"))
        .stderr(predicate::str::contains("Missing folder path").not())
        .stderr(predicate::str::contains("Usage: soul import <folder>").not());
}

#[test]
fn import_no_folder_arg_exits_1() {
    let tmp = tempdir().unwrap();
    soul()
        .env("HOME", tmp.path().to_str().unwrap())
        .arg("import")
        .assert()
        .code(1);
}

#[test]
fn import_no_folder_no_panic() {
    // Must NOT contain panic traces
    let tmp = tempdir().unwrap();
    let output = soul()
        .env("HOME", tmp.path().to_str().unwrap())
        .arg("import")
        .output()
        .expect("should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("thread 'main' panicked"),
        "Should not panic"
    );
    assert!(
        !stderr.contains("RUST_BACKTRACE"),
        "Should not suggest backtrace"
    );
}

#[test]
fn import_nonexistent_path_shows_helpful_error() {
    soul()
        .args(["import", "/nonexistent/path/that/does/not/exist"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Path not found"))
        .stderr(predicate::str::contains("Check the path"));
}

#[test]
fn import_nonexistent_path_exits_1() {
    soul()
        .args(["import", "/nonexistent/path"])
        .assert()
        .code(1);
}

#[test]
fn import_nonexistent_path_no_panic() {
    let output = soul()
        .args(["import", "/nonexistent/path/xyz"])
        .output()
        .expect("should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("thread 'main' panicked"));
}

#[test]
fn import_empty_folder_shows_no_files() {
    let tmp = tempdir().unwrap();

    soul()
        .args(["import", tmp.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("No files to process")
                .or(predicate::str::contains("No supported files")),
        );
}

#[test]
fn import_unsupported_file_types_only() {
    let tmp = tempdir().unwrap();
    fs::write(tmp.path().join("document.pdf"), "fake pdf content").unwrap();
    fs::write(tmp.path().join("image.png"), "fake png content").unwrap();
    fs::write(tmp.path().join("spreadsheet.xlsx"), "fake xlsx").unwrap();
    fs::write(tmp.path().join("word.docx"), "fake docx").unwrap();

    soul()
        .args(["import", tmp.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("No files to process")
                .or(predicate::str::contains("No supported files")),
        );
}

#[test]
fn import_force_flag_short() {
    // -f should be accepted as --force
    soul()
        .args(["import", "-f", "/nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Path not found"));
}

#[test]
fn import_force_flag_long() {
    soul()
        .args(["import", "--force", "/nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Path not found"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3b. REMOVED COMMANDS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ingest_command_is_rejected() {
    soul()
        .arg("ingest")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn pull_command_is_rejected() {
    soul()
        .arg("pull")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 4. EXPORT COMMAND
// ═══════════════════════════════════════════════════════════════════════════════

// Note: These tests run against the REAL ~/soul-vault/ vault since there's no
// isolation mechanism. They test behavior, not content.

#[test]
fn export_default_format_is_markdown() {
    soul()
        .arg("export")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Soul Vault Memory"));
}

#[test]
fn export_markdown_has_generated_date() {
    soul()
        .arg("export")
        .assert()
        .success()
        .stdout(predicate::str::contains("> Generated:"));
}

#[test]
fn export_json_is_valid_json() {
    let output = soul()
        .args(["export", "--format", "json"])
        .output()
        .expect("should run");
    assert!(
        output.status.success(),
        "export --format json should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(
        parsed.is_ok(),
        "JSON output should be valid JSON. Got: {}",
        &stdout[..stdout.len().min(200)]
    );
}

#[test]
fn export_json_has_expected_fields() {
    let output = soul()
        .args(["export", "--format", "json"])
        .output()
        .expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert!(
        parsed.get("identity").is_some(),
        "JSON should have 'identity' field"
    );
    assert!(
        parsed.get("preferences").is_some(),
        "JSON should have 'preferences' field"
    );
    assert!(
        parsed.get("memories").is_some(),
        "JSON should have 'memories' field"
    );
    assert!(
        parsed.get("topics").is_some(),
        "JSON should have 'topics' field"
    );
    assert!(
        parsed.get("people").is_some(),
        "JSON should have 'people' field"
    );
}

#[test]
fn export_to_file_creates_file() {
    let tmp = tempdir().unwrap();
    let output_path = tmp.path().join("export-test.md");

    soul()
        .args(["export", "-o", output_path.to_str().unwrap()])
        .assert()
        .success();

    assert!(output_path.exists(), "Export file should be created");
    let content = fs::read_to_string(&output_path).unwrap();
    assert!(
        content.contains("Soul Vault Memory"),
        "File should contain vault header"
    );
}

#[test]
fn export_to_file_shows_confirmation() {
    let tmp = tempdir().unwrap();
    let output_path = tmp.path().join("export-confirm.md");

    soul()
        .args(["export", "-o", output_path.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Exported to").or(predicate::str::contains("words")));
}

#[test]
fn export_json_to_file() {
    let tmp = tempdir().unwrap();
    let output_path = tmp.path().join("export-test.json");

    soul()
        .args([
            "export",
            "--format",
            "json",
            "-o",
            output_path.to_str().unwrap(),
        ])
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
    soul()
        .args(["export", "--topic", "zzz_nonexistent_topic_zzz"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Soul Vault Memory"));
}

#[test]
fn export_topic_filter_excludes_unmatched() {
    // With a very specific nonexistent topic, the Topics section should be absent
    let output = soul()
        .args(["export", "--topic", "zzz_definitely_not_a_topic_zzz"])
        .output()
        .expect("should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("## Topics"),
        "Should not contain Topics section when filter matches nothing"
    );
    // Also should not contain People or Recent Memories (topic filter skips those)
    assert!(
        !stdout.contains("## People"),
        "Topic filter should skip People section"
    );
    assert!(
        !stdout.contains("## Recent Memories"),
        "Topic filter should skip Memories section"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 5. STATUS COMMAND
// ═══════════════════════════════════════════════════════════════════════════════

// Note: status reads from ~/soul-vault/ — these tests verify structure, not values.

#[test]
fn status_shows_vault_overview() {
    soul().arg("status").assert().success().stdout(
        predicate::str::contains("Vault Overview").or(predicate::str::contains("Soul Vault")),
    );
}

#[test]
fn status_shows_memory_counts() {
    soul()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Memories"))
        .stdout(predicate::str::contains("Topics"))
        .stdout(predicate::str::contains("People"));
}

#[test]
fn status_shows_providers() {
    soul()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Providers"));
}

#[test]
fn status_box_drawing_is_consistent() {
    let output = soul().arg("status").output().expect("should run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Count box-drawing characters — each box should have matching top/bottom
    let top_left = stdout.matches('┌').count();
    let bottom_left = stdout.matches('└').count();
    let top_right = stdout.matches('┐').count();
    let bottom_right = stdout.matches('┘').count();

    assert_eq!(
        top_left, bottom_left,
        "Mismatched box corners: ┌={} └={}",
        top_left, bottom_left
    );
    assert_eq!(
        top_right, bottom_right,
        "Mismatched box corners: ┐={} ┘={}",
        top_right, bottom_right
    );
    assert_eq!(
        top_left, top_right,
        "Mismatched box corners: ┌={} ┐={}",
        top_left, top_right
    );
}

#[test]
fn status_no_panic() {
    let output = soul().arg("status").output().expect("should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"), "Status should not panic");
}

#[test]
fn status_shows_vault_size() {
    soul()
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
    // No-args watch in non-TTY mode exits with helpful error
    soul()
        .arg("watch")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Auto-watch requires a terminal"))
        .stderr(predicate::str::contains("Usage: soul watch <folder>"));
}

#[test]
fn watch_no_folder_exits_1() {
    soul().arg("watch").assert().code(1);
}

#[test]
fn watch_nonexistent_path_shows_helpful_error() {
    soul()
        .args(["watch", "/nonexistent/watch/path"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Path not found"))
        .stderr(predicate::str::contains("Check the path"));
}

#[test]
fn watch_nonexistent_path_no_panic() {
    let output = soul()
        .args(["watch", "/nonexistent/watch/path"])
        .output()
        .expect("should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked"),
        "Watch should not panic on bad path"
    );
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
        let output = soul().args(args).output().expect("should run");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stderr.contains("thread 'main' panicked"),
            "Panic in `soul {}`: {}",
            args.join(" "),
            stderr
        );
        assert!(
            !stdout.contains("thread 'main' panicked"),
            "Panic in stdout `soul {}`: {}",
            args.join(" "),
            stdout
        );
    }
}

#[test]
fn error_messages_contain_actionable_guidance() {
    // Import nonexistent path should tell user what to do
    let output = soul()
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
    soul()
        .args(["import", "/nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("✗"));
}

#[test]
fn import_no_args_error_contains_cross_icon() {
    let tmp = tempdir().unwrap();
    soul()
        .env("HOME", tmp.path().to_str().unwrap())
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
    fs::write(
        tmp.path().join("notes.md"),
        "# My Notes\n\nSome content here",
    )
    .unwrap();
    fs::write(tmp.path().join("data.json"), r#"{"key": "value"}"#).unwrap();
    fs::write(tmp.path().join("photo.jpg"), "binary junk").unwrap();

    let output = soul()
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
        combined.contains("Found")
            || combined.contains("No API key")
            || combined.contains("API key"),
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

    let output = soul()
        .args(["import", tmp.path().to_str().unwrap()])
        .output()
        .expect("should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should find 2 files (root.md and nested.txt)
    assert!(
        combined.contains("Found 2 files")
            || combined.contains("API key")
            || combined.contains("No API key"),
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

    let output = soul()
        .args(["import", tmp.path().to_str().unwrap()])
        .output()
        .expect("should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should find only 1 file (visible.md), not the hidden one
    assert!(
        combined.contains("Found 1 files")
            || combined.contains("Found 1 file")
            || combined.contains("API key")
            || combined.contains("No API key"),
        "Should find 1 file (skip hidden). Got: {}",
        combined
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 9. EXPORT FORMAT EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn export_invalid_format_shows_helpful_error() {
    soul()
        .args(["export", "--format", "bogus"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported export format"))
        .stderr(predicate::str::contains("context, json, bundle"));
}

#[test]
fn export_format_json_flag() {
    soul()
        .args(["export", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("{"));
}

#[test]
fn export_format_markdown_explicit() {
    soul()
        .args(["export", "--format", "markdown"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Soul Vault Memory"));
}

#[test]
fn export_output_to_nonexistent_dir_fails_gracefully() {
    // Writing to a path where parent dir doesn't exist
    let output = soul()
        .args(["export", "-o", "/nonexistent/dir/output.md"])
        .output()
        .expect("should run");

    // Should fail but not panic
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked"),
        "Should not panic writing to bad path"
    );
    assert!(
        !output.status.success(),
        "Should fail when output dir doesn't exist"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 10. COMBINED EDGE CASES & REGRESSION
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn double_dash_terminates_flags() {
    // `soul import -- --help` should treat --help as a folder path, not a flag
    soul()
        .args(["import", "--", "--help"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Path not found").or(predicate::str::contains("not found")),
        );
}

#[test]
fn export_short_flags_work() {
    let tmp = tempdir().unwrap();
    let output_path = tmp.path().join("short-flag.md");

    // -o for output, -f for format, -t for topic
    soul()
        .args([
            "export",
            "-o",
            output_path.to_str().unwrap(),
            "-f",
            "markdown",
        ])
        .assert()
        .success();

    assert!(output_path.exists());
}

#[test]
fn multiple_unknown_flags_rejected() {
    soul()
        .args(["import", "--verbose", "--dry-run"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("unexpected argument").or(predicate::str::contains("error")),
        );
}

#[test]
fn status_exits_zero() {
    soul().arg("status").assert().success();
}

#[test]
fn export_exits_zero() {
    soul().arg("export").assert().success();
}

#[test]
fn import_exits_one_on_error() {
    // Nonexistent path
    soul().args(["import", "/no/such/path"]).assert().code(1);
}

#[test]
fn export_markdown_not_empty() {
    let output = soul().arg("export").output().expect("should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.len() > 50,
        "Export should produce non-trivial output. Got {} bytes",
        stdout.len()
    );
}

#[test]
fn export_json_not_empty() {
    let output = soul()
        .args(["export", "--format", "json"])
        .output()
        .expect("should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.len() > 20,
        "JSON export should produce non-trivial output. Got {} bytes",
        stdout.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 11. IMPORT EMPTY-CONTENT FILES
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn import_folder_with_empty_files() {
    let tmp = tempdir().unwrap();
    fs::write(tmp.path().join("empty.md"), "").unwrap();
    fs::write(tmp.path().join("whitespace.txt"), "   \n\n   ").unwrap();

    let output = soul()
        .args(["import", tmp.path().to_str().unwrap()])
        .output()
        .expect("should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should either report "No content to process" or get past scan
    // The key is: no panic
    assert!(
        !combined.contains("panicked"),
        "Should not panic on empty files"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 12. BANNER / UI CONSISTENCY
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn import_nonexistent_shows_banner() {
    // Import should show the Soul Vault banner before the error.
    // Banner goes to stdout, error goes to stderr.
    soul()
        .args(["import", "/nonexistent"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Soul Vault"));
}

#[test]
fn status_shows_banner() {
    soul()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Soul Vault"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 13. FORMATTING HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Strip all ANSI escape sequences from a string.
/// Handles CSI sequences (\x1b[...m), OSC sequences, and other common escapes.
fn strip_ansi(s: &str) -> String {
    let re = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]|\x1b\].*?\x07|\x1b\[.*?m").unwrap();
    re.replace_all(s, "").to_string()
}

/// Compute the visible (display) width of a string, accounting for Unicode
/// wide characters (CJK, emoji) and multi-byte chars like box-drawing.
fn visible_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Verify that all box-drawing lines in the output are consistently aligned.
///
/// Checks:
/// 1. All border lines (┌, └, ├) have the same total visible width
/// 2. All content lines (│...│) have exactly 2 │ per line
/// 3. The right │ aligns with the right edge of the border lines
/// 4. No trailing content after the closing │ (except whitespace)
fn verify_box_alignment(output: &str) {
    let lines: Vec<&str> = output.lines().collect();
    let mut expected_box_width: Option<usize> = None;
    let mut content_line_count = 0;

    for line in &lines {
        let stripped = strip_ansi(line);

        // Check border lines: ┌───┐, ├───┤, └───┘
        if stripped.contains('┌') || stripped.contains('└') || stripped.contains('├') {
            // Measure from the first box char to the end of the line (trimmed)
            let trimmed = stripped.trim_start();
            let width = visible_width(trimmed);
            if let Some(expected) = expected_box_width {
                assert_eq!(
                    width, expected,
                    "Box border width mismatch. Expected {} but got {} on line: '{}'",
                    expected, width, stripped
                );
            } else {
                expected_box_width = Some(width);
            }
        }

        // Check content lines: │...│
        if stripped.contains('│') {
            let positions: Vec<usize> = stripped
                .char_indices()
                .filter(|(_, c)| *c == '│')
                .map(|(i, _)| i)
                .collect();

            assert_eq!(
                positions.len(),
                2,
                "Expected exactly 2 box borders (│) on line, found {}. Line: '{}'",
                positions.len(),
                stripped
            );

            // The visible width from start to closing │ (inclusive) should be consistent
            // with the border width
            if let Some(expected_width) = expected_box_width {
                let trimmed = stripped.trim_start();
                let _leading_spaces = stripped.len() - trimmed.len();
                // The visible width of the trimmed line should match the border
                let line_width = visible_width(trimmed);
                assert_eq!(
                    line_width, expected_width,
                    "Content line visible width ({}) doesn't match border width ({}) on line: '{}'",
                    line_width, expected_width, stripped
                );
            }

            content_line_count += 1;
        }
    }

    assert!(
        expected_box_width.is_some(),
        "No box borders found in output"
    );
    assert!(
        content_line_count > 0,
        "No content lines (│...│) found in output"
    );
}

/// Extract all box sections from status output as separate strings.
/// A box section starts with a ┌ line and ends with a └ line.
fn extract_box_sections(output: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let mut current_section: Option<Vec<&str>> = None;

    for line in output.lines() {
        let stripped = strip_ansi(line);
        if stripped.contains('┌') {
            current_section = Some(vec![line]);
        } else if let Some(ref mut section) = current_section {
            section.push(line);
            if stripped.contains('└') {
                sections.push(section.join("\n"));
                current_section = None;
            }
        }
    }

    sections
}

// ═══════════════════════════════════════════════════════════════════════════════
// 14. BOX-DRAWING ALIGNMENT TESTS (formatting correctness)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn status_vault_overview_box_alignment() {
    let output = soul().arg("status").output().expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let sections = extract_box_sections(&stdout);
    assert!(
        !sections.is_empty(),
        "Status output should contain at least one box section"
    );

    // Verify the first box (Vault Overview)
    verify_box_alignment(&sections[0]);
}

#[test]
fn status_providers_box_alignment() {
    let output = soul().arg("status").output().expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let sections = extract_box_sections(&stdout);
    assert!(
        sections.len() >= 2,
        "Status output should contain at least 2 box sections (Vault Overview + Providers)"
    );

    // Verify the second box (Providers)
    verify_box_alignment(&sections[1]);
}

#[test]
fn status_all_boxes_aligned() {
    let output = soul().arg("status").output().expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let sections = extract_box_sections(&stdout);
    for (i, section) in sections.iter().enumerate() {
        verify_box_alignment(section);

        // Additionally verify all boxes use the same width
        let first_line = section.lines().next().map(strip_ansi).unwrap_or_default();
        let first_width = visible_width(first_line.trim_start());
        if i > 0 {
            let prev_first_line = sections[i - 1]
                .lines()
                .next()
                .map(strip_ansi)
                .unwrap_or_default();
            let prev_width = visible_width(prev_first_line.trim_start());
            assert_eq!(
                first_width, prev_width,
                "Box {} has width {} but box {} has width {}. All boxes should have the same width.",
                i, first_width, i - 1, prev_width
            );
        }
    }
}

#[test]
fn status_top_bottom_borders_same_length() {
    let output = soul().arg("status").output().expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let stripped = strip_ansi(line);
        // Match top borders
        if stripped.contains('┌') && stripped.contains('┐') {
            let _top_width = visible_width(stripped.trim_start());

            // Find matching bottom border (next └ line in the output)
            // We verified this in verify_box_alignment, but double-check
            // that ┌ and └ lines have equal dash counts
            let dash_count: usize = stripped.matches('─').count();
            assert!(
                dash_count > 0,
                "Border line should contain ─ characters: '{}'",
                stripped
            );
            // ┌ + N×─ + ┐ = total
            // The visible width should be dash_count + 2 (for the corner chars)
            let computed = dash_count + 2; // ┌ and ┐
            assert_eq!(
                visible_width(stripped.trim_start()),
                computed,
                "Top border visible width mismatch on: '{}'",
                stripped
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 15. STAT ROW SPACING TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn status_stat_rows_have_space_after_colon() {
    let output = soul().arg("status").output().expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let stat_labels = [
        "Memories:",
        "Topics:",
        "People:",
        "Vault size:",
        "Total files:",
        "Last activity:",
    ];

    for label in &stat_labels {
        let stripped_output = strip_ansi(&stdout);
        if stripped_output.contains(label) {
            // Find the line containing this label
            for line in stripped_output.lines() {
                if line.contains(label) {
                    // After the colon, there should be at least one space
                    let after_colon = line.split_once(label).unwrap().1;
                    assert!(
                        after_colon.starts_with(' '),
                        "Missing space after '{}' on line: '{}'",
                        label,
                        line.trim()
                    );
                }
            }
        }
    }
}

#[test]
fn status_stat_values_aligned_at_same_column() {
    let output = soul().arg("status").output().expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stripped = strip_ansi(&stdout);

    // Collect the column positions where stat values start
    let stat_labels = [
        "Memories:",
        "Topics:",
        "People:",
        "Vault size:",
        "Total files:",
        "Last activity:",
    ];
    let mut value_columns: Vec<(String, usize)> = Vec::new();

    for line in stripped.lines() {
        for label in &stat_labels {
            if line.contains(label) {
                // Find where the value starts (first non-space after label padding)
                if let Some((_before, after)) = line.split_once(label) {
                    let spaces_before_value = after.len() - after.trim_start().len();
                    let label_pos = line.find(label).unwrap();
                    let value_col = label_pos + label.len() + spaces_before_value;
                    value_columns.push((label.to_string(), value_col));
                }
            }
        }
    }

    // All stat values should start at the same column
    if value_columns.len() > 1 {
        let expected_col = value_columns[0].1;
        for (label, col) in &value_columns {
            assert_eq!(
                *col, expected_col,
                "Stat value for '{}' starts at column {} but expected column {} (misaligned). \
                 All stat values should be vertically aligned.",
                label, col, expected_col
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 16. PROVIDER LINE SPACING TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn status_provider_names_consistently_padded() {
    let output = soul().arg("status").output().expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let sections = extract_box_sections(&stdout);
    // Find the Providers section
    let providers_section = sections.iter().find(|s| {
        let s_stripped = strip_ansi(s);
        s_stripped.contains("Providers")
    });

    if let Some(section) = providers_section {
        let section_stripped = strip_ansi(section);
        // Provider lines contain status indicators (✓ or ○) followed by name
        let provider_names = ["Claude", "ChatGPT", "Gemini"];
        let mut status_text_columns: Vec<(String, usize)> = Vec::new();

        for line in section_stripped.lines() {
            for name in &provider_names {
                if line.contains(name) {
                    // Find where the status text starts (after the padded provider name)
                    if let Some(name_pos) = line.find(name) {
                        // The status text starts after name + padding
                        let after_name = &line[name_pos + name.len()..];
                        let status_start = after_name.len() - after_name.trim_start().len();
                        let status_col = name_pos + name.len() + status_start;
                        status_text_columns.push((name.to_string(), status_col));
                    }
                }
            }
        }

        // All provider status texts should start at the same column
        if status_text_columns.len() > 1 {
            let expected_col = status_text_columns[0].1;
            for (name, col) in &status_text_columns {
                assert_eq!(
                    *col, expected_col,
                    "Provider '{}' status starts at column {} but expected column {}. \
                     Provider status text should be vertically aligned.",
                    name, col, expected_col
                );
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 17. ANSI COLOR CODE HANDLING TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn status_ansi_codes_dont_inflate_box_width() {
    // The raw output (with ANSI codes) should have the same box structure
    // as the stripped output. If ANSI codes are used in width calculations,
    // the boxes will be misaligned.
    let output = soul().arg("status").output().expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse raw output and verify each content line has │ at consistent
    // *display column* positions after stripping ANSI codes.
    // We use display width (not byte offset) because multi-byte UTF-8 chars
    // like ✓ (3 bytes) and ○ (3 bytes) have display width of 1.
    let sections = extract_box_sections(&stdout);
    for section in &sections {
        let mut right_border_columns: Vec<usize> = Vec::new();

        for line in section.lines() {
            let stripped = strip_ansi(line);
            if stripped.contains('│') {
                // Compute display column of the right │
                // Walk through chars, accumulating display width
                let mut col = 0;
                let mut right_col = 0;
                let mut border_count = 0;
                for ch in stripped.chars() {
                    if ch == '│' {
                        border_count += 1;
                        if border_count == 2 {
                            right_col = col;
                        }
                    }
                    col += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                }

                if border_count == 2 {
                    right_border_columns.push(right_col);
                }
            }
        }

        // All right border │ should be at the same display column
        if right_border_columns.len() > 1 {
            let expected = right_border_columns[0];
            for (i, col) in right_border_columns.iter().enumerate() {
                assert_eq!(
                    *col, expected,
                    "Right border │ at inconsistent display column on content line {}. \
                     Expected column {} but got {}. ANSI codes may be inflating width.",
                    i, expected, col
                );
            }
        }
    }
}

#[test]
fn strip_ansi_helper_works() {
    // Verify our strip_ansi function handles common cases
    assert_eq!(strip_ansi("hello"), "hello");
    assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
    assert_eq!(
        strip_ansi("\x1b[1;38;2;245;166;35mbold gold\x1b[0m"),
        "bold gold"
    );
    assert_eq!(strip_ansi("no codes here"), "no codes here");
    assert_eq!(strip_ansi("│\x1b[1mtext\x1b[0m│"), "│text│");

    // Box-drawing characters should survive stripping
    assert_eq!(strip_ansi("┌──┐"), "┌──┐");
    assert_eq!(strip_ansi("└──┘"), "└──┘");
    assert_eq!(strip_ansi("│  │"), "│  │");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 18. EMOJI / UNICODE WIDTH REGRESSION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn verify_box_alignment_catches_misaligned_box() {
    // This is a unit test for the helper itself — verify it correctly
    // catches boxes with misaligned content lines.

    // Well-formed box should pass
    let good_box = "\
  ┌──────────┐\n\
  │  hello   │\n\
  │  world   │\n\
  └──────────┘";
    verify_box_alignment(good_box); // should not panic

    // Box with wrong-width content line should fail
    let bad_box = "\
  ┌──────────┐\n\
  │  hello  │\n\
  │  world   │\n\
  └──────────┘";
    let result = std::panic::catch_unwind(|| {
        verify_box_alignment(bad_box);
    });
    assert!(
        result.is_err(),
        "verify_box_alignment should catch misaligned content lines"
    );
}

#[test]
fn verify_box_alignment_catches_mismatched_borders() {
    // Top and bottom borders with different widths
    let bad_borders = "\
  ┌──────────┐\n\
  │  hello   │\n\
  └────────────┘";
    let result = std::panic::catch_unwind(|| {
        verify_box_alignment(bad_borders);
    });
    assert!(
        result.is_err(),
        "verify_box_alignment should catch mismatched border widths"
    );
}

#[test]
fn status_no_emoji_in_box_headers() {
    // Emoji in box headers cause terminal width inconsistencies because
    // different terminals render emoji at different widths (1 or 2 cells).
    // The fix was to remove emoji from box headers entirely.
    let output = soul().arg("status").output().expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let sections = extract_box_sections(&stdout);
    for section in &sections {
        for line in section.lines() {
            let stripped = strip_ansi(line);
            // Header lines are │ lines that contain a title (like "Vault Overview")
            if stripped.contains('│') && !stripped.contains(':') {
                // Check for common emoji that caused the original bug
                let emoji_chars = ['🧠', '📁', '🔑', '📊', '🌐', '⚡', '🔧'];
                for emoji in &emoji_chars {
                    assert!(
                        !stripped.contains(*emoji),
                        "Box header contains emoji '{}' which causes width misalignment. Line: '{}'",
                        emoji,
                        stripped
                    );
                }
            }
        }
    }
}

#[test]
fn status_box_content_no_trailing_text_after_border() {
    // After the closing │, there should be nothing but optional whitespace
    let output = soul().arg("status").output().expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let stripped = strip_ansi(line);
        if stripped.contains('│') {
            let positions: Vec<usize> = stripped
                .char_indices()
                .filter(|(_, c)| *c == '│')
                .map(|(i, _)| i)
                .collect();

            if positions.len() == 2 {
                let after_right_border = &stripped[positions[1] + '│'.len_utf8()..];
                assert!(
                    after_right_border.trim().is_empty(),
                    "Trailing content found after closing │: '{}'. Full line: '{}'",
                    after_right_border.trim(),
                    stripped
                );
            }
        }
    }
}

#[test]
fn status_border_lines_no_trailing_text() {
    // After ┐, ┘, ┤ there should be nothing
    let output = soul().arg("status").output().expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let stripped = strip_ansi(line);
        for end_char in ['┐', '┘', '┤'] {
            if stripped.contains(end_char) {
                let pos = stripped.rfind(end_char).unwrap();
                let after = &stripped[pos + end_char.len_utf8()..];
                assert!(
                    after.trim().is_empty(),
                    "Trailing content after '{}': '{}'. Full line: '{}'",
                    end_char,
                    after.trim(),
                    stripped
                );
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 19. FULL STATUS OUTPUT STRUCTURE VALIDATION
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn status_box_structure_complete() {
    // Each box should have exactly: top border, header, separator, content lines, bottom border
    let output = soul().arg("status").output().expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let sections = extract_box_sections(&stdout);
    for (i, section) in sections.iter().enumerate() {
        let lines: Vec<String> = section.lines().map(strip_ansi).collect();

        assert!(
            lines.len() >= 4,
            "Box section {} has only {} lines. Minimum is 4 (top, header, sep, bottom). Lines: {:?}",
            i,
            lines.len(),
            lines
        );

        // First line: ┌───┐
        assert!(
            lines[0].contains('┌') && lines[0].contains('┐'),
            "Box {} first line should be top border (┌...┐). Got: '{}'",
            i,
            lines[0]
        );

        // Second line: │  Title  │
        assert!(
            lines[1].contains('│'),
            "Box {} second line should be header (│...│). Got: '{}'",
            i,
            lines[1]
        );

        // Third line: ├───┤
        assert!(
            lines[2].contains('├') && lines[2].contains('┤'),
            "Box {} third line should be separator (├...┤). Got: '{}'",
            i,
            lines[2]
        );

        // Last line: └───┘
        let last = lines.last().unwrap();
        assert!(
            last.contains('└') && last.contains('┘'),
            "Box {} last line should be bottom border (└...┘). Got: '{}'",
            i,
            last
        );
    }
}

#[test]
fn status_consistent_indentation() {
    // All box lines should start with the same indentation (2 spaces)
    let output = soul().arg("status").output().expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let stripped = strip_ansi(line);
        let has_box_char = stripped.contains('┌')
            || stripped.contains('└')
            || stripped.contains('├')
            || stripped.contains('│')
            || stripped.contains('┐')
            || stripped.contains('┘')
            || stripped.contains('┤');

        if has_box_char {
            // Should start with exactly 2 spaces
            assert!(
                stripped.starts_with("  ") && !stripped.starts_with("   "),
                "Box line should start with exactly 2 spaces indent. Got: '{}'",
                stripped
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 20. ERROR FORMATTING CONSISTENCY
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn import_nonexistent_no_double_blank_lines() {
    // Regression: banner() ends with \n, and the error handler in main()
    // used to prepend \n, creating a double blank line between banner and error.
    let output = soul()
        .args(["import", "/nonexistent"])
        .output()
        .expect("should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    let stripped = strip_ansi(&combined);

    // Should not contain 3+ consecutive newlines (= 2+ blank lines in a row)
    assert!(
        !stripped.contains("\n\n\n"),
        "Double blank line found in import error output. Got:\n{}",
        stripped
    );
}

#[test]
fn watch_nonexistent_no_double_blank_lines() {
    let output = soul()
        .args(["watch", "/nonexistent"])
        .output()
        .expect("should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    let stripped = strip_ansi(&combined);

    assert!(
        !stripped.contains("\n\n\n"),
        "Double blank line found in watch error output. Got:\n{}",
        stripped
    );
}

#[test]
fn watch_no_folder_error_has_cross_icon() {
    // Watch (no args) in non-TTY shows ✗ icon
    soul()
        .arg("watch")
        .assert()
        .failure()
        .stderr(predicate::str::contains("✗"));
}

#[test]
fn watch_no_folder_error_shows_usage() {
    soul()
        .arg("watch")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage: soul watch"));
}

#[test]
fn watch_and_import_no_args_errors_consistent() {
    // Both import and watch should produce actionable, non-panic errors in test env.
    let tmp = tempdir().unwrap();
    let import_out = soul()
        .env("HOME", tmp.path().to_str().unwrap())
        .arg("import")
        .output()
        .expect("should run");
    let watch_out = soul().arg("watch").output().expect("should run");

    let import_stderr = strip_ansi(&String::from_utf8_lossy(&import_out.stderr));
    let watch_stderr = strip_ansi(&String::from_utf8_lossy(&watch_out.stderr));

    // Import no-folder is now providers mode and should hit initialization guidance.
    assert!(
        import_stderr.contains("Run `soul init`"),
        "Import error missing expected text. Got: {}",
        import_stderr
    );
    assert!(
        watch_stderr.contains("Auto-watch requires a terminal")
            || watch_stderr.contains("Usage: soul watch"),
        "Watch error missing expected text. Got: {}",
        watch_stderr
    );

    // Both should contain ✗
    assert!(import_stderr.contains("✗"), "Import error missing ✗ icon");
    assert!(watch_stderr.contains("✗"), "Watch error missing ✗ icon");

    // Both should exit with code 1
    assert_eq!(import_out.status.code(), Some(1));
    assert_eq!(watch_out.status.code(), Some(1));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 21. NO LEFTOVER "INGEST" IN USER-FACING OUTPUT
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn export_no_ingest_references() {
    // User-facing output should say "import" not "ingest"
    let output = soul().arg("export").output().expect("should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The word "ingest" should NOT appear in the export output
    // (except potentially in conversation content, which would be user data)
    // Check the frontmatter specifically
    assert!(
        !stdout.contains("sources: [ingest]"),
        "Export output contains 'sources: [ingest]' — should be 'sources: [import]'"
    );
}

#[test]
fn status_no_ingest_references() {
    let output = soul().arg("status").output().expect("should run");
    assert!(output.status.success());
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));

    // "ingest" should not appear in user-facing status output
    // (internal variable names won't be in the output)
    let lower = stdout.to_lowercase();
    assert!(
        !lower.contains("ingest"),
        "Status output contains 'ingest' — all user-facing text should say 'import'. Output:\n{}",
        stdout
    );
}

#[test]
fn help_no_ingest_references() {
    // Main help should not mention "ingest"
    let output = soul().arg("--help").output().expect("should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lower = stdout.to_lowercase();

    assert!(
        !lower.contains("ingest"),
        "Help output contains 'ingest'. Output:\n{}",
        stdout
    );
}

#[test]
fn import_help_no_ingest_references() {
    let output = soul()
        .args(["import", "--help"])
        .output()
        .expect("should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lower = stdout.to_lowercase();

    assert!(
        !lower.contains("ingest"),
        "Import help contains 'ingest'. Output:\n{}",
        stdout
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 22. RESET COMMAND
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn help_reset_subcommand() {
    soul()
        .args(["help", "reset"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Reset vault"))
        .stdout(predicate::str::contains("--force"));
}

#[test]
fn reset_dash_dash_help() {
    soul()
        .args(["reset", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--force"))
        .stdout(predicate::str::contains("Skip confirmation"));
}

#[test]
fn help_flag_shows_reset_command() {
    soul()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("reset"));
}

#[test]
fn reset_without_vault_shows_nothing_to_reset() {
    // Set HOME to a temp dir so ~/soul-vault/ doesn't exist
    let tmp = tempdir().unwrap();
    soul()
        .env("HOME", tmp.path().to_str().unwrap())
        .arg("reset")
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to reset"));
}

#[test]
fn reset_force_without_vault_shows_nothing_to_reset() {
    let tmp = tempdir().unwrap();
    soul()
        .env("HOME", tmp.path().to_str().unwrap())
        .args(["reset", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to reset"));
}

#[test]
fn reset_force_with_temp_vault_deletes_vault() {
    // Create a fake vault in a temp home directory
    let tmp_home = tempdir().unwrap();
    let vault_root = tmp_home.path().join("soul-vault");
    let config_dir = vault_root.join(".config");
    fs::create_dir_all(&config_dir).unwrap();
    fs::create_dir_all(vault_root.join("memories")).unwrap();
    fs::create_dir_all(vault_root.join("topics")).unwrap();
    fs::create_dir_all(vault_root.join("people")).unwrap();
    fs::create_dir_all(vault_root.join("identity")).unwrap();
    fs::create_dir_all(vault_root.join("sources")).unwrap();

    // Write a minimal config.json so is_initialized() returns true
    let config = r#"{
        "providers": [],
        "processingLlm": "claude",
        "vaultPath": "/tmp/soul-vault",
        "createdAt": "2026-02-14T00:00:00Z"
    }"#;
    fs::write(config_dir.join("config.json"), config).unwrap();

    // Write some memories
    fs::write(vault_root.join("memories").join("test.md"), "# Memory").unwrap();

    // Verify vault exists
    assert!(vault_root.exists());
    assert!(config_dir.join("config.json").exists());

    // Run reset --force with HOME set to our temp dir
    soul()
        .env("HOME", tmp_home.path().to_str().unwrap())
        .args(["reset", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Vault reset"));

    // Verify vault is deleted
    assert!(
        !vault_root.exists(),
        "Vault directory should be deleted after reset --force"
    );
}

#[test]
fn reset_without_force_in_non_tty_fails() {
    // In tests, stdin is not a TTY, so `soul reset` without --force should fail
    // But only if the vault exists. Use a temp home with a vault.
    let tmp_home = tempdir().unwrap();
    let vault_root = tmp_home.path().join("soul-vault");
    let config_dir = vault_root.join(".config");
    fs::create_dir_all(&config_dir).unwrap();
    fs::create_dir_all(vault_root.join("memories")).unwrap();
    fs::create_dir_all(vault_root.join("topics")).unwrap();
    fs::create_dir_all(vault_root.join("people")).unwrap();

    let config = r#"{
        "providers": [],
        "processingLlm": "claude",
        "vaultPath": "/tmp/soul-vault",
        "createdAt": "2026-02-14T00:00:00Z"
    }"#;
    fs::write(config_dir.join("config.json"), config).unwrap();

    soul()
        .env("HOME", tmp_home.path().to_str().unwrap())
        .arg("reset")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("--force").or(predicate::str::contains("non-interactive")),
        );
}

#[test]
fn reset_no_panic() {
    let output = soul().arg("reset").output().expect("should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stderr.contains("panicked"),
        "Reset should not panic. Stderr: {}",
        stderr
    );
    assert!(
        !stdout.contains("panicked"),
        "Reset should not panic. Stdout: {}",
        stdout
    );
}

#[test]
fn reset_force_short_flag() {
    let tmp = tempdir().unwrap();
    soul()
        .env("HOME", tmp.path().to_str().unwrap())
        .args(["reset", "-f"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to reset"));
}

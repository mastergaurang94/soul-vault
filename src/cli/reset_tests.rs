//! Tests for reset module.

use super::*;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn test_safe_to_delete_valid_soul_vault_path() {
    let home = dirs::home_dir().unwrap();
    let path = home.join("soul-vault");
    assert!(is_safe_to_delete(&path));
}

#[test]
fn test_safe_to_delete_nested_soul_vault_path() {
    let home = dirs::home_dir().unwrap();
    let path = home.join("projects").join("soul-vault");
    assert!(is_safe_to_delete(&path));
}

#[test]
fn test_reject_root() {
    assert!(!is_safe_to_delete(Path::new("/")));
}

#[test]
fn test_reject_home_dir() {
    let home = dirs::home_dir().unwrap();
    assert!(!is_safe_to_delete(&home));
}

#[test]
fn test_reject_tilde() {
    assert!(!is_safe_to_delete(Path::new("~")));
}

#[test]
fn test_reject_path_without_soul_vault() {
    let home = dirs::home_dir().unwrap();
    let path = home.join("Documents");
    assert!(!is_safe_to_delete(&path));
}

#[test]
fn test_reject_path_outside_home() {
    assert!(!is_safe_to_delete(Path::new("/tmp/soul-vault")));
}

#[test]
fn test_reject_etc_path_with_soul_vault() {
    assert!(!is_safe_to_delete(Path::new("/etc/soul-vault")));
}

#[test]
fn test_count_md_files_nonexistent() {
    assert_eq!(count_md_files(Path::new("/nonexistent/path")), 0);
}

#[test]
fn test_count_md_files_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(count_md_files(tmp.path()), 0);
}

#[test]
fn test_count_md_files_with_files() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("one.md"), "# One").unwrap();
    fs::write(tmp.path().join("two.md"), "# Two").unwrap();
    fs::write(tmp.path().join("three.txt"), "Not markdown").unwrap();
    assert_eq!(count_md_files(tmp.path()), 2);
}

#[test]
fn test_safe_to_delete_rejects_slash_soul_vault_outside_home() {
    let path = PathBuf::from("/soul-vault");
    assert!(!is_safe_to_delete(&path));
}

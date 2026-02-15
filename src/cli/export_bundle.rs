//! Bundle export directory creation helpers.

use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::export_types::{default_output_path, include, ExportFormat, ExportSection};
use crate::vault::config::{identity_dir, memories_dir, people_dir, topics_dir};

struct BundlePaths {
    identity: PathBuf,
    topics: PathBuf,
    people: PathBuf,
    memories: PathBuf,
}

pub(crate) fn output_bundle(output_path: Option<&str>, sections: &[ExportSection]) -> Result<()> {
    let destination = output_path
        .map(PathBuf::from)
        .unwrap_or_else(|| default_output_path(ExportFormat::Bundle));
    let paths = BundlePaths {
        identity: identity_dir(),
        topics: topics_dir(),
        people: people_dir(),
        memories: memories_dir(),
    };

    create_bundle_dir(&paths, &destination, sections)?;
    Ok(())
}

fn create_bundle_dir(
    paths: &BundlePaths,
    destination: &Path,
    sections: &[ExportSection],
) -> Result<()> {
    if destination.exists() {
        bail!(
            "Bundle destination already exists: {}\n      → Choose a different `--output` path.",
            destination.display()
        );
    }
    fs::create_dir_all(destination)?;

    if include(sections, ExportSection::Identity) {
        copy_file_if_exists(
            &paths.identity.join("profile.md"),
            &destination.join("identity").join("profile.md"),
        )?;
    }
    if include(sections, ExportSection::Preferences) {
        copy_file_if_exists(
            &paths.identity.join("preferences.md"),
            &destination.join("identity").join("preferences.md"),
        )?;
    }
    if include(sections, ExportSection::Topics) {
        copy_markdown_dir(&paths.topics, &destination.join("topics"))?;
    }
    if include(sections, ExportSection::People) {
        copy_markdown_dir(&paths.people, &destination.join("people"))?;
    }
    if include(sections, ExportSection::Memories) {
        copy_markdown_dir(&paths.memories, &destination.join("memories"))?;
    }

    Ok(())
}

fn copy_markdown_dir(source: &Path, destination: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|ext| ext == "md") {
            fs::copy(entry.path(), destination.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn copy_file_if_exists(source: &Path, destination: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_directory_creation() {
        let tmp = tempfile::tempdir().unwrap();
        let identity = tmp.path().join("identity");
        let topics = tmp.path().join("topics");
        let people = tmp.path().join("people");
        let memories = tmp.path().join("memories");
        fs::create_dir_all(&identity).unwrap();
        fs::create_dir_all(&topics).unwrap();
        fs::create_dir_all(&people).unwrap();
        fs::create_dir_all(&memories).unwrap();
        fs::write(identity.join("profile.md"), "profile").unwrap();
        fs::write(identity.join("preferences.md"), "prefs").unwrap();
        fs::write(topics.join("rust.md"), "rust").unwrap();
        fs::write(people.join("alice.md"), "alice").unwrap();
        fs::write(memories.join("today.md"), "today").unwrap();

        let destination = tmp.path().join("bundle");
        let paths = BundlePaths {
            identity,
            topics,
            people,
            memories,
        };

        create_bundle_dir(&paths, &destination, &ExportSection::all()).unwrap();

        assert!(destination.join("identity/profile.md").exists());
        assert!(destination.join("identity/preferences.md").exists());
        assert!(destination.join("topics/rust.md").exists());
        assert!(destination.join("people/alice.md").exists());
        assert!(destination.join("memories/today.md").exists());
    }
}

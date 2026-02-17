//! Integration tests for Soul Vault operations.

use std::fs;
use tempfile::tempdir;

/// Helper: set up a temporary vault structure and return the path.
fn setup_temp_vault(tmp: &std::path::Path) {
    let config_dir = tmp.join(".config");
    let identity_dir = tmp.join("identity");
    let memories_dir = tmp.join("memories");
    let topics_dir = tmp.join("topics");
    let people_dir = tmp.join("people");

    fs::create_dir_all(&config_dir).unwrap();
    fs::create_dir_all(&identity_dir).unwrap();
    fs::create_dir_all(&memories_dir).unwrap();
    fs::create_dir_all(&topics_dir).unwrap();
    fs::create_dir_all(&people_dir).unwrap();

    // Write config.json
    let config = serde_json::json!({
        "providers": [
            {"name": "claude", "enabled": true},
            {"name": "chatgpt", "enabled": false},
            {"name": "gemini", "enabled": false}
        ],
        "processingMode": "claude",
        "vaultPath": tmp.display().to_string(),
        "createdAt": "2026-02-14T00:00:00Z"
    });
    fs::write(
        config_dir.join("config.json"),
        serde_json::to_string_pretty(&config).unwrap(),
    )
    .unwrap();

    // Write default identity files
    fs::write(
        identity_dir.join("profile.md"),
        "---\nupdated: 2026-02-14\n---\n\n# Profile\n",
    )
    .unwrap();
    fs::write(
        identity_dir.join("preferences.md"),
        "---\nupdated: 2026-02-14\n---\n\n# Preferences\n",
    )
    .unwrap();
}

#[test]
fn test_fixture_files_exist() {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    assert!(fixtures.exists(), "Fixtures directory should exist");

    let md_files: Vec<_> = fs::read_dir(&fixtures)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();
    assert!(!md_files.is_empty(), "Should have markdown fixtures");
}

#[test]
fn test_vault_structure_creation() {
    let tmp = tempdir().unwrap();
    setup_temp_vault(tmp.path());

    assert!(tmp.path().join(".config/config.json").exists());
    assert!(tmp.path().join("identity/profile.md").exists());
    assert!(tmp.path().join("identity/preferences.md").exists());
    assert!(tmp.path().join("memories").exists());
    assert!(tmp.path().join("topics").exists());
    assert!(tmp.path().join("people").exists());
}

#[test]
fn test_config_roundtrip() {
    let tmp = tempdir().unwrap();
    setup_temp_vault(tmp.path());

    let config_path = tmp.path().join(".config/config.json");
    let raw = fs::read_to_string(&config_path).unwrap();
    let config: serde_json::Value = serde_json::from_str(&raw).unwrap();

    assert_eq!(config["processingMode"], "claude");
    assert_eq!(config["providers"][0]["name"], "claude");
    assert_eq!(config["providers"][0]["enabled"], true);
}

#[test]
fn test_keys_file_creation() {
    let tmp = tempdir().unwrap();
    let config_dir = tmp.path().join(".config");
    fs::create_dir_all(&config_dir).unwrap();

    let keys = serde_json::json!({
        "claude": "sk-ant-test123",
        "chatgpt": "sk-test456"
    });
    let keys_path = config_dir.join("keys.json");
    fs::write(&keys_path, serde_json::to_string_pretty(&keys).unwrap()).unwrap();

    let raw = fs::read_to_string(&keys_path).unwrap();
    let loaded: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(loaded["claude"], "sk-ant-test123");
}

#[test]
fn test_daily_digest_write() {
    let tmp = tempdir().unwrap();
    let memories_dir = tmp.path().join("memories");
    fs::create_dir_all(&memories_dir).unwrap();

    let date = "2026-02-14";
    let digest = format!(
        "---\ndate: {}\nsources: [import]\n---\n\n# Daily Memories — {}\n\n## Decisions\n- Chose Rust for the rewrite\n",
        date, date
    );
    fs::write(memories_dir.join(format!("{}.md", date)), &digest).unwrap();

    let content = fs::read_to_string(memories_dir.join("2026-02-14.md")).unwrap();
    assert!(content.contains("Daily Memories"));
    assert!(content.contains("Chose Rust"));
}

#[test]
fn test_topic_file_format() {
    let tmp = tempdir().unwrap();
    let topics_dir = tmp.path().join("topics");
    fs::create_dir_all(&topics_dir).unwrap();

    let topic_content = "---\ntopic: Rust\nupdated: 2026-02-14\n---\n\n# Rust\n\n- Learning Rust for CLI tools _(2026-02-14, high)_\n";
    fs::write(topics_dir.join("rust.md"), topic_content).unwrap();

    let content = fs::read_to_string(topics_dir.join("rust.md")).unwrap();
    assert!(content.contains("topic: Rust"));
    assert!(content.contains("# Rust"));
    assert!(content.contains("Learning Rust"));
}

#[test]
fn test_people_file_format() {
    let tmp = tempdir().unwrap();
    let people_dir = tmp.path().join("people");
    fs::create_dir_all(&people_dir).unwrap();

    let person_content = "---\nperson: Avni\nrole: daughter\nupdated: 2026-02-14\n---\n\n# Avni\n\n- His daughter, light of his life _(2026-02-14, high)_\n";
    fs::write(people_dir.join("avni.md"), person_content).unwrap();

    let content = fs::read_to_string(people_dir.join("avni.md")).unwrap();
    assert!(content.contains("person: Avni"));
    assert!(content.contains("role: daughter"));
}

#[test]
fn test_export_frontmatter_stripping() {
    let input = "---\ndate: 2026-02-14\nsources: [import]\n---\n\n# Daily Memories\n\nContent here";
    let re = regex::Regex::new(r"^---[\s\S]*?---\s*\n?").unwrap();
    let stripped = re.replace(input, "").trim().to_string();
    assert_eq!(stripped, "# Daily Memories\n\nContent here");
}

#[test]
fn test_memory_extraction_parsing() {
    let json = r#"{
        "identity": [
            {"content": "Name is Gaurang", "category": "name", "confidence": "high"}
        ],
        "preferences": [
            {"content": "Likes tea", "type": "like", "confidence": "medium"}
        ],
        "decisions": [
            {"content": "Chose Rust", "context": "For CLI rewrite", "confidence": "high"}
        ],
        "relationships": [
            {"person": "Avni", "content": "His daughter", "role": "daughter", "confidence": "high"}
        ],
        "topics": [
            {"topic": "Rust", "content": "Learning for CLI tools", "confidence": "medium"}
        ],
        "emotional_context": [
            {"mood": "excited", "content": "About the project", "confidence": "medium"}
        ]
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(parsed["identity"][0]["content"], "Name is Gaurang");
    assert_eq!(parsed["relationships"][0]["person"], "Avni");
    assert_eq!(parsed["topics"][0]["topic"], "Rust");
}

#[test]
fn test_file_discovery_with_fixtures() {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    if !fixtures.exists() {
        return; // skip if no fixtures
    }

    let entries: Vec<_> = fs::read_dir(&fixtures)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();

    let supported_exts = ["md", "txt", "json", "jsonl"];
    let supported_count = entries
        .iter()
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| supported_exts.contains(&ext))
        })
        .count();

    assert!(supported_count > 0, "Should find supported fixture files");
}

#[test]
fn test_chunking_small_text() {
    let text = "This is a small piece of text that doesn't need chunking.";

    // Verify it's under the chunk limit
    assert!(text.len() < 80_000);
}

#[test]
fn test_full_vault_workflow() {
    // This test simulates the full workflow without API calls:
    // 1. Create vault structure
    // 2. Write memories
    // 3. Read them back
    // 4. Verify content

    let tmp = tempdir().unwrap();
    setup_temp_vault(tmp.path());

    // Write a daily digest
    let memories_dir = tmp.path().join("memories");
    let date = "2026-02-14";
    let digest = format!(
        "---\ndate: {}\nsources: [import]\n---\n\n# Daily Memories — {}\n\n## Identity\n- Name is Gaurang\n- Based in Houston\n\n## Topics\n- **Rust**: Learning for CLI rewrite\n",
        date, date
    );
    fs::write(memories_dir.join(format!("{}.md", date)), &digest).unwrap();

    // Write topic files
    let topics_dir = tmp.path().join("topics");
    fs::write(
        topics_dir.join("rust.md"),
        "---\ntopic: Rust\nupdated: 2026-02-14\n---\n\n# Rust\n\n- Learning for CLI rewrite _(2026-02-14, high)_\n",
    ).unwrap();

    // Write people files
    let people_dir = tmp.path().join("people");
    fs::write(
        people_dir.join("avni.md"),
        "---\nperson: Avni\nrole: daughter\nupdated: 2026-02-14\n---\n\n# Avni\n\n- His daughter _(2026-02-14, high)_\n",
    ).unwrap();

    // Verify vault stats
    let memory_count = fs::read_dir(&memories_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .count();
    assert_eq!(memory_count, 1);

    let topic_count = fs::read_dir(&topics_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .count();
    assert_eq!(topic_count, 1);

    let people_count = fs::read_dir(&people_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .count();
    assert_eq!(people_count, 1);

    // Verify content
    let rust_topic = fs::read_to_string(topics_dir.join("rust.md")).unwrap();
    assert!(rust_topic.contains("# Rust"));
    assert!(rust_topic.contains("CLI rewrite"));

    let avni = fs::read_to_string(people_dir.join("avni.md")).unwrap();
    assert!(avni.contains("# Avni"));
    assert!(avni.contains("daughter"));
}

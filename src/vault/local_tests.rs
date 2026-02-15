use std::fs;
use std::io::Write;

use crate::types::FileInfo;
use crate::vault::local::{discover_files, extract_file_content};
use crate::vault::local_extract::is_chatgpt_conversations_file;

#[test]
fn test_discover_files_basics_and_recursive() {
    let tmp = tempfile::tempdir().unwrap();
    let subdir = tmp.path().join("subdir");
    fs::create_dir_all(&subdir).unwrap();
    fs::write(tmp.path().join("test.md"), "# Hello").unwrap();
    fs::write(tmp.path().join("notes.txt"), "Some notes").unwrap();
    fs::write(tmp.path().join("data.json"), "{}").unwrap();
    fs::write(tmp.path().join("log.jsonl"), "{}").unwrap();
    fs::write(subdir.join("nested.md"), "nested").unwrap();
    fs::write(tmp.path().join("image.png"), "binary").unwrap();

    let files = discover_files(tmp.path()).unwrap();
    assert_eq!(files.len(), 5);
}

#[test]
fn test_discover_files_includes_zip_and_ignores_hidden() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("visible.md"), "visible").unwrap();

    let hidden = tmp.path().join(".hidden");
    fs::create_dir_all(&hidden).unwrap();
    fs::write(hidden.join("secret.md"), "secret").unwrap();

    let zip_path = tmp.path().join("archive.zip");
    let file = fs::File::create(&zip_path).unwrap();
    let mut zip_writer = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip_writer.start_file("readme.txt", options).unwrap();
    zip_writer.write_all(b"hello").unwrap();
    zip_writer.finish().unwrap();

    let files = discover_files(tmp.path()).unwrap();
    assert_eq!(files.len(), 2);
    assert!(files.iter().any(|f| f.extension == ".zip"));
}

#[test]
fn test_discover_chatgpt_export_dir_only_conversations_file() {
    let tmp = tempfile::tempdir().unwrap();
    let conv_json = serde_json::json!([{
        "title": "Test",
        "mapping": {"root": {"id": "root", "parent": null, "children": [], "message": null}}
    }]);
    fs::write(tmp.path().join("conversations.json"), conv_json.to_string()).unwrap();
    fs::write(tmp.path().join("chat.html"), "<html></html>").unwrap();
    fs::write(tmp.path().join("other.md"), "notes").unwrap();

    let files = discover_files(tmp.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "conversations");
}

#[test]
fn test_extract_text_json_and_jsonl() {
    let tmp = tempfile::tempdir().unwrap();

    let txt = tmp.path().join("notes.txt");
    fs::write(&txt, "  Hello world  \n").unwrap();
    let txt_info = FileInfo {
        path: txt,
        name: "notes".to_string(),
        extension: ".txt".to_string(),
        size: 15,
    };
    assert_eq!(extract_file_content(&txt_info).unwrap(), "Hello world");

    let json = tmp.path().join("chat.json");
    fs::write(
        &json,
        r#"[{"title": "Test", "messages": [{"role": "user", "content": "Hello"}]}]"#,
    )
    .unwrap();
    let json_info = FileInfo {
        path: json,
        name: "chat".to_string(),
        extension: ".json".to_string(),
        size: 0,
    };
    let json_out = extract_file_content(&json_info).unwrap();
    assert!(json_out.contains("## Test"));
    assert!(json_out.contains("user: Hello"));

    let jsonl = tmp.path().join("session.jsonl");
    fs::write(
        &jsonl,
        "{\"role\": \"user\", \"content\": \"Hello\"}\n{\"role\": \"assistant\", \"content\": \"Hi there\"}",
    )
    .unwrap();
    let jsonl_info = FileInfo {
        path: jsonl,
        name: "session".to_string(),
        extension: ".jsonl".to_string(),
        size: 0,
    };
    let jsonl_out = extract_file_content(&jsonl_info).unwrap();
    assert!(jsonl_out.contains("Hello"));
}

#[test]
fn test_extract_chatgpt_conversations_and_zip() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("conversations.json");
    let json = serde_json::json!([{
        "title": "Parsed via ChatGPT parser",
        "create_time": 1700000000.0,
        "mapping": {
            "root": {"id": "root", "parent": null, "children": ["n1"], "message": null},
            "n1": {"id": "n1", "parent": "root", "children": ["n2"], "message": {"author": {"role": "user"}, "content": {"content_type": "text", "parts": ["Hello via parser"]}, "create_time": 1700000000.0}},
            "n2": {"id": "n2", "parent": "n1", "children": [], "message": {"author": {"role": "assistant"}, "content": {"content_type": "text", "parts": ["Reply via parser"]}, "create_time": 1700000001.0}}
        }
    }]);
    fs::write(&path, json.to_string()).unwrap();

    let file = FileInfo {
        path,
        name: "conversations".to_string(),
        extension: ".json".to_string(),
        size: 0,
    };
    let out = extract_file_content(&file).unwrap();
    assert!(out.contains("## Parsed via ChatGPT parser"));
    assert!(out.contains("assistant: Reply via parser"));

    let zip_path = tmp.path().join("chatgpt-export.zip");
    let file = fs::File::create(&zip_path).unwrap();
    let mut zip_writer = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip_writer
        .start_file("conversations.json", options)
        .unwrap();
    zip_writer.write_all(json.to_string().as_bytes()).unwrap();
    zip_writer.finish().unwrap();
    let zip_info = FileInfo {
        path: zip_path,
        name: "chatgpt-export".to_string(),
        extension: ".zip".to_string(),
        size: 0,
    };
    let zip_out = extract_file_content(&zip_info).unwrap();
    assert!(zip_out.contains("## Parsed via ChatGPT parser"));
}

#[test]
fn test_extract_non_chatgpt_zip_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let zip_path = tmp.path().join("random.zip");

    let file = fs::File::create(&zip_path).unwrap();
    let mut zip_writer = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip_writer.start_file("readme.txt", options).unwrap();
    zip_writer.write_all(b"not chatgpt").unwrap();
    zip_writer.finish().unwrap();

    let file_info = FileInfo {
        path: zip_path,
        name: "random".to_string(),
        extension: ".zip".to_string(),
        size: 0,
    };
    assert!(extract_file_content(&file_info).is_err());
}

#[test]
fn test_is_chatgpt_conversations_file_detection() {
    let tmp = tempfile::tempdir().unwrap();

    let good = tmp.path().join("conversations.json");
    fs::write(
        &good,
        serde_json::json!([{"title": "Test", "mapping": {"root": {"id": "root"}}}]).to_string(),
    )
    .unwrap();
    assert!(is_chatgpt_conversations_file(&good));

    let wrong = tmp.path().join("other.json");
    fs::write(&wrong, "[]").unwrap();
    assert!(!is_chatgpt_conversations_file(&wrong));

    let no_mapping = tmp.path().join("conversations-empty.json");
    fs::write(
        &no_mapping,
        serde_json::json!([{"title": "Test"}]).to_string(),
    )
    .unwrap();
    assert!(!is_chatgpt_conversations_file(&no_mapping));
}

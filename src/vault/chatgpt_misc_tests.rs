//! Tests for vault module.
use chrono::Datelike;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::vault::chatgpt_detect::{is_chatgpt_export_dir, is_chatgpt_zip};
use crate::vault::chatgpt_format::format_conversations;
use crate::vault::chatgpt_parse::{parse_chatgpt_zip, parse_conversation};
use crate::vault::chatgpt_types::{timestamp_to_datetime, ParsedConversation, ParsedMessage};

#[test]
fn test_is_chatgpt_zip_true() {
    let tmp = tempfile::TempDir::new().unwrap();
    let zip_path = tmp.path().join("export.zip");

    let file = fs::File::create(&zip_path).unwrap();
    let mut zip_writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    zip_writer
        .start_file("conversations.json", options)
        .unwrap();
    zip_writer.write_all(b"[]").unwrap();
    zip_writer.finish().unwrap();

    assert!(is_chatgpt_zip(&zip_path));
}

#[test]
fn test_is_chatgpt_zip_false_and_missing() {
    let tmp = tempfile::TempDir::new().unwrap();
    let zip_path = tmp.path().join("other.zip");

    let file = fs::File::create(&zip_path).unwrap();
    let mut zip_writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    zip_writer
        .start_file("something-else.txt", options)
        .unwrap();
    zip_writer.write_all(b"hello").unwrap();
    zip_writer.finish().unwrap();

    assert!(!is_chatgpt_zip(&zip_path));
    assert!(!is_chatgpt_zip(Path::new("/nonexistent/file.zip")));
}

#[test]
fn test_is_chatgpt_export_dir() {
    let tmp = tempfile::TempDir::new().unwrap();
    assert!(!is_chatgpt_export_dir(tmp.path()));
    fs::write(tmp.path().join("conversations.json"), "[]").unwrap();
    assert!(is_chatgpt_export_dir(tmp.path()));
}

#[test]
fn test_format_conversations() {
    let convs = vec![
        ParsedConversation {
            title: "Chat A".to_string(),
            created_at: None,
            messages: vec![
                ParsedMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                    timestamp: None,
                },
                ParsedMessage {
                    role: "assistant".to_string(),
                    content: "Hi!".to_string(),
                    timestamp: None,
                },
            ],
        },
        ParsedConversation {
            title: "Chat B".to_string(),
            created_at: None,
            messages: vec![ParsedMessage {
                role: "user".to_string(),
                content: "Goodbye".to_string(),
                timestamp: None,
            }],
        },
    ];

    let formatted = format_conversations(&convs);
    assert!(formatted.contains("## Chat A"));
    assert!(formatted.contains("assistant: Hi!"));
    assert!(formatted.contains("---"));
    assert!(formatted.contains("## Chat B"));
}

#[test]
fn test_timestamp_parsing_and_untitled() {
    let ts = timestamp_to_datetime(1700000000.0).unwrap();
    assert_eq!(ts.year(), 2023);

    let conv_json = serde_json::json!({
        "mapping": {
            "root": {"id": "root", "parent": null, "children": ["n1"], "message": null},
            "n1": {
                "id": "n1", "parent": "root", "children": [],
                "message": {
                    "author": {"role": "user"},
                    "content": {"content_type": "text", "parts": ["test"]},
                    "create_time": 1700000000.0
                }
            }
        }
    });

    let conv = parse_conversation(&conv_json).unwrap();
    assert_eq!(conv.title, "Untitled conversation");
    assert!(conv.created_at.is_none());
}

#[test]
fn test_deep_tree_and_tool_role() {
    let conv_json = serde_json::json!({
        "title": "Tool Chat",
        "create_time": 1700000000.0,
        "mapping": {
            "root": {"id": "root", "parent": null, "children": ["n1"], "message": null},
            "n1": {"id": "n1", "parent": "root", "children": ["n2"], "message": {"author": {"role": "user"}, "content": {"content_type": "text", "parts": ["search"]}, "create_time": 1700000000.0}},
            "n2": {"id": "n2", "parent": "n1", "children": ["n3"], "message": {"author": {"role": "tool"}, "content": {"content_type": "text", "parts": ["results"]}, "create_time": 1700000001.0}},
            "n3": {"id": "n3", "parent": "n2", "children": ["n4"], "message": {"author": {"role": "assistant"}, "content": {"content_type": "text", "parts": ["answer"]}, "create_time": 1700000002.0}},
            "n4": {"id": "n4", "parent": "n3", "children": [], "message": {"author": {"role": "assistant"}, "content": {"content_type": "text", "parts": ["tail"]}, "create_time": 1700000003.0}}
        }
    });

    let conv = parse_conversation(&conv_json).unwrap();
    assert_eq!(conv.messages.len(), 4);
    assert_eq!(conv.messages[1].role, "tool");
}

#[test]
fn test_zip_without_conversations_json() {
    let tmp = tempfile::TempDir::new().unwrap();
    let zip_path = tmp.path().join("bad-export.zip");

    let file = fs::File::create(&zip_path).unwrap();
    let mut zip_writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    zip_writer.start_file("readme.txt", options).unwrap();
    zip_writer.write_all(b"not a chatgpt export").unwrap();
    zip_writer.finish().unwrap();

    let result = parse_chatgpt_zip(&zip_path);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No conversations.json"));
}

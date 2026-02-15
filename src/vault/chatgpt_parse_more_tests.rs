//! Tests for vault module.
use std::fs;

use crate::vault::chatgpt_parse::{
    parse_chatgpt_json, parse_chatgpt_zip, parse_conversation, parse_conversations_json_str,
};

#[test]
fn test_parse_empty_conversation() {
    let conv_json = serde_json::json!({
        "title": "Empty",
        "mapping": {"root": {"id": "root", "parent": null, "children": [], "message": null}}
    });

    let conv = parse_conversation(&conv_json).unwrap();
    assert_eq!(conv.title, "Empty");
    assert!(conv.messages.is_empty());
}

#[test]
fn test_parse_missing_mapping() {
    let bad_json = serde_json::json!({"title": "No mapping"});
    let result = parse_conversation(&bad_json);
    assert!(result.is_err());
}

#[test]
fn test_parse_conversations_json_str() {
    let json = serde_json::json!([
        {
            "title": "Chat 1",
            "create_time": 1700000000.0,
            "mapping": {
                "root": {"id": "root", "parent": null, "children": ["n1"], "message": null},
                "n1": {
                    "id": "n1", "parent": "root", "children": [],
                    "message": {
                        "author": {"role": "user"},
                        "content": {"content_type": "text", "parts": ["Hello"]},
                        "create_time": 1700000000.0
                    }
                }
            }
        },
        {
            "title": "Chat 2",
            "create_time": 1700001000.0,
            "mapping": {
                "root": {"id": "root", "parent": null, "children": ["n1"], "message": null},
                "n1": {
                    "id": "n1", "parent": "root", "children": [],
                    "message": {
                        "author": {"role": "user"},
                        "content": {"content_type": "text", "parts": ["World"]},
                        "create_time": 1700001000.0
                    }
                }
            }
        }
    ]);

    let convs = parse_conversations_json_str(&json.to_string()).unwrap();
    assert_eq!(convs.len(), 2);
    assert_eq!(convs[0].title, "Chat 1");
    assert_eq!(convs[1].title, "Chat 2");
}

#[test]
fn test_parse_conversations_skips_empty() {
    let json = serde_json::json!([
        {
            "title": "Empty Conv",
            "mapping": {
                "root": {"id": "root", "parent": null, "children": [], "message": null}
            }
        },
        {
            "title": "Real Conv",
            "mapping": {
                "root": {"id": "root", "parent": null, "children": ["n1"], "message": null},
                "n1": {
                    "id": "n1", "parent": "root", "children": [],
                    "message": {
                        "author": {"role": "user"},
                        "content": {"content_type": "text", "parts": ["Hi"]},
                        "create_time": 1700001000.0
                    }
                }
            }
        }
    ]);

    let convs = parse_conversations_json_str(&json.to_string()).unwrap();
    assert_eq!(convs.len(), 1);
    assert_eq!(convs[0].title, "Real Conv");
}

#[test]
fn test_parse_chatgpt_json_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let json_path = tmp.path().join("conversations.json");

    let json = serde_json::json!([{
        "title": "File Test",
        "mapping": {
            "root": {"id": "root", "parent": null, "children": ["n1"], "message": null},
            "n1": {
                "id": "n1", "parent": "root", "children": [],
                "message": {
                    "author": {"role": "user"},
                    "content": {"content_type": "text", "parts": ["From file"]},
                    "create_time": 1700000000.0
                }
            }
        }
    }]);

    fs::write(&json_path, json.to_string()).unwrap();
    let convs = parse_chatgpt_json(&json_path).unwrap();
    assert_eq!(convs.len(), 1);
    assert_eq!(convs[0].messages[0].content, "From file");
}

#[test]
fn test_parse_chatgpt_zip() {
    let tmp = tempfile::TempDir::new().unwrap();
    let zip_path = tmp.path().join("chatgpt-export.zip");

    let json = serde_json::json!([{
        "title": "Zip Test",
        "mapping": {
            "root": {"id": "root", "parent": null, "children": ["n1"], "message": null},
            "n1": {
                "id": "n1", "parent": "root", "children": ["n2"],
                "message": {
                    "author": {"role": "user"},
                    "content": {"content_type": "text", "parts": ["From zip"]},
                    "create_time": 1700000000.0
                }
            },
            "n2": {
                "id": "n2", "parent": "n1", "children": [],
                "message": {
                    "author": {"role": "assistant"},
                    "content": {"content_type": "text", "parts": ["Zip reply"]},
                    "create_time": 1700000001.0
                }
            }
        }
    }]);

    let file = fs::File::create(&zip_path).unwrap();
    let mut zip_writer = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    use std::io::Write;
    zip_writer
        .start_file("conversations.json", options)
        .unwrap();
    zip_writer.write_all(json.to_string().as_bytes()).unwrap();
    zip_writer.start_file("chat.html", options).unwrap();
    zip_writer.write_all(b"<html>not used</html>").unwrap();
    zip_writer.finish().unwrap();

    let convs = parse_chatgpt_zip(&zip_path).unwrap();
    assert_eq!(convs.len(), 1);
    assert_eq!(convs[0].title, "Zip Test");
    assert_eq!(convs[0].messages.len(), 2);
}

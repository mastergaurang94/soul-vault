//! Tests for vault module.
use serde_json::Value;

use crate::vault::chatgpt_parse::parse_conversation;

type TestNode<'a> = (
    &'a str,
    Option<&'a str>,
    Vec<&'a str>,
    Option<(&'a str, &'a str)>,
);

fn make_conversation_json(title: &str, nodes: Vec<TestNode<'_>>) -> Value {
    let mut mapping = serde_json::Map::new();
    for (id, parent, children, message) in nodes {
        let children_val: Vec<Value> = children
            .iter()
            .map(|c| Value::String(c.to_string()))
            .collect();
        let parent_val = match parent {
            Some(p) => Value::String(p.to_string()),
            None => Value::Null,
        };
        let message_val = match message {
            Some((role, content)) => serde_json::json!({
                "id": format!("msg-{}", id),
                "author": {"role": role},
                "content": {"content_type": "text", "parts": [content]},
                "create_time": 1700000000.0
            }),
            None => Value::Null,
        };
        mapping.insert(
            id.to_string(),
            serde_json::json!({
                "id": id,
                "parent": parent_val,
                "children": children_val,
                "message": message_val,
            }),
        );
    }

    serde_json::json!({
        "title": title,
        "create_time": 1700000000.0,
        "update_time": 1700001000.0,
        "mapping": mapping,
    })
}

#[test]
fn test_parse_simple_conversation() {
    let conv_json = make_conversation_json(
        "Hello Chat",
        vec![
            ("root", None, vec!["n1"], None),
            ("n1", Some("root"), vec!["n2"], Some(("user", "Hello!"))),
            ("n2", Some("n1"), vec![], Some(("assistant", "Hi there!"))),
        ],
    );

    let conv = parse_conversation(&conv_json).unwrap();
    assert_eq!(conv.title, "Hello Chat");
    assert!(conv.created_at.is_some());
    assert_eq!(conv.messages.len(), 2);
    assert_eq!(conv.messages[0].role, "user");
    assert_eq!(conv.messages[0].content, "Hello!");
    assert_eq!(conv.messages[1].role, "assistant");
    assert_eq!(conv.messages[1].content, "Hi there!");
}

#[test]
fn test_parse_conversation_with_system_node() {
    let conv_json = make_conversation_json(
        "System Chat",
        vec![
            ("root", None, vec!["sys"], None),
            (
                "sys",
                Some("root"),
                vec!["n1"],
                Some(("system", "You are helpful")),
            ),
            ("n1", Some("sys"), vec!["n2"], Some(("user", "What's up?"))),
            ("n2", Some("n1"), vec![], Some(("assistant", "Not much!"))),
        ],
    );

    let conv = parse_conversation(&conv_json).unwrap();
    assert_eq!(conv.messages.len(), 2);
    assert_eq!(conv.messages[0].role, "user");
    assert_eq!(conv.messages[1].role, "assistant");
}

#[test]
fn test_parse_conversation_null_messages() {
    let conv_json = make_conversation_json(
        "Null Messages",
        vec![
            ("root", None, vec!["n1"], None),
            ("n1", Some("root"), vec!["n2"], None),
            (
                "n2",
                Some("n1"),
                vec![],
                Some(("user", "Only real message")),
            ),
        ],
    );

    let conv = parse_conversation(&conv_json).unwrap();
    assert_eq!(conv.messages.len(), 1);
    assert_eq!(conv.messages[0].content, "Only real message");
}

#[test]
fn test_parse_conversation_empty_parts() {
    let mut conv_json = make_conversation_json(
        "Empty Parts",
        vec![
            ("root", None, vec!["n1"], None),
            ("n1", Some("root"), vec![], Some(("user", "Real text"))),
        ],
    );

    if let Some(mapping) = conv_json.get_mut("mapping").and_then(|m| m.as_object_mut()) {
        mapping.insert(
            "n-empty".to_string(),
            serde_json::json!({
                "id": "n-empty",
                "parent": "root",
                "children": ["n1"],
                "message": {
                    "author": {"role": "user"},
                    "content": {"content_type": "text", "parts": []}
                }
            }),
        );
        if let Some(root) = mapping.get_mut("root") {
            root["children"] = serde_json::json!(["n-empty"]);
        }
    }

    let conv = parse_conversation(&conv_json).unwrap();
    assert_eq!(conv.messages.len(), 1);
    assert_eq!(conv.messages[0].content, "Real text");
}

#[test]
fn test_parse_conversation_non_text_parts() {
    let mut conv_json =
        make_conversation_json("Mixed Parts", vec![("root", None, vec!["n1"], None)]);

    if let Some(mapping) = conv_json.get_mut("mapping").and_then(|m| m.as_object_mut()) {
        mapping.insert(
            "n1".to_string(),
            serde_json::json!({
                "id": "n1",
                "parent": "root",
                "children": [],
                "message": {
                    "author": {"role": "user"},
                    "content": {
                        "content_type": "multimodal_text",
                        "parts": [
                            "Here is an image:",
                            {"content_type": "image_asset_pointer", "asset_pointer": "file-xxx"},
                            "And some more text"
                        ]
                    },
                    "create_time": 1700000000.0
                }
            }),
        );
    }

    let conv = parse_conversation(&conv_json).unwrap();
    assert_eq!(conv.messages.len(), 1);
    assert_eq!(
        conv.messages[0].content,
        "Here is an image:\nAnd some more text"
    );
}

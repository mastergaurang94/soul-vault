//! JSON response parsing and validation for LLM extraction output.

use crate::types::ExtractedMemories;
use regex::Regex;

// ─── Response Parsing ─────────────────────────────────────────────────────────

/// Parses raw LLM text into validated ExtractedMemories.
/// Handles markdown fencing, partial JSON, and gracefully degrades.
pub fn parse_extraction_response(text: &str, source: &str, date: &str) -> ExtractedMemories {
    let json_str = extract_json_from_response(text);

    match try_parse(&json_str, source, date) {
        Ok(memories) => memories,
        Err(e) => {
            eprintln!(
                "  [warn] Failed to parse LLM response for \"{}\": {}",
                source, e
            );
            ExtractedMemories::default()
        }
    }
}

fn try_parse(json_str: &str, source: &str, date: &str) -> Result<ExtractedMemories, String> {
    let raw: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("JSON parse error: {}", e))?;

    // Parse into a lenient structure, then enrich with source/date
    let mut memories: ExtractedMemories =
        serde_json::from_value(raw).map_err(|e| format!("Schema error: {}", e))?;

    // Enrich all facts with source and date
    for fact in &mut memories.identity {
        if fact.source.is_empty() {
            fact.source = source.to_string();
        }
        if fact.date.is_empty() {
            fact.date = date.to_string();
        }
    }
    for fact in &mut memories.preferences {
        if fact.source.is_empty() {
            fact.source = source.to_string();
        }
        if fact.date.is_empty() {
            fact.date = date.to_string();
        }
    }
    for fact in &mut memories.decisions {
        if fact.source.is_empty() {
            fact.source = source.to_string();
        }
        if fact.date.is_empty() {
            fact.date = date.to_string();
        }
    }
    for fact in &mut memories.relationships {
        if fact.source.is_empty() {
            fact.source = source.to_string();
        }
        if fact.date.is_empty() {
            fact.date = date.to_string();
        }
    }
    for fact in &mut memories.topics {
        if fact.source.is_empty() {
            fact.source = source.to_string();
        }
        if fact.date.is_empty() {
            fact.date = date.to_string();
        }
    }
    for fact in &mut memories.emotional_context {
        if fact.source.is_empty() {
            fact.source = source.to_string();
        }
        if fact.date.is_empty() {
            fact.date = date.to_string();
        }
    }

    Ok(memories)
}

/// Extracts JSON from a response that may contain markdown fencing.
fn extract_json_from_response(text: &str) -> String {
    let trimmed = text.trim();

    // Try to extract from ```json ... ``` or ``` ... ```
    let re = Regex::new(r"```(?:json)?\s*\n?([\s\S]*?)\n?```").unwrap();
    if let Some(caps) = re.captures(trimmed) {
        return caps[1].trim().to_string();
    }

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_response() {
        let json = r#"{
            "identity": [
                {"content": "Name is Gaurang", "category": "name", "confidence": "high"}
            ],
            "preferences": [
                {"content": "Likes tea", "type": "like", "confidence": "medium"}
            ],
            "decisions": [],
            "relationships": [
                {"person": "Avni", "content": "His daughter", "role": "daughter", "confidence": "high"}
            ],
            "topics": [
                {"topic": "Rust", "content": "Learning Rust for CLI tools", "confidence": "medium"}
            ],
            "emotional_context": []
        }"#;

        let m = parse_extraction_response(json, "test-source", "2026-02-14");
        assert_eq!(m.identity.len(), 1);
        assert_eq!(m.identity[0].content, "Name is Gaurang");
        assert_eq!(m.identity[0].source, "test-source");
        assert_eq!(m.identity[0].date, "2026-02-14");
        assert_eq!(m.preferences.len(), 1);
        assert_eq!(m.relationships.len(), 1);
        assert_eq!(m.topics.len(), 1);
    }

    #[test]
    fn test_parse_with_markdown_fencing() {
        let response = r#"```json
{
    "identity": [{"content": "Lives in Houston", "category": "location", "confidence": "high"}],
    "preferences": [],
    "decisions": [],
    "relationships": [],
    "topics": [],
    "emotional_context": []
}
```"#;

        let m = parse_extraction_response(response, "test", "2026-02-14");
        assert_eq!(m.identity.len(), 1);
        assert_eq!(m.identity[0].content, "Lives in Houston");
    }

    #[test]
    fn test_parse_empty_categories() {
        let json = r#"{
            "identity": [],
            "preferences": [],
            "decisions": [],
            "relationships": [],
            "topics": [],
            "emotional_context": []
        }"#;

        let m = parse_extraction_response(json, "test", "2026-02-14");
        assert!(m.is_empty());
    }

    #[test]
    fn test_parse_missing_categories() {
        let json =
            r#"{"identity": [{"content": "Test", "category": "other", "confidence": "low"}]}"#;
        let m = parse_extraction_response(json, "test", "2026-02-14");
        assert_eq!(m.identity.len(), 1);
        assert!(m.preferences.is_empty());
    }

    #[test]
    fn test_parse_invalid_json() {
        let m = parse_extraction_response("not json at all", "test", "2026-02-14");
        assert!(m.is_empty());
    }

    #[test]
    fn test_parse_garbled_response() {
        let m =
            parse_extraction_response("Here is the extracted data: {invalid", "test", "2026-02-14");
        assert!(m.is_empty());
    }

    #[test]
    fn test_extract_json_from_plain() {
        let input = r#"  {"identity": []}  "#;
        let result = extract_json_from_response(input);
        assert_eq!(result, r#"{"identity": []}"#);
    }

    #[test]
    fn test_extract_json_from_fenced() {
        let input = "```json\n{\"identity\": []}\n```";
        let result = extract_json_from_response(input);
        assert_eq!(result, "{\"identity\": []}");
    }
}

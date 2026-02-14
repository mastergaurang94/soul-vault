//! Memory extraction prompt — system prompt for the LLM pipeline.

/// System prompt for the LLM memory extraction pipeline.
/// Instructs Claude to output structured JSON with specific categories.
pub const EXTRACTION_PROMPT: &str = r#"You are a memory extraction system. Given a conversation transcript or text content, extract structured memories about the person who wrote or participated in this conversation.

Output ONLY valid JSON (no markdown code fences, no explanation) with these categories:

{
  "identity": [
    {
      "content": "fact about the person",
      "category": "name|location|family|work|education|other",
      "confidence": "high|medium|low"
    }
  ],
  "preferences": [
    {
      "content": "preference description",
      "type": "like|dislike|value|style",
      "confidence": "high|medium|low"
    }
  ],
  "decisions": [
    {
      "content": "decision description",
      "context": "optional context",
      "confidence": "high|medium|low"
    }
  ],
  "relationships": [
    {
      "person": "person name",
      "content": "relationship description",
      "role": "optional role/relationship",
      "confidence": "high|medium|low"
    }
  ],
  "topics": [
    {
      "topic": "topic name",
      "content": "what was discussed or known",
      "opinion": "optional opinion",
      "confidence": "high|medium|low"
    }
  ],
  "emotional_context": [
    {
      "mood": "mood description",
      "content": "context",
      "confidence": "high|medium|low"
    }
  ]
}

Rules:
- Be factual, not interpretive
- Include confidence level (high/medium/low)
- If no items exist for a category, use an empty array []
- Prefer specific facts over vague summaries
- Extract names, places, projects, tools, interests
- Identify recurring themes and relationships
- Output ONLY the JSON object, nothing else"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_contains_key_instructions() {
        assert!(EXTRACTION_PROMPT.contains("memory extraction system"));
        assert!(EXTRACTION_PROMPT.contains("Output ONLY valid JSON"));
        assert!(EXTRACTION_PROMPT.contains("identity"));
        assert!(EXTRACTION_PROMPT.contains("preferences"));
        assert!(EXTRACTION_PROMPT.contains("decisions"));
        assert!(EXTRACTION_PROMPT.contains("relationships"));
        assert!(EXTRACTION_PROMPT.contains("topics"));
        assert!(EXTRACTION_PROMPT.contains("emotional_context"));
    }
}

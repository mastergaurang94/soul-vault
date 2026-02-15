//! Formatting for parsed ChatGPT conversations.

use crate::vault::chatgpt_types::ParsedConversation;

/// Formats parsed conversations into readable text matching local import output.
pub fn format_conversations(conversations: &[ParsedConversation]) -> String {
    conversations
        .iter()
        .map(format_single_conversation)
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

fn format_single_conversation(conv: &ParsedConversation) -> String {
    let mut lines = Vec::with_capacity(conv.messages.len() + 2);
    lines.push(format!("## {}", conv.title));
    lines.push(String::new());

    for msg in &conv.messages {
        lines.push(format!("{}: {}", msg.role, msg.content));
    }

    lines.join("\n")
}

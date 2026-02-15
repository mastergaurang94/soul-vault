//! Provider and confidence enums with display/parse helpers.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    #[serde(alias = "chatgpt")]
    ChatGpt,
    Gemini,
}

impl std::str::FromStr for Provider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "claude" => Ok(Provider::Claude),
            "chatgpt" | "chat_gpt" | "chat-gpt" | "openai" => Ok(Provider::ChatGpt),
            "gemini" | "google" => Ok(Provider::Gemini),
            other => Err(format!(
                "Unknown provider: {}. Use one of: claude, chatgpt, gemini.",
                other
            )),
        }
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::Claude => write!(f, "claude"),
            Provider::ChatGpt => write!(f, "chatgpt"),
            Provider::Gemini => write!(f, "gemini"),
        }
    }
}

impl Provider {
    pub fn display_name(&self) -> &str {
        match self {
            Provider::Claude => "Claude",
            Provider::ChatGpt => "ChatGPT",
            Provider::Gemini => "Gemini",
        }
    }

    pub fn api_key_hint(&self) -> &str {
        match self {
            Provider::Claude => "sk-ant-...",
            Provider::ChatGpt => "sk-...",
            Provider::Gemini => "AI...",
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    #[default]
    Medium,
    Low,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::High => write!(f, "high"),
            Confidence::Medium => write!(f, "medium"),
            Confidence::Low => write!(f, "low"),
        }
    }
}

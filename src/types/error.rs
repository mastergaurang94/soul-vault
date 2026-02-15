//! Typed domain errors with actionable remediation hints.

#[derive(Debug, thiserror::Error)]
pub enum SoulVaultError {
    #[error("Soul Vault not initialized.\n      → Run `soul init` to create your vault.")]
    NotInitialized,

    #[error(
        "No API key found for {provider}.\n      → Run `soul init` to configure your API key."
    )]
    MissingApiKey { provider: String },

    #[error("Failed to parse LLM response: {reason}")]
    #[allow(dead_code)]
    ParseError { reason: String },

    #[error("Path not found: {path}\n      → Check the path and try again.")]
    PathNotFound { path: String },

    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Rate limited. Waiting before retry...")]
    RateLimited,

    #[error("{0}")]
    #[allow(dead_code)]
    Other(String),
}

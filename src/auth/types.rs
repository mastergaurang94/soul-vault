use crate::types::Provider;

#[derive(Debug, Clone)]
pub struct AuthCredentials {
    pub provider: Provider,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Default)]
pub(super) struct AuthStore {
    pub(super) credentials: Vec<AuthCredentials>,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct TokenResponse {
    pub(super) access_token: Option<String>,
    pub(super) refresh_token: Option<String>,
    pub(super) expires_in: Option<i64>,
    pub(super) expires_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub provider: Provider,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub scope: String,
}

//! OAuth credential storage and token lifecycle management.

mod connect;
mod oauth;
mod store;
mod types;

#[allow(unused_imports)]
pub use connect::connect_provider;
#[allow(unused_imports)]
pub use oauth::{exchange_code_for_token, oauth_config, oauth_is_configured, refresh_access_token};
#[allow(unused_imports)]
pub use store::{
    auth_path, clear_credentials, ensure_valid_credentials, is_logged_in, load_credentials,
    remove_credentials, save_credentials,
};
#[allow(unused_imports)]
pub use types::{AuthCredentials, OAuthConfig};

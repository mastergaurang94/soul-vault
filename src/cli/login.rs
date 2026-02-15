//! `soul login` — OAuth login flow for cloud providers.

use anyhow::{bail, Context, Result};

use crate::auth::{exchange_code_for_token, oauth_config, save_credentials};
use crate::cli::login_oauth::{
    open_browser, percent_encode, spawn_callback_listener, CALLBACK_TIMEOUT,
};
use crate::types::Provider;
use crate::ui::theme::*;
use crate::vault::config::assert_initialized;

pub async fn run(provider: Option<&str>) -> Result<()> {
    assert_initialized()?;
    let provider = parse_provider(provider)?;

    println!("{}", banner());
    println!(
        "  {} Logging into {} via OAuth\n",
        ICON_KEY,
        bold_white(provider.display_name())
    );
    println!("{}", line());

    match provider {
        Provider::Claude => login_claude().await,
        Provider::ChatGpt | Provider::Gemini => {
            println!(
                "  {} OAuth scaffold for {} is in place but provider wiring is pending.",
                amber(ICON_STAR),
                provider.display_name()
            );
            println!(
                "  {} Coming soon — for now use {}.\n",
                dim(ICON_DOT),
                cyan("soul import <your-export-folder>")
            );
            Ok(())
        }
    }
}

fn parse_provider(raw: Option<&str>) -> Result<Provider> {
    match raw {
        Some(value) => value.parse::<Provider>().map_err(anyhow::Error::msg),
        None => Ok(Provider::Claude),
    }
}

async fn login_claude() -> Result<()> {
    let oauth = oauth_config(&Provider::Claude);
    if oauth.client_id.contains("placeholder") {
        println!(
            "  {} Using placeholder client ID. Set {} for real login.",
            amber(ICON_STAR),
            cyan("SOUL_OAUTH_CLAUDE_CLIENT_ID")
        );
    }

    let state = format!("soul-{}", chrono::Utc::now().timestamp_millis());
    let (port, callback_rx) = spawn_callback_listener()?;
    let redirect_uri = format!("http://127.0.0.1:{port}/oauth/callback");

    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
        oauth.auth_url,
        percent_encode(&oauth.client_id),
        percent_encode(&redirect_uri),
        percent_encode(&oauth.scope),
        percent_encode(&state)
    );

    println!("  {} Opening browser for OAuth consent...", dim(ICON_DOT));
    open_browser(&auth_url)?;
    println!(
        "  {} Waiting for callback on {} ({}s timeout)\n",
        dim(ICON_DOT),
        cyan(&format!("127.0.0.1:{port}")),
        CALLBACK_TIMEOUT.as_secs()
    );

    let callback = callback_rx
        .recv_timeout(CALLBACK_TIMEOUT)
        .context("Timed out waiting for OAuth callback. Try `soul login` again.")??;

    if callback.state != state {
        bail!("OAuth state mismatch. Please retry `soul login`.");
    }

    let credentials = exchange_code_for_token(&oauth, &callback.code, &redirect_uri).await?;
    save_credentials(&credentials)?;

    println!(
        "{}",
        check("OAuth login successful. Credentials saved to ~/soul-vault/auth.yaml")
    );
    println!(
        "  {} Use {} to test cloud import scaffolding.\n",
        dim(ICON_DOT),
        cyan("soul import --cloud")
    );

    Ok(())
}

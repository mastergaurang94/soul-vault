//! `soul login` — OAuth login flow for cloud providers.

use anyhow::Result;

use crate::auth::connect_provider;
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

    connect_provider(&provider).await?;
    println!(
        "{}",
        check("OAuth login successful. Credentials saved to ~/soul-vault/auth.yaml")
    );
    println!(
        "  {} Use {} to import cloud conversations.\n",
        dim(ICON_DOT),
        cyan(&format!("soul import --cloud --provider {}", provider))
    );
    Ok(())
}

fn parse_provider(raw: Option<&str>) -> Result<Provider> {
    match raw {
        Some(value) => value.parse::<Provider>().map_err(anyhow::Error::msg),
        None => anyhow::bail!("Provider required. Use `soul login <claude|chatgpt|gemini>`."),
    }
}

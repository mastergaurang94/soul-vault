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

    match provider {
        Provider::Claude => {
            connect_provider(&Provider::Claude).await?;
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

//! `soul logout` — remove stored OAuth credentials.

use anyhow::Result;

use crate::auth::{clear_credentials, remove_credentials};
use crate::types::Provider;
use crate::ui::theme::*;
use crate::vault::config::assert_initialized;

pub fn run(provider: Option<&str>) -> Result<()> {
    assert_initialized()?;

    println!("{}", banner());

    if let Some(raw) = provider {
        let parsed = raw.parse::<Provider>().map_err(anyhow::Error::msg)?;
        let removed = remove_credentials(&parsed)?;
        if removed {
            println!(
                "{}",
                check(&format!("Logged out from {}.", parsed.display_name()))
            );
        } else {
            println!(
                "  {} No saved OAuth credentials for {}.",
                dim(ICON_DOT),
                parsed.display_name()
            );
        }
    } else {
        let removed = clear_credentials()?;
        if removed {
            println!("{}", check("Logged out from all providers."));
        } else {
            println!("  {} No saved OAuth credentials found.", dim(ICON_DOT));
        }
    }

    println!();
    Ok(())
}

//! Unified `soul import` command.
//!
//! - `soul import` -> provider auto-discovery import
//! - `soul import <folder>` -> local folder import

use anyhow::Result;

pub async fn run(
    folder: Option<&str>,
    force: bool,
    cloud: bool,
    provider: Option<&str>,
) -> Result<()> {
    match folder {
        Some(path) => {
            if cloud {
                anyhow::bail!(
                    "`--cloud` is only valid for provider import mode.\n      \
                     → Run `soul import --cloud` (without a folder) or remove `--cloud`."
                );
            }
            if provider.is_some() {
                anyhow::bail!(
                    "`--provider` is only valid for provider import mode.\n      \
                     → Run `soul import --provider <claude|chatgpt|gemini>` (without a folder)."
                );
            }
            crate::cli::ingest::run(path, force).await
        }
        None => crate::cli::pull::run(force, cloud, provider).await,
    }
}

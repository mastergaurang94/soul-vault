//! `soul init` — first-time setup wizard.

use anyhow::Result;
use std::io::{self, Write};

use crate::auth::{connect_provider, is_logged_in};
use crate::cli::init_validate::{validate_api_key, ApiKeyValidation};
use crate::types::{Provider, ProviderConfig, SoulVaultConfig};
use crate::ui::theme::*;
use crate::vault::config::{
    create_default_files, create_gitignore, create_vault_structure, get_api_key, is_initialized,
    set_api_key, set_key_health, vault_root, write_config, ApiKeyHealth,
};

// ─── Init Command ─────────────────────────────────────────────────────────────

pub async fn run() -> Result<()> {
    println!("{}", banner());
    println!("{}", dim("  First-time setup wizard\n"));
    println!("{}", line());

    if is_initialized() {
        println!(
            "\n  {} Soul Vault already initialized at {}",
            amber(ICON_STAR),
            cyan(&vault_root().display().to_string())
        );
        let answer = ask(&format!(
            "  Reinitialize? This won't delete existing memories. {} ",
            dim("(y/N)")
        ))?;
        if answer.to_lowercase() != "y" {
            println!("{}", dim("\n  Cancelled.\n"));
            return Ok(());
        }
        println!();
    }

    // Step 1: Create vault structure
    print!("  Creating vault structure... ");
    io::stdout().flush()?;
    create_vault_structure()?;
    create_gitignore()?;
    create_default_files()?;
    println!("{}", check("Vault structure created"));

    // Step 2: Provider setup
    println!("\n{}", gold("  Configure providers:"));
    println!(
        "{}",
        dim("  Choose one provider at a time, then select Done when finished.\n")
    );
    let mut providers = default_provider_configs();
    configure_providers_loop(&mut providers).await?;

    // Step 3: Processing LLM selection
    println!("\n{}", gold("  Select processing LLM:"));
    println!(
        "{}",
        dim("  This is the AI that will extract memories from your conversations.\n")
    );

    let llm_options = [Provider::Claude, Provider::ChatGpt, Provider::Gemini];
    let llm_labels = ["Claude", "ChatGPT", "Gemini"];

    for (i, label) in llm_labels.iter().enumerate() {
        println!("    {} {}", dim(&format!("{}.", i + 1)), label);
    }

    let choice = ask(&format!("\n  Choose {} ", dim("(1-3, default: 1)")))?;
    let llm_index: usize = choice.trim().parse().unwrap_or(1);
    let processing_llm = if (1..=3).contains(&llm_index) {
        llm_options[llm_index - 1].clone()
    } else {
        Provider::Claude
    };
    println!(
        "{}",
        check(&format!(
            "Processing LLM: {}",
            processing_llm.display_name()
        ))
    );

    // Step 4: Ensure processing LLM is configured
    if !provider_has_credentials(&processing_llm) {
        println!(
            "\n  {} {} is selected as processing LLM but has no credentials.",
            amber(ICON_STAR),
            processing_llm.display_name()
        );
        let answer = ask(&format!(
            "  Configure {} now? {} ",
            processing_llm.display_name(),
            dim("(Y/n)")
        ))?;
        if answer.trim().to_lowercase() != "n" {
            configure_single_provider(&processing_llm, &mut providers).await?;
        }
    }

    // Step 5: Final setup summary + confirmation
    render_setup_summary(&providers, &processing_llm);
    let finish = ask(&format!(
        "\n  Finish setup and save configuration? {} ",
        dim("(Y/n)")
    ))?;
    if finish.trim().to_lowercase() == "n" {
        println!(
            "\n  {} Setup not finalized. Your vault folders and any entered credentials were kept.",
            dim(ICON_DOT)
        );
        println!(
            "  {} Run {} again to finish setup.\n",
            dim(ICON_DOT),
            cyan("soul init")
        );
        return Ok(());
    }

    // Step 6: Save config
    print!("  Saving configuration... ");
    io::stdout().flush()?;
    let config = SoulVaultConfig {
        providers,
        processing_llm,
        vault_path: vault_root().display().to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_sync: None,
    };
    write_config(&config)?;
    println!("{}", check("Configuration saved"));

    // Done!
    println!("\n{}", line());
    println!(
        "\n  {} {} {}",
        amber(ICON_STAR),
        bold_gold("Soul Vault initialized!"),
        dim(&format!("→ {}", vault_root().display()))
    );
    println!("\n{}", dim("  Next steps:"));
    let next_steps: &[(&str, &str)] = &[
        ("soul import <folder>", "Import your AI conversations"),
        ("soul status", "Check your vault"),
        ("soul export", "Output context for any AI"),
    ];
    let col_width = 24; // visible width for command column
    for (cmd, desc) in next_steps {
        let pad = if cmd.len() < col_width {
            col_width - cmd.len()
        } else {
            2
        };
        println!("    {}{}{}", cyan(cmd), " ".repeat(pad), dim(desc));
    }
    println!();

    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn default_provider_configs() -> Vec<ProviderConfig> {
    vec![
        ProviderConfig {
            name: Provider::Claude,
            enabled: false,
            last_import: None,
        },
        ProviderConfig {
            name: Provider::ChatGpt,
            enabled: false,
            last_import: None,
        },
        ProviderConfig {
            name: Provider::Gemini,
            enabled: false,
            last_import: None,
        },
    ]
}

async fn configure_providers_loop(providers: &mut [ProviderConfig]) -> Result<()> {
    loop {
        print_provider_menu(providers);
        let choice = ask(&format!(
            "\n  Select provider {} ",
            dim("(1-4, default: 4)")
        ))?;
        let idx: usize = choice.trim().parse().unwrap_or(4);
        match idx {
            1 => configure_single_provider(&Provider::Claude, providers).await?,
            2 => configure_single_provider(&Provider::ChatGpt, providers).await?,
            3 => configure_single_provider(&Provider::Gemini, providers).await?,
            4 => break,
            _ => println!("  {} Choose 1, 2, 3, or 4.", amber(ICON_STAR)),
        }
    }
    Ok(())
}

fn print_provider_menu(providers: &[ProviderConfig]) {
    println!("\n{}", line());
    println!("{}", gold("  Providers"));
    for (i, provider) in [Provider::Claude, Provider::ChatGpt, Provider::Gemini]
        .iter()
        .enumerate()
    {
        let status = provider_menu_status(provider, providers);
        println!(
            "    {} {:<8} {}",
            dim(&format!("{}.", i + 1)),
            provider.display_name(),
            status
        );
    }
    println!("    {} Done", dim("4."));
}

fn provider_menu_status(provider: &Provider, providers: &[ProviderConfig]) -> String {
    if provider_has_credentials(provider) {
        return emerald("configured");
    }
    if provider_enabled(providers, provider) {
        return amber("enabled");
    }
    dim("not set")
}

fn provider_enabled(providers: &[ProviderConfig], provider: &Provider) -> bool {
    providers
        .iter()
        .find(|p| p.name == *provider)
        .map(|p| p.enabled)
        .unwrap_or(false)
}

fn set_provider_enabled(providers: &mut [ProviderConfig], provider: &Provider, enabled: bool) {
    if let Some(entry) = providers.iter_mut().find(|p| p.name == *provider) {
        entry.enabled = enabled;
    }
}

async fn configure_single_provider(
    provider: &Provider,
    providers: &mut [ProviderConfig],
) -> Result<()> {
    loop {
        println!("\n{}", line());
        println!(
            "  {} {}",
            gold("Configure provider:"),
            bold_white(provider.display_name())
        );
        println!("    {} API key", dim("1."));
        if supports_oauth(provider) {
            println!("    {} OAuth", dim("2."));
            println!("    {} Back", dim("3."));
        } else {
            println!("    {} OAuth {}", dim("2."), dim("(coming soon)"));
            println!("    {} Back", dim("3."));
        }

        let choice = ask(&format!(
            "  Select auth method {} ",
            dim("(1-3, default: 3)")
        ))?;
        let option: usize = choice.trim().parse().unwrap_or(3);
        match option {
            1 => {
                let saved = setup_api_key(provider).await?;
                if saved {
                    set_provider_enabled(providers, provider, true);
                }
                return Ok(());
            }
            2 => {
                if !supports_oauth(provider) {
                    println!(
                        "  {} OAuth for {} is coming soon.",
                        amber(ICON_STAR),
                        provider.display_name()
                    );
                    continue;
                }
                println!(
                    "  {} Starting OAuth for {}...",
                    dim(ICON_DOT),
                    provider.display_name()
                );
                match connect_provider(provider).await {
                    Ok(()) => {
                        println!(
                            "{}",
                            check(&format!("Connected {}.", provider.display_name()))
                        );
                        set_provider_enabled(providers, provider, true);
                    }
                    Err(e) => println!("  {} {}", red("✗"), e),
                }
                return Ok(());
            }
            3 => return Ok(()),
            _ => println!("  {} Choose 1, 2, or 3.", amber(ICON_STAR)),
        }
    }
}

async fn setup_api_key(provider: &Provider) -> Result<bool> {
    println!(
        "{}",
        dim("  Keys are stored locally in ~/soul-vault/.config/keys.json")
    );
    loop {
        let key_input = ask(&format!(
            "    {} {} API key {} ",
            ICON_KEY,
            provider.display_name(),
            dim(&format!("({})", provider.api_key_hint()))
        ))?;
        let key = key_input.trim();
        if key.is_empty() {
            println!(
                "    {} {} key skipped (you can add it later)",
                dim(ICON_DOT),
                provider.display_name()
            );
            return Ok(false);
        }

        print!("    {} Validating key... ", dim(ICON_DOT));
        io::stdout().flush()?;
        match validate_api_key(provider, key).await {
            ApiKeyValidation::Verified => {
                println!("{}", check("valid"));
                set_api_key(&provider.to_string(), key)?;
                set_key_health(provider, ApiKeyHealth::Verified, None)?;
                println!(
                    "{}",
                    check(&format!("{} key saved", provider.display_name()))
                );
                return Ok(true);
            }
            ApiKeyValidation::Unverified(reason) => {
                println!("{}", amber("unverified"));
                println!("      {} {}", amber("!"), dim(&reason));
                set_api_key(&provider.to_string(), key)?;
                set_key_health(provider, ApiKeyHealth::Unverified, Some(reason.clone()))?;
                println!(
                    "{}",
                    check(&format!("{} key saved", provider.display_name()))
                );
                return Ok(true);
            }
            ApiKeyValidation::Invalid(reason) => {
                println!("{}", red("invalid"));
                println!("      {} {}", red("✗"), reason);
                set_key_health(provider, ApiKeyHealth::Invalid, Some(reason.clone()))?;
                let retry = ask(&format!(
                    "      Re-enter {} key? {} ",
                    provider.display_name(),
                    dim("(Y/n)")
                ))?;
                if retry.trim().to_lowercase() == "n" {
                    return Ok(false);
                }
            }
        }
    }
}

fn supports_oauth(provider: &Provider) -> bool {
    matches!(provider, Provider::Claude)
}

fn provider_has_credentials(provider: &Provider) -> bool {
    let has_key = get_api_key(&provider.to_string())
        .ok()
        .flatten()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    let has_oauth = is_logged_in(provider).unwrap_or(false);
    has_key || has_oauth
}

fn render_setup_summary(providers: &[ProviderConfig], processing_llm: &Provider) {
    println!("\n{}", line());
    println!("{}", gold("  Setup summary"));
    for provider in [Provider::Claude, Provider::ChatGpt, Provider::Gemini] {
        let status = provider_setup_status(&provider, providers);
        println!("    {:<8} {}", provider.display_name(), status);
    }
    println!(
        "    {:<8} {}",
        "Processor",
        bold_white(processing_llm.display_name())
    );
}

fn provider_setup_status(provider: &Provider, providers: &[ProviderConfig]) -> String {
    if is_logged_in(provider).unwrap_or(false) {
        return emerald("Connected");
    }

    let has_key = get_api_key(&provider.to_string())
        .ok()
        .flatten()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    if has_key {
        return emerald("API key set");
    }

    if provider_enabled(providers, provider) {
        return amber("Enabled");
    }

    dim("Skipped")
}

fn ask(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_provider_configs_contains_all_disabled() {
        let providers = default_provider_configs();
        assert_eq!(providers.len(), 3);
        assert!(providers.iter().all(|p| !p.enabled));
    }

    #[test]
    fn set_provider_enabled_updates_target_only() {
        let mut providers = default_provider_configs();
        set_provider_enabled(&mut providers, &Provider::Gemini, true);
        assert!(!provider_enabled(&providers, &Provider::Claude));
        assert!(!provider_enabled(&providers, &Provider::ChatGpt));
        assert!(provider_enabled(&providers, &Provider::Gemini));
    }

    #[test]
    fn provider_setup_status_shows_enabled_without_credentials() {
        let mut providers = default_provider_configs();
        set_provider_enabled(&mut providers, &Provider::Claude, true);
        assert_eq!(
            provider_setup_status(&Provider::Claude, &providers),
            amber("Enabled")
        );
    }
}

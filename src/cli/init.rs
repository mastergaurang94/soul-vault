//! `soul init` — first-time setup wizard.

use anyhow::Result;
use crossterm::{
    cursor,
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use std::io::{self, IsTerminal, Write};

use crate::auth::{connect_provider, is_logged_in, oauth_connect_available, save_setup_token};
use crate::cli::init_validate::{validate_api_key, ApiKeyValidation};
use crate::types::{ProcessingMode, Provider, ProviderConfig, SoulVaultConfig};
use crate::ui::theme::*;
use crate::vault::config::{
    create_default_files, create_gitignore, create_vault_structure, get_api_key, is_initialized,
    set_api_key, set_key_health, vault_root, write_config, ApiKeyHealth,
};

// ─── Init Command ─────────────────────────────────────────────────────────────

pub async fn run() -> Result<()> {
    run_with_banner(true).await
}

pub async fn run_without_banner() -> Result<()> {
    run_with_banner(false).await
}

async fn run_with_banner(show_banner: bool) -> Result<()> {
    if show_banner {
        println!("{}", banner());
        println!("{}", dim("  First-time setup wizard\n"));
        println!("{}", line());
    } else {
        println!("{}", line());
    }

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

    // Prepare vault structure quietly; errors will bubble with actionable context.
    create_vault_structure()?;
    create_gitignore()?;
    create_default_files()?;

    // Step 1: Provider setup
    let mut providers = default_provider_configs();
    configure_providers_loop(&mut providers).await?;

    // Step 2: Processing mode selection
    let mut processing_mode = select_processing_mode(&mut providers).await?;

    // Step 5: Final setup summary + confirmation
    loop {
        render_finalize_screen(&providers, &processing_mode)?;
        let finish = ask(&format!(
            "\n  Finish setup and save configuration? {} ",
            dim("(Y/n)")
        ))?;
        if finish.trim().to_lowercase() != "n" {
            break;
        }

        println!("\n  What would you like to do next?");
        println!("    {} Configure providers", dim("1."));
        println!("    {} Change processing mode", dim("2."));
        println!("    {} Cancel setup", dim("3."));
        let next = ask(&format!("\n  Choose {} ", dim("(1-3, default: 1)")))?;
        match next.trim() {
            "2" => {
                processing_mode = select_processing_mode(&mut providers).await?;
            }
            "3" => {
                println!(
                    "\n  Setup not finalized. Your vault folders and any entered credentials were kept."
                );
                println!("  Run {} again to finish setup.\n", cyan("soul init"));
                return Ok(());
            }
            _ => {
                configure_providers_loop(&mut providers).await?;
            }
        }
    }

    // Step 6: Save config
    print!("  Saving configuration... ");
    io::stdout().flush()?;
    let config = SoulVaultConfig {
        providers,
        processing_mode,
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
        render_provider_setup_screen(providers)?;
        println!();
        let choice = ask("  Choose a provider [1-4] (Enter = Done): ")?;
        match parse_provider_choice(&choice) {
            ProviderChoice::Provider(provider) => {
                configure_single_provider(&provider, providers).await?
            }
            ProviderChoice::Done => break,
            ProviderChoice::Invalid => println!(
                "  {} Use 1-4, provider name (claude/chatgpt/gemini), or Enter for Done.",
                amber(ICON_STAR)
            ),
        }
    }
    Ok(())
}

async fn select_processing_mode(providers: &mut [ProviderConfig]) -> Result<ProcessingMode> {
    println!("\n{}", gold("  Step 2/2: Select processing mode"));
    println!();
    println!(
        "{}",
        dim("  This is the AI that will extract memories from your conversations.\n")
    );

    let llm_options = [Provider::Claude, Provider::ChatGpt, Provider::Gemini];
    let llm_labels = ["Claude", "ChatGPT", "Gemini"];

    for (i, label) in llm_labels.iter().enumerate() {
        println!("    {} {}", dim(&format!("{}.", i + 1)), label);
    }
    println!(
        "    {} Soul Vault Cloud {}",
        dim("4."),
        dim("(coming soon)")
    );
    println!("    {} Skip processing {}", dim("5."), dim("(raw mode)"));

    let processing_mode = loop {
        let choice = ask(&format!("\n  Choose {} ", dim("(1-5)")))?;
        let llm_index: usize = match choice.trim().parse() {
            Ok(value) => value,
            Err(_) => {
                println!("  {} Choose 1, 2, 3, 4, or 5.", amber(ICON_STAR));
                continue;
            }
        };

        let candidate_provider = match llm_index {
            1..=3 => llm_options[llm_index - 1].clone(),
            4 => {
                println!(
                    "  {} Soul Vault Cloud processing is coming soon. Please choose 1-3 for now.",
                    amber(ICON_STAR)
                );
                continue;
            }
            5 => {
                println!(
                    "  {} Processing disabled. Soul Vault will keep raw sessions, but memory extraction features will be unavailable",
                    amber(ICON_STAR)
                );
                println!("    until you enable processing.");
                break ProcessingMode::Disabled;
            }
            _ => {
                println!("  {} Choose 1, 2, 3, 4, or 5.", amber(ICON_STAR));
                continue;
            }
        };

        if provider_has_credentials(&candidate_provider) {
            break ProcessingMode::from_provider(&candidate_provider);
        }

        println!(
            "\n  {} {} was selected as processing mode but has no credentials.",
            amber(ICON_STAR),
            candidate_provider.display_name()
        );
        println!();
        let answer = ask(&format!(
            "  Configure {} now? {} ",
            candidate_provider.display_name(),
            dim("(Y/n)")
        ))?;
        if answer.trim().to_lowercase() != "n" {
            configure_single_provider(&candidate_provider, providers).await?;
        }

        if provider_has_credentials(&candidate_provider) {
            break ProcessingMode::from_provider(&candidate_provider);
        }

        println!(
            "  {} {} is still not configured. Choose a processor that is ready.",
            amber(ICON_STAR),
            candidate_provider.display_name()
        );
    };

    println!();
    println!(
        "{}",
        check(&format!("Processing: {}", processing_mode.display_name()))
    );

    Ok(processing_mode)
}

fn render_provider_setup_screen(providers: &[ProviderConfig]) -> Result<()> {
    clear_screen_if_tty()?;
    println!("{}", banner());
    println!("{}", dim("  First-time setup wizard"));
    println!("{}", line());
    println!("\n{}", gold("  Step 1/2: Configure providers"));
    println!();
    println!(
        "{}",
        dim("  Providers are where Soul Vault imports and/or syncs your conversations from.")
    );
    println!(
        "{}",
        dim("  Configure one at a time, then choose Done when finished.")
    );
    println!();
    print_provider_menu(providers);
    Ok(())
}

fn print_provider_menu(providers: &[ProviderConfig]) {
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

enum ProviderChoice {
    Provider(Provider),
    Done,
    Invalid,
}

fn parse_provider_choice(input: &str) -> ProviderChoice {
    let trimmed = input.trim().to_lowercase();
    if trimmed.is_empty() || trimmed == "4" || trimmed == "done" {
        return ProviderChoice::Done;
    }

    match trimmed.as_str() {
        "1" | "claude" => ProviderChoice::Provider(Provider::Claude),
        "2" | "chatgpt" | "chat-gpt" | "chat_gpt" | "openai" => {
            ProviderChoice::Provider(Provider::ChatGpt)
        }
        "3" | "gemini" | "google" => ProviderChoice::Provider(Provider::Gemini),
        _ => ProviderChoice::Invalid,
    }
}

async fn configure_single_provider(
    provider: &Provider,
    providers: &mut [ProviderConfig],
) -> Result<()> {
    loop {
        render_auth_method_screen(provider)?;
        if *provider == Provider::Claude {
            println!("    {} API key", dim("1."));
            println!("    {} Setup-token", dim("2."));
            println!("    {} Back", dim("3."));
        } else {
            println!("    {} API key", dim("1."));
            if supports_oauth(provider) {
                println!("    {} OAuth", dim("2."));
            } else {
                println!("    {} OAuth {}", dim("2."), dim("(not configured)"));
            }
            println!("    {} Back", dim("3."));
        }
        println!();

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
                if *provider == Provider::Claude {
                    let saved = setup_claude_setup_token().await?;
                    if saved {
                        set_provider_enabled(providers, provider, true);
                    }
                    return Ok(());
                }
                if !supports_oauth(provider) {
                    let reason = match provider {
                        Provider::ChatGpt => "Codex CLI (`codex`) is not available.",
                        Provider::Gemini => "Gemini CLI (`gemini`) is not available.",
                        Provider::Claude => "Claude browser OAuth is not configured.",
                    };
                    println!("  {} OAuth unavailable: {}", amber(ICON_STAR), reason);
                    continue;
                }
                println!();
                println!("  Starting OAuth for {}...", provider.display_name());
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

fn render_auth_method_screen(provider: &Provider) -> Result<()> {
    clear_screen_if_tty()?;
    println!("{}", banner());
    println!("{}", dim("  First-time setup wizard"));
    println!("{}", line());
    println!("\n{}", gold("  Step 1/2: Configure providers"));
    println!();
    println!(
        "  {} {}",
        bold_white(provider.display_name()),
        dim("Choose auth method")
    );
    println!();
    Ok(())
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

async fn setup_claude_setup_token() -> Result<bool> {
    println!(
        "{}",
        dim("  Setup-token comes from `claude setup-token` and is saved locally in ~/soul-vault/auth.yaml")
    );
    let token_input = ask(&format!(
        "    {} Paste Claude setup-token {} ",
        ICON_KEY,
        dim("(Enter to skip)")
    ))?;
    let token = token_input.trim();
    if token.is_empty() {
        println!("    {} Claude setup-token skipped.", dim(ICON_DOT));
        return Ok(false);
    }
    save_setup_token(&Provider::Claude, token)?;
    println!("{}", check("Claude setup-token saved"));
    Ok(true)
}

fn supports_oauth(provider: &Provider) -> bool {
    oauth_connect_available(provider)
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

fn render_setup_summary(providers: &[ProviderConfig], processing_mode: &ProcessingMode) {
    println!("\n{}", line());
    println!("{}", gold("  Setup summary"));
    for provider in [Provider::Claude, Provider::ChatGpt, Provider::Gemini] {
        let status = provider_setup_status(&provider, providers);
        println!("    {:<8} {}", provider.display_name(), status);
    }
    println!(
        "    {:<8} {}",
        "Processing",
        bold_white(processing_mode.display_name())
    );
}

fn render_finalize_screen(
    providers: &[ProviderConfig],
    processing_mode: &ProcessingMode,
) -> Result<()> {
    clear_screen_if_tty()?;
    println!("{}", banner());
    println!("{}", dim("  First-time setup wizard"));
    render_setup_summary(providers, processing_mode);
    Ok(())
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

fn clear_screen_if_tty() -> Result<()> {
    if io::stdout().is_terminal() {
        let mut out = io::stdout();
        out.execute(Clear(ClearType::All))?;
        out.execute(cursor::MoveTo(0, 0))?;
    }
    Ok(())
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

    #[test]
    fn parse_provider_choice_accepts_number_name_and_enter() {
        assert!(matches!(
            parse_provider_choice("1"),
            ProviderChoice::Provider(Provider::Claude)
        ));
        assert!(matches!(
            parse_provider_choice("chatgpt"),
            ProviderChoice::Provider(Provider::ChatGpt)
        ));
        assert!(matches!(parse_provider_choice(""), ProviderChoice::Done));
    }
}

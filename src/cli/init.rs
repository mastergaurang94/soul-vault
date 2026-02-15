//! `soul init` — first-time setup wizard.

use anyhow::Result;
use std::io::{self, Write};

use crate::types::{Provider, ProviderConfig, SoulVaultConfig};
use crate::ui::theme::*;
use crate::vault::config::{
    create_default_files, create_gitignore, create_vault_structure, is_initialized, set_api_key,
    vault_root, write_config,
};

// ─── Init Command ─────────────────────────────────────────────────────────────

pub fn run() -> Result<()> {
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

    // Step 2: Provider selection
    println!("\n{}\n", gold("  Select providers to connect:"));
    let mut providers = Vec::new();

    for provider in &[Provider::Claude, Provider::ChatGpt, Provider::Gemini] {
        let answer = ask(&format!(
            "    Connect {}? {} ",
            bold_white(provider.display_name()),
            dim("(Y/n)")
        ))?;
        let enabled = answer.to_lowercase() != "n";
        providers.push(ProviderConfig {
            name: provider.clone(),
            enabled,
            last_pull: None,
        });
        if enabled {
            println!("{}", check(&format!("{} enabled", provider.display_name())));
        } else {
            println!("    {} {} skipped", dim(ICON_DOT), provider.display_name());
        }
    }

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

    // Step 4: API Keys
    let needed = providers_needing_keys(&providers, &processing_llm);
    if !needed.is_empty() {
        println!("\n{}", gold("  API Keys:"));
        println!(
            "{}",
            dim("  Keys are stored locally in ~/soul-vault/.config/keys.json\n")
        );

        for provider in &needed {
            let key_input = ask(&format!(
                "    {} {} API key {} ",
                ICON_KEY,
                provider.display_name(),
                dim(&format!("({})", provider.api_key_hint()))
            ))?;

            let key = key_input.trim();
            if !key.is_empty() {
                set_api_key(&provider.to_string(), key)?;
                println!(
                    "{}",
                    check(&format!("{} key saved", provider.display_name()))
                );
            } else {
                println!(
                    "    {} {} key skipped (you can add it later)",
                    dim(ICON_DOT),
                    provider.display_name()
                );
            }
        }
    }

    // Step 5: Save config
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

fn providers_needing_keys(
    providers: &[ProviderConfig],
    processing_llm: &Provider,
) -> Vec<Provider> {
    let mut needed = vec![processing_llm.clone()];
    for provider in providers {
        if provider.enabled && !needed.contains(&provider.name) {
            needed.push(provider.name.clone());
        }
    }
    needed
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
    fn providers_needing_keys_includes_processing_llm_even_if_no_providers_enabled() {
        let providers = vec![
            ProviderConfig {
                name: Provider::Claude,
                enabled: false,
                last_pull: None,
            },
            ProviderConfig {
                name: Provider::ChatGpt,
                enabled: false,
                last_pull: None,
            },
            ProviderConfig {
                name: Provider::Gemini,
                enabled: false,
                last_pull: None,
            },
        ];

        let needed = providers_needing_keys(&providers, &Provider::Gemini);
        assert_eq!(needed, vec![Provider::Gemini]);
    }

    #[test]
    fn providers_needing_keys_deduplicates_processing_llm_when_enabled() {
        let providers = vec![
            ProviderConfig {
                name: Provider::Claude,
                enabled: true,
                last_pull: None,
            },
            ProviderConfig {
                name: Provider::Gemini,
                enabled: true,
                last_pull: None,
            },
        ];

        let needed = providers_needing_keys(&providers, &Provider::Claude);
        assert_eq!(needed, vec![Provider::Claude, Provider::Gemini]);
    }
}

//! Soul Vault color palette, icons, and text formatting utilities.

use colored::Colorize;

// ─── Color Palette ────────────────────────────────────────────────────────────
// Gold/Amber = primary brand (warm, soulful)
// Cyan/Electric Blue = accents, links, active states
// Emerald = success
// Red = errors
// Gray = muted/secondary

/// Gold (#FFBF00) — primary brand color
pub fn gold(text: &str) -> String {
    text.truecolor(255, 191, 0).to_string()
}

/// Cyan (#06B6D4) — accents, links, active states
pub fn cyan(text: &str) -> String {
    text.truecolor(6, 182, 212).to_string()
}

/// Amber (#F59E0B) — warm highlights, warnings
pub fn amber(text: &str) -> String {
    text.truecolor(245, 158, 11).to_string()
}

/// Emerald (#10B981) — success
pub fn emerald(text: &str) -> String {
    text.truecolor(16, 185, 129).to_string()
}

/// Red (#EF4444) — errors
pub fn red(text: &str) -> String {
    text.truecolor(239, 68, 68).to_string()
}

pub fn dim(text: &str) -> String {
    text.dimmed().to_string()
}

pub fn bold_white(text: &str) -> String {
    text.white().bold().to_string()
}

pub fn bold_gold(text: &str) -> String {
    text.truecolor(255, 191, 0).bold().to_string()
}

// ─── Ratatui Colors ───────────────────────────────────────────────────────────

pub mod rat {
    use ratatui::style::Color;

    /// Gold (#FFBF00) — primary: headers, highlights, selection
    pub const GOLD: Color = Color::Rgb(255, 191, 0);
    /// Electric Cyan (#06B6D4) — secondary: accents, links, active states
    pub const CYAN: Color = Color::Rgb(6, 182, 212);
    /// Amber (#F59E0B) — warm highlights, warnings
    pub const AMBER: Color = Color::Rgb(245, 158, 11);
    /// Emerald (#10B981) — success
    pub const EMERALD: Color = Color::Rgb(16, 185, 129);
    /// Red (#EF4444) — errors
    pub const RED: Color = Color::Rgb(239, 68, 68);
    /// Muted gray — secondary text
    pub const DIM: Color = Color::DarkGray;
}

// ─── Icons ────────────────────────────────────────────────────────────────────

// All icons are plain ASCII to guarantee consistent 1-column width across all terminals.
pub const ICON_CHECK: &str = "+";
pub const ICON_CROSS: &str = "x";
#[allow(dead_code)]
pub const ICON_ARROW: &str = ">";
pub const ICON_DOT: &str = "-";
pub const ICON_STAR: &str = "*";
pub const ICON_FOLDER: &str = "#";
pub const ICON_BRAIN: &str = "*";
pub const ICON_KEY: &str = ">";

// ─── Formatted Elements ──────────────────────────────────────────────────────

pub fn check(text: &str) -> String {
    format!("  {} {}", emerald(ICON_CHECK), text)
}

pub fn cross(text: &str) -> String {
    format!("  {} {}", red(ICON_CROSS), red(text))
}

pub fn line() -> String {
    dim(&"━".repeat(60))
}

pub fn banner() -> String {
    format!(
        "\n{} {} {}\n",
        bold_gold("  Soul Vault"),
        amber(ICON_STAR),
        dim("Your AI memory, unified.")
    )
}

#[allow(dead_code)]
pub fn label(key: &str, value: &str) -> String {
    format!("  {}{}", dim(&format!("{:<14}", key)), bold_white(value))
}

#[allow(dead_code)]
pub fn provider_line(name: &str, connected: bool, last_pull: Option<&str>) -> String {
    let icon = if connected {
        emerald(ICON_CHECK)
    } else {
        dim(ICON_DOT)
    };
    let status = if connected {
        let pull_info = match last_pull {
            Some(lp) => format!(" (last pull: {})", lp),
            None => String::new(),
        };
        dim(&format!("connected{}", pull_info))
    } else {
        dim("not connected")
    };
    format!(
        "    {} {}{}",
        icon,
        bold_white(&format!("{:<14}", name)),
        status
    )
}

/// Formats byte count as human-readable string.
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Formats a progress bar string.
#[allow(dead_code)]
pub fn progress_bar(current: usize, total: usize) -> String {
    let w = 20;
    let filled = if total > 0 {
        (current * w) / total
    } else {
        0
    };
    let pct = if total > 0 {
        (current * 100) / total
    } else {
        0
    };
    format!(
        "{}{} {}",
        gold(&"█".repeat(filled)),
        dim(&"░".repeat(w - filled)),
        dim(&format!("{}%", pct))
    )
}

/// Formats a list as a parenthetical preview: "(a, b, c...)".
pub fn summarize_list(items: &[String]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let preview: Vec<&str> = items.iter().take(5).map(|s| s.as_str()).collect();
    let more = if items.len() > 5 { "..." } else { "" };
    dim(&format!(" ({}{})", preview.join(", "), more))
}

/// Format time ago from an ISO date string.
pub fn format_time_ago(date_str: &str) -> String {
    let Ok(date) = chrono::DateTime::parse_from_rfc3339(date_str) else {
        return date_str.to_string();
    };
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(date);
    let secs = duration.num_seconds();

    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{} minutes ago", secs / 60)
    } else if secs < 86400 {
        format!("{} hours ago", secs / 3600)
    } else if secs < 604800 {
        format!("{} days ago", secs / 86400)
    } else {
        date_str.split('T').next().unwrap_or(date_str).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1500), "1.5 KB");
        assert_eq!(format_bytes(1_500_000), "1.4 MB");
    }

    #[test]
    fn test_summarize_list_empty() {
        let items: Vec<String> = vec![];
        assert_eq!(summarize_list(&items), "");
    }

    #[test]
    fn test_summarize_list_few() {
        let items = vec!["a".to_string(), "b".to_string()];
        let result = summarize_list(&items);
        assert!(result.contains("a, b"));
    }
}

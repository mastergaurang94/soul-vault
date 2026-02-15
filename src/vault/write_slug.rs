//! Slug generation for topic/person filenames.

pub(crate) fn slugify(text: &str) -> String {
    let slug: String = text
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();

    let slug = slug.trim_matches('-').to_string();
    let mut result = String::new();
    let mut prev_dash = false;

    for c in slug.chars() {
        if c == '-' {
            if !prev_dash {
                result.push(c);
            }
            prev_dash = true;
        } else {
            result.push(c);
            prev_dash = false;
        }
    }

    if result.len() > 50 {
        result.truncate(50);
    }

    result
}

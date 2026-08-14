use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

/// Normalize Persona Markdown before validation, hashing, history, and writes.
pub fn normalize_markdown(content: &str) -> String {
    content
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .nfc()
        .collect::<String>()
        .trim()
        .to_string()
}

/// Count visible grapheme clusters after canonical normalization.
pub fn visible_len(content: &str) -> usize {
    UnicodeSegmentation::graphemes(normalize_markdown(content).as_str(), true)
        .filter(|grapheme| !grapheme.chars().all(char::is_whitespace))
        .count()
}

/// Return the body beneath an exact level-two Markdown heading.
pub fn extract_markdown_section<'a>(content: &'a str, heading: &str) -> Option<&'a str> {
    let marker = format!("## {heading}");
    let mut offset = 0usize;
    let mut start = None;
    for line in content.split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n').trim_end();
        if start.is_some() && trimmed.starts_with("## ") {
            return start.map(|value| content[value..offset].trim());
        }
        if trimmed == marker {
            start = Some(offset + line.len());
        }
        offset += line.len();
    }
    start.map(|value| content[value..].trim())
}

pub(crate) fn replace_markdown_section(content: &str, heading: &str, body: &str) -> String {
    let normalized = normalize_markdown(content);
    let marker = format!("## {heading}");
    let lines: Vec<&str> = normalized.lines().collect();
    let Some(start) = lines.iter().position(|line| line.trim_end() == marker) else {
        return normalize_markdown(&format!("{normalized}\n\n{marker}\n{}", body.trim()));
    };
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find_map(|(index, line)| line.starts_with("## ").then_some(index))
        .unwrap_or(lines.len());
    let mut output = Vec::with_capacity(lines.len() + 2);
    output.extend_from_slice(&lines[..=start]);
    if !body.trim().is_empty() {
        output.extend(body.trim().lines());
    }
    output.extend_from_slice(&lines[end..]);
    normalize_markdown(&output.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_and_visible_count_share_one_shape() {
        assert_eq!(normalize_markdown("  e\u{301}\r\n猫  "), "é\n猫");
        assert_eq!(visible_len("  e\u{301}\r\n猫  "), 2);
    }

    #[test]
    fn extracts_and_replaces_exact_h2_section() {
        let input = "# User\n\n## Preferences\nshort\n\n## Observations\nold";
        assert_eq!(
            extract_markdown_section(input, "Preferences"),
            Some("short")
        );
        let replaced = replace_markdown_section(input, "Observations", "new");
        assert!(replaced.contains("## Preferences\nshort"));
        assert!(replaced.ends_with("## Observations\nnew"));
    }
}

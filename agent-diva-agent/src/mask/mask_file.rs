//! Mask file parsing — Markdown with YAML frontmatter.

use agent_diva_core::config::schema::MaskConfig;

use super::error::MaskError;

/// Frontmatter delimiter used to separate YAML metadata from markdown body.
const FRONTMATTER_DELIMITER: &str = "---";

/// A parsed mask file consisting of YAML frontmatter (`MaskConfig`) and a
/// markdown body (the system prompt injected when this mask is active).
#[derive(Debug, Clone, PartialEq)]
pub struct MaskFile {
    /// Parsed configuration from the YAML frontmatter.
    pub frontmatter: MaskConfig,
    /// Markdown body after the closing `---` delimiter (system prompt).
    pub body: String,
}

impl MaskFile {
    /// The default mask identity — "我就是我" (I am who I am).
    ///
    /// This mask applies no model override, no tool limits, and no extra
    /// system prompt. It represents the agent's unmasked, default state.
    pub const DEFAULT_NAME: &'static str = "我就是我";

    /// Create the default mask (no overrides, empty body).
    pub fn default_mask() -> Self {
        Self {
            frontmatter: MaskConfig {
                name: Self::DEFAULT_NAME.to_string(),
                ..Default::default()
            },
            body: String::new(),
        }
    }

    /// Parse a mask file from raw UTF-8 content.
    ///
    /// Expects the file to start with `---`, contain YAML frontmatter, and
    /// close with `---`. Everything after the closing delimiter is the body.
    ///
    /// # Errors
    ///
    /// Returns [`MaskError::InvalidFrontmatter`] if the delimiters are missing
    /// or the YAML cannot be parsed.
    pub fn parse(content: &str) -> Result<Self, MaskError> {
        Self::parse_with_path(content, "<inline>")
    }

    /// Parse with an explicit file path for error messages.
    pub fn parse_with_path(content: &str, path: &str) -> Result<Self, MaskError> {
        let trimmed = content.trim_start();

        // Must start with opening delimiter
        if !trimmed.starts_with(FRONTMATTER_DELIMITER) {
            return Err(MaskError::InvalidFrontmatter {
                path: path.to_string(),
                reason: "missing opening '---' delimiter".to_string(),
            });
        }

        // Skip the opening delimiter line
        let after_open = &trimmed[FRONTMATTER_DELIMITER.len()..];
        let after_open = after_open.strip_prefix('\n').or_else(|| after_open.strip_prefix("\r\n")).unwrap_or(after_open);

        // Find the closing delimiter
        let close_idx = after_open
            .find(FRONTMATTER_DELIMITER)
            .ok_or_else(|| MaskError::InvalidFrontmatter {
                path: path.to_string(),
                reason: "missing closing '---' delimiter".to_string(),
            })?;

        let yaml_str = &after_open[..close_idx];
        let body_start = close_idx + FRONTMATTER_DELIMITER.len();
        let body = after_open[body_start..]
            .trim_start_matches(|c: char| c == '\n' || c == '\r')
            .to_string();

        let frontmatter: MaskConfig =
            serde_yaml::from_str(yaml_str).map_err(|e| MaskError::InvalidFrontmatter {
                path: path.to_string(),
                reason: e.to_string(),
            })?;

        Ok(Self { frontmatter, body })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_mask_file() {
        let content = r#"---
name: "研究员"
icon: "🔍"
description: "专注调研与分析"
model: "deepseek-chat"
subagent_defaults:
  model: "gpt-4o-mini"
  max_iterations: 10
tool_limits:
  allow: [read_file, search_files, web_search]
  deny: [terminal, write_file]
---

你是一个专注调研与分析的研究员。"#;

        let mask = MaskFile::parse(content).expect("should parse valid mask");

        assert_eq!(mask.frontmatter.name, "研究员");
        assert_eq!(mask.frontmatter.icon.as_deref(), Some("🔍"));
        assert_eq!(mask.frontmatter.description.as_deref(), Some("专注调研与分析"));
        assert_eq!(mask.frontmatter.model.as_deref(), Some("deepseek-chat"));
        assert_eq!(
            mask.frontmatter.subagent_defaults.model.as_deref(),
            Some("gpt-4o-mini")
        );
        assert_eq!(mask.frontmatter.subagent_defaults.max_iterations, Some(10));
        assert_eq!(
            mask.frontmatter.tool_limits.allow,
            vec!["read_file", "search_files", "web_search"]
        );
        assert_eq!(
            mask.frontmatter.tool_limits.deny,
            vec!["terminal", "write_file"]
        );
        assert_eq!(mask.body, "你是一个专注调研与分析的研究员。");
    }

    #[test]
    fn parse_mask_with_missing_optional_fields() {
        let content = r#"---
name: "简约面具"
---

仅名称，无额外配置。"#;

        let mask = MaskFile::parse(content).expect("should parse mask with minimal fields");

        assert_eq!(mask.frontmatter.name, "简约面具");
        assert!(mask.frontmatter.icon.is_none());
        assert!(mask.frontmatter.description.is_none());
        assert!(mask.frontmatter.model.is_none());
        assert!(mask.frontmatter.subagent_defaults.model.is_none());
        assert!(mask.frontmatter.subagent_defaults.max_iterations.is_none());
        assert!(mask.frontmatter.tool_limits.allow.is_empty());
        assert!(mask.frontmatter.tool_limits.deny.is_empty());
        assert_eq!(mask.body, "仅名称，无额外配置。");
    }

    #[test]
    fn reject_invalid_yaml_frontmatter() {
        let content = r#"---
name: [invalid yaml
::::broken
---

body"#;

        let result = MaskFile::parse(content);
        assert!(result.is_err());
        match result.unwrap_err() {
            MaskError::InvalidFrontmatter { .. } => {} // expected
            other => panic!("expected InvalidFrontmatter, got: {other}"),
        }
    }

    #[test]
    fn reject_missing_frontmatter_delimiters() {
        let content = "name: no delimiters\n\nbody";
        let result = MaskFile::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn reject_missing_closing_delimiter() {
        let content = "---\nname: test\n\nno closing delimiter";
        let result = MaskFile::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn default_mask_properties() {
        let mask = MaskFile::default_mask();

        assert_eq!(mask.frontmatter.name, MaskFile::DEFAULT_NAME);
        assert_eq!(mask.frontmatter.name, "我就是我");
        assert!(mask.frontmatter.icon.is_none());
        assert!(mask.frontmatter.description.is_none());
        assert!(mask.frontmatter.model.is_none());
        assert!(mask.frontmatter.tool_limits.allow.is_empty());
        assert!(mask.frontmatter.tool_limits.deny.is_empty());
        assert!(mask.body.is_empty());
    }

    #[test]
    fn parse_mask_with_body_after_delimiter() {
        let content = "---\nname: test\n---\n\nHello world\n\nSecond paragraph";
        let mask = MaskFile::parse(content).unwrap();
        assert_eq!(mask.body, "Hello world\n\nSecond paragraph");
    }

    #[test]
    fn parse_mask_with_crlf_line_endings() {
        let content = "---\r\nname: win\r\n---\r\nWindows body";
        let mask = MaskFile::parse(content).unwrap();
        assert_eq!(mask.frontmatter.name, "win");
        assert_eq!(mask.body, "Windows body");
    }
}

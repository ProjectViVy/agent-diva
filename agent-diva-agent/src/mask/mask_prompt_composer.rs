//! Mask prompt composer — extracts the injectable system prompt from a mask.

use super::mask_file::MaskFile;

/// Stateless helper that decides whether a mask contributes extra prompt text.
///
/// The default mask ("我就是我") represents the agent's unmasked identity and
/// never injects additional prompt content.  Only custom masks with a non-empty
/// body produce a prompt fragment.
pub struct MaskPromptComposer;

impl MaskPromptComposer {
    /// Return the mask's body text for prompt injection, or `None` if the mask
    /// is absent or is the default mask.
    ///
    /// # Arguments
    ///
    /// * `mask` — an optional reference to the currently active [`MaskFile`].
    pub fn compose(mask: Option<&MaskFile>) -> Option<String> {
        let mask = mask?;

        // The default mask never injects extra prompt.
        if mask.frontmatter.name == MaskFile::DEFAULT_NAME {
            return None;
        }

        // Empty body → nothing to inject.
        if mask.body.is_empty() {
            return None;
        }

        Some(mask.body.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_with_none_returns_none() {
        assert!(MaskPromptComposer::compose(None).is_none());
    }

    #[test]
    fn compose_with_default_mask_returns_none() {
        let mask = MaskFile::default_mask();
        assert!(MaskPromptComposer::compose(Some(&mask)).is_none());
    }

    #[test]
    fn compose_with_custom_mask_returns_body() {
        let content = r#"---
name: "研究员"
---

你是一个专注调研与分析的研究员。"#;
        let mask = MaskFile::parse(content).unwrap();
        let result = MaskPromptComposer::compose(Some(&mask));
        assert_eq!(result.as_deref(), Some("你是一个专注调研与分析的研究员。"));
    }

    #[test]
    fn compose_with_custom_mask_empty_body_returns_none() {
        let content = r#"---
name: "空面具"
---
"#;
        let mask = MaskFile::parse(content).unwrap();
        assert!(MaskPromptComposer::compose(Some(&mask)).is_none());
    }

    #[test]
    fn compose_preserves_multiline_body() {
        let content = r#"---
name: "writer"
---

You are a technical writer.

Rules:
- Be concise
- Use examples"#;
        let mask = MaskFile::parse(content).unwrap();
        let result = MaskPromptComposer::compose(Some(&mask)).unwrap();
        assert!(result.contains("You are a technical writer."));
        assert!(result.contains("Be concise"));
    }
}

//! LLM-facing prompt contracts for long-term memory consolidation.

pub(crate) const PROMPT_ID: &str = "memory.consolidation";
pub(crate) const PROMPT_VERSION: u16 = 3;

pub(crate) const SYSTEM: &str = r#"You are a memory consolidation assistant. Analyze the conversation and extract important information.

You MUST call the `save_memory` tool with your findings. Do not respond with text.

Guidelines:
- `items`: array of memory operations. Each item has:
  - `action`: one of "add", "update", "remove"
  - `id`: record id (required for "update" and "remove")
  - `content`: memory content (required for "add" and "update")
  - `reason`: removal reason (required for "remove")
  Merge new facts, update changed facts, and remove obsolete facts as separate items.
- `history_entry`: a one-line summary of the conversation segment."#;

pub(crate) fn retry(score: f64, issues: &[String], base: &str) -> String {
    format!(
        "The previous consolidation failed the quality gate ({score:.2}/1.0). Issues: {}.\nGenerate a more detailed and complete memory update covering every material fact.\n\n{base}",
        issues.join("; ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_has_stable_identity() {
        assert_eq!(PROMPT_ID, "memory.consolidation");
        assert_eq!(PROMPT_VERSION, 3);
        assert!(SYSTEM.contains("items"));
    }
}

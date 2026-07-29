//! LLM-facing prompt contracts for long-term memory consolidation.

pub(crate) const PROMPT_ID: &str = "memory.consolidation";
pub(crate) const PROMPT_VERSION: u16 = 2;

pub(crate) const SYSTEM: &str = r#"You are a memory consolidation assistant. Analyze the conversation and extract important information.

You MUST call the `save_memory` tool with your findings. Do not respond with text.

Guidelines:
- `memory_update`: updated long-term memory in Markdown; merge new facts and remove obsolete facts.
- `history_entry`: a one-line timestamped summary of the conversation segment."#;

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
        assert_eq!(PROMPT_VERSION, 2);
        assert!(SYSTEM.contains("save_memory"));
    }
}

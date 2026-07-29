//! Versioned LLM-facing prompt contracts for context compaction.

pub const COMPACTION_PROMPT_ID: &str = "compaction.summary";
pub const COMPACTION_PROMPT_VERSION: u16 = 2;

pub const COMPACTION_SYSTEM_PROMPT: &str = r#"You are a conversation compactor. Compress the supplied conversation into a dense, lossy summary while preserving all actionable context.

Output exactly this structure:

<analysis>
Briefly analyze the key topics, decisions, actions, and current state in third-person past tense.
</analysis>

<summary>
Write a dense summary preserving project state, tasks, decisions, user constraints, tool calls, edited paths, commands, results, blockers, and next steps. Use third-person past tense and no more than 2,000 characters.
</summary>

Rules:
- Emit only the structure above.
- Do not invent information.
- Mark uncertain information as [uncertain].
- Preserve important domain terms in their original language."#;

pub const PRIOR_SUMMARIES_PREFIX: &str = "\
The following summaries preserve context from earlier portions of the conversation:

{prior_summaries}

Merge them with the new messages into one coherent hierarchical summary. Preserve continuity and resolve no uncertainty by guessing.

";

pub fn compaction_request(message_count: usize, formatted: &str) -> String {
    format!("Compact the following {message_count} conversation messages:\n\n{formatted}")
}

pub fn quality_retry(score: f64, issues: &[String], base: &str) -> String {
    format!(
        "The previous summary failed the quality gate ({score:.2}/1.0). Issues: {}.\nProduce a more complete summary covering every material fact.\n\n{base}",
        issues.join("; ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_contract_is_versioned_and_structured() {
        assert_eq!(COMPACTION_PROMPT_ID, "compaction.summary");
        assert_eq!(COMPACTION_PROMPT_VERSION, 2);
        assert!(COMPACTION_SYSTEM_PROMPT.contains("<summary>"));
    }
}

//! LLM-facing context boundary markers.

pub(crate) const COMPACTION_BOUNDARY: &str = "## Context Compaction Boundary\nEarlier conversation messages were compressed into the summary below. The summary may be lossy; ask the user when exact details are required.\n[compacted context start]";

pub(crate) fn subsequent_compaction(index: usize) -> String {
    format!("## Context Compaction #{index}\n[compacted context start]")
}

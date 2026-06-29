//! Summary data model with pointer chain support for LLM summary compaction.
//!
//! This module provides the core data structures for building a chain of
//! conversation summaries that can be traversed backwards, enabling
//! hierarchical compaction of long-running agent sessions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A compressed summary of a conversation segment.
///
/// Each summary carries a pointer to its predecessor (`prev_summary_id`),
/// forming a singly-linked chain from the most recent summary back to the
/// oldest. The `source_message_range` records which messages in the original
/// conversation this summary was derived from.
#[derive(Clone, Serialize, Deserialize)]
pub struct Summary {
    /// Unique identifier for this summary.
    pub id: String,
    /// The summary text content.
    pub content: String,
    /// When this summary was created.
    pub created_at: DateTime<Utc>,
    /// Pointer to the previous (older) summary in the chain, if any.
    pub prev_summary_id: Option<String>,
    /// The inclusive range of source messages this summary covers
    /// `(start_index, end_index)`.
    pub source_message_range: (usize, usize),
    /// Estimated token count of this summary.
    pub token_count: u32,
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Summary(id={}, created_at={}, tokens={})",
            self.id, self.created_at, self.token_count
        )
    }
}

impl fmt::Debug for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let preview: &str = if self.content.len() > 50 {
            &self.content[..50]
        } else {
            self.content.as_str()
        };
        f.debug_struct("Summary")
            .field("id", &self.id)
            .field("content", &preview)
            .field("created_at", &self.created_at)
            .field("prev_summary_id", &self.prev_summary_id)
            .field("source_message_range", &self.source_message_range)
            .field("token_count", &self.token_count)
            .finish()
    }
}

/// A doubly-linked chain of summaries anchored at the most recent entry.
///
/// New summaries are appended to the front (becoming the new `latest`), and
/// their `prev_summary_id` is automatically set to the previous latest.
/// The chain can be traversed from latest backwards via an iterator.
#[derive(Clone, Serialize, Deserialize)]
pub struct SummaryChain {
    summaries: Vec<Summary>,
    latest_id: Option<String>,
}

impl Default for SummaryChain {
    fn default() -> Self {
        Self {
            summaries: Vec::new(),
            latest_id: None,
        }
    }
}

impl SummaryChain {
    /// Create a new empty summary chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new summary onto the chain.
    ///
    /// The summary's `prev_summary_id` is automatically set to the previous
    /// latest entry (if any), linking the new summary as the head of the chain.
    pub fn push(&mut self, mut summary: Summary) {
        summary.prev_summary_id = self.latest_id.clone();
        self.latest_id = Some(summary.id.clone());
        self.summaries.push(summary);
    }

    /// Return the number of summaries in the chain.
    pub fn depth(&self) -> usize {
        self.summaries.len()
    }

    /// Return a reference to the most recent summary, if any.
    pub fn latest(&self) -> Option<&Summary> {
        self.latest_id
            .as_ref()
            .and_then(|id| self.get(id))
    }

    /// Look up a summary by its id.
    pub fn get(&self, id: &str) -> Option<&Summary> {
        self.summaries.iter().find(|s| s.id == id)
    }

    /// Return an iterator that traverses from the latest summary backwards
    /// through the `prev_summary_id` chain.
    pub fn iter_from_latest(&self) -> SummaryChainIter<'_> {
        SummaryChainIter {
            chain: self,
            current_id: self.latest_id.clone(),
        }
    }
}

impl fmt::Display for SummaryChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SummaryChain(depth={}, latest={:?})",
            self.depth(),
            self.latest_id
        )
    }
}

impl fmt::Debug for SummaryChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SummaryChain")
            .field("depth", &self.depth())
            .field("latest_id", &self.latest_id)
            .finish()
    }
}

/// An iterator that walks the summary chain backwards from the latest entry.
///
/// Each call to `next()` returns the current summary and advances to the
/// previous summary via `prev_summary_id`.
pub struct SummaryChainIter<'a> {
    chain: &'a SummaryChain,
    current_id: Option<String>,
}

impl<'a> Iterator for SummaryChainIter<'a> {
    type Item = &'a Summary;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.current_id.as_ref()?;
        let summary = self.chain.get(id)?;
        self.current_id = summary.prev_summary_id.clone();
        Some(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_summary(id: &str, token_count: u32, range: (usize, usize)) -> Summary {
        Summary {
            id: id.to_string(),
            content: format!("Summary content for {}", id),
            created_at: Utc::now(),
            prev_summary_id: None,
            source_message_range: range,
            token_count,
        }
    }

    #[test]
    fn test_summary_push_and_chain() {
        let mut chain = SummaryChain::new();

        let s1 = make_summary("s1", 100, (0, 10));
        chain.push(s1);

        assert_eq!(chain.depth(), 1);
        assert_eq!(chain.latest().unwrap().id, "s1");
        assert!(chain.latest().unwrap().prev_summary_id.is_none());

        let s2 = make_summary("s2", 150, (11, 25));
        chain.push(s2);

        assert_eq!(chain.depth(), 2);
        assert_eq!(chain.latest().unwrap().id, "s2");
        assert_eq!(
            chain.latest().unwrap().prev_summary_id.as_deref(),
            Some("s1")
        );

        // Verify backward traversal via pointer chain
        let s2_ref = chain.get("s2").unwrap();
        let s1_ref = chain.get(s2_ref.prev_summary_id.as_ref().unwrap()).unwrap();
        assert_eq!(s1_ref.id, "s1");
        assert_eq!(s1_ref.token_count, 100);
        assert_eq!(s1_ref.source_message_range, (0, 10));
    }

    #[test]
    fn test_summary_depth() {
        let mut chain = SummaryChain::new();
        assert_eq!(chain.depth(), 0);

        chain.push(make_summary("a", 50, (0, 5)));
        assert_eq!(chain.depth(), 1);

        chain.push(make_summary("b", 60, (6, 10)));
        assert_eq!(chain.depth(), 2);

        chain.push(make_summary("c", 70, (11, 15)));
        assert_eq!(chain.depth(), 3);
    }

    #[test]
    fn test_summary_serde_roundtrip() {
        let mut chain = SummaryChain::new();
        chain.push(make_summary("s1", 120, (0, 20)));
        chain.push(make_summary("s2", 80, (21, 30)));

        let json = serde_json::to_string(&chain).unwrap();
        let deserialized: SummaryChain = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.depth(), 2);
        assert_eq!(
            deserialized.latest().unwrap().prev_summary_id.as_deref(),
            Some("s1")
        );
        assert_eq!(deserialized.latest().unwrap().token_count, 80);

        // Verify the summary content survived roundtrip
        let s1 = deserialized.get("s1").unwrap();
        assert_eq!(s1.token_count, 120);
        assert_eq!(s1.source_message_range, (0, 20));
    }

    #[test]
    fn test_summary_chain_empty() {
        let chain = SummaryChain::new();
        assert_eq!(chain.depth(), 0);
        assert!(chain.latest().is_none());
        assert!(chain.get("nonexistent").is_none());
        assert!(chain.iter_from_latest().next().is_none());
    }

    #[test]
    fn test_summary_chain_iter_from_latest() {
        let mut chain = SummaryChain::new();
        chain.push(make_summary("s1", 100, (0, 10)));
        chain.push(make_summary("s2", 150, (11, 25)));
        chain.push(make_summary("s3", 200, (26, 40)));

        let ids: Vec<&str> = chain.iter_from_latest().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["s3", "s2", "s1"]);
    }

    #[test]
    fn test_summary_display() {
        let summary = make_summary("test-1", 75, (0, 10));
        let display = format!("{}", summary);
        assert!(display.contains("test-1"));
        assert!(display.contains("75"));
    }
}

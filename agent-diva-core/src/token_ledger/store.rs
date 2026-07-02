//! JSONL append-only store for token usage ledger entries

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use tracing::warn;

/// A single token usage record in the ledger.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenLedgerEntry {
    /// When this usage occurred
    pub timestamp: DateTime<Utc>,
    /// Session that consumed the tokens
    pub session_id: String,
    /// Model used (e.g. "deepseek-chat", "anthropic/claude-sonnet-4")
    pub model: String,
    /// Tokens consumed in the prompt
    pub input_tokens: u32,
    /// Tokens generated in the completion
    pub output_tokens: u32,
    /// Total tokens for this entry (input + output)
    pub total_tokens: u32,
    /// Estimated cost in USD (optional — depends on pricing data availability)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_estimate: Option<f64>,
}

impl TokenLedgerEntry {
    /// Create a new entry with computed total_tokens.
    pub fn new(
        session_id: impl Into<String>,
        model: impl Into<String>,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            session_id: session_id.into(),
            model: model.into(),
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            cost_estimate: None,
        }
    }

    /// Attach a cost estimate to this entry.
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost_estimate = Some(cost);
        self
    }
}

/// Filters for querying ledger entries.
#[derive(Debug, Clone, Default)]
pub struct UsageFilters {
    /// Filter by session ID
    pub session_id: Option<String>,
    /// Filter by model name
    pub model: Option<String>,
    /// Only include entries after this timestamp
    pub since: Option<DateTime<Utc>>,
}

/// Append-only JSONL store for token ledger entries.
///
/// Follows the same pattern as `JsonlTodoStore`:
/// - Append-only writes (no in-place updates)
/// - Read with optional filters
/// - Skip corrupted lines gracefully (log warning, continue)
pub struct JsonlTokenLedger {
    path: PathBuf,
}

impl JsonlTokenLedger {
    /// Create a new ledger rooted at `data_root`.
    /// The JSONL file will be at `{data_root}/token_ledger.jsonl`.
    /// Creates the file if it does not exist.
    pub fn new(data_root: &Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(data_root)?;
        let path = data_root.join("token_ledger.jsonl");
        OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self { path })
    }

    /// Append a token usage entry to the ledger.
    pub fn append(&self, entry: TokenLedgerEntry) -> std::io::Result<()> {
        let mut line = serde_json::to_string(&entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        line.push('\n');

        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        file.write_all(line.as_bytes())?;
        file.flush()?;
        Ok(())
    }

    /// Read all entries matching the given filters.
    /// Corrupted lines are skipped with a warning log.
    pub fn read(&self, filters: &UsageFilters) -> std::io::Result<Vec<TokenLedgerEntry>> {
        let file = std::fs::File::open(&self.path)?;
        let reader = std::io::BufReader::new(file);
        let mut entries = Vec::new();
        for (line_num, line) in reader.lines().enumerate() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    warn!("token_ledger: IO error reading line {}: {}", line_num + 1, e);
                    continue;
                }
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<TokenLedgerEntry>(trimmed) {
                Ok(entry) => {
                    if Self::matches_filters(&entry, filters) {
                        entries.push(entry);
                    }
                }
                Err(e) => {
                    warn!(
                        "token_ledger: skipping corrupted line {}: {}",
                        line_num + 1,
                        e
                    );
                }
            }
        }
        Ok(entries)
    }

    /// Compute cumulative total tokens for a given session.
    pub fn session_total(&self, session_id: &str) -> std::io::Result<u64> {
        let filters = UsageFilters {
            session_id: Some(session_id.to_string()),
            ..Default::default()
        };
        let entries = self.read(&filters)?;
        Ok(entries.iter().map(|e| e.total_tokens as u64).sum())
    }

    fn matches_filters(entry: &TokenLedgerEntry, filters: &UsageFilters) -> bool {
        if let Some(ref sid) = filters.session_id {
            if entry.session_id != *sid {
                return false;
            }
        }
        if let Some(ref model) = filters.model {
            if entry.model != *model {
                return false;
            }
        }
        if let Some(since) = filters.since {
            if entry.timestamp < since {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, JsonlTokenLedger) {
        let dir = TempDir::new().expect("tempdir");
        let ledger = JsonlTokenLedger::new(dir.path()).expect("ledger creation");
        (dir, ledger)
    }

    #[test]
    fn test_entry_new_computes_total() {
        let entry = TokenLedgerEntry::new("sess-1", "deepseek-chat", 100, 50);
        assert_eq!(entry.total_tokens, 150);
        assert!(entry.cost_estimate.is_none());
    }

    #[test]
    fn test_entry_with_cost() {
        let entry = TokenLedgerEntry::new("sess-1", "deepseek-chat", 100, 50).with_cost(0.003);
        assert_eq!(entry.cost_estimate, Some(0.003));
    }

    #[test]
    fn test_append_and_read() {
        let (_dir, ledger) = setup();
        let entry = TokenLedgerEntry::new("sess-1", "deepseek-chat", 100, 50);
        ledger.append(entry.clone()).expect("append");

        let entries = ledger.read(&UsageFilters::default()).expect("read");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].session_id, "sess-1");
        assert_eq!(entries[0].model, "deepseek-chat");
        assert_eq!(entries[0].input_tokens, 100);
        assert_eq!(entries[0].output_tokens, 50);
        assert_eq!(entries[0].total_tokens, 150);
    }

    #[test]
    fn test_read_filter_by_session() {
        let (_dir, ledger) = setup();
        ledger
            .append(TokenLedgerEntry::new("sess-1", "model-a", 10, 5))
            .expect("append");
        ledger
            .append(TokenLedgerEntry::new("sess-2", "model-a", 20, 10))
            .expect("append");
        ledger
            .append(TokenLedgerEntry::new("sess-1", "model-b", 30, 15))
            .expect("append");

        let filters = UsageFilters {
            session_id: Some("sess-1".to_string()),
            ..Default::default()
        };
        let entries = ledger.read(&filters).expect("read");
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.session_id == "sess-1"));
    }

    #[test]
    fn test_read_filter_by_model() {
        let (_dir, ledger) = setup();
        ledger
            .append(TokenLedgerEntry::new("sess-1", "model-a", 10, 5))
            .expect("append");
        ledger
            .append(TokenLedgerEntry::new("sess-2", "model-b", 20, 10))
            .expect("append");

        let filters = UsageFilters {
            model: Some("model-a".to_string()),
            ..Default::default()
        };
        let entries = ledger.read(&filters).expect("read");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].model, "model-a");
    }

    #[test]
    fn test_session_total() {
        let (_dir, ledger) = setup();
        ledger
            .append(TokenLedgerEntry::new("sess-1", "model-a", 100, 50))
            .expect("append");
        ledger
            .append(TokenLedgerEntry::new("sess-1", "model-b", 200, 100))
            .expect("append");
        ledger
            .append(TokenLedgerEntry::new("sess-2", "model-a", 50, 25))
            .expect("append");

        assert_eq!(ledger.session_total("sess-1").expect("total"), 450);
        assert_eq!(ledger.session_total("sess-2").expect("total"), 75);
        assert_eq!(ledger.session_total("sess-unknown").expect("total"), 0);
    }

    #[test]
    fn test_corrupted_line_skipped() {
        let (_dir, ledger) = setup();
        ledger
            .append(TokenLedgerEntry::new("sess-1", "model-a", 10, 5))
            .expect("append");

        // Manually append a corrupted line
        let mut file = OpenOptions::new()
            .append(true)
            .open(&ledger.path)
            .expect("open");
        writeln!(file, "{{invalid json}}").expect("write bad line");

        ledger
            .append(TokenLedgerEntry::new("sess-1", "model-b", 20, 10))
            .expect("append");

        let entries = ledger.read(&UsageFilters::default()).expect("read");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].model, "model-a");
        assert_eq!(entries[1].model, "model-b");
    }

    #[test]
    fn test_jsonl_format() {
        let dir = TempDir::new().expect("tempdir");
        let ledger = JsonlTokenLedger::new(dir.path()).expect("ledger");
        ledger
            .append(TokenLedgerEntry::new("sess-1", "model-a", 10, 5))
            .expect("append");
        ledger
            .append(TokenLedgerEntry::new("sess-2", "model-b", 20, 10))
            .expect("append");

        let content =
            std::fs::read_to_string(dir.path().join("token_ledger.jsonl")).expect("read");
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 2);

        for line in &lines {
            let parsed: serde_json::Value = serde_json::from_str(line).expect("valid json");
            assert!(parsed.is_object());
        }
    }

    #[test]
    fn test_empty_ledger_reads_zero() {
        let (_dir, ledger) = setup();
        let entries = ledger.read(&UsageFilters::default()).expect("read");
        assert!(entries.is_empty());
        assert_eq!(ledger.session_total("any").expect("total"), 0);
    }

    #[test]
    fn test_read_filter_by_since() {
        let (_dir, ledger) = setup();
        let entry1 = TokenLedgerEntry::new("sess-1", "model-a", 10, 5);
        let ts1 = entry1.timestamp;
        ledger.append(entry1).expect("append");

        // Create entry with a later timestamp
        let mut entry2 = TokenLedgerEntry::new("sess-1", "model-b", 20, 10);
        entry2.timestamp = ts1 + chrono::Duration::hours(1);
        ledger.append(entry2).expect("append");

        let filters = UsageFilters {
            since: Some(ts1 + chrono::Duration::minutes(30)),
            ..Default::default()
        };
        let entries = ledger.read(&filters).expect("read");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].model, "model-b");
    }

    #[test]
    fn test_cost_estimate_serialization() {
        let entry = TokenLedgerEntry::new("sess-1", "model-a", 100, 50).with_cost(0.003);
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"cost_estimate\":0.003"));

        // Without cost, field should be omitted
        let entry_no_cost = TokenLedgerEntry::new("sess-2", "model-b", 10, 5);
        let json_no_cost = serde_json::to_string(&entry_no_cost).unwrap();
        assert!(!json_no_cost.contains("cost_estimate"));
    }
}

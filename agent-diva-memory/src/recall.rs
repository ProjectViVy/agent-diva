//! File-backed recall engine foundation for diary and compatibility memory.

use crate::compat_source::MemoryMdChunkSource;
use crate::contracts::{DiaryStore, RecallEngine};
use crate::diary::FileDiaryStore;
use crate::types::{DiaryEntry, DiaryFilter, DiaryPartition, MemoryQuery, MemoryRecord};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct FileRecallEngine {
    diary_store: FileDiaryStore,
    compat_source: MemoryMdChunkSource,
}

impl FileRecallEngine {
    pub fn for_workspace<P: AsRef<Path>>(workspace: P) -> Self {
        let diary_store = FileDiaryStore::new(workspace.as_ref());
        Self::new(diary_store, MemoryMdChunkSource::new(workspace))
    }

    pub(crate) fn new(diary_store: FileDiaryStore, compat_source: MemoryMdChunkSource) -> Self {
        Self {
            diary_store,
            compat_source,
        }
    }

    fn compatibility_records(
        &self,
        query: &MemoryQuery,
    ) -> agent_diva_core::Result<Vec<ScoredRecord>> {
        let records = self.compat_source.load_records()?;
        Ok(records
            .into_iter()
            .filter_map(|record| {
                let score = record_match_score(&record, query)?;
                Some(ScoredRecord {
                    record,
                    score,
                    source_rank: 1,
                })
            })
            .collect())
    }
}

impl RecallEngine for FileRecallEngine {
    fn recall(&self, query: &MemoryQuery) -> agent_diva_core::Result<Vec<MemoryRecord>> {
        let mut candidates = Vec::new();

        for partition in [DiaryPartition::Rational, DiaryPartition::Emotional] {
            let filter = DiaryFilter {
                partition: Some(partition),
                domain: query.domain.clone(),
                scope: query.scope.clone(),
                since: query.since,
                until: query.until,
                tag: None,
                limit: None,
            };
            let entries = self.diary_store.filter_entries(&filter)?;
            candidates.extend(entries.into_iter().filter_map(|entry| {
                let score = diary_match_score(&entry, query)?;
                Some(ScoredRecord {
                    record: memory_record_from_diary_entry(entry),
                    score,
                    source_rank: 2,
                })
            }));
        }

        candidates.extend(self.compatibility_records(query)?);

        candidates.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then(right.source_rank.cmp(&left.source_rank))
                .then(right.record.timestamp.cmp(&left.record.timestamp))
                .then(left.record.id.cmp(&right.record.id))
        });
        candidates.truncate(query.limit.max(1));

        Ok(candidates
            .into_iter()
            .map(|candidate| candidate.record)
            .collect())
    }
}

#[derive(Debug, Clone)]
struct ScoredRecord {
    record: MemoryRecord,
    score: usize,
    source_rank: u8,
}

fn diary_match_score(entry: &DiaryEntry, query: &MemoryQuery) -> Option<usize> {
    if let Some(scope) = &query.scope {
        if &entry.scope != scope {
            return None;
        }
    }
    let source_paths = entry
        .source_refs
        .iter()
        .filter_map(|source| source.path.as_deref())
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
    let haystack = [
        entry.title.to_lowercase(),
        entry.summary.to_lowercase(),
        entry.body.to_lowercase(),
        entry.tags.join(" ").to_lowercase(),
        source_paths,
    ]
    .join("\n");
    query_match_score(&haystack, query.query.as_deref())
}

fn record_match_score(record: &MemoryRecord, query: &MemoryQuery) -> Option<usize> {
    if let Some(domain) = &query.domain {
        if &record.domain != domain {
            return None;
        }
    }
    if let Some(scope) = &query.scope {
        if &record.scope != scope {
            return None;
        }
    }
    if let Some(since) = query.since {
        if record.timestamp < since {
            return None;
        }
    }
    if let Some(until) = query.until {
        if record.timestamp > until {
            return None;
        }
    }

    let haystack = [
        record.title.to_lowercase(),
        record.summary.to_lowercase(),
        record.content.to_lowercase(),
        record.tags.join(" ").to_lowercase(),
        record
            .source_refs
            .iter()
            .filter_map(|source| source.path.as_deref())
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase(),
    ]
    .join("\n");
    query_match_score(&haystack, query.query.as_deref())
}

pub(crate) fn query_match_score(haystack: &str, query: Option<&str>) -> Option<usize> {
    let Some(query) = query else {
        return Some(1);
    };
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Some(1);
    }

    let normalized_haystack = haystack.to_lowercase();
    if normalized_haystack.contains(&needle) {
        return Some(1000 + needle.len());
    }

    let terms = normalized_query_terms(&needle);
    if terms.is_empty() {
        return None;
    }

    let matched = terms
        .iter()
        .filter(|term| normalized_haystack.contains(term.as_str()))
        .count();
    (matched > 0).then_some(matched)
}

pub(crate) fn normalized_query_terms(query: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut ascii_term = String::new();
    let mut cjk_run = String::new();

    for ch in query.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '/' {
            if !cjk_run.is_empty() {
                collect_cjk_terms(&cjk_run, &mut terms);
                cjk_run.clear();
            }
            ascii_term.push(ch);
        } else {
            if !ascii_term.is_empty() {
                push_term(&ascii_term, &mut terms);
                ascii_term.clear();
            }
            if is_cjk(ch) {
                cjk_run.push(ch);
            } else if !cjk_run.is_empty() {
                collect_cjk_terms(&cjk_run, &mut terms);
                cjk_run.clear();
            }
        }
    }

    if !ascii_term.is_empty() {
        push_term(&ascii_term, &mut terms);
    }
    if !cjk_run.is_empty() {
        collect_cjk_terms(&cjk_run, &mut terms);
    }

    terms.sort();
    terms.dedup();
    terms
}

fn collect_cjk_terms(run: &str, terms: &mut Vec<String>) {
    let chars = run.chars().collect::<Vec<_>>();
    if chars.len() < 2 {
        return;
    }

    for window in chars.windows(2) {
        let term = window.iter().collect::<String>();
        push_term(&term, terms);
    }

    for window in chars.windows(3) {
        let term = window.iter().collect::<String>();
        push_term(&term, terms);
    }
}

fn push_term(term: &str, terms: &mut Vec<String>) {
    let normalized = term.trim().to_lowercase();
    if normalized.len() < 2 || is_stop_term(&normalized) {
        return;
    }
    terms.push(normalized);
}

fn is_stop_term(term: &str) -> bool {
    matches!(
        term,
        "之前"
            | "以前"
            | "最近"
            | "上次"
            | "我们"
            | "什么"
            | "一下"
            | "关于"
            | "一下子"
            | "what"
            | "did"
            | "last"
            | "time"
            | "the"
            | "for"
            | "and"
    )
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x4E00..=0x9FFF
            | 0x3400..=0x4DBF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
            | 0xF900..=0xFAFF
            | 0x2F800..=0x2FA1F
    )
}

fn memory_record_from_diary_entry(entry: DiaryEntry) -> MemoryRecord {
    MemoryRecord {
        id: entry.id,
        timestamp: entry.timestamp,
        domain: entry.domain,
        scope: entry.scope,
        title: entry.title,
        summary: entry.summary,
        content: entry.body,
        tags: entry.tags,
        source_refs: entry.source_refs,
        confidence: entry.confidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::RecallEngine;
    use crate::types::{MemoryDomain, MemoryScope, MemorySourceRef};
    use chrono::{DateTime, Utc};
    use tempfile::TempDir;

    fn sample_engine() -> (TempDir, FileRecallEngine) {
        let temp_dir = TempDir::new().unwrap();
        let diary_store = FileDiaryStore::new(temp_dir.path());

        let mut recent = DiaryEntry::new(
            DiaryPartition::Rational,
            MemoryDomain::Workspace,
            MemoryScope::Workspace,
            "Architecture note",
            "Mapped the memory split",
            "The agent-diva-memory crate now owns diary storage.",
        );
        recent.timestamp = DateTime::parse_from_rfc3339("2026-03-26T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        recent.tags = vec!["memory".into(), "architecture".into()];
        recent.source_refs = vec![MemorySourceRef {
            path: Some("agent-diva-memory/src/service.rs".into()),
            section: None,
            note: None,
        }];
        diary_store.append_entry(&recent).unwrap();

        std::fs::create_dir_all(temp_dir.path().join("memory")).unwrap();
        std::fs::write(
            temp_dir.path().join("memory").join("MEMORY.md"),
            "# Long-term Memory\n\n## Preferences\nUse Chinese in replies.\n\n## Decisions\nKeep core minimal and move enhanced memory into agent-diva-memory.\n",
        )
        .unwrap();

        let engine = FileRecallEngine::new(diary_store, MemoryMdChunkSource::new(temp_dir.path()));
        (temp_dir, engine)
    }

    #[test]
    fn test_query_match_score_with_mixed_language_terms() {
        assert!(query_match_score(
            "memory split conclusion keep core minimal and move enhanced memory into agent-diva-memory",
            Some("之前我们对 memory 拆分做了什么结论？"),
        )
        .is_some());
    }

    #[test]
    fn test_recall_engine_includes_memory_md_chunks() {
        let (_temp, engine) = sample_engine();
        let records = engine
            .recall(&MemoryQuery {
                query: Some("Use Chinese".into()),
                limit: 5,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].title, "Preferences");
        assert_eq!(
            records[0].source_refs[0].path.as_deref(),
            Some("memory/MEMORY.md")
        );
    }

    #[test]
    fn test_recall_engine_returns_multiple_memory_md_chunks() {
        let (_temp, engine) = sample_engine();
        let records = engine
            .recall(&MemoryQuery {
                query: None,
                limit: 5,
                domain: Some(MemoryDomain::Fact),
                ..MemoryQuery::default()
            })
            .unwrap();
        assert!(records.iter().any(|record| record.title == "Preferences"));
        assert!(records.iter().any(|record| record.title == "Decisions"));
    }

    #[test]
    fn test_recall_engine_prefers_recent_diary_when_scores_tie() {
        let (_temp, engine) = sample_engine();
        let records = engine
            .recall(&MemoryQuery {
                query: Some("memory".into()),
                limit: 5,
                ..MemoryQuery::default()
            })
            .unwrap();
        assert_eq!(records[0].title, "Architecture note");
    }
}

//! Retrieval layer for keyword, semantic, and hybrid memory recall.

use crate::contracts::RecallEngine;
use crate::embeddings::{cosine_similarity, EmbeddingProvider, EmbeddingProviderConfig};
use crate::sqlite_recall::SqliteRecallEngine;
use crate::store::SqliteMemoryStore;
use crate::sync::{canonical_record_key, fingerprint};
use crate::types::{MemoryQuery, MemoryRecord, RecallMode};
use agent_diva_core::Result;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::warn;

pub trait KeywordRetriever: Send + Sync {
    fn recall(&self, query: &MemoryQuery) -> Result<Vec<MemoryRecord>>;
}

pub trait SemanticRetriever: Send + Sync {
    fn score_records(&self, query: &MemoryQuery, records: &[MemoryRecord]) -> Result<Vec<f32>>;
}

pub trait HybridReranker: Send + Sync {
    fn rerank(&self, query: &MemoryQuery, records: Vec<MemoryRecord>) -> Result<Vec<MemoryRecord>>;
}

pub struct RetrievalEngine {
    keyword: Arc<dyn KeywordRetriever>,
    reranker: Arc<dyn HybridReranker>,
}

impl RetrievalEngine {
    pub fn new(keyword: Arc<dyn KeywordRetriever>, reranker: Arc<dyn HybridReranker>) -> Self {
        Self { keyword, reranker }
    }

    pub fn recall(&self, query: &MemoryQuery) -> Result<Vec<MemoryRecord>> {
        let widened_query = MemoryQuery {
            limit: query.limit.max(1).saturating_mul(4),
            ..query.clone()
        };
        let records = self.keyword.recall(&widened_query)?;
        self.reranker.rerank(query, records)
    }
}

pub struct MergedKeywordRetriever {
    sqlite: Arc<SqliteRecallEngine>,
    file: Arc<dyn RecallEngine>,
}

impl MergedKeywordRetriever {
    pub fn new(sqlite: Arc<SqliteRecallEngine>, file: Arc<dyn RecallEngine>) -> Self {
        Self { sqlite, file }
    }
}

impl KeywordRetriever for MergedKeywordRetriever {
    fn recall(&self, query: &MemoryQuery) -> Result<Vec<MemoryRecord>> {
        let sqlite_records = self.sqlite.recall(query)?;
        let file_records = self.file.recall(query)?;
        Ok(merge_records(query, sqlite_records, file_records))
    }
}

pub trait EmbeddingCacheStore: Send + Sync {
    fn record_embedding(
        &self,
        record_id: &str,
    ) -> Result<Option<(Vec<f32>, EmbeddingProviderConfig, String)>>;

    fn upsert_record_embedding(
        &self,
        record_id: &str,
        provider: &EmbeddingProviderConfig,
        content: &str,
        embedding: &[f32],
    ) -> Result<()>;

    fn query_embedding(&self, query: &str) -> Result<Option<Vec<f32>>>;

    fn upsert_query_embedding(
        &self,
        query: &str,
        provider: &EmbeddingProviderConfig,
        embedding: &[f32],
    ) -> Result<()>;
}

impl EmbeddingCacheStore for SqliteMemoryStore {
    fn record_embedding(
        &self,
        record_id: &str,
    ) -> Result<Option<(Vec<f32>, EmbeddingProviderConfig, String)>> {
        SqliteMemoryStore::record_embedding(self, record_id)
    }

    fn upsert_record_embedding(
        &self,
        record_id: &str,
        provider: &EmbeddingProviderConfig,
        content: &str,
        embedding: &[f32],
    ) -> Result<()> {
        SqliteMemoryStore::upsert_record_embedding(self, record_id, provider, content, embedding)
    }

    fn query_embedding(&self, query: &str) -> Result<Option<Vec<f32>>> {
        SqliteMemoryStore::query_embedding(self, query)
    }

    fn upsert_query_embedding(
        &self,
        query: &str,
        provider: &EmbeddingProviderConfig,
        embedding: &[f32],
    ) -> Result<()> {
        SqliteMemoryStore::upsert_query_embedding(self, query, provider, embedding)
    }
}

pub struct CachedSemanticRetriever {
    cache_store: Arc<dyn EmbeddingCacheStore>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    embedding_config: EmbeddingProviderConfig,
}

impl CachedSemanticRetriever {
    pub fn new(
        cache_store: Arc<dyn EmbeddingCacheStore>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        embedding_config: EmbeddingProviderConfig,
    ) -> Self {
        Self {
            cache_store,
            embedding_provider,
            embedding_config,
        }
    }

    fn ensure_query_embedding(&self, query_text: &str) -> Result<Option<Vec<f32>>> {
        if !self.embedding_provider.is_enabled() {
            return Ok(None);
        }

        match self.cache_store.query_embedding(query_text) {
            Ok(Some(embedding)) => Ok(Some(embedding)),
            Ok(None) => {
                let embedding = self.embedding_provider.embed_one(query_text)?;
                let _ = self.cache_store.upsert_query_embedding(
                    query_text,
                    &self.embedding_config,
                    &embedding,
                );
                Ok(Some(embedding))
            }
            Err(error) => Err(error),
        }
    }

    fn ensure_record_embedding(&self, record: &MemoryRecord) -> Result<Option<Vec<f32>>> {
        if !self.embedding_provider.is_enabled() {
            return Ok(None);
        }

        let content = embedding_text(record);
        let content_hash = compute_content_hash(&content);
        if let Some((embedding, provider, stored_hash)) =
            self.cache_store.record_embedding(&record.id)?
        {
            if stored_hash == content_hash
                && provider.provider == self.embedding_config.provider
                && provider.model == self.embedding_config.model
            {
                return Ok(Some(embedding));
            }
        }

        let embedding = self.embedding_provider.embed_one(&content)?;
        self.cache_store.upsert_record_embedding(
            &record.id,
            &self.embedding_config,
            &content,
            &embedding,
        )?;
        Ok(Some(embedding))
    }
}

impl SemanticRetriever for CachedSemanticRetriever {
    fn score_records(&self, query: &MemoryQuery, records: &[MemoryRecord]) -> Result<Vec<f32>> {
        let Some(query_text) = query
            .query
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        else {
            return Ok(vec![0.0; records.len()]);
        };

        let Some(query_embedding) = self.ensure_query_embedding(query_text)? else {
            return Ok(vec![0.0; records.len()]);
        };

        let mut scores = Vec::with_capacity(records.len());
        for record in records {
            let score = match self.ensure_record_embedding(record) {
                Ok(Some(embedding)) => cosine_similarity(&query_embedding, &embedding),
                Ok(None) => 0.0,
                Err(error) => {
                    warn!("record embedding unavailable for {}: {error}", record.id);
                    0.0
                }
            };
            scores.push(score);
        }
        Ok(scores)
    }
}

pub struct DefaultHybridReranker {
    semantic: Arc<dyn SemanticRetriever>,
}

impl DefaultHybridReranker {
    pub fn new(semantic: Arc<dyn SemanticRetriever>) -> Self {
        Self { semantic }
    }
}

impl HybridReranker for DefaultHybridReranker {
    fn rerank(
        &self,
        query: &MemoryQuery,
        mut records: Vec<MemoryRecord>,
    ) -> Result<Vec<MemoryRecord>> {
        let mode = query.recall_mode.clone().unwrap_or(RecallMode::HybridReady);
        if matches!(mode, RecallMode::KeywordOnly | RecallMode::SemanticDisabled) {
            records.truncate(query.limit.max(1));
            return Ok(records);
        }

        let semantic_scores = match self.semantic.score_records(query, &records) {
            Ok(scores) => scores,
            Err(error) => {
                warn!("memory semantic recall failed, degrading to keyword recall: {error}");
                records.truncate(query.limit.max(1));
                return Ok(records);
            }
        };

        let mut scored = records
            .into_iter()
            .zip(semantic_scores)
            .map(|(record, semantic_score)| {
                let keyword_score = record_score(&record, query) as f32;
                let combined = keyword_score + semantic_score * 100.0;
                (record, combined)
            })
            .collect::<Vec<_>>();

        scored.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(Ordering::Equal)
                .then(source_rank(&right.0).cmp(&source_rank(&left.0)))
                .then(right.0.timestamp.cmp(&left.0.timestamp))
                .then(left.0.id.cmp(&right.0.id))
        });
        scored.truncate(query.limit.max(1));
        Ok(scored.into_iter().map(|(record, _)| record).collect())
    }
}

fn merge_records(
    query: &MemoryQuery,
    sqlite_records: Vec<MemoryRecord>,
    file_records: Vec<MemoryRecord>,
) -> Vec<MemoryRecord> {
    let mut merged = Vec::new();
    let mut seen_ids = HashSet::new();
    let mut sqlite_fingerprints = HashSet::new();

    for record in sqlite_records {
        let key = canonical_record_key(&record);
        seen_ids.insert(key);
        sqlite_fingerprints.insert(fingerprint(&record));
        merged.push(record);
    }

    for record in file_records {
        let key = canonical_record_key(&record);
        if seen_ids.contains(&key) {
            continue;
        }
        if sqlite_fingerprints.contains(&fingerprint(&record)) {
            continue;
        }
        seen_ids.insert(key);
        merged.push(record);
    }

    let mut score_cache = HashMap::new();
    merged.sort_by(|left, right| {
        let right_score = *score_cache
            .entry(right.id.clone())
            .or_insert_with(|| record_score(right, query));
        let left_score = *score_cache
            .entry(left.id.clone())
            .or_insert_with(|| record_score(left, query));
        right_score
            .cmp(&left_score)
            .then(source_rank(right).cmp(&source_rank(left)))
            .then(right.timestamp.cmp(&left.timestamp))
            .then(left.id.cmp(&right.id))
    });
    merged
}

fn record_score(record: &MemoryRecord, query: &MemoryQuery) -> usize {
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
    crate::recall::query_match_score(&haystack, query.query.as_deref()).unwrap_or(0)
}

fn source_rank(record: &MemoryRecord) -> u8 {
    if record.id.starts_with("diary:")
        || (!record.id.starts_with("compat:")
            && record
                .source_refs
                .iter()
                .all(|source| source.path.as_deref() != Some("memory/MEMORY.md")))
    {
        3
    } else if record.id.starts_with("compat:")
        || record.id.starts_with("compat-memory-md-")
        || record
            .source_refs
            .iter()
            .any(|source| source.path.as_deref() == Some("memory/MEMORY.md"))
    {
        1
    } else {
        2
    }
}

fn embedding_text(record: &MemoryRecord) -> String {
    format!(
        "{}\n{}\n{}\n{}",
        record.title,
        record.summary,
        record.content,
        record.tags.join(" ")
    )
}

fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MemoryDomain, MemoryScope, MemorySourceRef};
    use chrono::{DateTime, Utc};
    use std::sync::Mutex;

    struct StubSemanticRetriever {
        scores: Vec<f32>,
        error: Option<&'static str>,
    }

    impl SemanticRetriever for StubSemanticRetriever {
        fn score_records(
            &self,
            _query: &MemoryQuery,
            _records: &[MemoryRecord],
        ) -> Result<Vec<f32>> {
            match self.error {
                Some(message) => Err(agent_diva_core::Error::Internal(message.into())),
                None => Ok(self.scores.clone()),
            }
        }
    }

    struct RecordingKeywordRetriever {
        seen_limits: Mutex<Vec<usize>>,
        records: Vec<MemoryRecord>,
    }

    impl KeywordRetriever for RecordingKeywordRetriever {
        fn recall(&self, query: &MemoryQuery) -> Result<Vec<MemoryRecord>> {
            self.seen_limits.lock().unwrap().push(query.limit);
            Ok(self.records.clone())
        }
    }

    fn sample_record(id: &str, title: &str, content: &str) -> MemoryRecord {
        MemoryRecord {
            id: id.into(),
            timestamp: DateTime::parse_from_rfc3339("2026-03-27T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            domain: MemoryDomain::Workspace,
            scope: MemoryScope::Workspace,
            title: title.into(),
            summary: content.into(),
            content: content.into(),
            tags: vec!["memory".into()],
            source_refs: vec![MemorySourceRef::default()],
            confidence: 0.8,
        }
    }

    #[test]
    fn retrieval_engine_widens_keyword_limit_before_rerank() {
        let keyword = Arc::new(RecordingKeywordRetriever {
            seen_limits: Mutex::new(Vec::new()),
            records: vec![sample_record("rec-1", "A", "A")],
        });
        let reranker = Arc::new(DefaultHybridReranker::new(Arc::new(
            StubSemanticRetriever {
                scores: vec![0.0],
                error: None,
            },
        )));
        let engine = RetrievalEngine::new(keyword.clone(), reranker);

        let _ = engine
            .recall(&MemoryQuery {
                query: Some("a".into()),
                limit: 2,
                ..MemoryQuery::default()
            })
            .unwrap();

        assert_eq!(*keyword.seen_limits.lock().unwrap(), vec![8]);
    }

    #[test]
    fn hybrid_reranker_degrades_to_keyword_on_semantic_error() {
        let reranker = DefaultHybridReranker::new(Arc::new(StubSemanticRetriever {
            scores: Vec::new(),
            error: Some("semantic failed"),
        }));
        let records = vec![
            sample_record("rec-1", "First", "first"),
            sample_record("rec-2", "Second", "second"),
        ];

        let results = reranker
            .rerank(
                &MemoryQuery {
                    query: Some("first".into()),
                    limit: 1,
                    ..MemoryQuery::default()
                },
                records,
            )
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "rec-1");
    }

    #[test]
    fn hybrid_reranker_uses_semantic_scores_when_available() {
        let reranker = DefaultHybridReranker::new(Arc::new(StubSemanticRetriever {
            scores: vec![0.1, 0.9],
            error: None,
        }));
        let records = vec![
            sample_record("rec-1", "Alpha", "alpha"),
            sample_record("rec-2", "Beta", "beta"),
        ];

        let results = reranker
            .rerank(
                &MemoryQuery {
                    query: Some("alpha beta".into()),
                    limit: 2,
                    ..MemoryQuery::default()
                },
                records,
            )
            .unwrap();

        assert_eq!(results[0].id, "rec-2");
    }
}

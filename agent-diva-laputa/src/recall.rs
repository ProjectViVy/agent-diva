//! Shadow-only Recall v2 integration for Embedded Laputa.
//!
//! The types in this module are deliberately not registered with the
//! production `MemoryProvider`, AgentLoop, Manager, Tauri, or GUI.

use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};

use agent_diva_core::{
    governance::ContentDigest,
    memory::{
        RecallCandidate, RecallCandidateSource, RecallOutcome, RecallPipeline, RecallRequest,
        RecallRetrievalSource, RecallSelectionReason, RecallSourceError, RecallValidationError,
        MAX_CONFIDENCE_BPS,
    },
};
use serde::{Deserialize, Serialize};

use crate::{MemorySearchHit, TypedMemoryStore, TypedMemoryStoreError};

const MAX_QUERY_CHARS: usize = 512;
const MAX_QUERY_TERMS: usize = 32;

/// Payload-free count for one stable Recall selection reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecallReasonCount {
    pub reason: RecallSelectionReason,
    pub count: u32,
}

/// Payload-free measurements produced by one shadow Recall execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaputaRecallMetrics {
    pub candidate_count: u32,
    pub selected_count: u32,
    pub rejected_count: u32,
    pub duplicate_count: u32,
    pub duplicate_rate_bps: u16,
    pub used_tokens: u32,
    pub retrieval_micros: u64,
    pub total_micros: u64,
    pub selected_record_ids: Vec<String>,
    pub selected_content_digests: Vec<ContentDigest>,
    pub reason_counts: Vec<RecallReasonCount>,
}

/// Recall output and its content-free shadow measurements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaputaRecallShadow {
    pub outcome: RecallOutcome,
    pub metrics: LaputaRecallMetrics,
}

/// FTS5/BM25 source that returns normalized Embedded Laputa records.
#[derive(Debug)]
pub struct LaputaRecallCandidateSource {
    store: TypedMemoryStore,
    last_retrieval_micros: AtomicU64,
    last_candidate_count: AtomicU32,
}

impl LaputaRecallCandidateSource {
    pub fn new(store: TypedMemoryStore) -> Self {
        Self {
            store,
            last_retrieval_micros: AtomicU64::new(0),
            last_candidate_count: AtomicU32::new(0),
        }
    }

    fn measurements(&self) -> (u64, u32) {
        (
            self.last_retrieval_micros.load(Ordering::Relaxed),
            self.last_candidate_count.load(Ordering::Relaxed),
        )
    }
}

#[async_trait::async_trait]
impl RecallCandidateSource for LaputaRecallCandidateSource {
    async fn retrieve(
        &self,
        request: &RecallRequest,
    ) -> Result<Vec<RecallCandidate>, RecallSourceError> {
        let started = Instant::now();
        self.last_candidate_count.store(0, Ordering::Relaxed);
        self.last_retrieval_micros.store(0, Ordering::Relaxed);
        let expression = normalize_fts_query(&request.query).ok_or(RecallSourceError)?;
        let hits = self
            .store
            .search_visible(&expression, &request.scope, request.max_candidates)
            .await;
        self.last_retrieval_micros.store(
            started.elapsed().as_micros().min(u64::MAX as u128) as u64,
            Ordering::Relaxed,
        );
        let mut hits = hits.map_err(map_store_error)?;
        hits.sort_by(compare_hits);
        let candidates = normalize_hits(hits);
        self.last_candidate_count
            .store(candidates.len() as u32, Ordering::Relaxed);
        Ok(candidates)
    }
}

/// Public shadow boundary. It never mutates records or injects prompt content.
#[derive(Debug)]
pub struct LaputaRecallService {
    source: Arc<LaputaRecallCandidateSource>,
    run_lock: tokio::sync::Mutex<()>,
}

impl LaputaRecallService {
    /// Open the Embedded Laputa store for shadow Recall.
    pub async fn open(
        workspace_root: impl AsRef<Path>,
        workspace_id: impl Into<String>,
    ) -> Result<Self, TypedMemoryStoreError> {
        let store = TypedMemoryStore::open(workspace_root, workspace_id).await?;
        Ok(Self::new(store))
    }

    pub fn new(store: TypedMemoryStore) -> Self {
        Self {
            source: Arc::new(LaputaRecallCandidateSource::new(store)),
            run_lock: tokio::sync::Mutex::new(()),
        }
    }

    /// Execute deterministic Recall v2 and return payload-free measurements.
    pub async fn recall_shadow(
        &self,
        request: &RecallRequest,
    ) -> Result<LaputaRecallShadow, RecallValidationError> {
        let _guard = self.run_lock.lock().await;
        let started = Instant::now();
        let outcome = RecallPipeline
            .execute(request, self.source.as_ref())
            .await?;
        let total_micros = started.elapsed().as_micros().min(u64::MAX as u128) as u64;
        let (retrieval_micros, measured_candidates) = self.source.measurements();
        let candidate_count = if outcome.trace.is_empty() {
            measured_candidates.min(request.max_candidates)
        } else {
            outcome.trace.len() as u32
        };
        Ok(LaputaRecallShadow {
            metrics: metrics_for(&outcome, candidate_count, retrieval_micros, total_micros),
            outcome,
        })
    }
}

fn normalize_fts_query(query: &str) -> Option<String> {
    let bounded = query.chars().take(MAX_QUERY_CHARS).collect::<String>();
    let terms = bounded
        .split_whitespace()
        .filter_map(|term| {
            let normalized = term
                .chars()
                .filter(|character| !character.is_control())
                .collect::<String>();
            (!normalized.is_empty()).then(|| format!("\"{}\"", normalized.replace('"', "\"\"")))
        })
        .take(MAX_QUERY_TERMS)
        .collect::<Vec<_>>();
    (!terms.is_empty()).then(|| terms.join(" OR "))
}

fn compare_hits(left: &MemorySearchHit, right: &MemorySearchHit) -> std::cmp::Ordering {
    left.bm25
        .total_cmp(&right.bm25)
        .then_with(|| {
            right
                .stored
                .record
                .effective_at
                .cmp(&left.stored.record.effective_at)
        })
        .then_with(|| left.stored.record.id.cmp(&right.stored.record.id))
}

fn normalize_hits(hits: Vec<MemorySearchHit>) -> Vec<RecallCandidate> {
    let best = hits.first().map(|hit| hit.bm25).unwrap_or(0.0);
    let worst = hits.last().map(|hit| hit.bm25).unwrap_or(best);
    let spread = worst - best;
    hits.into_iter()
        .map(|hit| {
            let relevance = if !spread.is_finite() || spread.abs() <= f64::EPSILON {
                MAX_CONFIDENCE_BPS
            } else {
                let normalized = ((worst - hit.bm25) / spread).clamp(0.0, 1.0);
                (5_000.0 + normalized * 5_000.0).round() as u16
            };
            RecallCandidate {
                record: hit.stored.record,
                relevance_bps: relevance,
                retrieval_source: RecallRetrievalSource::Laputa,
            }
        })
        .collect()
}

fn map_store_error(_error: TypedMemoryStoreError) -> RecallSourceError {
    RecallSourceError
}

fn metrics_for(
    outcome: &RecallOutcome,
    candidate_count: u32,
    retrieval_micros: u64,
    total_micros: u64,
) -> LaputaRecallMetrics {
    let mut counts = BTreeMap::<&'static str, (RecallSelectionReason, u32)>::new();
    for trace in &outcome.trace {
        let key = reason_key(&trace.reason);
        let entry = counts.entry(key).or_insert((trace.reason.clone(), 0));
        entry.1 += 1;
    }
    let duplicate_count = outcome
        .trace
        .iter()
        .filter(|trace| trace.reason == RecallSelectionReason::Duplicate)
        .count() as u32;
    let duplicate_rate_bps = if candidate_count == 0 {
        0
    } else {
        ((u64::from(duplicate_count) * 10_000) / u64::from(candidate_count)) as u16
    };
    LaputaRecallMetrics {
        candidate_count,
        selected_count: outcome.budget.selected_count,
        rejected_count: outcome.budget.rejected_count,
        duplicate_count,
        duplicate_rate_bps,
        used_tokens: outcome.budget.used_tokens,
        retrieval_micros,
        total_micros,
        selected_record_ids: outcome
            .selected_records
            .iter()
            .map(|record| record.id.clone())
            .collect(),
        selected_content_digests: outcome
            .selected_records
            .iter()
            .map(|record| record.provenance.content_digest.clone())
            .collect(),
        reason_counts: counts
            .into_values()
            .map(|(reason, count)| RecallReasonCount { reason, count })
            .collect(),
    }
}

fn reason_key(reason: &RecallSelectionReason) -> &'static str {
    match reason {
        RecallSelectionReason::Selected => "selected",
        RecallSelectionReason::WorkspaceMismatch => "workspace_mismatch",
        RecallSelectionReason::SessionMismatch => "session_mismatch",
        RecallSelectionReason::Expired => "expired",
        RecallSelectionReason::Tombstoned => "tombstoned",
        RecallSelectionReason::Superseded => "superseded",
        RecallSelectionReason::Duplicate => "duplicate",
        RecallSelectionReason::TrustDenied => "trust_denied",
        RecallSelectionReason::SensitivityDenied => "sensitivity_denied",
        RecallSelectionReason::Invalid => "invalid",
        RecallSelectionReason::OverBudget => "over_budget",
        RecallSelectionReason::CandidateLimit => "candidate_limit",
        RecallSelectionReason::Unknown => "unknown",
    }
}

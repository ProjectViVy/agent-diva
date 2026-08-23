//! Phase-level structured diagnostics for AutoDream runs.
//!
//! Tracing fields and durable JSONL events share the same payload: `run_id`,
//! `phase`, input summary, gate rejection, `proposal_id`, and failure code.
//! This module does not write memory, persona, or BML authority.

use agent_diva_core::evolution::{AutoDreamFailureCode, AutoDreamInputSummary};

use crate::AutoDreamEvent;

pub(crate) struct Diagnostic {
    run_id: String,
    kind: String,
    message: String,
    phase: Option<String>,
    input_summary: Option<String>,
    gate_code: Option<String>,
    proposal_id: Option<String>,
    failure_code: Option<String>,
}

impl Diagnostic {
    pub(crate) fn new(
        run_id: impl Into<String>,
        kind: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            kind: kind.into(),
            message: message.into(),
            phase: None,
            input_summary: None,
            gate_code: None,
            proposal_id: None,
            failure_code: None,
        }
    }

    pub(crate) fn phase(mut self, phase: impl Into<String>) -> Self {
        self.phase = Some(phase.into());
        self
    }

    pub(crate) fn input_summary(mut self, summary: impl Into<String>) -> Self {
        self.input_summary = Some(summary.into());
        self
    }

    pub(crate) fn gate_code(mut self, code: impl Into<String>) -> Self {
        self.gate_code = Some(code.into());
        self
    }

    pub(crate) fn proposal_id(mut self, id: impl Into<String>) -> Self {
        self.proposal_id = Some(id.into());
        self
    }

    pub(crate) fn failure_code(mut self, code: impl Into<String>) -> Self {
        self.failure_code = Some(code.into());
        self
    }

    pub(crate) fn into_event(self) -> AutoDreamEvent {
        emit_tracing(&self);
        AutoDreamEvent {
            id: format!("evt-{}", uuid::Uuid::new_v4()),
            run_id: Some(self.run_id),
            kind: self.kind,
            message: self.message,
            created_at: chrono::Utc::now(),
            phase: self.phase,
            input_summary: self.input_summary,
            gate_code: self.gate_code,
            proposal_id: self.proposal_id,
            failure_code: self.failure_code,
        }
    }
}

pub(crate) fn format_input_summary(summary: &AutoDreamInputSummary) -> String {
    let sources = summary
        .included_sources
        .iter()
        .map(|source| source.source.as_str())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "items={} sources={} truncated={} bytes={} omissions={}",
        summary.total_items,
        sources,
        summary.truncated,
        summary.total_bytes,
        summary.omissions.len()
    )
}

pub(crate) fn failure_code_name(code: AutoDreamFailureCode) -> &'static str {
    match code {
        AutoDreamFailureCode::Cancelled => "cancelled",
        AutoDreamFailureCode::InputUnavailable => "input_unavailable",
        AutoDreamFailureCode::WorkerTimeout => "worker_timeout",
        AutoDreamFailureCode::WorkerFailed => "worker_failed",
        AutoDreamFailureCode::ReportGenerationFailed => "report_generation_failed",
        AutoDreamFailureCode::StaleRunRecovered => "stale_run_recovered",
        AutoDreamFailureCode::LegacyIncomplete => "legacy_incomplete",
        AutoDreamFailureCode::ProviderUnavailable => "provider_unavailable",
        AutoDreamFailureCode::ProviderTimeout => "provider_timeout",
        AutoDreamFailureCode::ProviderFailed => "provider_failed",
        AutoDreamFailureCode::InvalidCandidate => "invalid_candidate",
    }
}

fn emit_tracing(diagnostic: &Diagnostic) {
    let phase = diagnostic.phase.as_deref().unwrap_or("");
    let input_summary = diagnostic.input_summary.as_deref().unwrap_or("");
    let gate_code = diagnostic.gate_code.as_deref().unwrap_or("");
    let proposal_id = diagnostic.proposal_id.as_deref().unwrap_or("");
    let failure_code = diagnostic.failure_code.as_deref().unwrap_or("");
    if is_warn(&diagnostic.kind) {
        tracing::warn!(
            run_id = %diagnostic.run_id,
            phase,
            kind = %diagnostic.kind,
            input_summary,
            gate_code,
            proposal_id,
            failure_code,
            "{}",
            diagnostic.message
        );
    } else {
        tracing::info!(
            run_id = %diagnostic.run_id,
            phase,
            kind = %diagnostic.kind,
            input_summary,
            gate_code,
            proposal_id,
            failure_code,
            "{}",
            diagnostic.message
        );
    }
}

fn is_warn(kind: &str) -> bool {
    matches!(
        kind,
        "worker_failed"
            | "worker_cancelled"
            | "worker_timed_out"
            | "skill_candidate_rejected"
            | "skill_reflection_degraded"
            | "actmem_work_retry"
            | "input_collection_failed"
            | "actmem_work_failed"
            | "memory_home_missing"
            | "restricted_profile_denied"
    )
}

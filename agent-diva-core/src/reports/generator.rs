use async_trait::async_trait;

use super::{
    fact_bundle::ReportFactBundle,
    narrative::{CuratedReportNarrative, ReportGenerationMetadata},
    period::ReportPeriod,
};

/// Options for a single narrative generation request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportNarrativeOptions {
    pub period: ReportPeriod,
    pub language: String,
    pub max_output_tokens: u32,
    pub timeout_secs: u64,
}

impl Default for ReportNarrativeOptions {
    fn default() -> Self {
        Self {
            period: ReportPeriod::Daily,
            language: "zh-CN".to_string(),
            max_output_tokens: 1500,
            timeout_secs: 60,
        }
    }
}

/// Sanitized narrative generation failures (no raw session content).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReportNarrativeError {
    #[error("llm curation disabled")]
    Disabled,
    #[error("narrative generation timed out")]
    Timeout,
    #[error("provider error")]
    Provider,
    #[error("invalid narrative json")]
    InvalidJson,
    #[error("evidence validation failed")]
    EvidenceValidation,
    #[error("input or output budget exceeded")]
    Budget,
    #[error("empty narrative content")]
    Empty,
}

impl ReportNarrativeError {
    pub fn category(&self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Timeout => "timeout",
            Self::Provider => "provider",
            Self::InvalidJson => "invalid_json",
            Self::EvidenceValidation => "evidence_validation",
            Self::Budget => "budget",
            Self::Empty => "empty",
        }
    }
}

/// Provider-agnostic report narrative generator.
///
/// Implementations must not register tools, browse the network for open-ended
/// research, or accept arbitrary caller-controlled prompt templates.
#[async_trait]
pub trait ReportNarrativeGenerator: Send + Sync {
    async fn generate(
        &self,
        bundle: &ReportFactBundle,
        options: &ReportNarrativeOptions,
    ) -> Result<(CuratedReportNarrative, ReportGenerationMetadata), ReportNarrativeError>;
}

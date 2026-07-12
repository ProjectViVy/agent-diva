use serde::{Deserialize, Serialize};

/// How the report narrative was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationMode {
    LlmCurated,
    DeterministicFallback,
}

impl GenerationMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LlmCurated => "llm_curated",
            Self::DeterministicFallback => "deterministic_fallback",
        }
    }
}

/// Coverage quality for a generated report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageStatus {
    Complete,
    Partial,
    Fallback,
}

impl CoverageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Fallback => "fallback",
        }
    }
}

pub const NARRATIVE_SCHEMA_VERSION: u32 = 1;
pub const PROMPT_VERSION: &str = "report-curation-v1";

/// Single narrative bullet that must cite evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NarrativeItem {
    pub text: String,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
}

/// Structured LLM-curated report narrative (Chinese body content).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CuratedReportNarrative {
    pub executive_summary: String,
    #[serde(default)]
    pub themes: Vec<NarrativeItem>,
    #[serde(default)]
    pub accomplishments: Vec<NarrativeItem>,
    #[serde(default)]
    pub decisions: Vec<NarrativeItem>,
    #[serde(default)]
    pub risks_or_blockers: Vec<NarrativeItem>,
    #[serde(default)]
    pub next_actions: Vec<NarrativeItem>,
    #[serde(default)]
    pub coverage_notes: Vec<NarrativeItem>,
}

/// Audit metadata attached to a report generation attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportGenerationMetadata {
    pub generation_mode: GenerationMode,
    pub narrative_schema_version: u32,
    pub prompt_version: String,
    pub coverage_status: CoverageStatus,
    /// Provider model id used for audit only (never rewritten).
    pub model: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub duration_ms: Option<u64>,
    /// Sanitized failure category when falling back.
    pub failure_reason: Option<String>,
}

impl ReportGenerationMetadata {
    pub fn llm_curated(
        model: Option<String>,
        coverage_status: CoverageStatus,
        input_tokens: Option<u64>,
        output_tokens: Option<u64>,
        duration_ms: Option<u64>,
    ) -> Self {
        Self {
            generation_mode: GenerationMode::LlmCurated,
            narrative_schema_version: NARRATIVE_SCHEMA_VERSION,
            prompt_version: PROMPT_VERSION.to_string(),
            coverage_status,
            model,
            input_tokens,
            output_tokens,
            duration_ms,
            failure_reason: None,
        }
    }

    pub fn deterministic_fallback(reason: impl Into<String>, coverage: CoverageStatus) -> Self {
        Self {
            generation_mode: GenerationMode::DeterministicFallback,
            narrative_schema_version: NARRATIVE_SCHEMA_VERSION,
            prompt_version: PROMPT_VERSION.to_string(),
            coverage_status: coverage,
            model: None,
            input_tokens: None,
            output_tokens: None,
            duration_ms: None,
            failure_reason: Some(reason.into()),
        }
    }
}

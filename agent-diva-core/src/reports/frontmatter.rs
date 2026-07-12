use serde::{Deserialize, Serialize};

/// YAML frontmatter parsed from rhythm / notebook report markdown.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RhythmReportFrontmatter {
    pub period: Option<String>,
    pub date: Option<String>,
    pub week: Option<String>,
    pub month: Option<String>,
    pub generated_at: Option<String>,
    pub generated_by: Option<String>,
    pub source: Option<String>,
    pub session_count: Option<u64>,
    pub token_used: Option<u64>,
    pub schema_version: Option<serde_yaml::Value>,
    pub fallback_used: Option<bool>,
    pub daily_inputs_count: Option<u64>,
    pub missing_daily_dates_count: Option<u64>,
    /// `llm_curated` or `deterministic_fallback`.
    pub generation_mode: Option<String>,
    pub narrative_schema_version: Option<u32>,
    pub prompt_version: Option<String>,
    /// `complete`, `partial`, or `fallback`.
    pub coverage_status: Option<String>,
    /// Sanitized fallback reason category (never raw session text).
    pub fallback_reason: Option<String>,
}

/// Parsed rhythm report document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RhythmReportDocument {
    pub frontmatter: RhythmReportFrontmatter,
    pub title: String,
    pub summary: String,
    pub body: String,
}

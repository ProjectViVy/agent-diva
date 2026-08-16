//! Stable domain contracts shared by AutoDream, reports, memory, and UI.
//!
//! The mixed-domain proposal envelope was removed by the cognitive clean
//! break: memory CRUD is direct on BML, persona changes flow through the
//! persona workspace, and skill evolution flows through SkillHome requests.

use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Errors returned by governance domain helpers.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EvolutionError {
    /// The Laputa section string is not part of the v1 governance contract.
    #[error("unknown Laputa section: {0}")]
    UnknownLaputaSection(String),
}

/// Evidence origin for a governance proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    Session,
    Report,
    AutoDreamRun,
    UserInput,
    File,
    ContextCompaction,
    ExperienceJournal,
    RecallFeedback,
}

/// Typed pointer to bounded evidence used by governance review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub id: String,
    pub source: EvidenceSource,
    pub uri: String,
    pub excerpt: Option<String>,
    pub hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Return true when evidence can act as primary authority evidence.
///
/// Context compaction is session-local prompt survival. It can support review as
/// secondary evidence, but cannot be the sole authority behind durable changes.
pub fn is_primary_governance_evidence(evidence: &EvidenceRef) -> bool {
    !matches!(evidence.source, EvidenceSource::ContextCompaction)
}

/// Canonical names for the legacy Laputa sections.
///
/// Retired as production authority by the cognitive clean break; the variants
/// remain as the import vocabulary consumed by the offline migration tool.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaputaSectionName {
    Identity,
    Relationship,
    Commitment,
    Preferences,
    MemoryMd,
    Daily,
    Weekly,
    Monthly,
    Changelog,
}

impl LaputaSectionName {
    /// Stable snake_case section name used by file paths and APIs.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Relationship => "relationship",
            Self::Commitment => "commitment",
            Self::Preferences => "preferences",
            Self::MemoryMd => "memory_md",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Changelog => "changelog",
        }
    }
}

impl FromStr for LaputaSectionName {
    type Err = EvolutionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "identity" => Ok(Self::Identity),
            "relationship" => Ok(Self::Relationship),
            "commitment" => Ok(Self::Commitment),
            "preferences" => Ok(Self::Preferences),
            "memory_md" => Ok(Self::MemoryMd),
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            "monthly" => Ok(Self::Monthly),
            "changelog" => Ok(Self::Changelog),
            // Retired section names intentionally fall through to the stable
            // UnknownLaputaSection failure code.
            other => Err(EvolutionError::UnknownLaputaSection(other.to_string())),
        }
    }
}

impl fmt::Display for LaputaSectionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Lifecycle status for an AutoDream run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoDreamRunState {
    Pending,
    Running,
    Cancelled,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoDreamOrchestrationPhase {
    Queued,
    Gathering,
    Reflecting,
    Validating,
    Publishing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoDreamOrchestrationRecord {
    pub schema_version: u32,
    pub phase: AutoDreamOrchestrationPhase,
    pub attempt: u32,
    pub deadline_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Stable, payload-free reason code for a terminal AutoDream run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoDreamFailureCode {
    Cancelled,
    InputUnavailable,
    WorkerTimeout,
    WorkerFailed,
    ReportGenerationFailed,
    StaleRunRecovered,
    LegacyIncomplete,
    ProviderUnavailable,
    ProviderTimeout,
    ProviderFailed,
    InvalidCandidate,
}

/// AutoDream run summary referenced by reports, and audit trails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoDreamRunRecord {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub state: AutoDreamRunState,
    pub trigger: String,
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input_summary: Option<AutoDreamInputSummary>,
    pub proposal_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub orchestration: Option<AutoDreamOrchestrationRecord>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub failure_code: Option<AutoDreamFailureCode>,
    pub error: Option<String>,
}

/// Bounded AutoDream input collection summary persisted on the run record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoDreamInputSummary {
    pub total_items: usize,
    pub included_sources: Vec<AutoDreamInputSourceSummary>,
    pub omissions: Vec<AutoDreamInputOmission>,
    pub truncated: bool,
    pub total_bytes: usize,
}

/// Per-source input accounting for a single AutoDream run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoDreamInputSourceSummary {
    pub source: String,
    pub included_items: usize,
    pub total_bytes: usize,
    pub truncated: bool,
}

/// Structured omission captured during AutoDream input collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoDreamInputOmission {
    pub source: String,
    pub detail: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_time() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-06-13T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn test_retired_section_names_fail_with_stable_error_code() {
        for retired in [
            "history_md",
            "journal_reflective",
            "proposal_inbox",
            "report_indexes",
            "aaak_summaries",
        ] {
            let error = retired.parse::<LaputaSectionName>().unwrap_err();
            assert_eq!(
                error,
                EvolutionError::UnknownLaputaSection(retired.to_string())
            );
            // On-disk history referencing retired sections must fail serde
            // deserialization deterministically instead of panicking.
            assert!(serde_json::from_str::<LaputaSectionName>(&format!("\"{retired}\"")).is_err());
        }
    }

    #[test]
    fn test_compaction_only_evidence_is_not_authoritative_for_proposals() {
        let evidence = vec![EvidenceRef {
            id: "compact-1".to_string(),
            source: EvidenceSource::ContextCompaction,
            uri: "capsule://compact-1.md".to_string(),
            excerpt: Some("current-session summary".to_string()),
            hash: None,
            created_at: sample_time(),
        }];

        assert!(!evidence.iter().all(is_primary_governance_evidence));
    }
}

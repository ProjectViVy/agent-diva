//! Stable domain contracts shared by Laputa, AutoDream, reports, and UI.

use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Errors returned by governance domain helpers.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EvolutionError {
    /// The proposal type string is not part of the v1 governance contract.
    #[error("unknown proposal type: {0}")]
    UnknownProposalType(String),
}

/// Evidence origin for a governance proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    Session,
    Report,
    AutoDreamRun,
    LaputaSection,
    UserInput,
    File,
    ContextCompaction,
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

/// Supported EVO-DIVA governance proposal categories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalType {
    MemoryPatch,
    JournalNote,
    LearningNote,
    IdentityPatch,
    RelationshipUpdate,
    CommitmentSet,
    SopCreate,
    Deprecation,
}

impl ProposalType {
    /// Route this proposal type to the canonical v1 Laputa section.
    pub fn target_section(&self) -> LaputaSectionName {
        match self {
            Self::MemoryPatch => LaputaSectionName::MemoryMd,
            Self::JournalNote => LaputaSectionName::JournalReflective,
            Self::LearningNote => LaputaSectionName::Preferences,
            Self::IdentityPatch | Self::SopCreate => LaputaSectionName::Identity,
            Self::RelationshipUpdate => LaputaSectionName::Relationship,
            Self::CommitmentSet => LaputaSectionName::Commitment,
            Self::Deprecation => LaputaSectionName::Changelog,
        }
    }
}

impl FromStr for ProposalType {
    type Err = EvolutionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "memory_patch" => Ok(Self::MemoryPatch),
            "journal_note" => Ok(Self::JournalNote),
            "learning_note" => Ok(Self::LearningNote),
            "identity_patch" => Ok(Self::IdentityPatch),
            "relationship_update" => Ok(Self::RelationshipUpdate),
            "commitment_set" => Ok(Self::CommitmentSet),
            "sop_create" => Ok(Self::SopCreate),
            "deprecation" => Ok(Self::Deprecation),
            other => Err(EvolutionError::UnknownProposalType(other.to_string())),
        }
    }
}

impl fmt::Display for ProposalType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::MemoryPatch => "memory_patch",
            Self::JournalNote => "journal_note",
            Self::LearningNote => "learning_note",
            Self::IdentityPatch => "identity_patch",
            Self::RelationshipUpdate => "relationship_update",
            Self::CommitmentSet => "commitment_set",
            Self::SopCreate => "sop_create",
            Self::Deprecation => "deprecation",
        };
        f.write_str(value)
    }
}

/// Lifecycle state of a governance proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalState {
    PendingReview,
    Approved,
    Rejected,
    Edited,
    Applied,
    Reverted,
    Superseded,
    NeedsAttention,
    RunFailed,
}

/// Risk level attached to a proposal during review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Canonical names for the 14 Laputa v1 sections.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaputaSectionName {
    Identity,
    Relationship,
    Commitment,
    Preferences,
    MemoryMd,
    HistoryMd,
    Daily,
    Weekly,
    Monthly,
    JournalReflective,
    ProposalInbox,
    Changelog,
    ReportIndexes,
    AaakSummaries,
}

/// Shared governance proposal envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionProposal {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub proposal_type: ProposalType,
    pub target_section: LaputaSectionName,
    pub evidence_refs: Vec<EvidenceRef>,
    pub proposed_patch: String,
    pub risk_level: RiskLevel,
    pub state: ProposalState,
    pub source_run_id: Option<String>,
}

/// Action represented by a Laputa changelog record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangelogAction {
    Apply,
    Revert,
    Rollback,
}

/// Durable record of an applied, reverted, or rolled-back governance change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangelogRecord {
    pub id: String,
    pub action: ChangelogAction,
    pub target_section: LaputaSectionName,
    pub before: String,
    pub after: String,
    pub diff: String,
    pub proposal_id: Option<String>,
    pub audit_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub applied_by: String,
}

/// Kind of audit event emitted by the governance spine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventKind {
    ProposalCreated,
    ProposalApproved,
    ProposalRejected,
    ProposalEdited,
    ProposalApplied,
    ProposalReverted,
    RollbackRequested,
    RollbackApplied,
    WriteFailed,
    NeedsAttention,
}

/// Auditable governance event for proposal and Laputa operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub kind: AuditEventKind,
    pub actor: String,
    pub proposal_id: Option<String>,
    pub target_section: Option<LaputaSectionName>,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

/// Request to rollback a previously recorded changelog entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackRequest {
    pub changelog_id: String,
    pub requested_by: String,
    pub reason: String,
    pub requested_at: DateTime<Utc>,
}

/// Lifecycle status for an AutoDream run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoDreamRunState {
    Pending,
    Running,
    Completed,
    Failed,
}

/// AutoDream run summary referenced by proposals, reports, and audit trails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoDreamRunRecord {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub state: AutoDreamRunState,
    pub trigger: String,
    pub summary: Option<String>,
    pub proposal_ids: Vec<String>,
    pub error: Option<String>,
}

/// Route a raw proposal type string to its canonical v1 Laputa section.
pub fn route_proposal_type(value: &str) -> Result<LaputaSectionName, EvolutionError> {
    value
        .parse::<ProposalType>()
        .map(|kind| kind.target_section())
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
    fn test_evolution_proposal_json_round_trip_uses_snake_case_fields_and_variants() {
        let now = sample_time();
        let proposal = EvolutionProposal {
            id: "proposal-1".to_string(),
            created_at: now,
            updated_at: now,
            created_by: "autodream".to_string(),
            proposal_type: ProposalType::MemoryPatch,
            target_section: LaputaSectionName::MemoryMd,
            evidence_refs: vec![EvidenceRef {
                id: "evidence-1".to_string(),
                source: EvidenceSource::Session,
                uri: "session://abc".to_string(),
                excerpt: Some("bounded excerpt".to_string()),
                hash: Some("sha256:abc".to_string()),
                created_at: now,
            }],
            proposed_patch: "append memory note".to_string(),
            risk_level: RiskLevel::Low,
            state: ProposalState::PendingReview,
            source_run_id: Some("run-1".to_string()),
        };

        let json = serde_json::to_string(&proposal).unwrap();
        assert!(json.contains("\"proposal_type\":\"memory_patch\""));
        assert!(json.contains("\"target_section\":\"memory_md\""));
        assert!(json.contains("\"risk_level\":\"low\""));
        assert!(json.contains("\"state\":\"pending_review\""));
        assert!(json.contains("\"evidence_refs\""));

        let decoded: EvolutionProposal = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, proposal);
    }

    #[test]
    fn test_all_v1_proposal_types_route_to_expected_laputa_sections() {
        let cases = [
            (ProposalType::MemoryPatch, LaputaSectionName::MemoryMd),
            (
                ProposalType::JournalNote,
                LaputaSectionName::JournalReflective,
            ),
            (ProposalType::LearningNote, LaputaSectionName::Preferences),
            (ProposalType::IdentityPatch, LaputaSectionName::Identity),
            (
                ProposalType::RelationshipUpdate,
                LaputaSectionName::Relationship,
            ),
            (ProposalType::CommitmentSet, LaputaSectionName::Commitment),
            (ProposalType::SopCreate, LaputaSectionName::Identity),
            (ProposalType::Deprecation, LaputaSectionName::Changelog),
        ];

        for (proposal_type, expected_section) in cases {
            assert_eq!(proposal_type.target_section(), expected_section);
        }
    }

    #[test]
    fn test_unknown_proposal_type_returns_typed_error() {
        let error = route_proposal_type("unsupported_change").unwrap_err();

        assert_eq!(
            error,
            EvolutionError::UnknownProposalType("unsupported_change".to_string())
        );
    }
}

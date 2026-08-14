use agent_diva_core::{
    evolution::{CandidateValue, EvidenceRef, EvidenceSource, MemoryCandidate, ProposalType},
    memory::{MemoryScope, MemorySensitivity},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflectionEvidence {
    pub evidence: EvidenceRef,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedReflectionInput {
    pub schema_version: u32,
    pub workspace_id: String,
    pub run_id: String,
    pub evidence: Vec<ReflectionEvidence>,
    pub existing_memory_digests: Vec<String>,
    /// Digests of records targeted by a supersedes tombstone. Candidates
    /// whose content matches one of these digests are rejected with
    /// `Superseded` (Wave 5) — the source record was already deposed, so
    /// re-adding equivalent content would resurrect dead authority.
    #[serde(default)]
    pub superseded_memory_digests: Vec<String>,
    pub max_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflectionOutput {
    pub schema_version: u32,
    pub candidates: Vec<MemoryCandidate>,
    #[serde(default)]
    pub diagnostic_codes: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ReflectionError {
    #[error("reflection provider is unavailable")]
    ProviderUnavailable,
    #[error("reflection provider timed out")]
    ProviderTimeout,
    #[error("reflection provider returned an invalid schema")]
    InvalidSchema,
    #[error("reflection provider failed")]
    ProviderFailed,
}

#[async_trait]
pub trait ReflectionEngine: Send + Sync {
    async fn reflect(
        &self,
        input: BoundedReflectionInput,
    ) -> std::result::Result<ReflectionOutput, ReflectionError>;
}

/// Bounded Skill index supplied to the Skill reflection provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillReflectionIndex {
    pub slug: String,
    pub content_hash: String,
}

/// Strict S4 reflection input. ACTMEM is organized before this is assembled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillReflectionInput {
    pub schema_version: u32,
    pub run_id: String,
    pub organized_work: String,
    pub pulse: String,
    pub recap: String,
    pub evidence: Vec<ReflectionEvidence>,
    pub memrules: String,
    pub skills: Vec<SkillReflectionIndex>,
    pub max_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillReflectionCandidate {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub proposed_markdown: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillReflectionOutput {
    pub schema_version: u32,
    pub candidates: Vec<SkillReflectionCandidate>,
    #[serde(default)]
    pub diagnostic_codes: Vec<String>,
}

#[async_trait]
pub trait SkillReflectionEngine: Send + Sync {
    async fn reflect_skills(
        &self,
        input: SkillReflectionInput,
    ) -> std::result::Result<SkillReflectionOutput, ReflectionError>;
}

#[derive(Debug, Clone)]
pub struct DeterministicReflectionEngine {
    output: Option<ReflectionOutput>,
}

impl DeterministicReflectionEngine {
    pub fn new(output: ReflectionOutput) -> Self {
        Self {
            output: Some(output),
        }
    }

    pub fn evidence_echo() -> Self {
        Self { output: None }
    }
}

#[async_trait]
impl ReflectionEngine for DeterministicReflectionEngine {
    async fn reflect(
        &self,
        input: BoundedReflectionInput,
    ) -> std::result::Result<ReflectionOutput, ReflectionError> {
        if let Some(output) = &self.output {
            return Ok(output.clone());
        }
        let Some(evidence) = input
            .evidence
            .iter()
            .find(|item| item.evidence.source != EvidenceSource::ContextCompaction)
        else {
            return Ok(ReflectionOutput {
                schema_version: 1,
                candidates: Vec::new(),
                diagnostic_codes: vec!["no_primary_evidence".to_string()],
            });
        };
        let (proposal_type, content) = if evidence.evidence.source == EvidenceSource::RecallFeedback
            && evidence.summary.contains("corrected=true")
        {
            let Some(record_id) = feedback_record_id(&evidence.summary) else {
                return Ok(ReflectionOutput {
                    schema_version: 1,
                    candidates: Vec::new(),
                    diagnostic_codes: vec!["corrected_feedback_missing_record_id".to_string()],
                });
            };
            (
                ProposalType::Deprecation,
                serde_json::json!({
                    "schema_version": 1,
                    "target_record_id": record_id,
                    "reason": "user_correction"
                })
                .to_string(),
            )
        } else {
            (
                ProposalType::MemoryPatch,
                format!("Observed durable evidence: {}", evidence.summary),
            )
        };
        let candidate_id = format!(
            "candidate-{:x}",
            sha2::Sha256::digest(format!("{}\0{content}", input.run_id).as_bytes())
        );
        Ok(ReflectionOutput {
            schema_version: 1,
            candidates: vec![MemoryCandidate {
                candidate_id,
                proposal_type,
                content,
                evidence_refs: vec![evidence.evidence.clone()],
                confidence: 80,
                scope: MemoryScope {
                    tenant_id: "local".to_string(),
                    workspace_id: input.workspace_id,
                    session_id: None,
                },
                sensitivity: MemorySensitivity::Private,
                expected_value: CandidateValue::Medium,
                invalidation_conditions: vec!["contradicted_by_verified_evidence".to_string()],
            }],
            diagnostic_codes: Vec::new(),
        })
    }
}

fn feedback_record_id(summary: &str) -> Option<&str> {
    summary
        .split_whitespace()
        .find_map(|item| item.strip_prefix("record="))
        .filter(|value| !value.is_empty())
}

use agent_diva_core::evolution::EvidenceRef;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflectionEvidence {
    pub evidence: EvidenceRef,
    pub summary: String,
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

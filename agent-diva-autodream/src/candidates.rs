use std::collections::{HashMap, HashSet};

use agent_diva_core::{
    evolution::{CandidateValue, EvidenceSource, MemoryCandidate},
    memory::MemorySensitivity,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::BoundedReflectionInput;

const MAX_CANDIDATES: usize = 8;
const MAX_CONTENT_BYTES: usize = 2_048;
const MIN_CONFIDENCE: u8 = 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateRejectionCode {
    MissingEvidence,
    SecondaryEvidenceOnly,
    UnknownEvidence,
    WorkspaceMismatch,
    InvalidSensitivity,
    LowConfidence,
    LowValue,
    EmptyContent,
    CapacityExceeded,
    Duplicate,
    PromptInjection,
    SensitiveContent,
    Contradiction,
    UnsupportedType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateRejection {
    pub candidate_id: String,
    pub code: CandidateRejectionCode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateGateResult {
    pub accepted: Vec<MemoryCandidate>,
    pub rejected: Vec<CandidateRejection>,
}

#[derive(Debug, Default, Clone)]
pub struct CandidateGate;

impl CandidateGate {
    pub fn evaluate(
        &self,
        input: &BoundedReflectionInput,
        candidates: Vec<MemoryCandidate>,
        local_existing_memory: &[String],
    ) -> CandidateGateResult {
        let allowed_evidence = input
            .evidence
            .iter()
            .map(|item| (item.evidence.id.as_str(), &item.evidence))
            .collect::<HashMap<_, _>>();
        let existing = input
            .existing_memory_digests
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        let mut accepted_digests = HashSet::new();
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();

        for mut candidate in candidates {
            candidate.candidate_id = format!(
                "candidate-{}-{}",
                input.run_id,
                content_digest(&candidate.content).trim_start_matches("sha256:")
            );
            let code = self.rejection_code(
                input,
                &candidate,
                &allowed_evidence,
                &existing,
                &accepted_digests,
                accepted.len(),
                local_existing_memory,
            );
            if let Some(code) = code {
                rejected.push(CandidateRejection {
                    candidate_id: candidate.candidate_id,
                    code,
                });
            } else {
                accepted_digests.insert(content_digest(&candidate.content));
                accepted.push(candidate);
            }
        }
        CandidateGateResult { accepted, rejected }
    }

    #[allow(clippy::too_many_arguments)]
    fn rejection_code(
        &self,
        input: &BoundedReflectionInput,
        candidate: &MemoryCandidate,
        allowed_evidence: &HashMap<&str, &agent_diva_core::evolution::EvidenceRef>,
        existing: &HashSet<String>,
        accepted_digests: &HashSet<String>,
        accepted_count: usize,
        local_existing_memory: &[String],
    ) -> Option<CandidateRejectionCode> {
        let content = candidate.content.trim();
        if content.is_empty() || content.len() > MAX_CONTENT_BYTES {
            return Some(CandidateRejectionCode::EmptyContent);
        }
        if candidate.proposal_type == agent_diva_core::evolution::ProposalType::SopCreate {
            return Some(CandidateRejectionCode::UnsupportedType);
        }
        if candidate.scope.workspace_id != input.workspace_id {
            return Some(CandidateRejectionCode::WorkspaceMismatch);
        }
        if candidate.sensitivity == MemorySensitivity::Unknown {
            return Some(CandidateRejectionCode::InvalidSensitivity);
        }
        if candidate.evidence_refs.is_empty() {
            return Some(CandidateRejectionCode::MissingEvidence);
        }
        if candidate
            .evidence_refs
            .iter()
            .any(|item| allowed_evidence.get(item.id.as_str()).copied() != Some(item))
        {
            return Some(CandidateRejectionCode::UnknownEvidence);
        }
        if candidate
            .evidence_refs
            .iter()
            .all(|item| item.source == EvidenceSource::ContextCompaction)
        {
            return Some(CandidateRejectionCode::SecondaryEvidenceOnly);
        }
        if candidate.confidence < MIN_CONFIDENCE {
            return Some(CandidateRejectionCode::LowConfidence);
        }
        if candidate.expected_value == CandidateValue::Low {
            return Some(CandidateRejectionCode::LowValue);
        }
        if contains_prompt_injection(content) {
            return Some(CandidateRejectionCode::PromptInjection);
        }
        if !agent_diva_core::security::redact_pii(
            content,
            &agent_diva_core::security::PiiConfig::default(),
        )
        .detected
        .is_empty()
        {
            return Some(CandidateRejectionCode::SensitiveContent);
        }
        let digest = content_digest(content);
        if existing.contains(&digest) || accepted_digests.contains(&digest) {
            return Some(CandidateRejectionCode::Duplicate);
        }
        if local_existing_memory
            .iter()
            .any(|existing| are_direct_negations(content, existing))
        {
            return Some(CandidateRejectionCode::Contradiction);
        }
        if accepted_count >= input.max_candidates.min(MAX_CANDIDATES) {
            return Some(CandidateRejectionCode::CapacityExceeded);
        }
        None
    }
}

fn are_direct_negations(left: &str, right: &str) -> bool {
    let left = normalize_statement(left);
    let right = normalize_statement(right);
    let left_positive = strip_negation(&left);
    let right_positive = strip_negation(&right);
    left_positive.0 != right_positive.0 && left_positive.1 == right_positive.1
}

fn normalize_statement(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .filter(|character| character.is_alphanumeric() || character.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_negation(value: &str) -> (bool, String) {
    if let Some(rest) = value.strip_prefix("not ") {
        return (true, rest.to_string());
    }
    if let Some(index) = value.find(" not ") {
        let mut positive = value.to_string();
        positive.replace_range(index..index + " not".len(), "");
        return (true, positive);
    }
    (false, value.to_string())
}

pub fn content_digest(content: &str) -> String {
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
    format!("sha256:{:x}", Sha256::digest(normalized.as_bytes()))
}

fn contains_prompt_injection(content: &str) -> bool {
    let normalized = content.to_ascii_lowercase();
    [
        "ignore previous",
        "ignore all previous",
        "system prompt",
        "developer message",
        "reveal secret",
        "execute command",
        "run shell",
        "<|system|>",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

//! Independent characterization of current AutoDream versus the product contract.
//!
//! Product (2026-08-14): AutoDream must organize STM, and may propose persona
//! changes. This suite records what the crate actually does today so the
//! "we did not test" failure cannot repeat as an unmeasured claim.

use agent_diva_autodream::{
    content_digest, AutoDreamInputCollector, AutoDreamInputCollectorConfig, AutoDreamStorage,
    BoundedReflectionInput, CandidateGate, DeterministicReflectionEngine, ReflectionEngine,
    ReflectionEvidence,
};
use agent_diva_core::{
    evolution::{
        CandidateValue, EvidenceRef, EvidenceSource, LaputaSectionName, MemoryCandidate,
        ProposalType,
    },
    memory::{MemoryScope, MemorySensitivity},
    session::SessionManager,
};
use agent_diva_laputa::LaputaService;
use chrono::{TimeZone, Utc};

fn evidence(id: &str, source: EvidenceSource) -> EvidenceRef {
    EvidenceRef {
        id: id.to_string(),
        source,
        uri: format!("evidence://{id}"),
        excerpt: Some("verified bounded evidence".to_string()),
        hash: Some("sha256:evidence".to_string()),
        created_at: Utc.with_ymd_and_hms(2026, 8, 14, 0, 0, 0).unwrap(),
    }
}

fn gate_input(primary: EvidenceRef) -> BoundedReflectionInput {
    BoundedReflectionInput {
        schema_version: 1,
        workspace_id: "workspace-a".to_string(),
        run_id: "run-contract".to_string(),
        evidence: vec![ReflectionEvidence {
            evidence: primary,
            summary: "verified bounded evidence".to_string(),
        }],
        existing_memory_digests: Vec::new(),
        superseded_memory_digests: Vec::new(),
        max_candidates: 4,
    }
}

fn candidate(proposal_type: ProposalType, evidence: EvidenceRef, content: &str) -> MemoryCandidate {
    MemoryCandidate {
        candidate_id: format!("candidate-{}", content_digest(content)),
        proposal_type,
        content: content.to_string(),
        evidence_refs: vec![evidence],
        confidence: 80,
        scope: MemoryScope {
            tenant_id: "local".to_string(),
            workspace_id: "workspace-a".to_string(),
            session_id: None,
        },
        sensitivity: MemorySensitivity::Private,
        expected_value: CandidateValue::Medium,
        invalidation_conditions: vec!["user correction".to_string()],
    }
}

#[test]
fn current_default_inputs_read_identity_json_and_memory_md_not_stm() {
    let config = AutoDreamInputCollectorConfig::default();
    assert_eq!(
        config.laputa_sections,
        vec![LaputaSectionName::MemoryMd, LaputaSectionName::Identity],
        "current collector still reads retired Identity JSON + memory_md"
    );

    let temp = tempfile::tempdir().unwrap();
    let mut sessions = SessionManager::new(temp.path());
    let session = sessions.get_or_create("chat:contract");
    session.add_message("user", "recent session evidence for contract test");
    let saved = session.clone();
    sessions.save(&saved).unwrap();

    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let collected =
        AutoDreamInputCollector::new(storage, LaputaService::open(temp.path()).unwrap())
            .collect("run-contract")
            .unwrap();

    let sources: Vec<&str> = collected
        .items
        .iter()
        .map(|item| item.source.as_str())
        .collect();
    assert!(
        sources.iter().all(|source| *source != "stm"),
        "current collector has no STM source; product requires STM organize: {sources:?}"
    );
    assert!(
        collected
            .summary
            .included_sources
            .iter()
            .all(|item| item.source != "stm"),
        "input summary must not pretend STM was collected"
    );
}

#[tokio::test]
async fn current_default_reflection_emits_memory_patch_not_persona_or_stm() {
    let primary = evidence("primary", EvidenceSource::ExperienceJournal);
    let output = DeterministicReflectionEngine::evidence_echo()
        .reflect(gate_input(primary))
        .await
        .unwrap();

    assert_eq!(output.candidates.len(), 1);
    assert_eq!(
        output.candidates[0].proposal_type,
        ProposalType::MemoryPatch,
        "current default emit is MemoryPatch; product forbids MemoryPatch and requires STM organize + persona proposals"
    );
}

#[test]
fn current_gate_accepts_identity_patch_and_only_rejects_sop_create_as_unsupported() {
    let primary = evidence("primary", EvidenceSource::ExperienceJournal);
    let identity = candidate(
        ProposalType::IdentityPatch,
        primary.clone(),
        "The agent currently appears as a compact square desk lamp.",
    );
    let mut sop = candidate(
        ProposalType::SopCreate,
        primary,
        "Create a standalone SOP from this reflection.",
    );
    sop.proposal_type = ProposalType::SopCreate;

    let result = CandidateGate.evaluate(
        &gate_input(identity.evidence_refs[0].clone()),
        vec![identity, sop],
        &[],
        &[],
    );

    assert_eq!(result.accepted.len(), 1);
    assert_eq!(
        result.accepted[0].proposal_type,
        ProposalType::IdentityPatch
    );
    assert_eq!(result.rejected.len(), 1);
    assert_eq!(
        result.rejected[0].code,
        agent_diva_autodream::CandidateRejectionCode::UnsupportedType
    );
}

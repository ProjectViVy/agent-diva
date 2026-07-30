use agent_diva_core::{
    evolution::{
        EvidenceRef, EvidenceSource, EvolutionProposal, LaputaSectionName, ProposalState,
        ProposalType, RiskLevel,
    },
    governance::{
        ApprovalGrant, ApprovalReceipt, ApprovalRequest, ApprovalStatus, AuditCorrelation,
        Capability, Decision, GovernanceSubject, GovernanceSubjectKind, ResourceKind,
        ResourceScope, RiskClass,
    },
    memory::{
        memory_content_digest, MemoryProvenance, MemoryProvenanceSource, MemoryRecord,
        MemoryRecordKind, MemoryScope, MemorySensitivity, MemoryTrust,
    },
};
use agent_diva_laputa::{
    proposal_digest, GovernedMemoryApply, MemoryGovernanceCoordinator, TypedMemoryStore,
    TypedMemoryStoreError,
};
use chrono::{Duration, TimeZone, Utc};

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 30, 8, 0, 0).single().unwrap()
}

fn proposal(risk: RiskLevel) -> EvolutionProposal {
    EvolutionProposal {
        id: "proposal-1".into(),
        created_at: now(),
        updated_at: now(),
        created_by: "session-sync".into(),
        proposal_type: ProposalType::MemoryPatch,
        target_section: LaputaSectionName::MemoryMd,
        evidence_refs: vec![EvidenceRef {
            id: "evidence-1".into(),
            source: EvidenceSource::Session,
            uri: "session://one".into(),
            excerpt: Some("bounded".into()),
            hash: None,
            created_at: now(),
        }],
        proposed_patch: "durable governed memory".into(),
        risk_level: risk,
        state: ProposalState::PendingReview,
        source_run_id: None,
    }
}

#[tokio::test]
async fn governance_requires_human_and_rebinds_after_edit() {
    let temp = tempfile::tempdir().unwrap();
    let coordinator = MemoryGovernanceCoordinator::open_lazy(temp.path(), "workspace-1").unwrap();
    let original = proposal(RiskLevel::High);
    let pending = coordinator
        .submit(&original, Some("session-1"), now())
        .await
        .unwrap();
    assert_eq!(pending.status, ApprovalStatus::Pending);
    assert_eq!(
        pending.policy.decision,
        agent_diva_core::governance::Decision::RequireHuman
    );

    let allowed = coordinator
        .decide(
            &original,
            pending.request_version,
            Decision::Allow,
            ApprovalGrant::Once,
            GovernanceSubject {
                kind: GovernanceSubjectKind::User,
                id: "reviewer".into(),
            },
            "decision-1",
            now() + Duration::minutes(1),
        )
        .await
        .unwrap();
    assert_eq!(allowed.status, ApprovalStatus::Allowed);
    let (_, receipt) = coordinator
        .allowed_receipt(&original, now() + Duration::minutes(2))
        .await
        .unwrap();
    assert_eq!(receipt.grant, ApprovalGrant::Once);

    let mut edited = original.clone();
    edited.proposed_patch = "edited governed memory".into();
    edited.state = ProposalState::Edited;
    edited.updated_at = now() + Duration::minutes(3);
    let replacement = coordinator
        .submit(&edited, Some("session-1"), edited.updated_at)
        .await
        .unwrap();
    assert_ne!(replacement.request_id, allowed.request_id);
    assert_eq!(replacement.status, ApprovalStatus::Pending);
}

#[tokio::test]
async fn high_risk_rejects_reusable_grants_and_versions_conflict() {
    let temp = tempfile::tempdir().unwrap();
    let coordinator = MemoryGovernanceCoordinator::open_lazy(temp.path(), "workspace-1").unwrap();
    let proposal = proposal(RiskLevel::Critical);
    let pending = coordinator.submit(&proposal, None, now()).await.unwrap();
    let invalid = coordinator
        .decide(
            &proposal,
            pending.request_version,
            Decision::Allow,
            ApprovalGrant::Session,
            GovernanceSubject {
                kind: GovernanceSubjectKind::User,
                id: "reviewer".into(),
            },
            "decision-invalid",
            now() + Duration::minutes(1),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        invalid,
        agent_diva_laputa::MemoryGovernanceError::InvalidGrant
    ));
}

#[tokio::test]
async fn governed_typed_apply_is_atomic_idempotent_and_receipt_bound() {
    let temp = tempfile::tempdir().unwrap();
    let store = TypedMemoryStore::open(temp.path(), "workspace-1")
        .await
        .unwrap();
    let proposal = proposal(RiskLevel::Low);
    let digest = proposal_digest(&proposal);
    let request = ApprovalRequest {
        correlation: AuditCorrelation {
            request_id: "request-1".into(),
            turn_id: "turn-1".into(),
            session_id: "session-1".into(),
            trace_id: None,
        },
        subject: GovernanceSubject {
            kind: GovernanceSubjectKind::Service,
            id: "memory".into(),
        },
        capability: Capability::MemoryApply,
        resource: ResourceScope {
            workspace_id: "workspace-1".into(),
            session_id: None,
            kind: ResourceKind::Memory,
            resource_id: proposal.id.clone(),
            boundary: Some("memory_md".into()),
        },
        risk: RiskClass::Low,
        content_digest: digest.clone(),
        policy_version: "memory-apply-v1".into(),
        created_at: now(),
        expires_at: now() + Duration::hours(1),
        evidence_refs: proposal.evidence_refs.clone(),
        payload: (),
    };
    let receipt = ApprovalReceipt {
        request_id: request.correlation.request_id.clone(),
        content_digest: digest,
        policy_version: request.policy_version.clone(),
        capability: Capability::MemoryApply,
        resource: request.resource.clone(),
        decision: Decision::Allow,
        decided_by: GovernanceSubject {
            kind: GovernanceSubjectKind::User,
            id: "reviewer".into(),
        },
        decided_at: now() + Duration::minutes(1),
        expires_at: now() + Duration::hours(1),
        grant: ApprovalGrant::Once,
    };
    let content = "durable governed memory";
    let record = MemoryRecord {
        id: "record-1".into(),
        kind: MemoryRecordKind::LongTerm,
        content: content.into(),
        provenance: MemoryProvenance {
            source: MemoryProvenanceSource::LaputaAppliedSection,
            source_id: "memory_md".into(),
            content_digest: memory_content_digest(content.as_bytes()),
            captured_at: now(),
            correlation: request.correlation.clone(),
        },
        evidence_refs: proposal.evidence_refs,
        confidence_bps: 10_000,
        sensitivity: MemorySensitivity::Private,
        trust: MemoryTrust::AppliedAuthority,
        scope: MemoryScope {
            tenant_id: "tenant-1".into(),
            workspace_id: "workspace-1".into(),
            session_id: None,
        },
        created_at: now(),
        effective_at: now(),
        expires_at: None,
        supersedes: Vec::new(),
        tombstone: None,
    };
    let applied = store
        .put_governed(
            record.clone(),
            0,
            None,
            GovernedMemoryApply {
                proposal_id: "proposal-1",
                idempotency_key: "apply-1",
                request: &request,
                receipt: &receipt,
                applied_at: now() + Duration::minutes(2),
            },
        )
        .await
        .unwrap();
    let replay = store
        .put_governed(
            record,
            0,
            None,
            GovernedMemoryApply {
                proposal_id: "proposal-1",
                idempotency_key: "apply-1",
                request: &request,
                receipt: &receipt,
                applied_at: now() + Duration::minutes(2),
            },
        )
        .await
        .unwrap();
    assert_eq!(applied, replay);
    assert_eq!(store.metadata().await.unwrap().store_revision, 1);

    let mut wrong = request.clone();
    wrong.correlation.request_id = "wrong".into();
    let error = store
        .put_governed(
            applied.record,
            1,
            Some(1),
            GovernedMemoryApply {
                proposal_id: "proposal-1",
                idempotency_key: "apply-2",
                request: &wrong,
                receipt: &receipt,
                applied_at: now() + Duration::minutes(3),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(error, TypedMemoryStoreError::InvalidReceipt(_)));
}

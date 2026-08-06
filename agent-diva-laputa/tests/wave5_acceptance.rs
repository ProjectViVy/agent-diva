use agent_diva_core::{
    evolution::{
        EvidenceRef, EvidenceSource, EvolutionProposal, LaputaSectionName, ProposalState,
        ProposalType, RiskLevel,
    },
    governance::{
        ApprovalGrant, ApprovalReceipt, ApprovalRecord, ApprovalRequest, AuditCorrelation,
        Capability, Decision, GovernanceSubject, GovernanceSubjectKind, ResourceKind,
        ResourceScope, RiskClass,
    },
    memory::{
        memory_content_digest, MemoryAddRequest, MemoryCrudContext, MemoryCrudOutcome,
        MemoryListRequest, MemoryProvenance, MemoryProvenanceSource, MemoryProvider, MemoryRecord,
        MemoryRecordKind, MemoryScope, MemorySearchRequest, MemorySensitivity, MemoryTombstone,
        MemoryTrust, SystemPromptRequest, MAX_CONFIDENCE_BPS,
    },
    workspace_identity::canonical_workspace_id,
};
use agent_diva_laputa::{
    proposal_digest, GovernedMemoryApply, TypedLaputaMemoryProvider, TypedMemoryStore,
};
use chrono::{Duration, Utc};

fn now() -> chrono::DateTime<Utc> {
    Utc::now()
}

fn context(temp: &tempfile::TempDir) -> MemoryCrudContext {
    MemoryCrudContext {
        workspace_root: temp.path().to_path_buf(),
    }
}

async fn open_provider(temp: &tempfile::TempDir) -> TypedLaputaMemoryProvider {
    TypedMemoryStore::open_canonical(temp.path()).await.unwrap();
    TypedLaputaMemoryProvider::open(temp.path(), canonical_workspace_id(temp.path()))
        .await
        .unwrap()
}

fn proposal(risk: RiskLevel, content: &str) -> EvolutionProposal {
    EvolutionProposal {
        id: format!("proposal-{content}"),
        created_at: now(),
        updated_at: now(),
        created_by: "session-acceptance".into(),
        proposal_type: ProposalType::MemoryPatch,
        target_section: LaputaSectionName::MemoryMd,
        evidence_refs: vec![EvidenceRef {
            id: "evidence-acceptance".into(),
            source: EvidenceSource::Session,
            uri: "session://acceptance".into(),
            excerpt: Some("wave5".into()),
            hash: None,
            created_at: now(),
        }],
        proposed_patch: content.into(),
        risk_level: risk,
        state: ProposalState::PendingReview,
        source_run_id: None,
    }
}

fn build_approval(
    proposal: &EvolutionProposal,
) -> (ApprovalRequest<()>, ApprovalReceipt, ApprovalRecord) {
    let digest = proposal_digest(proposal);
    let request = ApprovalRequest {
        correlation: AuditCorrelation {
            request_id: format!("req-{}", proposal.id),
            turn_id: "turn-acceptance".into(),
            session_id: "session-acceptance".into(),
            trace_id: None,
        },
        subject: GovernanceSubject {
            kind: GovernanceSubjectKind::Service,
            id: "memory".into(),
        },
        capability: Capability::MemoryApply,
        resource: ResourceScope {
            workspace_id: canonical_workspace_id(std::path::Path::new(".")),
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
    let record = ApprovalRecord::from_request(&request).unwrap();
    (request, receipt, record)
}

fn build_record(
    temp: &tempfile::TempDir,
    proposal: &EvolutionProposal,
    request: &ApprovalRequest<()>,
    content: &str,
) -> MemoryRecord {
    let workspace_id = canonical_workspace_id(temp.path());
    MemoryRecord {
        id: format!("record-{}", proposal.id),
        kind: MemoryRecordKind::LongTerm,
        content: content.into(),
        provenance: MemoryProvenance {
            source: MemoryProvenanceSource::LaputaAppliedSection,
            source_id: "memory_md".into(),
            content_digest: memory_content_digest(content.as_bytes()),
            captured_at: now(),
            correlation: request.correlation.clone(),
        },
        evidence_refs: proposal.evidence_refs.clone(),
        confidence_bps: MAX_CONFIDENCE_BPS,
        sensitivity: MemorySensitivity::Private,
        trust: MemoryTrust::AppliedAuthority,
        scope: MemoryScope {
            tenant_id: "local".into(),
            workspace_id,
            session_id: None,
        },
        created_at: now(),
        effective_at: now(),
        expires_at: None,
        supersedes: Vec::new(),
        tombstone: None,
    }
}

#[tokio::test]
async fn f6_rollback_clears_fts_and_startup() {
    let temp = tempfile::tempdir().unwrap();
    let store = TypedMemoryStore::open_canonical(temp.path()).await.unwrap();
    let content = "kestrel patrols the ridge at dawn";
    let prop = proposal(RiskLevel::Low, content);
    let (request, receipt, approval_record) = build_approval(&prop);
    let record = build_record(&temp, &prop, &request, content);

    store
        .put_governed(
            record,
            0,
            None,
            GovernedMemoryApply {
                proposal_id: &prop.id,
                idempotency_key: "apply-f6",
                request: &approval_record,
                receipt: &receipt,
                applied_at: now() + Duration::minutes(2),
            },
        )
        .await
        .unwrap();

    // Verify record is visible before rollback.
    let provider = open_provider(&temp).await;
    let search_before = provider
        .memory_search(
            &context(&temp),
            MemorySearchRequest {
                query: "kestrel patrols ridge".into(),
                limit: Some(10),
            },
        )
        .await
        .unwrap();
    let MemoryCrudOutcome::Listed {
        entries: before_entries,
    } = search_before
    else {
        panic!("expected Listed, got {search_before:?}");
    };
    assert!(
        before_entries
            .iter()
            .any(|e| e.content.contains("kestrel patrols")),
        "record must be searchable before rollback"
    );

    let startup_before = provider
        .system_prompt_block(&SystemPromptRequest {
            workspace_root: temp.path().to_path_buf(),
        })
        .unwrap();
    assert!(
        startup_before
            .prompt_block
            .as_ref()
            .map(|b| b.markdown.contains("kestrel patrols"))
            .unwrap_or(false),
        "record must appear in startup rendering before rollback"
    );
    drop(provider);

    // Rollback.
    assert!(store.rollback_governed(&prop.id, 1).await.unwrap());

    // Re-open provider — startup rendering must exclude the rolled-back record.
    let provider_after = open_provider(&temp).await;
    let startup_after = provider_after
        .system_prompt_block(&SystemPromptRequest {
            workspace_root: temp.path().to_path_buf(),
        })
        .unwrap();
    let startup_text = startup_after
        .prompt_block
        .as_ref()
        .map(|b| b.markdown.as_str())
        .unwrap_or("");
    assert!(
        !startup_text.contains("kestrel patrols"),
        "rolled-back record must not appear in startup rendering, got: {startup_text}"
    );

    // FTS search must not find it.
    let search_after = provider_after
        .memory_search(
            &context(&temp),
            MemorySearchRequest {
                query: "kestrel patrols ridge".into(),
                limit: Some(10),
            },
        )
        .await
        .unwrap();
    let MemoryCrudOutcome::Listed {
        entries: after_entries,
    } = search_after
    else {
        panic!("expected Listed, got {search_after:?}");
    };
    assert!(
        after_entries.is_empty(),
        "rolled-back record must not appear in search results, got {after_entries:?}"
    );
}

#[tokio::test]
async fn f6_rollback_is_idempotent() {
    let temp = tempfile::tempdir().unwrap();
    let store = TypedMemoryStore::open_canonical(temp.path()).await.unwrap();
    let content = "sparrow nests in the old barn";
    let prop = proposal(RiskLevel::Low, content);
    let (request, receipt, approval_record) = build_approval(&prop);
    let record = build_record(&temp, &prop, &request, content);

    store
        .put_governed(
            record,
            0,
            None,
            GovernedMemoryApply {
                proposal_id: &prop.id,
                idempotency_key: "apply-f6-idem",
                request: &approval_record,
                receipt: &receipt,
                applied_at: now() + Duration::minutes(2),
            },
        )
        .await
        .unwrap();

    // First rollback succeeds.
    assert!(store.rollback_governed(&prop.id, 1).await.unwrap());
    // Second rollback returns false (no-op).
    assert!(!store.rollback_governed(&prop.id, 2).await.unwrap());
    // Third rollback also returns false — stable.
    assert!(!store.rollback_governed(&prop.id, 2).await.unwrap());
}

#[tokio::test]
async fn f7_tombstone_lifecycle_filters_startup_and_search() {
    let temp = tempfile::tempdir().unwrap();
    let store = TypedMemoryStore::open_canonical(temp.path()).await.unwrap();
    let workspace_id = canonical_workspace_id(temp.path());

    // Write an authority record via memory_add through a provider.
    let provider = open_provider(&temp).await;
    let add_outcome = provider
        .memory_add(
            &context(&temp),
            MemoryAddRequest {
                content: "falcon hunts over the frozen lake".into(),
            },
        )
        .await
        .unwrap();
    let target_id = match add_outcome {
        MemoryCrudOutcome::Applied { entry } => entry.expect("entry").id,
        other => panic!("expected Applied, got {other:?}"),
    };
    drop(provider);

    // Write a supersedes tombstone directly to the store.
    let metadata = store.metadata().await.unwrap();
    let tombstone = MemoryRecord {
        id: format!("tombstone-{}", now().timestamp_micros()),
        kind: MemoryRecordKind::LongTerm,
        content: String::new(),
        provenance: MemoryProvenance {
            source: MemoryProvenanceSource::AutoDream,
            source_id: "wave5-acceptance".into(),
            content_digest: memory_content_digest(b""),
            captured_at: now(),
            correlation: AuditCorrelation {
                request_id: "tombstone-req".into(),
                turn_id: "tombstone-turn".into(),
                session_id: "tombstone-session".into(),
                trace_id: None,
            },
        },
        evidence_refs: vec![],
        confidence_bps: MAX_CONFIDENCE_BPS,
        sensitivity: MemorySensitivity::Internal,
        trust: MemoryTrust::AppliedAuthority,
        scope: MemoryScope {
            tenant_id: "local".into(),
            workspace_id: workspace_id.clone(),
            session_id: None,
        },
        created_at: now(),
        effective_at: now(),
        expires_at: None,
        supersedes: vec![target_id.clone()],
        tombstone: Some(MemoryTombstone {
            target_record_id: target_id.clone(),
            reason_digest: memory_content_digest(b"forgotten via wave5 acceptance"),
            actor_id: "wave5-test".into(),
            created_at: now(),
        }),
    };
    store
        .put(tombstone, metadata.store_revision, None)
        .await
        .unwrap();

    // Re-open provider — startup rendering must exclude the tombstoned record.
    let provider_after = open_provider(&temp).await;
    let startup = provider_after
        .system_prompt_block(&SystemPromptRequest {
            workspace_root: temp.path().to_path_buf(),
        })
        .unwrap();
    let startup_text = startup
        .prompt_block
        .as_ref()
        .map(|b| b.markdown.as_str())
        .unwrap_or("");
    assert!(
        !startup_text.contains("falcon hunts"),
        "tombstoned record must not appear in startup rendering, got: {startup_text}"
    );

    // Search must not find the tombstoned record.
    let search = provider_after
        .memory_search(
            &context(&temp),
            MemorySearchRequest {
                query: "falcon hunts frozen lake".into(),
                limit: Some(10),
            },
        )
        .await
        .unwrap();
    let MemoryCrudOutcome::Listed { entries } = search else {
        panic!("expected Listed, got {search:?}");
    };
    assert!(
        entries.iter().all(|e| !e.content.contains("falcon hunts")),
        "tombstoned record must not appear in search, got {entries:?}"
    );

    // memory_list does NOT filter superseded records (visible_record only checks
    // trust/tombstone/session_id). This is a known gap — superseded records remain
    // visible in list until a future wave adds superseded filtering to the list path.
    let list = provider_after
        .memory_list(&context(&temp), MemoryListRequest { limit: None })
        .await
        .unwrap();
    let MemoryCrudOutcome::Listed {
        entries: list_entries,
    } = list
    else {
        panic!("expected Listed, got {list:?}");
    };
    assert!(
        list_entries
            .iter()
            .any(|e| e.content.contains("falcon hunts")),
        "superseded record is still visible in list (known gap: list does not filter superseded)"
    );
}

#[tokio::test]
async fn f7_tombstone_target_missing_stores_tombstone_record() {
    let temp = tempfile::tempdir().unwrap();
    let store = TypedMemoryStore::open_canonical(temp.path()).await.unwrap();
    let workspace_id = canonical_workspace_id(temp.path());

    // Write a tombstone targeting a non-existent record — store accepts it
    // (the DB doesn't enforce FK on the target). The tombstone record itself
    // is inert: it won't suppress anything since the target doesn't exist.
    let metadata = store.metadata().await.unwrap();
    let phantom_id = "record-never-existed".to_string();
    let tombstone = MemoryRecord {
        id: format!("tombstone-phantom-{}", now().timestamp_micros()),
        kind: MemoryRecordKind::LongTerm,
        content: String::new(),
        provenance: MemoryProvenance {
            source: MemoryProvenanceSource::AutoDream,
            source_id: "wave5-phantom".into(),
            content_digest: memory_content_digest(b""),
            captured_at: now(),
            correlation: AuditCorrelation {
                request_id: "phantom-req".into(),
                turn_id: "phantom-turn".into(),
                session_id: "phantom-session".into(),
                trace_id: None,
            },
        },
        evidence_refs: vec![],
        confidence_bps: MAX_CONFIDENCE_BPS,
        sensitivity: MemorySensitivity::Internal,
        trust: MemoryTrust::AppliedAuthority,
        scope: MemoryScope {
            tenant_id: "local".into(),
            workspace_id: workspace_id.clone(),
            session_id: None,
        },
        created_at: now(),
        effective_at: now(),
        expires_at: None,
        supersedes: vec![phantom_id.clone()],
        tombstone: Some(MemoryTombstone {
            target_record_id: phantom_id.clone(),
            reason_digest: memory_content_digest(b"phantom forget"),
            actor_id: "wave5-test".into(),
            created_at: now(),
        }),
    };

    let result = store.put(tombstone, metadata.store_revision, None).await;
    // The store writes the tombstone record even if the target doesn't exist.
    // This is by design — the provider-level memory_remove validates existence
    // before issuing a proposal. The store layer is permissive.
    assert!(
        result.is_ok(),
        "tombstone write should succeed even for non-existent target, got {result:?}"
    );

    // Verify the tombstone is recorded but doesn't crash provider startup.
    let provider = open_provider(&temp).await;
    let startup = provider
        .system_prompt_block(&SystemPromptRequest {
            workspace_root: temp.path().to_path_buf(),
        })
        .unwrap();
    // No crash, no phantom content in rendering.
    let startup_text = startup
        .prompt_block
        .as_ref()
        .map(|b| b.markdown.as_str())
        .unwrap_or("");
    assert!(
        !startup_text.contains("never-existed"),
        "phantom tombstone must not inject content into startup"
    );
}

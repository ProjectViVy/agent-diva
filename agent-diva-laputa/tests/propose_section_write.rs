//! `propose_section_write` governed Frozen Core write integration tests.

use agent_diva_core::evolution::{
    EvidenceSource, LaputaSectionName, ProposalState, ProposalType, RiskLevel,
};
use agent_diva_core::memory::{
    MemoryCrudContext, MemoryCrudOutcome, MemoryProvider, SectionWriteProposalRequest,
};
use agent_diva_laputa::{LaputaService, TypedLaputaMemoryProvider, TypedMemoryStore};

async fn provider_for(workspace: &std::path::Path) -> TypedLaputaMemoryProvider {
    TypedMemoryStore::open(workspace, "workspace-1")
        .await
        .unwrap();
    TypedLaputaMemoryProvider::open_with_l1_budget(workspace, "workspace-1", 10)
        .await
        .unwrap()
}

#[tokio::test]
async fn propose_section_write_creates_governed_identity_proposal() {
    let temp = tempfile::tempdir().unwrap();
    let provider = provider_for(temp.path()).await;

    let outcome = provider
        .propose_section_write(
            &MemoryCrudContext {
                workspace_root: temp.path().to_path_buf(),
            },
            SectionWriteProposalRequest {
                section: LaputaSectionName::Identity,
                content: "{\"name\":\"diva\"}".into(),
                summary: Some("identity onboarding".into()),
            },
        )
        .await
        .unwrap();

    let proposal_id = match outcome {
        MemoryCrudOutcome::ProposalCreated { proposal_id } => proposal_id,
        other => panic!("expected ProposalCreated, got {other:?}"),
    };

    let service = LaputaService::open(temp.path()).unwrap();
    let proposal = service.get_proposal(&proposal_id).unwrap();
    assert_eq!(proposal.proposal_type, ProposalType::IdentityPatch);
    assert_eq!(proposal.target_section, LaputaSectionName::Identity);
    assert_eq!(proposal.risk_level, RiskLevel::High);
    assert_eq!(proposal.state, ProposalState::PendingReview);
    assert_eq!(proposal.created_by, "laputa_propose_section_write");
    assert!(proposal
        .evidence_refs
        .iter()
        .any(|evidence| evidence.source == EvidenceSource::UserInput));
    assert_eq!(proposal.proposed_patch, "{\"name\":\"diva\"}");
}

#[tokio::test]
async fn propose_section_write_routes_preferences_to_learning_note() {
    let temp = tempfile::tempdir().unwrap();
    let provider = provider_for(temp.path()).await;

    let outcome = provider
        .propose_section_write(
            &MemoryCrudContext {
                workspace_root: temp.path().to_path_buf(),
            },
            SectionWriteProposalRequest {
                section: LaputaSectionName::Preferences,
                content: "{\"style\":\"direct\"}".into(),
                summary: None,
            },
        )
        .await
        .unwrap();

    let proposal_id = match outcome {
        MemoryCrudOutcome::ProposalCreated { proposal_id } => proposal_id,
        other => panic!("expected ProposalCreated, got {other:?}"),
    };
    let service = LaputaService::open(temp.path()).unwrap();
    let proposal = service.get_proposal(&proposal_id).unwrap();
    assert_eq!(proposal.proposal_type, ProposalType::LearningNote);
    assert_eq!(proposal.target_section, LaputaSectionName::Preferences);
    assert_eq!(proposal.risk_level, RiskLevel::Medium);
}

#[tokio::test]
async fn propose_section_write_rejects_non_json_content() {
    let temp = tempfile::tempdir().unwrap();
    let provider = provider_for(temp.path()).await;

    let outcome = provider
        .propose_section_write(
            &MemoryCrudContext {
                workspace_root: temp.path().to_path_buf(),
            },
            SectionWriteProposalRequest {
                section: LaputaSectionName::Identity,
                content: "not json".into(),
                summary: None,
            },
        )
        .await
        .unwrap();
    assert!(
        matches!(outcome, MemoryCrudOutcome::Failed { .. }),
        "non-JSON content must fail: {outcome:?}"
    );
}

#[tokio::test]
async fn propose_section_write_rejects_empty_content() {
    let temp = tempfile::tempdir().unwrap();
    let provider = provider_for(temp.path()).await;

    let outcome = provider
        .propose_section_write(
            &MemoryCrudContext {
                workspace_root: temp.path().to_path_buf(),
            },
            SectionWriteProposalRequest {
                section: LaputaSectionName::Relationship,
                content: "   ".into(),
                summary: None,
            },
        )
        .await
        .unwrap();
    assert!(
        matches!(outcome, MemoryCrudOutcome::Failed { .. }),
        "empty content must fail: {outcome:?}"
    );
}

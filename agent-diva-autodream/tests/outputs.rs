use std::fs;

use agent_diva_autodream::{
    AutoDreamArtifactSummary, AutoDreamOutputEmitter, AutoDreamOutputRequest,
    AutoDreamProposalCandidateDraft, AutoDreamStorage,
};
use agent_diva_core::evolution::{
    AutoDreamRunRecord, AutoDreamRunState, EvidenceRef, EvidenceSource, ProposalState, RiskLevel,
};
use agent_diva_laputa::LaputaService;
use chrono::{TimeZone, Utc};

fn sample_time() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 6, 14, 10, 0, 0).unwrap()
}

fn sample_run() -> AutoDreamRunRecord {
    AutoDreamRunRecord {
        id: "run-123".to_string(),
        started_at: sample_time(),
        completed_at: None,
        state: AutoDreamRunState::Running,
        trigger: "manual".to_string(),
        summary: None,
        input_summary: None,
        proposal_ids: Vec::new(),
        orchestration: None,
        failure_code: None,
        error: None,
    }
}

fn sample_evidence() -> EvidenceRef {
    EvidenceRef {
        id: "evidence-1".to_string(),
        source: EvidenceSource::AutoDreamRun,
        uri: "autodream://runs/run-123/input/1".to_string(),
        excerpt: Some("bounded evidence".to_string()),
        hash: Some("sha256:abc".to_string()),
        created_at: sample_time(),
    }
}

fn compaction_evidence() -> EvidenceRef {
    EvidenceRef {
        id: "compact-1".to_string(),
        source: EvidenceSource::ContextCompaction,
        uri: "capsule://compact-1.md".to_string(),
        excerpt: Some("session-local summary; secondary evidence only".to_string()),
        hash: None,
        created_at: sample_time(),
    }
}

#[test]
fn emit_outputs_persists_artifact_links_run_and_appends_events() {
    let temp = tempfile::tempdir().unwrap();
    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let laputa = LaputaService::open(temp.path()).unwrap();
    let emitter = AutoDreamOutputEmitter::new(storage.clone(), laputa.clone());
    let run = sample_run();
    fs::create_dir_all(temp.path().join(".agent-diva/autodream/runs").join(&run.id)).unwrap();
    fs::write(
        temp.path()
            .join(".agent-diva/autodream/runs")
            .join(&run.id)
            .join("record.json"),
        serde_json::to_vec_pretty(&run).unwrap(),
    )
    .unwrap();

    let result = emitter
        .emit_outputs(AutoDreamOutputRequest {
            run,
            generated_at: sample_time(),
            confidence: 87,
            evidence_refs: vec![sample_evidence()],
            output_summary: AutoDreamArtifactSummary {
                headline: "Reflection identified one durable memory update".to_string(),
                details: vec!["bounded output only".to_string()],
            },
            proposal_candidates: vec![AutoDreamProposalCandidateDraft {
                proposal_type: "memory_patch".to_string(),
                proposed_patch: "Remember that the user prefers concise changelogs.".to_string(),
                risk_level: RiskLevel::Low,
                evidence_refs: vec![sample_evidence()],
                metadata: None,
            }],
        })
        .unwrap();

    let artifact_path = temp
        .path()
        .join(".agent-diva/autodream/runs/run-123/autodream_run.json");
    assert!(artifact_path.exists());
    let artifact_json = fs::read_to_string(&artifact_path).unwrap();
    let artifact: serde_json::Value = serde_json::from_str(&artifact_json).unwrap();
    assert_eq!(artifact["review_required"], serde_json::Value::Bool(true));
    assert_eq!(artifact["schema_version"], "1.0.0");
    assert_eq!(result.run.proposal_ids.len(), 1);
    assert_eq!(result.artifact.proposal_ids, result.run.proposal_ids);
    assert_eq!(result.proposals[0].state, ProposalState::PendingReview);
    assert_eq!(
        result.proposals[0].source_run_id.as_deref(),
        Some("run-123")
    );

    let proposal = laputa.get_proposal(&result.run.proposal_ids[0]).unwrap();
    assert_eq!(proposal.state, ProposalState::PendingReview);
    assert_eq!(proposal.source_run_id.as_deref(), Some("run-123"));

    let updated_run = fs::read_to_string(
        temp.path()
            .join(".agent-diva/autodream/runs/run-123/record.json"),
    )
    .unwrap();
    assert!(updated_run.contains(&result.run.proposal_ids[0]));
    assert!(updated_run.contains("Reflection identified one durable memory update"));

    let events =
        fs::read_to_string(temp.path().join(".agent-diva/autodream/events.jsonl")).unwrap();
    assert!(events.contains("proposal_created"));
    assert!(events.contains("outputs_persisted"));
}

#[test]
fn emit_outputs_replay_returns_the_persisted_result_without_duplicate_proposals() {
    let temp = tempfile::tempdir().unwrap();
    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let laputa = LaputaService::open(temp.path()).unwrap();
    let emitter = AutoDreamOutputEmitter::new(storage, laputa.clone());
    let run = sample_run();
    fs::create_dir_all(temp.path().join(".agent-diva/autodream/runs").join(&run.id)).unwrap();
    fs::write(
        temp.path()
            .join(".agent-diva/autodream/runs")
            .join(&run.id)
            .join("record.json"),
        serde_json::to_vec_pretty(&run).unwrap(),
    )
    .unwrap();
    let request = AutoDreamOutputRequest {
        run,
        generated_at: sample_time(),
        confidence: 87,
        evidence_refs: vec![sample_evidence()],
        output_summary: AutoDreamArtifactSummary {
            headline: "Replay-safe reflection".to_string(),
            details: vec!["bounded output only".to_string()],
        },
        proposal_candidates: vec![AutoDreamProposalCandidateDraft {
            proposal_type: "memory_patch".to_string(),
            proposed_patch: "Remember deterministic replay evidence.".to_string(),
            risk_level: RiskLevel::Low,
            evidence_refs: vec![sample_evidence()],
            metadata: None,
        }],
    };

    let first = emitter.emit_outputs(request.clone()).unwrap();
    let second = emitter.emit_outputs(request).unwrap();

    assert_eq!(first.artifact, second.artifact);
    assert_eq!(first.proposals, second.proposals);
    assert!(second.events.is_empty());
    assert_eq!(
        laputa
            .list_proposals(agent_diva_laputa::ProposalFilter::default())
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn emit_outputs_uses_laputa_service_without_direct_proposal_directory_writes() {
    let temp = tempfile::tempdir().unwrap();
    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let laputa = LaputaService::open(temp.path()).unwrap();
    let emitter = AutoDreamOutputEmitter::new(storage, laputa);
    let run = sample_run();
    fs::create_dir_all(temp.path().join(".agent-diva/autodream/runs").join(&run.id)).unwrap();
    fs::write(
        temp.path()
            .join(".agent-diva/autodream/runs")
            .join(&run.id)
            .join("record.json"),
        serde_json::to_vec_pretty(&run).unwrap(),
    )
    .unwrap();

    emitter
        .emit_outputs(AutoDreamOutputRequest {
            run,
            generated_at: sample_time(),
            confidence: 60,
            evidence_refs: vec![sample_evidence()],
            output_summary: AutoDreamArtifactSummary {
                headline: "One proposal generated".to_string(),
                details: Vec::new(),
            },
            proposal_candidates: vec![AutoDreamProposalCandidateDraft {
                proposal_type: "memory_patch".to_string(),
                proposed_patch: "Reflect on the latest user interaction.".to_string(),
                risk_level: RiskLevel::Medium,
                evidence_refs: vec![sample_evidence()],
                metadata: None,
            }],
        })
        .unwrap();

    let proposal_dir = temp.path().join(".laputa/proposals");
    assert!(proposal_dir.exists());
    let files = fs::read_dir(&proposal_dir).unwrap().count();
    assert_eq!(files, 1);
}

#[test]
fn emit_outputs_rejects_unknown_proposal_type_before_persistence() {
    let temp = tempfile::tempdir().unwrap();
    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let laputa = LaputaService::open(temp.path()).unwrap();
    let emitter = AutoDreamOutputEmitter::new(storage, laputa);
    let run = sample_run();
    fs::create_dir_all(temp.path().join(".agent-diva/autodream/runs").join(&run.id)).unwrap();
    fs::write(
        temp.path()
            .join(".agent-diva/autodream/runs")
            .join(&run.id)
            .join("record.json"),
        serde_json::to_vec_pretty(&run).unwrap(),
    )
    .unwrap();

    let error = emitter
        .emit_outputs(AutoDreamOutputRequest {
            run,
            generated_at: sample_time(),
            confidence: 50,
            evidence_refs: vec![sample_evidence()],
            output_summary: AutoDreamArtifactSummary {
                headline: "bad".to_string(),
                details: Vec::new(),
            },
            proposal_candidates: vec![AutoDreamProposalCandidateDraft {
                proposal_type: "unsupported_change".to_string(),
                proposed_patch: "x".to_string(),
                risk_level: RiskLevel::Low,
                evidence_refs: vec![sample_evidence()],
                metadata: None,
            }],
        })
        .unwrap_err();

    assert!(error.to_string().contains("unknown proposal type"));
    let proposal_dir = temp.path().join(".laputa/proposals");
    let files = fs::read_dir(&proposal_dir)
        .map(|iter| iter.count())
        .unwrap_or_default();
    assert_eq!(files, 0);
}

#[test]
fn emit_outputs_rejects_compaction_only_proposal_evidence_before_persistence() {
    let temp = tempfile::tempdir().unwrap();
    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let laputa = LaputaService::open(temp.path()).unwrap();
    let emitter = AutoDreamOutputEmitter::new(storage, laputa);
    let run = sample_run();
    fs::create_dir_all(temp.path().join(".agent-diva/autodream/runs").join(&run.id)).unwrap();
    fs::write(
        temp.path()
            .join(".agent-diva/autodream/runs")
            .join(&run.id)
            .join("record.json"),
        serde_json::to_vec_pretty(&run).unwrap(),
    )
    .unwrap();

    let error = emitter
        .emit_outputs(AutoDreamOutputRequest {
            run,
            generated_at: sample_time(),
            confidence: 70,
            evidence_refs: vec![compaction_evidence()],
            output_summary: AutoDreamArtifactSummary {
                headline: "bad compaction-only proposal".to_string(),
                details: Vec::new(),
            },
            proposal_candidates: vec![AutoDreamProposalCandidateDraft {
                proposal_type: "memory_patch".to_string(),
                proposed_patch: "Persist a durable memory from compaction alone.".to_string(),
                risk_level: RiskLevel::Low,
                evidence_refs: vec![compaction_evidence()],
                metadata: None,
            }],
        })
        .unwrap_err();

    assert!(error.to_string().contains("context compaction"));
    let proposal_dir = temp.path().join(".laputa/proposals");
    let files = fs::read_dir(&proposal_dir)
        .map(|iter| iter.count())
        .unwrap_or_default();
    assert_eq!(files, 0);
}

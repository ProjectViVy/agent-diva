use std::fs;

use agent_diva_autodream::{
    AutoDreamArtifactSummary, AutoDreamOutputEmitter, AutoDreamOutputRequest,
    AutoDreamProposalCandidateDraft, AutoDreamReportWriter, AutoDreamStorage, RhythmReportContent,
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
        error: None,
    }
}

fn sample_evidence(id: &str) -> EvidenceRef {
    EvidenceRef {
        id: id.to_string(),
        source: EvidenceSource::AutoDreamRun,
        uri: format!("autodream://runs/run-123/evidence/{id}"),
        excerpt: Some("bounded evidence".to_string()),
        hash: Some("sha256:abc".to_string()),
        created_at: sample_time(),
    }
}

fn assert_no_mentle_state(workspace: &std::path::Path) {
    assert!(!workspace.join("memory/palace.db").exists());
    assert!(!workspace.join(".mentle").exists());
}

#[test]
fn output_emission_and_proposal_creation_do_not_create_mentle_state() {
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

    let result = emitter
        .emit_outputs(AutoDreamOutputRequest {
            run,
            generated_at: sample_time(),
            confidence: 87,
            evidence_refs: vec![sample_evidence("evidence-1")],
            output_summary: AutoDreamArtifactSummary {
                headline: "Reflection identified one durable memory update".to_string(),
                details: vec!["bounded output only".to_string()],
            },
            proposal_candidates: vec![AutoDreamProposalCandidateDraft {
                proposal_type: "memory_patch".to_string(),
                proposed_patch: "Remember that governance remains file-first.".to_string(),
                risk_level: RiskLevel::Low,
                evidence_refs: vec![sample_evidence("evidence-1")],
            }],
        })
        .unwrap();

    assert_eq!(result.proposals[0].state, ProposalState::PendingReview);
    assert_eq!(laputa.list_proposals(Default::default()).unwrap().len(), 1);
    assert_no_mentle_state(temp.path());
}

#[test]
fn rhythm_report_writes_do_not_sync_to_mentle_state() {
    let temp = tempfile::tempdir().unwrap();
    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let writer = AutoDreamReportWriter::new(storage);

    writer
        .write_daily_report(
            "2026-06-14",
            sample_time(),
            RhythmReportContent {
                title: "Daily Reflection".to_string(),
                summary: "Report stays in AutoDream report storage.".to_string(),
                sections: vec!["## Signals\n\n- No Mentle sync.".to_string()],
                evidence_refs: vec![sample_evidence("evidence-1")],
                source: Some("session_aggregate".to_string()),
                session_count: Some(1),
                token_used: Some(12),
                fallback_used: Some(false),
                daily_inputs_count: Some(0),
                missing_daily_dates_count: Some(0),
                generation_mode: None,
                narrative_schema_version: None,
                prompt_version: None,
                coverage_status: None,
                fallback_reason: None,
            },
        )
        .unwrap();

    assert!(temp
        .path()
        .join(".agent-diva/autodream/reports/daily/2026-06-14.md")
        .exists());
    assert_no_mentle_state(temp.path());
}

#[test]
fn autodream_crate_does_not_depend_on_mentle() {
    let manifest = include_str!("../Cargo.toml");

    assert!(!manifest.contains("mentle"));
    assert!(!manifest.contains("memtle"));
}

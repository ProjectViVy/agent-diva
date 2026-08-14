//! Context Plane negative invariant matrix (garden ADR-0004 §6).
//!
//! Each test plants a unique marker in a storage layer that must never
//! reach the context plane, then asserts the marker is absent from every
//! prompt-facing surface. The matrix rows:
//!
//! 1. MEMRULES never enters the prompt.
//! 2. The full WORLD file never enters the prompt.
//! 3. Reports are never injected by default.
//! 4. The Frozen Core stays immutable within a session.
//! 5. WORLD projections are scope-matched and budgeted.
//! 6. Retired section/proposal data never surfaces.
//! 7. Retired persona files never enter the prompt.
//! 8. Governance evidence is required and bounded.

use agent_diva_core::{
    evolution::{
        EvidenceRef, EvidenceSource, EvolutionProposal, LaputaSectionName, ProposalState,
        ProposalType, RiskLevel,
    },
    memory::{MemoryProvider, StartupStatus, SystemPromptRequest},
};
use agent_diva_laputa::{
    ClaimStatus, FrozenCoreSnapshot, LaputaMemoryProvider, LaputaService, PersonaInitialization,
    PersonaKind, PersonaService, ProposalFilter, WorldClaim, WorldStore,
};
use chrono::{DateTime, Utc};
use std::{fs, path::Path};

fn ts() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-08-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

/// Seed one applied authority section so `system_prompt_block` renders a
/// real block; otherwise the negative assertions below would be vacuous.
fn seed_applied_authority(workspace: &Path) {
    let sections = workspace.join(".laputa").join("sections");
    fs::create_dir_all(&sections).unwrap();
    fs::write(sections.join("identity.json"), r#"{"applied":"authority"}"#).unwrap();
}

fn seed_persona(workspace: &Path, identity: &str, world: &str) -> PersonaService {
    let service = PersonaService::open(workspace).unwrap();
    service
        .initialize(PersonaInitialization {
            identity: identity.to_string(),
            relationship: "Relationship".to_string(),
            redline: "Redline".to_string(),
            user: "Preferences".to_string(),
            world: world.to_string(),
        })
        .unwrap();
    service
}

/// Render the startup prompt block; degraded providers produce no block at
/// all, which is represented as an empty string here.
fn prompt_markdown(provider: &LaputaMemoryProvider, workspace: &Path) -> String {
    let response = provider
        .system_prompt_block(&SystemPromptRequest {
            workspace_root: workspace.to_path_buf(),
        })
        .unwrap();
    match response.status {
        StartupStatus::Ready => response.prompt_block.unwrap().markdown,
        _ => String::new(),
    }
}

fn write_file(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

#[test]
fn memrules_never_reach_the_prompt() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path();
    seed_applied_authority(workspace);
    write_file(
        &workspace
            .join(".laputa")
            .join("cognitive")
            .join("MEMRULES.MD"),
        "# MEMRULES\nR1-MARKER-NEVER-INJECTED\n",
    );
    let provider = LaputaMemoryProvider::open(workspace).unwrap();

    let markdown = prompt_markdown(&provider, workspace);
    assert!(markdown.contains("Applied Laputa Authority"), "sanity");
    assert!(!markdown.contains("R1-MARKER-NEVER-INJECTED"));

    let service = seed_persona(workspace, "Persona identity", "Persona world");
    let frozen = FrozenCoreSnapshot::capture(&service).unwrap();
    assert!(!frozen.render(0).contains("R1-MARKER-NEVER-INJECTED"));
}

#[test]
fn full_world_never_reaches_the_prompt() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path();
    seed_applied_authority(workspace);
    write_file(
        &workspace.join(".laputa").join("cognitive").join("WORLD.MD"),
        "# WORLD\n\n## [ops] marker\n- status: confirmed\n- confidence: high\n- scope: session\n- source: user\n- updated: 2026-08-01T00:00:00Z\n\nWORLD-MARKER-WHOLESALE-INJECTION\n",
    );
    let provider = LaputaMemoryProvider::open(workspace).unwrap();

    let markdown = prompt_markdown(&provider, workspace);
    assert!(markdown.contains("Applied Laputa Authority"), "sanity");
    assert!(!markdown.contains("WORLD-MARKER-WHOLESALE-INJECTION"));

    let service = seed_persona(
        workspace,
        "Persona identity",
        "WORLD-MARKER-WHOLESALE-INJECTION",
    );
    let frozen = FrozenCoreSnapshot::capture(&service).unwrap();
    assert!(!frozen
        .render(0)
        .contains("WORLD-MARKER-WHOLESALE-INJECTION"));
}

#[test]
fn reports_are_never_injected_by_default() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path();
    seed_applied_authority(workspace);
    write_file(
        &workspace
            .join(".laputa")
            .join("reports")
            .join("daily")
            .join("2026-08-01.md"),
        "# Daily Report\nREPORT-MARKER-DEFAULT-INJECTION\n",
    );
    let provider = LaputaMemoryProvider::open(workspace).unwrap();

    let markdown = prompt_markdown(&provider, workspace);
    assert!(markdown.contains("Applied Laputa Authority"), "sanity");
    assert!(!markdown.contains("REPORT-MARKER-DEFAULT-INJECTION"));

    let service = seed_persona(workspace, "Persona identity", "Persona world");
    let frozen = FrozenCoreSnapshot::capture(&service).unwrap();
    assert!(!frozen.render(0).contains("REPORT-MARKER-DEFAULT-INJECTION"));
}

#[test]
fn frozen_core_stays_immutable_within_a_session() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path();
    let service = seed_persona(workspace, "SESSION-START-MARKER", "Persona world");

    let snapshot = FrozenCoreSnapshot::capture(&service).unwrap();
    let identity = service.get_document(PersonaKind::Identity).unwrap();
    service
        .save_user_document(
            PersonaKind::Identity,
            "MID-SESSION-REWRITE",
            identity.revision,
            "test rewrite",
        )
        .unwrap();

    let rendered = snapshot.render(0);
    assert!(rendered.contains("SESSION-START-MARKER"));
    assert!(!rendered.contains("MID-SESSION-REWRITE"));

    // The next session observes the applied write.
    let next = FrozenCoreSnapshot::capture(&service).unwrap();
    assert!(next.render(0).contains("MID-SESSION-REWRITE"));
}

fn claim(domain: &str, title: &str, scopes: &[&str], body: &str) -> WorldClaim {
    WorldClaim {
        domain: domain.to_string(),
        title: title.to_string(),
        status: ClaimStatus::Observed,
        confidence: "medium".to_string(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        source: "test".to_string(),
        updated: ts(),
        text: body.to_string(),
    }
}

#[test]
fn world_projection_is_scope_matched_and_budgeted() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("WORLD.MD");
    let mut store = WorldStore::load(&path).unwrap();
    for index in 0..20 {
        store
            .governed_upsert(
                "user",
                claim(
                    "ops",
                    &format!("session claim {index}"),
                    &["session"],
                    &format!("body-{index}-{}", "x".repeat(270)),
                ),
            )
            .unwrap();
    }
    store
        .governed_upsert(
            "user",
            claim("ops", "workspace claim", &["workspace"], "other scope"),
        )
        .unwrap();
    assert_eq!(store.total(), 21);

    // Scope filter: only session-scoped claims may surface.
    let scoped = store.project(&["session".to_string()], 16000);
    assert_eq!(scoped.len(), 20);
    assert!(scoped.iter().all(|claim| claim.title != "workspace claim"));

    // Budget: the default 4000-char budget cannot fit 20 x 280-char bodies.
    let budgeted = store.project(&["session".to_string()], 0);
    assert!(budgeted.len() < 20);
    let used: usize = budgeted
        .iter()
        .map(|claim| claim.text.chars().count())
        .sum();
    assert!(
        used <= 4000,
        "projection must honor the budget, used={used}"
    );

    // Unmatched scopes project nothing.
    assert!(store
        .project(&["nonexistent".to_string()], 16000)
        .is_empty());
}

#[test]
fn retired_section_and_proposal_data_never_surface() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path();
    seed_applied_authority(workspace);
    // Residual file left behind by the pre-hard-delete registry.
    write_file(
        &workspace
            .join(".laputa")
            .join("sections")
            .join("history_md.json"),
        r#""RETIRED-SECTION-MARKER""#,
    );
    // Persisted proposal against a retired proposal type.
    write_file(
        &workspace
            .join(".laputa")
            .join("proposals")
            .join("retired.json"),
        r#"{"id":"retired","created_at":"2026-08-01T00:00:00Z","updated_at":"2026-08-01T00:00:00Z","created_by":"legacy","proposal_type":"history_patch","target_section":"history_md","evidence_refs":[],"proposed_patch":"\"x\"","risk_level":"low","state":"pending_review","source_run_id":null}"#,
    );
    let service = LaputaService::open(workspace).unwrap();

    // Snapshot enumerates only live sections.
    let snapshot = service.read_snapshot(None).unwrap();
    assert!(!snapshot.sections.contains_key("history_md"));
    assert!(snapshot.sections.values().all(
        |section| section.content != serde_json::Value::String("RETIRED-SECTION-MARKER".into())
    ));

    // Retired names fail closed on lookup.
    assert!("history_md".parse::<LaputaSectionName>().is_err());

    // Listing skips the unreadable retired proposal without failing.
    let proposals = service.list_proposals(ProposalFilter::default()).unwrap();
    assert!(proposals.is_empty());

    // Prompt assembly never reads the residual file.
    let provider = LaputaMemoryProvider::new(service);
    let markdown = prompt_markdown(&provider, workspace);
    assert!(markdown.contains("Applied Laputa Authority"), "sanity");
    assert!(!markdown.contains("RETIRED-SECTION-MARKER"));
}

#[test]
fn retired_persona_files_never_reach_the_prompt() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path();
    seed_applied_authority(workspace);
    for (name, marker) in [
        ("SOUL.md", "PERSONA-SOUL-MARKER"),
        ("IDENTITY.md", "PERSONA-IDENTITY-MARKER"),
        ("USER.md", "PERSONA-USER-MARKER"),
        ("BOOTSTRAP.md", "PERSONA-BOOTSTRAP-MARKER"),
        ("MEMORY.md", "PERSONA-MEMORY-MARKER"),
        ("HISTORY.md", "PERSONA-HISTORY-MARKER"),
    ] {
        write_file(&workspace.join(name), &format!("marker: {marker}\n"));
    }
    let provider = LaputaMemoryProvider::open(workspace).unwrap();

    let markdown = prompt_markdown(&provider, workspace);
    assert!(markdown.contains("Applied Laputa Authority"), "sanity");
    for marker in [
        "PERSONA-SOUL-MARKER",
        "PERSONA-IDENTITY-MARKER",
        "PERSONA-USER-MARKER",
        "PERSONA-BOOTSTRAP-MARKER",
        "PERSONA-MEMORY-MARKER",
        "PERSONA-HISTORY-MARKER",
    ] {
        assert!(!markdown.contains(marker), "{marker} leaked into prompt");
    }
}

fn proposal_with_evidence(id: &str, evidence_refs: Vec<EvidenceRef>) -> EvolutionProposal {
    EvolutionProposal {
        id: id.to_string(),
        created_at: ts(),
        updated_at: ts(),
        created_by: "invariant-matrix".to_string(),
        proposal_type: ProposalType::MemoryPatch,
        target_section: LaputaSectionName::MemoryMd,
        evidence_refs,
        proposed_patch: r#""candidate""#.to_string(),
        risk_level: RiskLevel::Medium,
        state: ProposalState::PendingReview,
        source_run_id: None,
    }
}

fn evidence(source: EvidenceSource, id: &str) -> EvidenceRef {
    EvidenceRef {
        id: id.to_string(),
        source,
        uri: format!("session://{id}"),
        excerpt: Some("bounded evidence".to_string()),
        hash: None,
        created_at: ts(),
    }
}

#[test]
fn governance_evidence_is_required_and_bounded() {
    let temp = tempfile::tempdir().unwrap();
    let service = LaputaService::open(temp.path()).unwrap();

    // Missing evidence is rejected.
    let error = service
        .create_proposal(proposal_with_evidence("no-evidence", Vec::new()))
        .unwrap_err();
    assert!(error.to_string().contains("evidence"), "{error}");

    // Context-compaction-only evidence is secondary and insufficient.
    let error = service
        .create_proposal(proposal_with_evidence(
            "secondary-only",
            vec![evidence(EvidenceSource::ContextCompaction, "ev-compact")],
        ))
        .unwrap_err();
    assert!(error.to_string().contains("secondary"), "{error}");

    // Primary evidence is accepted.
    let created = service
        .create_proposal(proposal_with_evidence(
            "with-evidence",
            vec![evidence(EvidenceSource::Session, "ev-session")],
        ))
        .unwrap();
    assert_eq!(created.evidence_refs.len(), 1);
}

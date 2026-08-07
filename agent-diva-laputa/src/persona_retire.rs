//! Persona file retirement (LAPUTA-COGNITIVE-SYNC S4).
//!
//! The workspace persona layer (SOUL.md / IDENTITY.md / USER.md) is retired
//! as prompt authority. Its content migrates into the Frozen Core sections
//! through the normal proposal review pipeline (`PendingReview` → approve →
//! apply); MEMORY.md and BOOTSTRAP.md never become proposals and are only
//! archived read-only under `.laputa/legacy/`. Archiving source files is a
//! separate step that an operator runs after the proposals were approved and
//! applied, so a rejected migration never destroys the originals.

use std::fs;
use std::path::{Path, PathBuf};

use agent_diva_core::evolution::{
    EvidenceRef, EvidenceSource, EvolutionProposal, LaputaSectionName, ProposalState, ProposalType,
    RiskLevel,
};
use chrono::{DateTime, Utc};
use serde_json::json;

use crate::{LaputaError, LaputaService, Result};

/// Persona files eligible for retirement, in deterministic scan order.
const PERSONA_SOURCES: [(&str, PersonaSourceKind); 5] = [
    ("SOUL.md", PersonaSourceKind::FrozenCore),
    ("IDENTITY.md", PersonaSourceKind::FrozenCore),
    ("USER.md", PersonaSourceKind::FrozenCore),
    ("MEMORY.md", PersonaSourceKind::ArchiveOnly),
    ("BOOTSTRAP.md", PersonaSourceKind::ArchiveOnly),
];

/// How a retired persona file is migrated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonaSourceKind {
    /// Content migrates into a Frozen Core section via proposal review.
    FrozenCore,
    /// Read-only archive only; never becomes prompt authority again.
    ArchiveOnly,
}

/// One scanned persona file still living in the workspace root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonaSource {
    pub path: PathBuf,
    pub kind: PersonaSourceKind,
}

/// Migration plan for a workspace: proposals to review plus archive-only files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonaRetirementPlan {
    /// One proposal spec per target section (multiple source files may merge).
    pub proposals: Vec<PersonaProposalSpec>,
    /// Files that are archived read-only without any proposal.
    pub archives: Vec<PathBuf>,
}

impl PersonaRetirementPlan {
    pub fn is_empty(&self) -> bool {
        self.proposals.is_empty() && self.archives.is_empty()
    }

    /// Every workspace file this plan would move into `.laputa/legacy/`.
    pub fn all_source_paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = self
            .proposals
            .iter()
            .flat_map(|spec| spec.sources.iter().map(|source| source.path.clone()))
            .collect();
        paths.extend(self.archives.iter().cloned());
        paths
    }
}

/// Proposal spec for one Frozen Core target section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonaProposalSpec {
    pub target_section: LaputaSectionName,
    pub proposal_type: ProposalType,
    pub sources: Vec<PersonaSource>,
}

/// Scan the workspace root for retired persona files and build a plan.
///
/// Pure read: never touches disk. Files already missing yield an empty plan.
pub fn scan_workspace(workspace: &Path) -> Result<PersonaRetirementPlan> {
    let mut plan = PersonaRetirementPlan::default();
    for (relative, kind) in PERSONA_SOURCES {
        let path = workspace.join(relative);
        if !path.is_file() {
            continue;
        }
        let source = PersonaSource { path, kind };
        match kind {
            PersonaSourceKind::FrozenCore => {
                let section = frozen_core_section_for(relative);
                let proposal_type = proposal_type_for(&section);
                match plan
                    .proposals
                    .iter_mut()
                    .find(|spec| spec.target_section == section)
                {
                    Some(spec) => spec.sources.push(source),
                    None => plan.proposals.push(PersonaProposalSpec {
                        target_section: section,
                        proposal_type,
                        sources: vec![source],
                    }),
                }
            }
            PersonaSourceKind::ArchiveOnly => plan.archives.push(source.path),
        }
    }
    Ok(plan)
}

/// Create `PendingReview` proposals for every spec in the plan.
///
/// Nothing is applied here: writes only land after human review approves the
/// proposals through the regular Laputa apply path.
pub fn create_proposals(
    service: &LaputaService,
    plan: &PersonaRetirementPlan,
    actor: &str,
    now: DateTime<Utc>,
) -> Result<Vec<EvolutionProposal>> {
    let mut created = Vec::new();
    for spec in &plan.proposals {
        let mut entries = Vec::new();
        for source in &spec.sources {
            let content = fs::read_to_string(&source.path)
                .map_err(|error| LaputaError::io(&source.path, error))?;
            entries.push(json!({
                "source_path": source.path,
                "content": content,
                "migrated_at": now,
            }));
        }
        let patch = json!({
            "status": "owned",
            "entries": entries,
            "metadata": {
                "migration": "persona_retirement",
                "content_type": "legacy_markdown",
            },
        });
        let id = format!(
            "persona-retire-{}-{}",
            spec.target_section.as_str(),
            now.timestamp_micros()
        );
        let evidence_excerpt = spec
            .sources
            .iter()
            .map(|source| source.path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let proposal = EvolutionProposal {
            id: id.clone(),
            created_at: now,
            updated_at: now,
            created_by: actor.to_string(),
            proposal_type: spec.proposal_type.clone(),
            target_section: spec.target_section.clone(),
            evidence_refs: vec![EvidenceRef {
                id: format!("evidence-{id}"),
                source: EvidenceSource::File,
                uri: format!("file://persona-retire/{}", spec.target_section.as_str()),
                excerpt: Some(evidence_excerpt.chars().take(240).collect()),
                hash: None,
                created_at: now,
            }],
            proposed_patch: patch.to_string(),
            risk_level: RiskLevel::High,
            state: ProposalState::PendingReview,
            source_run_id: None,
        };
        created.push(service.create_proposal(proposal)?);
    }
    Ok(created)
}

/// Move every planned source file into `.laputa/legacy/` (inert archive).
///
/// Call only after the migration proposals were approved and applied. Files
/// already missing are skipped, so the operation is idempotent. Existing
/// archive targets are never overwritten: the source is kept and reported.
pub fn archive_sources(
    workspace: &Path,
    plan: &PersonaRetirementPlan,
) -> Result<PersonaArchiveOutcome> {
    let legacy_dir = workspace.join(".laputa").join("legacy");
    let mut outcome = PersonaArchiveOutcome::default();
    for path in plan.all_source_paths() {
        if !path.is_file() {
            outcome.skipped.push(path);
            continue;
        }
        let Some(file_name) = path.file_name() else {
            outcome.skipped.push(path);
            continue;
        };
        let target = legacy_dir.join(file_name);
        if target.exists() {
            outcome.kept.push(path);
            continue;
        }
        fs::create_dir_all(&legacy_dir).map_err(|error| LaputaError::io(&legacy_dir, error))?;
        fs::rename(&path, &target).map_err(|error| LaputaError::io(&path, error))?;
        outcome.moved.push((path, target));
    }
    Ok(outcome)
}

/// Result of archiving retired persona sources.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonaArchiveOutcome {
    /// (source, archive target) pairs successfully moved.
    pub moved: Vec<(PathBuf, PathBuf)>,
    /// Sources left in place because an archive target already existed.
    pub kept: Vec<PathBuf>,
    /// Sources already missing (previous run archived them).
    pub skipped: Vec<PathBuf>,
}

fn frozen_core_section_for(relative: &str) -> LaputaSectionName {
    match relative {
        "SOUL.md" | "IDENTITY.md" => LaputaSectionName::Identity,
        "USER.md" => LaputaSectionName::Relationship,
        // PERSONA_SOURCES never routes other files into Frozen Core.
        _ => LaputaSectionName::Preferences,
    }
}

fn proposal_type_for(section: &LaputaSectionName) -> ProposalType {
    match section {
        LaputaSectionName::Identity => ProposalType::IdentityPatch,
        LaputaSectionName::Relationship => ProposalType::RelationshipUpdate,
        LaputaSectionName::Commitment => ProposalType::CommitmentSet,
        LaputaSectionName::Preferences => ProposalType::LearningNote,
        // Retirement only targets Frozen Core sections.
        _ => ProposalType::MemoryPatch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LaputaStorage;

    fn seed(workspace: &Path, files: &[(&str, &str)]) {
        for (name, body) in files {
            fs::write(workspace.join(name), body).unwrap();
        }
    }

    #[test]
    fn scan_maps_persona_files_to_frozen_core_sections() {
        let temp = tempfile::tempdir().unwrap();
        seed(
            temp.path(),
            &[
                ("SOUL.md", "# Soul"),
                ("IDENTITY.md", "# Identity"),
                ("USER.md", "# User"),
                ("MEMORY.md", "- legacy memory"),
                ("BOOTSTRAP.md", "# Bootstrap"),
                ("AGENTS.md", "# kept"),
            ],
        );
        let plan = scan_workspace(temp.path()).unwrap();

        assert_eq!(plan.proposals.len(), 2);
        let identity = plan
            .proposals
            .iter()
            .find(|spec| spec.target_section == LaputaSectionName::Identity)
            .unwrap();
        assert_eq!(identity.sources.len(), 2);
        assert_eq!(identity.proposal_type, ProposalType::IdentityPatch);
        let relationship = plan
            .proposals
            .iter()
            .find(|spec| spec.target_section == LaputaSectionName::Relationship)
            .unwrap();
        assert_eq!(relationship.proposal_type, ProposalType::RelationshipUpdate);
        assert_eq!(plan.archives.len(), 2);
        assert_eq!(plan.all_source_paths().len(), 5);
    }

    #[test]
    fn scan_of_clean_workspace_is_empty() {
        let temp = tempfile::tempdir().unwrap();
        let plan = scan_workspace(temp.path()).unwrap();
        assert!(plan.is_empty());
    }

    #[test]
    fn proposals_are_pending_review_and_carry_content() {
        let temp = tempfile::tempdir().unwrap();
        seed(temp.path(), &[("SOUL.md", "# Soul\nCore traits.")]);
        LaputaStorage::open(temp.path()).unwrap();
        let service = LaputaService::open(temp.path()).unwrap();
        let plan = scan_workspace(temp.path()).unwrap();

        let proposals = create_proposals(&service, &plan, "operator", Utc::now()).unwrap();
        assert_eq!(proposals.len(), 1);
        let proposal = &proposals[0];
        assert_eq!(proposal.state, ProposalState::PendingReview);
        assert_eq!(proposal.target_section, LaputaSectionName::Identity);
        assert_eq!(proposal.risk_level, RiskLevel::High);
        let patch: serde_json::Value = serde_json::from_str(&proposal.proposed_patch).unwrap();
        assert_eq!(patch["status"], "owned");
        assert!(patch["entries"][0]["content"]
            .as_str()
            .unwrap()
            .contains("Core traits"));
    }

    #[test]
    fn apply_then_archive_retires_sources() {
        let temp = tempfile::tempdir().unwrap();
        seed(
            temp.path(),
            &[("SOUL.md", "# Soul"), ("MEMORY.md", "- legacy memory")],
        );
        LaputaStorage::open(temp.path()).unwrap();
        let service = LaputaService::open(temp.path()).unwrap();
        let plan = scan_workspace(temp.path()).unwrap();
        let proposals = create_proposals(&service, &plan, "operator", Utc::now()).unwrap();

        // Approval: review transition, then apply writes the Frozen Core section.
        service
            .transition_proposal(&proposals[0].id, ProposalState::Approved, Utc::now())
            .unwrap();
        service
            .apply_proposal(&proposals[0].id, "human", Utc::now())
            .unwrap();
        let section = service.read_section(LaputaSectionName::Identity).unwrap();
        assert!(section.content["entries"][0]["content"]
            .as_str()
            .unwrap()
            .contains("# Soul"));

        // Only after approval are sources moved to the inert archive.
        let outcome = archive_sources(temp.path(), &plan).unwrap();
        assert_eq!(outcome.moved.len(), 2);
        assert!(!temp.path().join("SOUL.md").exists());
        assert!(!temp.path().join("MEMORY.md").exists());
        assert!(temp
            .path()
            .join(".laputa")
            .join("legacy")
            .join("SOUL.md")
            .exists());
        assert!(temp
            .path()
            .join(".laputa")
            .join("legacy")
            .join("MEMORY.md")
            .exists());

        // Idempotent: a second run finds nothing left to move.
        let rerun = archive_sources(temp.path(), &plan).unwrap();
        assert!(rerun.moved.is_empty());
        assert_eq!(rerun.skipped.len(), 2);
    }

    #[test]
    fn archive_never_overwrites_existing_legacy_files() {
        let temp = tempfile::tempdir().unwrap();
        seed(temp.path(), &[("SOUL.md", "# newer")]);
        let legacy = temp.path().join(".laputa").join("legacy");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("SOUL.md"), "# older").unwrap();

        let plan = scan_workspace(temp.path()).unwrap();
        let outcome = archive_sources(temp.path(), &plan).unwrap();
        assert!(outcome.moved.is_empty());
        assert_eq!(outcome.kept.len(), 1);
        assert!(temp.path().join("SOUL.md").exists());
        assert_eq!(
            fs::read_to_string(legacy.join("SOUL.md")).unwrap(),
            "# older"
        );
    }
}

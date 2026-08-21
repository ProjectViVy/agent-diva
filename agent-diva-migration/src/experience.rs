use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use agent_diva_core::{
    experience::{ExperienceEvidence, ExperienceJournal},
    session::SessionManager,
};
use agent_diva_laputa::atomic_write_json;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize)]
pub struct ExperienceMigrationReport {
    pub migration_id: String,
    pub manifest_path: PathBuf,
    pub session_count: usize,
    pub candidate_count: usize,
    pub already_present_count: usize,
    pub rejected_count: usize,
    pub applied_count: usize,
    pub rolled_back_count: usize,
    pub records_digest: String,
    pub workspace_id: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExperienceMigrationManifest {
    schema_version: u32,
    migration_id: String,
    workspace_id: String,
    records_digest: String,
    evidence_ids: Vec<String>,
    preexisting_ids: Vec<String>,
    state: ManifestState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ManifestState {
    Prepared,
    Applied,
    RolledBack,
}

struct PreparedBackfill {
    migration_id: String,
    manifest_path: PathBuf,
    session_count: usize,
    rejected_count: usize,
    already_present_count: usize,
    records_digest: String,
    workspace_id: String,
    candidates: Vec<ExperienceEvidence>,
}

pub fn dry_run(workspace: &Path) -> Result<ExperienceMigrationReport> {
    let prepared = prepare(workspace)?;
    Ok(prepared.report("dry_run", 0, 0))
}

pub fn apply(workspace: &Path) -> Result<ExperienceMigrationReport> {
    let prepared = prepare(workspace)?;
    let journal = ExperienceJournal::open(workspace);
    let current = journal.read_recent(usize::MAX)?;
    let missing_count = prepared
        .candidates
        .len()
        .saturating_sub(prepared.already_present_count);
    if current.items.len() + missing_count > journal.capacity() {
        bail!(
            "experience backfill exceeds journal capacity: current={}, candidates={}, capacity={}",
            current.items.len(),
            missing_count,
            journal.capacity()
        );
    }

    if prepared.manifest_path.exists() {
        let manifest: ExperienceMigrationManifest =
            serde_json::from_slice(&fs::read(&prepared.manifest_path)?)?;
        validate_manifest(&manifest, &prepared)?;
        if manifest.state == ManifestState::RolledBack {
            bail!("experience migration was rolled back and cannot be replayed");
        }
        if manifest.state == ManifestState::Applied {
            return Ok(prepared.report("applied", 0, 0));
        }
    } else {
        let manifest = ExperienceMigrationManifest {
            schema_version: 1,
            migration_id: prepared.migration_id.clone(),
            workspace_id: prepared.workspace_id.clone(),
            records_digest: prepared.records_digest.clone(),
            evidence_ids: prepared
                .candidates
                .iter()
                .map(|item| item.id.clone())
                .collect(),
            preexisting_ids: prepared
                .candidates
                .iter()
                .filter(|item| current.items.iter().any(|existing| existing.id == item.id))
                .map(|item| item.id.clone())
                .collect(),
            state: ManifestState::Prepared,
        };
        atomic_write_json(&prepared.manifest_path, &manifest)?;
    }

    let mut applied = 0;
    for evidence in &prepared.candidates {
        if journal.append(evidence)? {
            applied += 1;
        }
    }
    let mut manifest: ExperienceMigrationManifest =
        serde_json::from_slice(&fs::read(&prepared.manifest_path)?)?;
    manifest.state = ManifestState::Applied;
    atomic_write_json(&prepared.manifest_path, &manifest)?;
    Ok(prepared.report("applied", applied, 0))
}

pub fn rollback(workspace: &Path, migration_id: &str) -> Result<ExperienceMigrationReport> {
    let manifest_path = manifest_path(workspace, migration_id);
    let mut manifest: ExperienceMigrationManifest = serde_json::from_slice(
        &fs::read(&manifest_path).context("experience migration manifest not found")?,
    )?;
    let journal = ExperienceJournal::open(workspace);
    if manifest.workspace_id != journal.workspace_id() || manifest.migration_id != migration_id {
        bail!("experience migration manifest identity mismatch");
    }
    if manifest.state == ManifestState::RolledBack {
        bail!("experience migration already rolled back");
    }
    let preexisting = manifest
        .preexisting_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let ids = manifest
        .evidence_ids
        .iter()
        .filter(|id| !preexisting.contains(id.as_str()))
        .cloned()
        .collect::<HashSet<_>>();
    let removed = journal.remove_ids(&ids)?;
    manifest.state = ManifestState::RolledBack;
    atomic_write_json(&manifest_path, &manifest)?;
    Ok(ExperienceMigrationReport {
        migration_id: manifest.migration_id,
        manifest_path,
        session_count: 0,
        candidate_count: manifest.evidence_ids.len(),
        already_present_count: 0,
        rejected_count: 0,
        applied_count: 0,
        rolled_back_count: removed,
        records_digest: manifest.records_digest,
        workspace_id: manifest.workspace_id,
        state: "rolled_back".to_string(),
    })
}

fn prepare(workspace: &Path) -> Result<PreparedBackfill> {
    let journal = ExperienceJournal::open(workspace);
    let existing = journal.read_recent(usize::MAX)?;
    let existing_ids = existing
        .items
        .iter()
        .map(|item| item.id.as_str())
        .collect::<HashSet<_>>();
    let mut sessions = SessionManager::new(workspace);
    let infos = sessions.list_sessions();
    let mut rejected_count = 0;
    let mut candidates = Vec::new();

    for info in &infos {
        let Some(session) = sessions.get_or_load(&info.key) else {
            rejected_count += 1;
            continue;
        };
        for message in &session.messages {
            if message.role != "tool" {
                continue;
            }
            let (Some(action_id), Some(tool_name)) =
                (message.tool_call_id.as_deref(), message.name.as_deref())
            else {
                rejected_count += 1;
                continue;
            };
            let evidence = journal.session_backfill_evidence(
                &session.key,
                action_id,
                tool_name,
                message.content.trim_start().starts_with("Error:"),
                message.timestamp,
            );
            candidates.push(evidence);
        }
    }
    candidates.sort_by_key(|candidate| candidate.id.clone());
    candidates.dedup_by(|left, right| left.id == right.id);
    let already_present_count = candidates
        .iter()
        .filter(|item| existing_ids.contains(item.id.as_str()))
        .count();
    let records_digest = digest_ids(candidates.iter().map(|item| item.id.as_str()));
    let migration_id = format!("experience-{}", &records_digest[..24]);
    Ok(PreparedBackfill {
        manifest_path: manifest_path(workspace, &migration_id),
        migration_id,
        session_count: infos.len(),
        rejected_count,
        already_present_count,
        records_digest,
        workspace_id: journal.workspace_id(),
        candidates,
    })
}

fn manifest_path(workspace: &Path, migration_id: &str) -> PathBuf {
    workspace
        .join(".agent-diva")
        .join("autodream")
        .join("experience")
        .join("migrations")
        .join(migration_id)
        .join("manifest.json")
}

fn digest_ids<'a>(ids: impl Iterator<Item = &'a str>) -> String {
    let mut digest = Sha256::new();
    for id in ids {
        digest.update(id.as_bytes());
        digest.update([0]);
    }
    format!("{:x}", digest.finalize())
}

fn validate_manifest(
    manifest: &ExperienceMigrationManifest,
    prepared: &PreparedBackfill,
) -> Result<()> {
    if manifest.schema_version != 1
        || manifest.migration_id != prepared.migration_id
        || manifest.workspace_id != prepared.workspace_id
        || manifest.records_digest != prepared.records_digest
    {
        bail!("experience migration manifest conflicts with current inputs");
    }
    Ok(())
}

impl PreparedBackfill {
    fn report(
        &self,
        state: &str,
        applied_count: usize,
        rolled_back_count: usize,
    ) -> ExperienceMigrationReport {
        ExperienceMigrationReport {
            migration_id: self.migration_id.clone(),
            manifest_path: self.manifest_path.clone(),
            session_count: self.session_count,
            candidate_count: self.candidates.len(),
            already_present_count: self.already_present_count,
            rejected_count: self.rejected_count,
            applied_count,
            rolled_back_count,
            records_digest: self.records_digest.clone(),
            workspace_id: self.workspace_id.clone(),
            state: state.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::session::{ChatMessage, SessionManager};

    #[test]
    fn dry_run_apply_replay_and_rollback_are_payload_free() {
        let temp = tempfile::tempdir().unwrap();
        seed_tool_result(temp.path(), "chat:1", "call-1", "exec", "secret-output");

        let preview = dry_run(temp.path()).unwrap();
        assert_eq!(preview.state, "dry_run");
        assert_eq!(preview.candidate_count, 1);
        assert!(!preview.manifest_path.exists());

        let applied = apply(temp.path()).unwrap();
        assert_eq!(applied.applied_count, 1);
        let journal = ExperienceJournal::open(temp.path());
        let raw = fs::read_to_string(journal.path()).unwrap();
        assert!(!raw.contains("secret-output"));
        assert_eq!(journal.read_recent(10).unwrap().items.len(), 1);

        let replay = apply(temp.path()).unwrap();
        assert_eq!(replay.migration_id, applied.migration_id);
        assert_eq!(replay.applied_count, 0);
        assert_eq!(journal.read_recent(10).unwrap().items.len(), 1);

        let rolled_back = rollback(temp.path(), &applied.migration_id).unwrap();
        assert_eq!(rolled_back.rolled_back_count, 1);
        assert!(journal.read_recent(10).unwrap().items.is_empty());
        assert!(rollback(temp.path(), &applied.migration_id).is_err());
    }

    #[test]
    fn missing_tool_identity_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let mut manager = SessionManager::new(temp.path());
        let session = manager.get_or_create("chat:1");
        session.messages.push(ChatMessage::new("tool", "output"));
        let saved = session.clone();
        manager.save(&saved).unwrap();

        let report = dry_run(temp.path()).unwrap();
        assert_eq!(report.candidate_count, 0);
        assert_eq!(report.rejected_count, 1);
    }

    #[test]
    fn rollback_preserves_evidence_that_predated_the_manifest() {
        let temp = tempfile::tempdir().unwrap();
        seed_tool_result(temp.path(), "chat:1", "call-1", "exec", "output");
        let journal = ExperienceJournal::open(temp.path());
        let mut manager = SessionManager::new(temp.path());
        let occurred_at = manager.get_or_load("chat:1").unwrap().messages[0].timestamp;
        let existing =
            journal.session_backfill_evidence("chat:1", "call-1", "exec", false, occurred_at);
        journal.append(&existing).unwrap();

        let applied = apply(temp.path()).unwrap();
        assert_eq!(applied.already_present_count, 1);
        assert_eq!(applied.applied_count, 0);
        let rolled_back = rollback(temp.path(), &applied.migration_id).unwrap();
        assert_eq!(rolled_back.rolled_back_count, 0);
        assert_eq!(journal.read_recent(10).unwrap().items.len(), 1);
    }

    fn seed_tool_result(
        workspace: &Path,
        session_key: &str,
        action_id: &str,
        tool_name: &str,
        content: &str,
    ) {
        let mut manager = SessionManager::new(workspace);
        let session = manager.get_or_create(session_key);
        let mut message = ChatMessage::new("tool", content);
        message.tool_call_id = Some(action_id.to_string());
        message.name = Some(tool_name.to_string());
        session.messages.push(message);
        let saved = session.clone();
        manager.save(&saved).unwrap();
    }
}

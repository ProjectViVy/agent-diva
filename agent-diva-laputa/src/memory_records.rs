//! Read-only normalization adapters and isolated Memory v2 migration artifacts.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use agent_diva_core::{
    evolution::{EvolutionProposal, LaputaSectionName},
    governance::{AuditCorrelation, ContentDigest},
    memory::{
        memory_content_digest, MemoryIntegrityFinding, MemoryIntegrityReport,
        MemoryIntegritySeverity, MemoryProvenance, MemoryProvenanceSource, MemoryRecord,
        MemoryRecordKind, MemoryScope, MemorySensitivity, MemoryTombstone, MemoryTrust,
    },
};

/// Normalize one governed proposal for typed authority apply.
pub fn adapt_governed_proposal(
    proposal: &EvolutionProposal,
    context: &MemoryAdapterContext,
) -> MemoryRecord {
    let proposal_digest = memory_content_digest(proposal.proposed_patch.as_bytes());
    let deprecation = (proposal.proposal_type
        == agent_diva_core::evolution::ProposalType::Deprecation)
        .then(|| parse_deprecation_patch(&proposal.proposed_patch))
        .flatten();
    let content = if deprecation.is_some() {
        String::new()
    } else {
        proposal.proposed_patch.clone()
    };
    let digest = memory_content_digest(content.as_bytes());
    MemoryRecord {
        id: deterministic_record_id("proposal", &proposal.id, &digest),
        kind: record_kind_for_section(&proposal.target_section),
        content,
        provenance: MemoryProvenance {
            source: if proposal.source_run_id.is_some() {
                MemoryProvenanceSource::AutoDream
            } else {
                MemoryProvenanceSource::LaputaAppliedSection
            },
            source_id: proposal
                .source_run_id
                .clone()
                .unwrap_or_else(|| proposal.id.clone()),
            content_digest: digest,
            captured_at: context.captured_at,
            correlation: context.correlation.clone(),
        },
        evidence_refs: proposal.evidence_refs.clone(),
        confidence_bps: 10_000,
        sensitivity: MemorySensitivity::Private,
        trust: MemoryTrust::AppliedAuthority,
        scope: scope(context),
        created_at: proposal.created_at,
        effective_at: context.captured_at,
        expires_at: None,
        supersedes: deprecation
            .as_ref()
            .map(|patch| vec![patch.target_record_id.clone()])
            .unwrap_or_default(),
        tombstone: deprecation.map(|patch| MemoryTombstone {
            target_record_id: patch.target_record_id,
            reason_digest: proposal_digest,
            actor_id: proposal.created_by.clone(),
            created_at: context.captured_at,
        }),
    }
}

#[derive(Deserialize)]
struct DeprecationPatch {
    schema_version: u32,
    target_record_id: String,
    reason: String,
}

fn parse_deprecation_patch(value: &str) -> Option<DeprecationPatch> {
    serde_json::from_str::<DeprecationPatch>(value)
        .ok()
        .filter(|patch| {
            patch.schema_version == 1
                && !patch.target_record_id.trim().is_empty()
                && !patch.reason.trim().is_empty()
        })
}
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{atomic_write_json, LaputaError, LaputaSection, Result, SectionStatus};

const ARTIFACT_SCHEMA_VERSION: &str = "1.0.0";
const ARTIFACT_OUTPUTS: [&str; 3] = ["records.json", "rollback.json", "manifest.json"];

/// Inputs required to assign ownership and audit correlation during adaptation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryAdapterContext {
    pub tenant_id: String,
    pub workspace_id: String,
    pub session_id: Option<String>,
    pub correlation: AuditCorrelation,
    pub captured_at: DateTime<Utc>,
}

/// Result of adapting one source without modifying it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryAdapterOutput {
    pub records: Vec<MemoryRecord>,
    pub findings: Vec<MemoryIntegrityFinding>,
}

/// Normalize one legacy owner Markdown file.
pub fn adapt_legacy_markdown(
    path: &Path,
    content: &str,
    kind: MemoryRecordKind,
    context: &MemoryAdapterContext,
    legacy_owner_selected: bool,
) -> MemoryAdapterOutput {
    let digest = memory_content_digest(content.as_bytes());
    let source_id = path.to_string_lossy().replace('\\', "/");
    let record = MemoryRecord {
        id: deterministic_record_id("legacy", &source_id, &digest),
        kind,
        content: content.to_string(),
        provenance: MemoryProvenance {
            source: MemoryProvenanceSource::LegacyMarkdownOwner,
            source_id,
            content_digest: digest,
            captured_at: context.captured_at,
            correlation: context.correlation.clone(),
        },
        evidence_refs: Vec::new(),
        confidence_bps: if legacy_owner_selected { 10_000 } else { 5_000 },
        sensitivity: MemorySensitivity::Private,
        trust: if legacy_owner_selected {
            MemoryTrust::AppliedAuthority
        } else {
            MemoryTrust::Untrusted
        },
        scope: scope(context),
        created_at: context.captured_at,
        effective_at: context.captured_at,
        expires_at: None,
        supersedes: Vec::new(),
        tombstone: None,
    };
    MemoryAdapterOutput {
        records: vec![record],
        findings: Vec::new(),
    }
}

/// Normalize an applied Laputa section and preserve unknown top-level keys as warnings.
pub fn adapt_laputa_section(
    section: &LaputaSection,
    raw_section: &serde_json::Value,
    context: &MemoryAdapterContext,
) -> Result<MemoryAdapterOutput> {
    let mut findings = unknown_section_findings(raw_section, section.name.as_str());
    if section.status != SectionStatus::Owned {
        findings.push(MemoryIntegrityFinding {
            code: "laputa_section_not_owned".into(),
            severity: MemoryIntegritySeverity::Warning,
            record_id: None,
            source_id: Some(section.name.as_str().into()),
        });
        return Ok(MemoryAdapterOutput {
            records: Vec::new(),
            findings,
        });
    }

    let content = match &section.content {
        serde_json::Value::String(content) => content.clone(),
        content => serde_json::to_string(content)?,
    };
    let digest = memory_content_digest(content.as_bytes());
    let source_id = section.name.as_str().to_string();
    let record = MemoryRecord {
        id: deterministic_record_id("laputa", &source_id, &digest),
        kind: record_kind_for_section(&section.name),
        content,
        provenance: MemoryProvenance {
            source: MemoryProvenanceSource::LaputaAppliedSection,
            source_id,
            content_digest: digest,
            captured_at: context.captured_at,
            correlation: context.correlation.clone(),
        },
        evidence_refs: Vec::new(),
        confidence_bps: 10_000,
        sensitivity: MemorySensitivity::Private,
        trust: MemoryTrust::AppliedAuthority,
        scope: scope(context),
        created_at: section.last_modified.unwrap_or(context.captured_at),
        effective_at: section.last_modified.unwrap_or(context.captured_at),
        expires_at: None,
        supersedes: Vec::new(),
        tombstone: None,
    };

    Ok(MemoryAdapterOutput {
        records: vec![record],
        findings,
    })
}

/// Compare source bytes and normalized records without changing either side.
pub fn compare_normalized_records(
    source_bytes: &[u8],
    records: &[MemoryRecord],
    mut findings: Vec<MemoryIntegrityFinding>,
    now: DateTime<Utc>,
) -> Result<MemoryIntegrityReport> {
    let normalized_bytes = serde_json::to_vec(records)?;
    let mut ids = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for record in records {
        if !ids.insert(record.id.clone()) {
            duplicates.insert(record.id.clone());
        }
    }
    let known_ids = records
        .iter()
        .map(|record| record.id.as_str())
        .collect::<BTreeSet<_>>();
    let broken_supersedes = records
        .iter()
        .flat_map(|record| record.supersedes.iter())
        .filter(|id| !known_ids.contains(id.as_str()))
        .cloned()
        .collect::<BTreeSet<_>>();

    for id in &duplicates {
        findings.push(MemoryIntegrityFinding {
            code: "duplicate_record_id".into(),
            severity: MemoryIntegritySeverity::Error,
            record_id: Some(id.clone()),
            source_id: None,
        });
    }
    for id in &broken_supersedes {
        findings.push(MemoryIntegrityFinding {
            code: "broken_supersedes".into(),
            severity: MemoryIntegritySeverity::Error,
            record_id: Some(id.clone()),
            source_id: None,
        });
    }

    Ok(MemoryIntegrityReport {
        source_digest: memory_content_digest(source_bytes),
        normalized_digest: memory_content_digest(&normalized_bytes),
        source_record_count: if source_bytes.is_empty() { 0 } else { 1 },
        normalized_record_count: records.len() as u64,
        duplicate_record_ids: duplicates.into_iter().collect(),
        broken_supersedes: broken_supersedes.into_iter().collect(),
        expired_record_count: records
            .iter()
            .filter(|record| record.expires_at.is_some_and(|expiry| expiry <= now))
            .count() as u64,
        tombstone_record_count: records
            .iter()
            .filter(|record| record.tombstone.is_some())
            .count() as u64,
        findings,
    })
}

/// Versioned, isolated Memory migration manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryMigrationManifest {
    pub schema_version: String,
    pub migration_id: String,
    pub input_digest: ContentDigest,
    pub records_digest: ContentDigest,
    pub record_count: u64,
    pub created_at: DateTime<Utc>,
    pub outputs: Vec<String>,
}

/// Content-free rollback manifest listing only files owned by one migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRollbackManifest {
    pub schema_version: String,
    pub migration_id: String,
    pub outputs: Vec<String>,
}

/// Dry-run result that can be reviewed before any artifact is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryMigrationPlan {
    pub directory: PathBuf,
    pub manifest: MemoryMigrationManifest,
    pub rollback: MemoryRollbackManifest,
    pub records: Vec<MemoryRecord>,
}

/// Writer for isolated Memory v2 migration artifacts.
#[derive(Debug, Clone)]
pub struct MemoryRecordMigration {
    artifact_root: PathBuf,
}

/// Test-only fault boundary used to prove artifact cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryMigrationTestFailure {
    AfterRecords,
    AfterRollbackManifest,
}

impl MemoryRecordMigration {
    pub fn new(artifact_root: impl Into<PathBuf>) -> Self {
        Self {
            artifact_root: artifact_root.into(),
        }
    }

    pub fn dry_run(
        &self,
        migration_id: impl Into<String>,
        input_bytes: &[u8],
        records: Vec<MemoryRecord>,
        created_at: DateTime<Utc>,
    ) -> Result<MemoryMigrationPlan> {
        let migration_id = migration_id.into();
        if !valid_migration_id(&migration_id) {
            return Err(LaputaError::InvalidMemoryMigrationId { migration_id });
        }
        let records_bytes = serde_json::to_vec(&records)?;
        let outputs = expected_outputs();
        let manifest = MemoryMigrationManifest {
            schema_version: ARTIFACT_SCHEMA_VERSION.into(),
            migration_id: migration_id.clone(),
            input_digest: memory_content_digest(input_bytes),
            records_digest: memory_content_digest(&records_bytes),
            record_count: records.len() as u64,
            created_at,
            outputs: outputs.clone(),
        };
        Ok(MemoryMigrationPlan {
            directory: self.artifact_root.join(&migration_id),
            rollback: MemoryRollbackManifest {
                schema_version: ARTIFACT_SCHEMA_VERSION.into(),
                migration_id,
                outputs,
            },
            manifest,
            records,
        })
    }

    pub fn execute(&self, plan: &MemoryMigrationPlan) -> Result<MemoryMigrationManifest> {
        self.execute_with_failure(plan, None)
    }

    pub fn execute_with_failure(
        &self,
        plan: &MemoryMigrationPlan,
        test_failure: Option<MemoryMigrationTestFailure>,
    ) -> Result<MemoryMigrationManifest> {
        self.validate_plan(plan)?;
        let manifest_path = plan.directory.join("manifest.json");
        if manifest_path.exists() {
            let existing: MemoryMigrationManifest = serde_json::from_slice(
                &fs::read(&manifest_path)
                    .map_err(|source| LaputaError::io(&manifest_path, source))?,
            )?;
            if existing == plan.manifest {
                return Ok(existing);
            }
            return Err(LaputaError::MemoryMigrationConflict {
                migration_id: plan.manifest.migration_id.clone(),
            });
        }

        fs::create_dir_all(&plan.directory)
            .map_err(|source| LaputaError::io(&plan.directory, source))?;
        let result = (|| {
            atomic_write_json(plan.directory.join("records.json"), &plan.records)?;
            if test_failure == Some(MemoryMigrationTestFailure::AfterRecords) {
                return Err(LaputaError::InjectedMemoryMigrationFailure);
            }
            atomic_write_json(plan.directory.join("rollback.json"), &plan.rollback)?;
            if test_failure == Some(MemoryMigrationTestFailure::AfterRollbackManifest) {
                return Err(LaputaError::InjectedMemoryMigrationFailure);
            }
            atomic_write_json(&manifest_path, &plan.manifest)?;
            Ok(plan.manifest.clone())
        })();
        if result.is_err() {
            remove_owned_outputs(&plan.directory, &plan.rollback.outputs);
        }
        result
    }

    pub fn rollback(&self, migration_id: &str) -> Result<()> {
        if !valid_migration_id(migration_id) {
            return Err(LaputaError::InvalidMemoryMigrationId {
                migration_id: migration_id.into(),
            });
        }
        let directory = self.artifact_root.join(migration_id);
        let rollback_path = directory.join("rollback.json");
        let rollback: MemoryRollbackManifest = serde_json::from_slice(
            &fs::read(&rollback_path).map_err(|source| LaputaError::io(&rollback_path, source))?,
        )?;
        if rollback.schema_version != ARTIFACT_SCHEMA_VERSION
            || rollback.migration_id != migration_id
            || rollback.outputs != expected_outputs()
        {
            return Err(LaputaError::MemoryMigrationConflict {
                migration_id: migration_id.into(),
            });
        }
        for output in &rollback.outputs {
            let path = directory.join(output);
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => return Err(LaputaError::io(path, source)),
            }
        }
        match fs::remove_dir(&directory) {
            Ok(()) => Ok(()),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(LaputaError::io(directory, source)),
        }
    }

    fn validate_plan(&self, plan: &MemoryMigrationPlan) -> Result<()> {
        let records_digest = memory_content_digest(&serde_json::to_vec(&plan.records)?);
        if !valid_migration_id(&plan.manifest.migration_id)
            || plan.directory != self.artifact_root.join(&plan.manifest.migration_id)
            || plan.manifest.schema_version != ARTIFACT_SCHEMA_VERSION
            || plan.manifest.outputs != expected_outputs()
            || plan.manifest.record_count != plan.records.len() as u64
            || plan.manifest.records_digest != records_digest
            || plan.rollback.schema_version != ARTIFACT_SCHEMA_VERSION
            || plan.rollback.migration_id != plan.manifest.migration_id
            || plan.rollback.outputs != expected_outputs()
        {
            return Err(LaputaError::MemoryMigrationConflict {
                migration_id: plan.manifest.migration_id.clone(),
            });
        }
        Ok(())
    }
}

fn scope(context: &MemoryAdapterContext) -> MemoryScope {
    MemoryScope {
        tenant_id: context.tenant_id.clone(),
        workspace_id: context.workspace_id.clone(),
        session_id: context.session_id.clone(),
    }
}

fn deterministic_record_id(prefix: &str, source_id: &str, digest: &ContentDigest) -> String {
    let binding = format!("{prefix}\0{source_id}\0{}", digest.value);
    format!(
        "memory-{prefix}-{}",
        memory_content_digest(binding.as_bytes()).value
    )
}

fn record_kind_for_section(section: &LaputaSectionName) -> MemoryRecordKind {
    match section {
        LaputaSectionName::Identity => MemoryRecordKind::Identity,
        LaputaSectionName::Relationship => MemoryRecordKind::Relationship,
        LaputaSectionName::Commitment => MemoryRecordKind::Commitment,
        LaputaSectionName::Preferences => MemoryRecordKind::Preference,
        LaputaSectionName::MemoryMd => MemoryRecordKind::LongTerm,
        // Daily/Weekly/Monthly are report surfaces, not memories: reports are
        // never normalized into typed memory records.
        _ => MemoryRecordKind::Unknown,
    }
}

fn unknown_section_findings(
    raw_section: &serde_json::Value,
    source_id: &str,
) -> Vec<MemoryIntegrityFinding> {
    const KNOWN: [&str; 6] = [
        "name",
        "status",
        "content",
        "metadata",
        "last_modified",
        "version",
    ];
    raw_section
        .as_object()
        .into_iter()
        .flat_map(|object| object.keys())
        .filter(|key| !KNOWN.contains(&key.as_str()))
        .map(|key| MemoryIntegrityFinding {
            code: format!("unknown_laputa_field:{key}"),
            severity: MemoryIntegritySeverity::Warning,
            record_id: None,
            source_id: Some(source_id.into()),
        })
        .collect()
}

fn remove_owned_outputs(directory: &Path, outputs: &[String]) {
    for output in outputs {
        let _ = fs::remove_file(directory.join(output));
    }
    let _ = fs::remove_dir(directory);
}

fn expected_outputs() -> Vec<String> {
    ARTIFACT_OUTPUTS
        .iter()
        .map(|output| (*output).to_string())
        .collect()
}

fn valid_migration_id(migration_id: &str) -> bool {
    !migration_id.trim().is_empty()
        && !migration_id.contains('/')
        && !migration_id.contains('\\')
        && migration_id != "."
        && migration_id != ".."
}

#[cfg(test)]
mod tests {
    use agent_diva_core::{
        governance::AuditCorrelation,
        memory::{MemoryProvenanceSource, MemoryRecordKind, MemoryTrust},
    };
    use chrono::TimeZone;
    use serde_json::json;
    use tempfile::tempdir;

    use super::*;

    fn ts() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 29, 12, 0, 0)
            .single()
            .unwrap()
    }

    fn context() -> MemoryAdapterContext {
        MemoryAdapterContext {
            tenant_id: "tenant-1".into(),
            workspace_id: "workspace-1".into(),
            session_id: Some("session-1".into()),
            correlation: AuditCorrelation {
                request_id: "request-1".into(),
                turn_id: "turn-1".into(),
                session_id: "session-1".into(),
                trace_id: None,
            },
            captured_at: ts(),
        }
    }

    #[test]
    fn legacy_adapter_is_deterministic_and_authority_is_explicit() {
        let first = adapt_legacy_markdown(
            Path::new("MEMORY.md"),
            "hello",
            MemoryRecordKind::LongTerm,
            &context(),
            true,
        );
        let second = adapt_legacy_markdown(
            Path::new("MEMORY.md"),
            "hello",
            MemoryRecordKind::LongTerm,
            &context(),
            true,
        );
        assert_eq!(first, second);
        assert_eq!(first.records[0].trust, MemoryTrust::AppliedAuthority);
        assert_eq!(
            first.records[0].provenance.source,
            MemoryProvenanceSource::LegacyMarkdownOwner
        );
    }

    #[test]
    fn non_owned_laputa_sections_are_not_normalized() {
        let section = LaputaSection {
            name: LaputaSectionName::Changelog,
            status: SectionStatus::Tbd,
            content: json!({"pending": "must not enter prompt"}),
            metadata: json!({}),
            last_modified: Some(ts()),
            version: "1.0.0".into(),
        };
        let output = adapt_laputa_section(
            &section,
            &json!({
                "name": "changelog",
                "status": "tbd",
                "content": {},
                "metadata": {},
                "last_modified": null,
                "version": "1.0.0",
                "future_field": true
            }),
            &context(),
        )
        .unwrap();
        assert!(output.records.is_empty());
        assert_eq!(output.findings.len(), 2);
    }

    #[test]
    fn migration_is_idempotent_conflict_safe_and_reversible() {
        let temp = tempdir().unwrap();
        let original = temp.path().join("MEMORY.md");
        fs::write(&original, "original").unwrap();
        let adapter = adapt_legacy_markdown(
            &original,
            "original",
            MemoryRecordKind::LongTerm,
            &context(),
            true,
        );
        let migration = MemoryRecordMigration::new(temp.path().join("artifacts"));
        let plan = migration
            .dry_run("gmh-21", b"original", adapter.records, ts())
            .unwrap();

        assert!(!plan.directory.exists());
        let first = migration.execute(&plan).unwrap();
        assert_eq!(migration.execute(&plan).unwrap(), first);
        let restarted = MemoryRecordMigration::new(temp.path().join("artifacts"));
        assert_eq!(restarted.execute(&plan).unwrap(), first);
        assert_eq!(fs::read_to_string(&original).unwrap(), "original");

        let conflicting = migration
            .dry_run("gmh-21", b"changed", Vec::new(), ts())
            .unwrap();
        assert!(matches!(
            migration.execute(&conflicting),
            Err(LaputaError::MemoryMigrationConflict { .. })
        ));

        migration.rollback("gmh-21").unwrap();
        assert!(!plan.directory.exists());
        assert_eq!(fs::read_to_string(&original).unwrap(), "original");
    }

    #[test]
    fn failed_migration_cleans_only_owned_artifacts() {
        let temp = tempdir().unwrap();
        let artifacts = temp.path().join("artifacts");
        fs::create_dir_all(&artifacts).unwrap();
        fs::write(artifacts.join("unrelated"), "keep").unwrap();
        let migration = MemoryRecordMigration::new(&artifacts);
        let plan = migration
            .dry_run("failed", b"input", Vec::new(), ts())
            .unwrap();
        assert!(matches!(
            migration.execute_with_failure(
                &plan,
                Some(MemoryMigrationTestFailure::AfterRollbackManifest)
            ),
            Err(LaputaError::InjectedMemoryMigrationFailure)
        ));
        assert!(!plan.directory.exists());
        assert_eq!(
            fs::read_to_string(artifacts.join("unrelated")).unwrap(),
            "keep"
        );
    }

    #[test]
    fn forged_artifact_paths_and_rollback_manifests_fail_closed() {
        let temp = tempdir().unwrap();
        let migration = MemoryRecordMigration::new(temp.path().join("artifacts"));
        let mut plan = migration
            .dry_run("safe", b"input", Vec::new(), ts())
            .unwrap();
        plan.directory = temp.path().join("outside");
        assert!(matches!(
            migration.execute(&plan),
            Err(LaputaError::MemoryMigrationConflict { .. })
        ));

        let valid = migration
            .dry_run("rollback", b"input", Vec::new(), ts())
            .unwrap();
        migration.execute(&valid).unwrap();
        let outside = temp.path().join("outside.txt");
        fs::write(&outside, "keep").unwrap();
        atomic_write_json(
            valid.directory.join("rollback.json"),
            &MemoryRollbackManifest {
                schema_version: ARTIFACT_SCHEMA_VERSION.into(),
                migration_id: "rollback".into(),
                outputs: vec!["../outside.txt".into()],
            },
        )
        .unwrap();
        assert!(matches!(
            migration.rollback("rollback"),
            Err(LaputaError::MemoryMigrationConflict { .. })
        ));
        assert_eq!(fs::read_to_string(outside).unwrap(), "keep");
    }

    #[test]
    fn integrity_report_detects_duplicates_and_broken_links() {
        let mut record = adapt_legacy_markdown(
            Path::new("HISTORY.md"),
            "history",
            MemoryRecordKind::History,
            &context(),
            true,
        )
        .records
        .remove(0);
        record.supersedes.push("missing".into());
        let report =
            compare_normalized_records(b"history", &[record.clone(), record], Vec::new(), ts())
                .unwrap();
        assert_eq!(report.duplicate_record_ids.len(), 1);
        assert_eq!(report.broken_supersedes, vec!["missing"]);
        assert_eq!(report.findings.len(), 2);
    }

    #[test]
    fn migration_manifest_json_protocol_is_stable() {
        let migration = MemoryRecordMigration::new("artifacts");
        let plan = migration
            .dry_run("gmh-21", b"input", Vec::new(), ts())
            .unwrap();
        let value = serde_json::to_value(&plan.manifest).unwrap();
        assert_eq!(value["schema_version"], "1.0.0");
        assert_eq!(value["migration_id"], "gmh-21");
        assert_eq!(value["input_digest"]["algorithm"], "sha256");
        assert_eq!(
            value["outputs"],
            json!(["records.json", "rollback.json", "manifest.json"])
        );
        assert_eq!(
            serde_json::from_value::<MemoryMigrationManifest>(value).unwrap(),
            plan.manifest
        );
    }
}

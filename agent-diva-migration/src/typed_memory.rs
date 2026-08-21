use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use agent_diva_core::{
    evolution::LaputaSectionName,
    governance::AuditCorrelation,
    memory::{memory_content_digest, MemoryRecord, MemoryRecordKind},
};
use agent_diva_laputa::{
    adapt_laputa_section, adapt_legacy_markdown, atomic_write_json, LaputaSection,
    MemoryAdapterContext, MemoryRecordMigration, MemoryStoreIntegrity, MemoryStoreMetadata,
    TypedMemoryStore,
};
use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct MemoryImportRequest {
    pub source_root: PathBuf,
    pub sources: Vec<PathBuf>,
    pub workspace: PathBuf,
    pub tenant_id: String,
    pub workspace_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryImportReport {
    pub migration_id: String,
    pub manifest_path: PathBuf,
    pub backup_path: PathBuf,
    pub source_count: usize,
    pub record_count: usize,
    pub rejected_count: usize,
    pub conflict_count: usize,
    pub input_digest: String,
    pub records_digest: String,
    pub expected_store_revision: i64,
    pub resulting_store_revision: i64,
    pub integrity: Option<MemoryStoreIntegrity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppliedMigrationManifest {
    schema_version: String,
    migration_id: String,
    workspace_id: String,
    input_digest: String,
    records_digest: String,
    backup_path: PathBuf,
    store_revision_before: i64,
    store_revision_after: i64,
    #[serde(default)]
    rolled_back: bool,
}

pub async fn dry_run(request: &MemoryImportRequest) -> Result<MemoryImportReport> {
    let prepared = prepare(request)?;
    let database = request.workspace.join(".laputa").join("memory.sqlite3");
    let metadata = if database.is_file() {
        TypedMemoryStore::open_existing(&request.workspace, &request.workspace_id)
            .await?
            .metadata()
            .await?
    } else {
        MemoryStoreMetadata {
            schema_version: 1,
            store_revision: 0,
            workspace_id: request.workspace_id.clone(),
        }
    };
    Ok(prepared.report(
        metadata.store_revision,
        metadata.store_revision + prepared.records.len() as i64,
        None,
    ))
}

pub async fn apply(request: &MemoryImportRequest) -> Result<MemoryImportReport> {
    let prepared = prepare(request)?;
    let store = TypedMemoryStore::open(&request.workspace, &request.workspace_id).await?;
    let before = store.metadata().await?;
    let migration_dir = request
        .workspace
        .join(".laputa")
        .join("migrations")
        .join(&prepared.migration_id);
    let manifest_path = migration_dir.join("applied-manifest.json");
    let mut reuse_existing_backup = false;
    if manifest_path.exists() {
        let existing: AppliedMigrationManifest =
            serde_json::from_slice(&fs::read(&manifest_path)?)?;
        if existing.input_digest != prepared.input_digest
            || existing.records_digest != prepared.records_digest
            || existing.workspace_id != request.workspace_id
        {
            bail!("migration id conflicts with an existing manifest");
        }
        if !existing.rolled_back {
            let integrity = store.integrity().await?;
            if integrity.store_revision != existing.store_revision_after {
                bail!("migration manifest and typed store revision disagree");
            }
            for record in &prepared.records {
                if store.get(&record.id).await?.is_none() {
                    bail!("migration manifest references a missing typed record");
                }
            }
            return Ok(prepared.report(
                existing.store_revision_before,
                existing.store_revision_after,
                Some(integrity),
            ));
        }
        if before.store_revision != existing.store_revision_before {
            bail!("rolled-back migration no longer matches the typed store revision");
        }
        reuse_existing_backup = true;
    }

    fs::create_dir_all(&migration_dir)?;
    let artifacts = MemoryRecordMigration::new(request.workspace.join(".laputa/migrations"));
    let artifact_plan = artifacts.dry_run(
        prepared.migration_id.clone(),
        &prepared.input_bytes,
        prepared.records.clone(),
        prepared.captured_at,
    )?;
    artifacts.execute(&artifact_plan)?;

    let backup_path = migration_dir.join("memory-before.sqlite3");
    if !reuse_existing_backup {
        store.backup(&backup_path).await?;
    }
    let after = match store
        .import_records(prepared.records.clone(), before.store_revision)
        .await
    {
        Ok(metadata) => metadata,
        Err(error) => {
            let _ = store.restore(&backup_path).await;
            return Err(error.into());
        }
    };
    let integrity = store.integrity().await?;
    if !integrity.corrupt_record_ids.is_empty()
        || integrity.orphan_fts_rows != 0
        || integrity.record_count != integrity.fts_row_count + integrity.tombstone_count
    {
        let _ = store.restore(&backup_path).await;
        bail!("post-import integrity validation failed");
    }
    atomic_write_json(
        &manifest_path,
        &AppliedMigrationManifest {
            schema_version: "1.0.0".into(),
            migration_id: prepared.migration_id.clone(),
            workspace_id: request.workspace_id.clone(),
            input_digest: prepared.input_digest.clone(),
            records_digest: prepared.records_digest.clone(),
            backup_path,
            store_revision_before: before.store_revision,
            store_revision_after: after.store_revision,
            rolled_back: false,
        },
    )?;
    Ok(prepared.report(before.store_revision, after.store_revision, Some(integrity)))
}

pub async fn rollback(
    workspace: &Path,
    workspace_id: &str,
    migration_id: &str,
) -> Result<MemoryImportReport> {
    validate_identifier(migration_id)?;
    let migration_dir = workspace
        .join(".laputa")
        .join("migrations")
        .join(migration_id);
    let manifest_path = migration_dir.join("applied-manifest.json");
    let mut manifest: AppliedMigrationManifest =
        serde_json::from_slice(&fs::read(&manifest_path).context("migration manifest not found")?)?;
    if manifest.migration_id != migration_id || manifest.workspace_id != workspace_id {
        bail!("migration manifest identity mismatch");
    }
    let store = TypedMemoryStore::open(workspace, workspace_id).await?;
    let restored = store.restore(&manifest.backup_path).await?;
    let integrity = restored.integrity().await?;
    manifest.rolled_back = true;
    atomic_write_json(&manifest_path, &manifest)?;
    Ok(MemoryImportReport {
        migration_id: migration_id.into(),
        manifest_path,
        backup_path: manifest.backup_path,
        source_count: 0,
        record_count: integrity.record_count as usize,
        rejected_count: 0,
        conflict_count: 0,
        input_digest: manifest.input_digest,
        records_digest: manifest.records_digest,
        expected_store_revision: manifest.store_revision_after,
        resulting_store_revision: integrity.store_revision,
        integrity: Some(integrity),
    })
}

struct PreparedImport {
    migration_id: String,
    input_bytes: Vec<u8>,
    input_digest: String,
    records_digest: String,
    records: Vec<MemoryRecord>,
    captured_at: chrono::DateTime<Utc>,
    source_count: usize,
    workspace: PathBuf,
}

impl PreparedImport {
    fn report(
        &self,
        expected_store_revision: i64,
        resulting_store_revision: i64,
        integrity: Option<MemoryStoreIntegrity>,
    ) -> MemoryImportReport {
        MemoryImportReport {
            migration_id: self.migration_id.clone(),
            manifest_path: self
                .workspace
                .join(".laputa/migrations")
                .join(&self.migration_id)
                .join("applied-manifest.json"),
            backup_path: self
                .workspace
                .join(".laputa/migrations")
                .join(&self.migration_id)
                .join("memory-before.sqlite3"),
            source_count: self.source_count,
            record_count: self.records.len(),
            rejected_count: 0,
            conflict_count: 0,
            input_digest: self.input_digest.clone(),
            records_digest: self.records_digest.clone(),
            expected_store_revision,
            resulting_store_revision,
            integrity,
        }
    }
}

fn prepare(request: &MemoryImportRequest) -> Result<PreparedImport> {
    let source_root = request
        .source_root
        .canonicalize()
        .context("source root does not exist")?;
    let canonical_sources = request
        .sources
        .iter()
        .map(|source| validate_source(&source_root, source))
        .collect::<Result<Vec<_>>>()?;
    let captured_at = canonical_sources
        .iter()
        .filter_map(|source| fs::metadata(source).ok()?.modified().ok())
        .map(chrono::DateTime::<Utc>::from)
        .max()
        .unwrap_or_else(|| chrono::DateTime::<Utc>::from(std::time::UNIX_EPOCH));
    let mut input_bytes = Vec::new();
    let mut records = Vec::new();
    let context = MemoryAdapterContext {
        tenant_id: request.tenant_id.clone(),
        workspace_id: request.workspace_id.clone(),
        session_id: None,
        correlation: AuditCorrelation {
            request_id: "offline-memory-import".into(),
            turn_id: "offline".into(),
            session_id: "offline".into(),
            trace_id: None,
        },
        captured_at,
    };
    for canonical in canonical_sources {
        let bytes = fs::read(&canonical)?;
        input_bytes.extend_from_slice(canonical.to_string_lossy().as_bytes());
        input_bytes.extend_from_slice(&bytes);
        match canonical.file_name().and_then(|name| name.to_str()) {
            Some("MEMORY.md") => {
                let content = String::from_utf8(bytes).context("MEMORY.md is not UTF-8")?;
                records.extend(
                    adapt_legacy_markdown(
                        &canonical,
                        &content,
                        MemoryRecordKind::LongTerm,
                        &context,
                        true,
                    )
                    .records,
                );
            }
            Some("HISTORY.md") => {
                let content = String::from_utf8(bytes).context("HISTORY.md is not UTF-8")?;
                records.extend(
                    adapt_legacy_markdown(
                        &canonical,
                        &content,
                        MemoryRecordKind::History,
                        &context,
                        true,
                    )
                    .records,
                );
            }
            Some(name) if name.ends_with(".json") => {
                let raw: serde_json::Value = serde_json::from_slice(&bytes)
                    .with_context(|| format!("invalid Laputa JSON: {}", canonical.display()))?;
                let section: LaputaSection = serde_json::from_value(raw.clone())
                    .context("unsupported Laputa section JSON")?;
                if matches!(section.name, LaputaSectionName::Changelog) {
                    bail!("non-authority Laputa section is not importable");
                }
                records.extend(adapt_laputa_section(&section, &raw, &context)?.records);
            }
            _ => bail!("unsupported Memory import source"),
        }
    }
    records.sort_by_key(|record| record.id.clone());
    let input_digest = memory_content_digest(&input_bytes).value;
    let records_digest = memory_content_digest(&serde_json::to_vec(&records)?).value;
    let migration_id = format!("gmh24-{}", &input_digest[..16.min(input_digest.len())]);
    Ok(PreparedImport {
        migration_id,
        input_bytes,
        input_digest,
        records_digest,
        records,
        captured_at,
        source_count: request.sources.len(),
        workspace: request.workspace.clone(),
    })
}

fn validate_source(root: &Path, source: &Path) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!("Memory import source must be a regular non-symlink file");
    }
    let canonical = source.canonicalize()?;
    if !canonical.starts_with(root) {
        bail!("Memory import source escapes source root");
    }
    if canonical.components().any(|component| {
        matches!(component, Component::Normal(name) if name.to_string_lossy().eq_ignore_ascii_case(".mentle"))
    }) {
        bail!("Mentle paths are forbidden import sources");
    }
    Ok(canonical)
}

fn validate_identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.contains('/')
        || value.contains('\\')
        || value == "."
        || value == ".."
    {
        return Err(anyhow!("invalid migration id"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(root: &Path, source: PathBuf) -> MemoryImportRequest {
        MemoryImportRequest {
            source_root: root.to_path_buf(),
            sources: vec![source],
            workspace: root.join("workspace"),
            tenant_id: "local".into(),
            workspace_id: "workspace-test".into(),
        }
    }

    #[tokio::test]
    async fn dry_run_is_deterministic_and_does_not_create_laputa() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("MEMORY.md");
        fs::write(&source, "remember this").unwrap();
        let request = request(temp.path(), source);

        let first = dry_run(&request).await.unwrap();
        let second = dry_run(&request).await.unwrap();

        assert_eq!(first.migration_id, second.migration_id);
        assert_eq!(first.records_digest, second.records_digest);
        assert!(!request.workspace.join(".laputa").exists());
    }

    #[tokio::test]
    async fn apply_replays_and_rollback_restores_pre_import_store() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("HISTORY.md");
        fs::write(&source, "history entry").unwrap();
        let request = request(temp.path(), source);

        let applied = apply(&request).await.unwrap();
        assert_eq!(applied.expected_store_revision, 0);
        assert_eq!(applied.resulting_store_revision, 1);
        assert_eq!(
            applied.integrity.as_ref().unwrap().store_revision,
            applied.resulting_store_revision
        );
        let replayed = apply(&request).await.unwrap();
        assert_eq!(applied.migration_id, replayed.migration_id);
        assert_eq!(replayed.expected_store_revision, 0);
        assert_eq!(replayed.resulting_store_revision, 1);
        assert_eq!(replayed.integrity.as_ref().unwrap().record_count, 1);

        let rolled_back = rollback(
            &request.workspace,
            &request.workspace_id,
            &applied.migration_id,
        )
        .await
        .unwrap();
        assert_eq!(rolled_back.integrity.unwrap().record_count, 0);

        let reapplied = apply(&request).await.unwrap();
        assert_eq!(reapplied.expected_store_revision, 0);
        assert_eq!(reapplied.resulting_store_revision, 1);
        assert_eq!(reapplied.integrity.unwrap().record_count, 1);
    }

    #[tokio::test]
    async fn mentle_path_and_unknown_format_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let forbidden = temp.path().join(".mentle");
        fs::create_dir_all(&forbidden).unwrap();
        let source = forbidden.join("MEMORY.md");
        fs::write(&source, "forbidden").unwrap();
        assert!(dry_run(&request(temp.path(), source)).await.is_err());

        let unknown = temp.path().join("memory.txt");
        fs::write(&unknown, "unknown").unwrap();
        assert!(dry_run(&request(temp.path(), unknown)).await.is_err());
    }
}

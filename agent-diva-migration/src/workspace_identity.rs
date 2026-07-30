use std::path::{Path, PathBuf};

use agent_diva_laputa::{
    MemoryStoreIntegrity, TypedMemoryStore, TypedMemoryStoreError, WorkspaceIdentityMigrationState,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct WorkspaceIdentityReport {
    pub operation: &'static str,
    pub state: &'static str,
    pub legacy_workspace_id: String,
    pub canonical_workspace_id: String,
    pub manifest_path: PathBuf,
    pub backup_path: PathBuf,
    pub integrity: Option<MemoryStoreIntegrity>,
}

pub async fn dry_run(workspace: &Path) -> Result<WorkspaceIdentityReport, TypedMemoryStoreError> {
    let legacy = agent_diva_core::workspace_identity::legacy_path_workspace_id(workspace);
    let canonical = agent_diva_core::workspace_identity::canonical_workspace_id(workspace);
    let (state, integrity) =
        match TypedMemoryStore::open_existing(workspace, canonical.clone()).await {
            Ok(store) => ("canonical", Some(store.integrity().await?)),
            Err(TypedMemoryStoreError::DatabaseWorkspaceMismatch { actual, .. })
                if actual == legacy =>
            {
                let store = TypedMemoryStore::open_existing(workspace, legacy.clone()).await?;
                ("migration_required", Some(store.integrity().await?))
            }
            Err(TypedMemoryStoreError::InvalidBackup) => ("store_missing", None),
            Err(error) => return Err(error),
        };
    Ok(report(
        "dry-run", state, workspace, legacy, canonical, integrity,
    ))
}

pub async fn apply(workspace: &Path) -> Result<WorkspaceIdentityReport, TypedMemoryStoreError> {
    let legacy = agent_diva_core::workspace_identity::legacy_path_workspace_id(workspace);
    let canonical = agent_diva_core::workspace_identity::canonical_workspace_id(workspace);
    let store = TypedMemoryStore::open_canonical(workspace).await?;
    let integrity = store.integrity().await?;
    Ok(report(
        "apply",
        "canonical",
        workspace,
        legacy,
        canonical,
        Some(integrity),
    ))
}

pub async fn rollback(workspace: &Path) -> Result<WorkspaceIdentityReport, TypedMemoryStoreError> {
    let manifest = TypedMemoryStore::rollback_canonical_identity(workspace).await?;
    let store =
        TypedMemoryStore::open_existing(workspace, manifest.legacy_workspace_id.clone()).await?;
    let integrity = store.integrity().await?;
    let state = match manifest.state {
        WorkspaceIdentityMigrationState::RolledBack => "rolled_back",
        _ => "unknown",
    };
    Ok(report(
        "rollback",
        state,
        workspace,
        manifest.legacy_workspace_id,
        manifest.canonical_workspace_id,
        Some(integrity),
    ))
}

fn report(
    operation: &'static str,
    state: &'static str,
    workspace: &Path,
    legacy_workspace_id: String,
    canonical_workspace_id: String,
    integrity: Option<MemoryStoreIntegrity>,
) -> WorkspaceIdentityReport {
    let migration_dir = workspace
        .join(".laputa")
        .join("migrations")
        .join("workspace-identity-v1");
    WorkspaceIdentityReport {
        operation,
        state,
        legacy_workspace_id,
        canonical_workspace_id,
        manifest_path: migration_dir.join("manifest.json"),
        backup_path: migration_dir.join("memory-before.sqlite3"),
        integrity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn identity_dry_run_apply_and_rollback_are_explicit_and_reversible() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = agent_diva_core::workspace_identity::legacy_path_workspace_id(temp.path());
        let store = TypedMemoryStore::open(temp.path(), &legacy).await.unwrap();
        drop(store);

        let preview = dry_run(temp.path()).await.unwrap();
        assert_eq!(preview.operation, "dry-run");
        assert_eq!(preview.state, "migration_required");
        assert!(!preview.manifest_path.exists());

        let applied = apply(temp.path()).await.unwrap();
        assert_eq!(applied.state, "canonical");
        assert!(applied.manifest_path.is_file());
        assert!(applied.backup_path.is_file());

        let restored = rollback(temp.path()).await.unwrap();
        assert_eq!(restored.state, "rolled_back");
        assert_eq!(
            TypedMemoryStore::open_existing(temp.path(), legacy)
                .await
                .unwrap()
                .metadata()
                .await
                .unwrap()
                .workspace_id,
            restored.legacy_workspace_id
        );
    }
}

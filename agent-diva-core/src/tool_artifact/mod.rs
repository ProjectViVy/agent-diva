//! Session-scoped persistent storage for sanitized tool-result artifacts.

use crate::security::{redact_pii, sanitize_tool_output, PiiConfig};
use crate::workspace_identity::canonical_workspace_id;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

pub const TOOL_ARTIFACT_VERSION: u32 = 1;
pub const MAX_ARTIFACT_BYTES: u64 = 10 * 1024 * 1024;
pub const MAX_SESSION_BYTES: u64 = 100 * 1024 * 1024;
pub const MAX_WORKSPACE_BYTES: u64 = 1024 * 1024 * 1024;
pub const MAX_READ_CHARS: usize = 12_000;
pub const ARTIFACT_TTL_DAYS: i64 = 30;
pub const INLINE_THRESHOLD_CHARS: usize = 12_000;
pub const PREVIEW_HEAD_CHARS: usize = 2_000;
pub const PREVIEW_TAIL_CHARS: usize = 1_000;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ToolResultRef {
    pub version: u32,
    pub artifact_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    pub status: String,
    pub char_count: usize,
    pub byte_count: u64,
    pub sha256: String,
    pub preview: String,
    pub truncated: bool,
    pub read_hint: String,
}

impl ToolResultRef {
    pub fn from_metadata(metadata: &ToolArtifactMetadata, content: &str) -> Self {
        Self {
            version: TOOL_ARTIFACT_VERSION,
            artifact_id: metadata.artifact_id.clone(),
            tool_call_id: metadata.tool_call_id.clone(),
            tool_name: metadata.tool_name.clone(),
            status: metadata.status.clone(),
            char_count: metadata.char_count,
            byte_count: metadata.byte_count,
            sha256: metadata.sha256.clone(),
            preview: tool_result_preview(content),
            truncated: true,
            read_hint: "Use read_tool_result with artifact_id and an optional [start,end) character range (maximum 12000 characters).".to_string(),
        }
    }

    pub fn render(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolArtifactSecurityContext {
    workspace_id: String,
    session_id: String,
    session_digest: String,
}

impl ToolArtifactSecurityContext {
    pub fn new(workspace: impl AsRef<Path>, session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        let session_digest = hex_digest(session_id.as_bytes());
        Self {
            workspace_id: canonical_workspace_id(workspace),
            session_id,
            session_digest,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ToolArtifactMetadata {
    pub version: u32,
    pub artifact_id: String,
    pub workspace_id: String,
    pub session_id: String,
    pub tool_name: String,
    pub tool_call_id: String,
    pub status: String,
    pub char_count: usize,
    pub byte_count: u64,
    pub sha256: String,
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolArtifactRead {
    pub content: String,
    pub start: usize,
    pub end: usize,
    pub total_chars: usize,
    pub sha256: String,
}

#[derive(Debug, Error)]
pub enum ToolArtifactError {
    #[error("artifact_missing")]
    Missing,
    #[error("artifact_expired")]
    Expired,
    #[error("artifact_forbidden")]
    Forbidden,
    #[error("artifact_corrupt")]
    Corrupt,
    #[error("artifact_capacity_exceeded")]
    CapacityExceeded,
    #[error("artifact_range_invalid")]
    RangeInvalid,
    #[error("artifact_io")]
    Io(#[source] io::Error),
}

impl ToolArtifactError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Missing => "artifact_missing",
            Self::Expired => "artifact_expired",
            Self::Forbidden => "artifact_forbidden",
            Self::Corrupt => "artifact_corrupt",
            Self::CapacityExceeded => "artifact_capacity_exceeded",
            Self::RangeInvalid => "artifact_range_invalid",
            Self::Io(_) => "artifact_io",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ToolArtifactStore {
    root: PathBuf,
}

impl ToolArtifactStore {
    pub fn new(workspace: impl AsRef<Path>) -> Self {
        Self {
            root: workspace
                .as_ref()
                .join(".agent-diva")
                .join("tool-artifacts"),
        }
    }

    pub fn put(
        &self,
        context: &ToolArtifactSecurityContext,
        tool_name: &str,
        tool_call_id: &str,
        status: &str,
        content: &str,
    ) -> Result<ToolArtifactMetadata, ToolArtifactError> {
        self.gc(context)?;
        let sanitized = sanitize_for_artifact(content);
        let byte_count = u64::try_from(sanitized.len()).unwrap_or(u64::MAX);
        if byte_count > MAX_ARTIFACT_BYTES {
            return Err(ToolArtifactError::CapacityExceeded);
        }
        self.ensure_capacity(context, byte_count)?;

        let artifact_id = format!("ta_v1_{}", Uuid::new_v4().simple());
        let now = Utc::now();
        let metadata = ToolArtifactMetadata {
            version: TOOL_ARTIFACT_VERSION,
            artifact_id: artifact_id.clone(),
            workspace_id: context.workspace_id.clone(),
            session_id: context.session_id.clone(),
            tool_name: tool_name.to_string(),
            tool_call_id: tool_call_id.to_string(),
            status: status.to_string(),
            char_count: sanitized.chars().count(),
            byte_count,
            sha256: hex_digest(sanitized.as_bytes()),
            created_at: now,
            last_accessed_at: now,
        };
        let directory = self.session_dir(context);
        fs::create_dir_all(&directory).map_err(ToolArtifactError::Io)?;
        atomic_write(
            &self.content_path(&directory, &artifact_id),
            sanitized.as_bytes(),
        )?;
        let encoded =
            serde_json::to_vec_pretty(&metadata).map_err(|_| ToolArtifactError::Corrupt)?;
        if let Err(error) = atomic_write(&self.metadata_path(&directory, &artifact_id), &encoded) {
            let _ = fs::remove_file(self.content_path(&directory, &artifact_id));
            return Err(error);
        }
        Ok(metadata)
    }

    pub fn read_range(
        &self,
        context: &ToolArtifactSecurityContext,
        artifact_id: &str,
        start: Option<usize>,
        end: Option<usize>,
    ) -> Result<ToolArtifactRead, ToolArtifactError> {
        validate_artifact_id(artifact_id)?;
        let directory = self.session_dir(context);
        let metadata_path = self.metadata_path(&directory, artifact_id);
        let mut metadata = match read_metadata(&metadata_path) {
            Err(ToolArtifactError::Missing)
                if artifact_exists_elsewhere(&self.workspace_dir(context), artifact_id)? =>
            {
                return Err(ToolArtifactError::Forbidden);
            }
            result => result?,
        };
        if metadata.workspace_id != context.workspace_id
            || metadata.session_id != context.session_id
        {
            return Err(ToolArtifactError::Forbidden);
        }
        if Utc::now().signed_duration_since(metadata.last_accessed_at)
            > Duration::days(ARTIFACT_TTL_DAYS)
        {
            return Err(ToolArtifactError::Expired);
        }
        let bytes = fs::read(self.content_path(&directory, artifact_id)).map_err(map_read_error)?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != metadata.byte_count
            || hex_digest(&bytes) != metadata.sha256
        {
            return Err(ToolArtifactError::Corrupt);
        }
        let content = String::from_utf8(bytes).map_err(|_| ToolArtifactError::Corrupt)?;
        let total_chars = content.chars().count();
        let start = start.unwrap_or(0);
        let end = end.unwrap_or_else(|| start.saturating_add(MAX_READ_CHARS).min(total_chars));
        if start > end || end > total_chars || end.saturating_sub(start) > MAX_READ_CHARS {
            return Err(ToolArtifactError::RangeInvalid);
        }
        let selected = content.chars().skip(start).take(end - start).collect();
        metadata.last_accessed_at = Utc::now();
        let encoded =
            serde_json::to_vec_pretty(&metadata).map_err(|_| ToolArtifactError::Corrupt)?;
        atomic_write(&metadata_path, &encoded)?;
        Ok(ToolArtifactRead {
            content: selected,
            start,
            end,
            total_chars,
            sha256: metadata.sha256,
        })
    }

    pub fn delete_session(
        &self,
        context: &ToolArtifactSecurityContext,
    ) -> Result<(), ToolArtifactError> {
        let path = self.session_dir(context);
        if path.exists() {
            fs::remove_dir_all(path).map_err(ToolArtifactError::Io)?;
        }
        Ok(())
    }

    pub fn gc(&self, context: &ToolArtifactSecurityContext) -> Result<usize, ToolArtifactError> {
        let workspace_dir = self.workspace_dir(context);
        let mut removed = 0;
        for entry in metadata_entries(&workspace_dir)? {
            match read_metadata(&entry) {
                Ok(metadata)
                    if Utc::now().signed_duration_since(metadata.last_accessed_at)
                        > Duration::days(ARTIFACT_TTL_DAYS) =>
                {
                    remove_artifact_files(&entry, &metadata.artifact_id)?;
                    removed += 1;
                }
                Ok(_) => {}
                Err(ToolArtifactError::Corrupt) => {
                    if let Some(id) = artifact_id_from_metadata_path(&entry) {
                        remove_artifact_files(&entry, &id)?;
                        removed += 1;
                    }
                }
                Err(error) => return Err(error),
            }
        }
        Ok(removed)
    }

    fn ensure_capacity(
        &self,
        context: &ToolArtifactSecurityContext,
        incoming: u64,
    ) -> Result<(), ToolArtifactError> {
        let workspace_dir = self.workspace_dir(context);
        let mut entries = load_entries(&workspace_dir)?;
        entries.sort_by(|left, right| {
            left.1
                .last_accessed_at
                .cmp(&right.1.last_accessed_at)
                .then_with(|| left.1.artifact_id.cmp(&right.1.artifact_id))
        });
        let mut workspace_size = entries.iter().map(|(_, item)| item.byte_count).sum::<u64>();
        let mut session_size = entries
            .iter()
            .filter(|(_, item)| item.session_id == context.session_id)
            .map(|(_, item)| item.byte_count)
            .sum::<u64>();
        for (path, item) in entries {
            let session_over = session_size.saturating_add(incoming) > MAX_SESSION_BYTES;
            let workspace_over = workspace_size.saturating_add(incoming) > MAX_WORKSPACE_BYTES;
            if !session_over && !workspace_over {
                break;
            }
            if session_over && item.session_id != context.session_id && !workspace_over {
                continue;
            }
            remove_artifact_files(&path, &item.artifact_id)?;
            workspace_size = workspace_size.saturating_sub(item.byte_count);
            if item.session_id == context.session_id {
                session_size = session_size.saturating_sub(item.byte_count);
            }
        }
        if session_size.saturating_add(incoming) > MAX_SESSION_BYTES
            || workspace_size.saturating_add(incoming) > MAX_WORKSPACE_BYTES
        {
            return Err(ToolArtifactError::CapacityExceeded);
        }
        Ok(())
    }

    fn workspace_dir(&self, context: &ToolArtifactSecurityContext) -> PathBuf {
        self.root.join(&context.workspace_id)
    }

    fn session_dir(&self, context: &ToolArtifactSecurityContext) -> PathBuf {
        self.workspace_dir(context).join(&context.session_digest)
    }

    fn content_path(&self, directory: &Path, artifact_id: &str) -> PathBuf {
        directory.join(format!("{artifact_id}.content"))
    }

    fn metadata_path(&self, directory: &Path, artifact_id: &str) -> PathBuf {
        directory.join(format!("{artifact_id}.json"))
    }
}

fn sanitize_for_artifact(content: &str) -> String {
    let injection_safe = sanitize_tool_output(content);
    redact_pii(&injection_safe, &PiiConfig::default()).redacted
}

pub fn tool_result_preview(content: &str) -> String {
    let total = content.chars().count();
    if total <= PREVIEW_HEAD_CHARS + PREVIEW_TAIL_CHARS {
        return content.to_string();
    }
    let head: String = content.chars().take(PREVIEW_HEAD_CHARS).collect();
    let tail: String = content.chars().skip(total - PREVIEW_TAIL_CHARS).collect();
    format!(
        "{head}\n... [artifact preview omitted {} characters] ...\n{tail}",
        total - PREVIEW_HEAD_CHARS - PREVIEW_TAIL_CHARS
    )
}

fn validate_artifact_id(id: &str) -> Result<(), ToolArtifactError> {
    let Some(uuid) = id.strip_prefix("ta_v1_") else {
        return Err(ToolArtifactError::Forbidden);
    };
    if uuid.len() != 32 || !uuid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ToolArtifactError::Forbidden);
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ToolArtifactError> {
    let parent = path.parent().ok_or(ToolArtifactError::Corrupt)?;
    fs::create_dir_all(parent).map_err(ToolArtifactError::Io)?;
    let temporary = parent.join(format!(".tmp-{}", Uuid::new_v4().simple()));
    let result = (|| {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        Ok::<_, io::Error>(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(ToolArtifactError::Io(error));
    }
    Ok(())
}

fn read_metadata(path: &Path) -> Result<ToolArtifactMetadata, ToolArtifactError> {
    let bytes = fs::read(path).map_err(map_read_error)?;
    let metadata: ToolArtifactMetadata =
        serde_json::from_slice(&bytes).map_err(|_| ToolArtifactError::Corrupt)?;
    if metadata.version != TOOL_ARTIFACT_VERSION {
        return Err(ToolArtifactError::Corrupt);
    }
    validate_artifact_id(&metadata.artifact_id).map_err(|_| ToolArtifactError::Corrupt)?;
    Ok(metadata)
}

fn map_read_error(error: io::Error) -> ToolArtifactError {
    if error.kind() == io::ErrorKind::NotFound {
        ToolArtifactError::Missing
    } else {
        ToolArtifactError::Io(error)
    }
}

fn metadata_entries(root: &Path) -> Result<Vec<PathBuf>, ToolArtifactError> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for session in fs::read_dir(root).map_err(ToolArtifactError::Io)? {
        let session = session.map_err(ToolArtifactError::Io)?;
        if !session.file_type().map_err(ToolArtifactError::Io)?.is_dir() {
            continue;
        }
        for item in fs::read_dir(session.path()).map_err(ToolArtifactError::Io)? {
            let path = item.map_err(ToolArtifactError::Io)?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                result.push(path);
            }
        }
    }
    result.sort();
    Ok(result)
}

fn artifact_exists_elsewhere(root: &Path, artifact_id: &str) -> Result<bool, ToolArtifactError> {
    Ok(metadata_entries(root)?
        .iter()
        .any(|path| artifact_id_from_metadata_path(path).as_deref() == Some(artifact_id)))
}

fn load_entries(root: &Path) -> Result<Vec<(PathBuf, ToolArtifactMetadata)>, ToolArtifactError> {
    metadata_entries(root)?
        .into_iter()
        .map(|path| read_metadata(&path).map(|metadata| (path, metadata)))
        .collect()
}

fn artifact_id_from_metadata_path(path: &Path) -> Option<String> {
    path.file_stem()?.to_str().map(str::to_string)
}

fn remove_artifact_files(metadata_path: &Path, artifact_id: &str) -> Result<(), ToolArtifactError> {
    let directory = metadata_path.parent().ok_or(ToolArtifactError::Corrupt)?;
    for path in [
        metadata_path.to_path_buf(),
        directory.join(format!("{artifact_id}.content")),
    ] {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(ToolArtifactError::Io(error)),
        }
    }
    Ok(())
}

fn hex_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_survives_store_recreation_and_redacts_secrets() {
        let workspace = tempfile::tempdir().unwrap();
        let context = ToolArtifactSecurityContext::new(workspace.path(), "session-a");
        let metadata = ToolArtifactStore::new(workspace.path())
            .put(
                &context,
                "exec",
                "call-1",
                "ok",
                "key=sk-abcdefghijklmnopqrstuvwx",
            )
            .unwrap();
        let read = ToolArtifactStore::new(workspace.path())
            .read_range(&context, &metadata.artifact_id, None, None)
            .unwrap();
        assert!(read.content.contains("[REDACTED:ApiKey]"));
        assert!(!read.content.contains("sk-abcdefghijklmnopqrstuvwx"));
    }

    #[test]
    fn session_and_workspace_binding_fail_closed() {
        let workspace = tempfile::tempdir().unwrap();
        let context = ToolArtifactSecurityContext::new(workspace.path(), "session-a");
        let store = ToolArtifactStore::new(workspace.path());
        let metadata = store.put(&context, "exec", "call-1", "ok", "safe").unwrap();
        let other_session = ToolArtifactSecurityContext::new(workspace.path(), "session-b");
        assert_eq!(
            store
                .read_range(&other_session, &metadata.artifact_id, None, None)
                .unwrap_err()
                .code(),
            "artifact_forbidden"
        );
        assert_eq!(
            store
                .read_range(&context, "../secret", None, None)
                .unwrap_err()
                .code(),
            "artifact_forbidden"
        );
    }

    #[test]
    fn range_is_unicode_safe_and_bounded() {
        let workspace = tempfile::tempdir().unwrap();
        let context = ToolArtifactSecurityContext::new(workspace.path(), "session-a");
        let store = ToolArtifactStore::new(workspace.path());
        let metadata = store
            .put(&context, "read", "call-1", "ok", "甲乙丙丁")
            .unwrap();
        let read = store
            .read_range(&context, &metadata.artifact_id, Some(1), Some(3))
            .unwrap();
        assert_eq!(read.content, "乙丙");
        assert_eq!(
            store
                .read_range(&context, &metadata.artifact_id, Some(3), Some(2))
                .unwrap_err()
                .code(),
            "artifact_range_invalid"
        );
    }

    #[test]
    fn corruption_is_detected_by_hash() {
        let workspace = tempfile::tempdir().unwrap();
        let context = ToolArtifactSecurityContext::new(workspace.path(), "session-a");
        let store = ToolArtifactStore::new(workspace.path());
        let metadata = store.put(&context, "read", "call-1", "ok", "safe").unwrap();
        fs::write(
            store.content_path(&store.session_dir(&context), &metadata.artifact_id),
            "changed",
        )
        .unwrap();
        assert_eq!(
            store
                .read_range(&context, &metadata.artifact_id, None, None)
                .unwrap_err()
                .code(),
            "artifact_corrupt"
        );
    }

    #[test]
    fn oversized_item_is_rejected_without_partial_files() {
        let workspace = tempfile::tempdir().unwrap();
        let context = ToolArtifactSecurityContext::new(workspace.path(), "session-a");
        let store = ToolArtifactStore::new(workspace.path());
        let error = store
            .put(
                &context,
                "exec",
                "call-1",
                "ok",
                &"x".repeat(MAX_ARTIFACT_BYTES as usize + 1),
            )
            .unwrap_err();
        assert_eq!(error.code(), "artifact_capacity_exceeded");
        assert!(metadata_entries(&store.workspace_dir(&context))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn delete_session_removes_only_bound_session() {
        let workspace = tempfile::tempdir().unwrap();
        let store = ToolArtifactStore::new(workspace.path());
        let first = ToolArtifactSecurityContext::new(workspace.path(), "first");
        let second = ToolArtifactSecurityContext::new(workspace.path(), "second");
        let first_meta = store.put(&first, "read", "a", "ok", "first").unwrap();
        let second_meta = store.put(&second, "read", "b", "ok", "second").unwrap();
        store.delete_session(&first).unwrap();
        assert_eq!(
            store
                .read_range(&first, &first_meta.artifact_id, None, None)
                .unwrap_err()
                .code(),
            "artifact_missing"
        );
        assert_eq!(
            store
                .read_range(&second, &second_meta.artifact_id, None, None)
                .unwrap()
                .content,
            "second"
        );
    }
}

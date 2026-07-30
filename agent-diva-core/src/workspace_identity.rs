//! Canonical, payload-free workspace identity shared by all persistence seams.

use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};

/// Stable workspace identity derived from a normalized absolute path.
///
/// The path itself is never persisted as the identity. Windows identities are
/// case-insensitive and separator-insensitive; Unix identities preserve case.
pub fn canonical_workspace_id(path: impl AsRef<Path>) -> String {
    // Preserve the identity algorithm historically used by
    // ExperienceJournal so existing evidence remains addressable. Filesystem
    // canonicalization already resolves equivalent Windows spelling for an
    // existing workspace; the fallback is only for paths not created yet.
    let normalized = path
        .as_ref()
        .canonicalize()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| normalized_workspace_path(path.as_ref()));
    let digest = Sha256::digest(normalized.as_bytes());
    format!("workspace-{:x}", digest)[..42].to_string()
}

/// Previous typed-store identity used before canonical workspace IDs.
///
/// This is exposed only so the migration layer can recognize and upgrade an
/// existing store. New records must always use [`canonical_workspace_id`].
pub fn legacy_path_workspace_id(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}

fn normalized_workspace_path(path: &Path) -> String {
    let absolute = path
        .canonicalize()
        .or_else(|_| absolute_without_io(path))
        .unwrap_or_else(|_| path.to_path_buf());
    let mut value = absolute.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = value.strip_prefix("//?/UNC/") {
        value = format!("//{stripped}");
    } else if let Some(stripped) = value.strip_prefix("//?/") {
        value = stripped.to_string();
    }
    while value.len() > 1 && value.ends_with('/') {
        value.pop();
    }
    if cfg!(windows) {
        value.make_ascii_lowercase();
    }
    value
}

fn absolute_without_io(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        return Ok(clean_components(path));
    }
    Ok(clean_components(&std::env::current_dir()?.join(path)))
}

fn clean_components(path: &Path) -> PathBuf {
    let mut cleaned = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                cleaned.pop();
            }
            other => cleaned.push(other.as_os_str()),
        }
    }
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equivalent_paths_share_one_identity() {
        let temp = tempfile::tempdir().unwrap();
        let direct = canonical_workspace_id(temp.path());
        let dotted = canonical_workspace_id(temp.path().join("."));
        assert_eq!(direct, dotted);
        assert!(direct.starts_with("workspace-"));
        assert_eq!(direct.len(), 42);
    }

    #[cfg(windows)]
    #[test]
    fn windows_identity_ignores_case_and_separator_spelling() {
        let temp = tempfile::tempdir().unwrap();
        let canonical = temp.path().canonicalize().unwrap();
        let lower = canonical.to_string_lossy().to_ascii_lowercase();
        let upper_with_slashes = canonical
            .to_string_lossy()
            .to_ascii_uppercase()
            .replace('\\', "/");
        assert_eq!(
            canonical_workspace_id(lower),
            canonical_workspace_id(upper_with_slashes)
        );
    }
}

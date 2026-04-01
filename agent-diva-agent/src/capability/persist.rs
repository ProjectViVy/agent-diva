//! Workspace-side capability manifest file — shared by GUI persist, gateway bootstrap, CLI doctor (Story 6.3).

use std::fs;
use std::path::{Path, PathBuf};

use super::{parse_and_validate_manifest_from_str, PlaceholderCapabilityRegistry};

/// Relative to workspace root.
pub const WORKSPACE_CAPABILITY_MANIFEST_REL: &str = ".agent-diva/capability-manifest.json";

#[must_use]
pub fn workspace_capability_manifest_path(workspace: &Path) -> PathBuf {
    workspace.join(WORKSPACE_CAPABILITY_MANIFEST_REL)
}

/// Writes validated JSON to the canonical workspace path (creates `.agent-diva` if needed).
pub fn persist_capability_manifest_json(workspace: &Path, json: &str) -> std::io::Result<()> {
    let path = workspace_capability_manifest_path(workspace);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, json)
}

/// Load manifest from disk into `registry` when the file exists. Ignores a missing file.
/// Parse/validation errors are returned (caller may log and continue startup).
pub fn load_capability_manifest_into_registry(
    workspace: &Path,
    registry: &PlaceholderCapabilityRegistry,
) -> Result<(), String> {
    let path = workspace_capability_manifest_path(workspace);
    if !path.exists() {
        return Ok(());
    }
    let s = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let manifest = parse_and_validate_manifest_from_str(s.trim())
        .map_err(|e| format_manifest_errors(&e))?;
    registry
        .replace_with_manifest(manifest)
        .map_err(|e| format_manifest_errors(&e))
}

#[must_use]
pub fn format_manifest_errors(
    e: &super::CapabilityManifestErrors,
) -> String {
    e.errors
        .iter()
        .map(|x| x.message.clone())
        .collect::<Vec<_>>()
        .join("; ")
}


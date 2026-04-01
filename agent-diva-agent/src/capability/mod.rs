//! Capability manifest **v0** validation (FR11) and placeholder registry.
//!
//! ## Relation to [`crate::skills::SkillsLoader`]
//!
//! - **SkillsLoader** loads per-skill **`SKILL.md`** trees (YAML frontmatter + markdown) from workspace/builtin dirs.
//! - **v0 capability manifest** is a separate JSON document that declares **package-level capability entries** (ids, optional priority, etc.) for validation and UI/doctor summaries. Parsing lives here alongside registry stubs; it does not replace skill file loading.
//!
//! ## Recommended Tauri DTO field names (Epic 4.2)
//!
//! Mirror the serde JSON shape of [`error::CapabilityManifestError`] and [`validate::ValidatedManifest`] / [`registry::RegistrySummary`]:
//! `code`, `severity`, `message`, `location_kind` (`file` | `field`), `path`; manifest: `schema_version`, `capabilities[]` with `id`, `name`, `description`, `priority`.
//!
//! Add `schema_version` at the payload root when the IPC contract evolves (NFR-I2).
//!
//! **Construction:** For untrusted input, use [`validate::parse_and_validate_manifest_from_str`] (or `from_bytes` / `from_path`). `Deserialize` on [`ValidatedManifest`] is for symmetry and trusted snapshots only; it skips v0 rules.

pub mod error;
pub mod persist;
pub mod registry;
pub mod validate;

pub use error::{
    codes as capability_error_codes, CapabilityErrorLocationKind, CapabilityManifestError,
    CapabilityManifestErrors,
};
pub use registry::{PlaceholderCapabilityRegistry, RegistrySummary};
pub use persist::{
    load_capability_manifest_into_registry, persist_capability_manifest_json,
    workspace_capability_manifest_path, WORKSPACE_CAPABILITY_MANIFEST_REL,
};
pub use validate::{
    parse_and_validate_manifest_from_bytes, parse_and_validate_manifest_from_path,
    parse_and_validate_manifest_from_str, ValidatedCapability, ValidatedManifest,
};

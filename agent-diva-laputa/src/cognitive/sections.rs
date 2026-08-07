//! Frozen Core section seed helpers (LAPUTA-COGNITIVE-SYNC).
//!
//! Seeds the four Frozen Core section files with a JSON `null` payload on
//! first `LaputaStorage::open`. Using `null` instead of `{}` (or no file)
//! keeps every downstream `content.is_null()` check (authority-block
//! rendering, AutoDream input collection, Frozen Core capture) treating
//! the seeded file as "no semantic content" while the file itself still
//! exists on disk — which is the invariant we actually need (workspace
//! layout is discoverable, `LaputaService::read_section` does not error,
//! and the "empty authority degrades" / "all mandatory inputs omitted"
//! contracts stay intact until a governance write lands real content).
//!
//! Hard boundaries:
//! - Seeding never overwrites an existing file (idempotent on re-open).
//! - Only the four Frozen Core sections are seeded here; the other five
//!   sections (MemoryMd / Daily / Weekly / Monthly / Changelog) stay
//!   empty until a write flow creates them.

use std::fs;

use agent_diva_core::evolution::LaputaSectionName;

use crate::{
    atomic_write, frozen_core::FROZEN_CORE_SECTIONS, layout::LaputaPaths, LaputaError, Result,
};

/// JSON `null`: the typed store deserializes this into `Value::Null`, so
/// every `content.is_null()` short-circuit in the codebase treats it as
/// "no semantic content".
const EMPTY_SECTION_JSON: &str = "null";

/// Seed the four Frozen Core section files when absent. Existing files
/// (including files human-edited to `{}` or richer content) are never
/// overwritten.
pub fn initialize_sections(paths: &LaputaPaths) -> Result<()> {
    for section in FROZEN_CORE_SECTIONS.iter().cloned() {
        let path = paths.section_file(section);
        if path.exists() {
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|source| LaputaError::io(parent.to_path_buf(), source))?;
        }
        atomic_write(&path, EMPTY_SECTION_JSON.as_bytes())?;
    }
    Ok(())
}
/// Read-only view over which section stems are seeded by `initialize_sections`.
/// Intended for tests and diagnostic tooling.
pub fn seeded_section_names() -> &'static [LaputaSectionName] {
    &FROZEN_CORE_SECTIONS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_sections_is_idempotent_and_preserves_existing_content() {
        let temp = tempfile::tempdir().unwrap();
        let paths = LaputaPaths::new(temp.path());
        fs::create_dir_all(paths.cognitive_dir()).unwrap();

        initialize_sections(&paths).unwrap();
        let first_identity =
            fs::read_to_string(paths.section_file(LaputaSectionName::Identity)).unwrap();
        assert_eq!(first_identity, EMPTY_SECTION_JSON);

        // Human edits must never be overwritten.
        let custom = r#"{"name":"vivy","voice":"dry"}"#;
        fs::write(paths.section_file(LaputaSectionName::Identity), custom).unwrap();

        initialize_sections(&paths).unwrap();
        let after = fs::read_to_string(paths.section_file(LaputaSectionName::Identity)).unwrap();
        assert_eq!(after, custom, "seeded content must not clobber human edits");

        // The other three sections remain at the null seed.
        for section in [
            LaputaSectionName::Relationship,
            LaputaSectionName::Commitment,
            LaputaSectionName::Preferences,
        ] {
            let content = fs::read_to_string(paths.section_file(section)).unwrap();
            assert_eq!(content, EMPTY_SECTION_JSON);
        }
    }

    #[test]
    fn initialize_sections_only_touches_frozen_core_four() {
        let temp = tempfile::tempdir().unwrap();
        let paths = LaputaPaths::new(temp.path());
        fs::create_dir_all(paths.cognitive_dir()).unwrap();

        initialize_sections(&paths).unwrap();

        // Non-Frozen-Core sections must NOT be seeded.
        for section in [
            LaputaSectionName::MemoryMd,
            LaputaSectionName::Daily,
            LaputaSectionName::Weekly,
            LaputaSectionName::Monthly,
            LaputaSectionName::Changelog,
        ] {
            assert!(
                !paths.section_file(section.clone()).exists(),
                "{section:?} should not be seeded by initialize_sections"
            );
        }
    }
}

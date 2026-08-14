//! Cognitive governance files (LAPUTA-COGNITIVE-SYNC).
//!
//! Workspace-level `.laputa/cognitive/` directory holding the governance
//! files defined by the cognitive partition decision (garden ADR-0002/0004):
//! MEMRULES.MD (rule handbook) and WORLD.MD (claim-based world model).
//!
//! Hard boundaries:
//! - These files are never injected into the system prompt or ContextView.
//! - No agent-facing write API exists; humans are the only editors.
//! - Seeding never overwrites an existing file (InitializeDir semantics).

pub mod memrules;
pub mod world;
pub mod world_governance;

pub use memrules::{MemRule, MemRules};
pub use world::{ClaimStatus, WorldClaim, WorldError, WorldStore};
pub use world_governance::{
    WorldClaimPayload, WorldGovernance, WorldGovernanceError, WorldProposalState,
    WorldUpsertProposal,
};

use std::{fs, path::Path};

use crate::{atomic_write, LaputaError, Result};

pub const MEMRULES_FILE_NAME: &str = "MEMRULES.MD";
pub const WORLD_FILE_NAME: &str = "WORLD.MD";

/// Create the cognitive directory and seed MEMRULES.MD when absent.
///
/// WORLD.MD is intentionally not created here. A missing world is an empty
/// world, and user-created content must remain untouched.
pub fn initialize_dir(dir: impl AsRef<Path>) -> Result<()> {
    let dir = dir.as_ref();
    fs::create_dir_all(dir).map_err(|source| LaputaError::io(dir, source))?;
    seed_file(
        dir.join(MEMRULES_FILE_NAME),
        memrules::DEFAULT_MEM_RULES_TEXT,
    )
}

fn seed_file(path: impl AsRef<Path>, content: &str) -> Result<()> {
    let path = path.as_ref();
    if path.exists() {
        return Ok(());
    }
    atomic_write(path, content.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_dir_seeds_memrules_but_not_world() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("cognitive");

        initialize_dir(&dir).unwrap();
        let seeded = fs::read_to_string(dir.join(MEMRULES_FILE_NAME)).unwrap();
        assert!(seeded.contains("## R1"));
        assert!(!dir.join(WORLD_FILE_NAME).exists());

        // Idempotent: re-running must not fail or change the file.
        initialize_dir(&dir).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join(MEMRULES_FILE_NAME)).unwrap(),
            seeded
        );

        // Human-edited content must never be overwritten.
        let custom = "# Memory Rules\n\n## R1 — custom\nhand-edited rule\n";
        fs::write(dir.join(MEMRULES_FILE_NAME), custom).unwrap();
        initialize_dir(&dir).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join(MEMRULES_FILE_NAME)).unwrap(),
            custom
        );

        // A user-created WORLD.MD is preserved, but never created by boot.
        let world = "# WORLD\n\n## [ops] marker\n";
        fs::write(dir.join(WORLD_FILE_NAME), world).unwrap();
        initialize_dir(&dir).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join(WORLD_FILE_NAME)).unwrap(),
            world
        );
    }
}

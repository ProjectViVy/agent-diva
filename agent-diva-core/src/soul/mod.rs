//! Soul state helpers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Runtime state for soul/bootstrap lifecycle.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SoulState {
    /// Timestamp when bootstrap was first seeded.
    pub bootstrap_seeded_at: Option<DateTime<Utc>>,
    /// Timestamp when bootstrap was marked as completed.
    pub bootstrap_completed_at: Option<DateTime<Utc>>,
}

/// Small persistence helper for soul lifecycle state.
pub struct SoulStateStore {
    path: PathBuf,
}

impl SoulStateStore {
    /// Create a state store under `<workspace>/.agent-diva/soul-state.json`.
    pub fn new(workspace: impl AsRef<Path>) -> Self {
        let path = workspace
            .as_ref()
            .join(".agent-diva")
            .join("soul-state.json");
        Self { path }
    }

    /// Read state from disk. Missing file returns defaults.
    pub fn load(&self) -> std::io::Result<SoulState> {
        if !self.path.exists() {
            return Ok(SoulState::default());
        }

        let raw = std::fs::read_to_string(&self.path)?;
        let state: SoulState = serde_json::from_str(&raw).unwrap_or_default();
        Ok(state)
    }

    /// Persist state to disk.
    pub fn save(&self, state: &SoulState) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(state)?;
        std::fs::write(&self.path, content)
    }

    /// Returns true if bootstrap completion was already recorded.
    pub fn is_bootstrap_completed(&self) -> bool {
        self.load()
            .ok()
            .and_then(|state| state.bootstrap_completed_at)
            .is_some()
    }

    /// Mark bootstrap as seeded once.
    pub fn mark_bootstrap_seeded(&self) -> std::io::Result<()> {
        let mut state = self.load().unwrap_or_default();
        if state.bootstrap_seeded_at.is_none() {
            state.bootstrap_seeded_at = Some(Utc::now());
            self.save(&state)?;
        }
        Ok(())
    }

    /// Mark bootstrap as completed.
    pub fn mark_bootstrap_completed(&self) -> std::io::Result<()> {
        let mut state = self.load().unwrap_or_default();
        if state.bootstrap_seeded_at.is_none() {
            state.bootstrap_seeded_at = Some(Utc::now());
        }
        if state.bootstrap_completed_at.is_none() {
            state.bootstrap_completed_at = Some(Utc::now());
            self.save(&state)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_missing_returns_default() {
        let temp = tempfile::tempdir().unwrap();
        let store = SoulStateStore::new(temp.path());
        let state = store.load().unwrap();
        assert!(state.bootstrap_seeded_at.is_none());
        assert!(state.bootstrap_completed_at.is_none());
    }

    #[test]
    fn test_mark_bootstrap_seeded_writes_once() {
        let temp = tempfile::tempdir().unwrap();
        let store = SoulStateStore::new(temp.path());

        store.mark_bootstrap_seeded().unwrap();
        let first = store.load().unwrap();
        assert!(first.bootstrap_seeded_at.is_some());

        store.mark_bootstrap_seeded().unwrap();
        let second = store.load().unwrap();
        assert_eq!(first.bootstrap_seeded_at, second.bootstrap_seeded_at);
    }

    #[test]
    fn test_is_bootstrap_completed() {
        let temp = tempfile::tempdir().unwrap();
        let store = SoulStateStore::new(temp.path());
        assert!(!store.is_bootstrap_completed());

        let mut state = SoulState::default();
        state.bootstrap_completed_at = Some(Utc::now());
        store.save(&state).unwrap();
        assert!(store.is_bootstrap_completed());
    }

    #[test]
    fn test_mark_bootstrap_completed_sets_completed_at() {
        let temp = tempfile::tempdir().unwrap();
        let store = SoulStateStore::new(temp.path());
        store.mark_bootstrap_completed().unwrap();
        let state = store.load().unwrap();
        assert!(state.bootstrap_seeded_at.is_some());
        assert!(state.bootstrap_completed_at.is_some());
    }
}

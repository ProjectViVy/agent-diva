//! Frozen Core session snapshot (LAPUTA-COGNITIVE-SYNC S3).
//!
//! The four Frozen Core sections (01 identity, 02 relationship,
//! 03 commitment, 04 preferences) are captured once at session start and
//! stay frozen for the whole session. Governance writes may still land on
//! the sections while the session runs, but they only take effect for the
//! next session (the next capture), aligned with garden ADR-0002 §2.A.

use agent_diva_core::evolution::LaputaSectionName;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::Path,
    sync::{OnceLock, RwLock},
};

use crate::{LaputaService, Result};

/// Sections forming the Frozen Core, in canonical order.
pub const FROZEN_CORE_SECTIONS: [LaputaSectionName; 4] = [
    LaputaSectionName::Identity,
    LaputaSectionName::Relationship,
    LaputaSectionName::Commitment,
    LaputaSectionName::Preferences,
];

/// Default projection budget for rendering the snapshot into a prompt.
pub const DEFAULT_FROZEN_CORE_BUDGET: usize = 4000;

/// Immutable session-start snapshot of the Frozen Core sections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenCoreSnapshot {
    pub captured_at: DateTime<Utc>,
    /// Section content serialized as compact JSON in canonical Frozen Core
    /// order; empty string means the section file did not exist at capture.
    pub sections: Vec<(LaputaSectionName, String)>,
    /// Stable content digests for comparing captured and current authority.
    pub section_versions: Vec<(LaputaSectionName, String)>,
}

/// Read-only runtime projection of one session's Frozen Core capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenCoreSessionProjection {
    pub session_key: String,
    pub captured_at: DateTime<Utc>,
    pub section_versions: Vec<(LaputaSectionName, String)>,
}

static SESSION_SNAPSHOTS: OnceLock<RwLock<HashMap<(String, String), FrozenCoreSnapshot>>> =
    OnceLock::new();

impl FrozenCoreSnapshot {
    /// Capture the four Frozen Core sections from the live service.
    /// Later section writes do not affect the returned snapshot.
    pub fn capture(service: &LaputaService) -> Result<Self> {
        let mut sections = Vec::new();
        let mut section_versions = Vec::new();
        for name in FROZEN_CORE_SECTIONS.iter().cloned() {
            let section = service.read_section(name.clone())?;
            let rendered = if section.content.is_null() {
                String::new()
            } else {
                serde_json::to_string(&section.content)?
            };
            let version = content_version(&rendered);
            sections.push((name.clone(), rendered));
            section_versions.push((name, version));
        }
        Ok(Self {
            captured_at: Utc::now(),
            sections,
            section_versions,
        })
    }

    pub fn content_of(&self, name: &LaputaSectionName) -> Option<&str> {
        self.sections
            .iter()
            .find(|(section, _)| section == name)
            .map(|(_, content)| content.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.sections.iter().all(|(_, content)| content.is_empty())
    }

    /// Bounded projection for prompt assembly. Empty sections are skipped;
    /// the budget is honored across all four sections in canonical order.
    pub fn render(&self, max_chars: usize) -> String {
        let budget = if max_chars == 0 {
            DEFAULT_FROZEN_CORE_BUDGET
        } else {
            max_chars
        };
        let mut out = String::new();
        let mut used = 0usize;
        for (name, content) in &self.sections {
            if content.is_empty() {
                continue;
            }
            let header = format!("## Frozen Core — {}\n", name.as_str());
            let cost = header.chars().count() + content.chars().count() + 1;
            if used + cost > budget {
                break;
            }
            out.push_str(&header);
            out.push_str(content);
            out.push('\n');
            used += cost;
        }
        out
    }
}

/// Capture once per workspace/session pair and retain a bounded, process-local
/// observability projection for the desktop control plane.
pub fn capture_for_session(workspace: &Path, session_key: &str) -> FrozenCoreSnapshot {
    let workspace_id = agent_diva_core::workspace_identity::canonical_workspace_id(workspace);
    let key = (workspace_id, session_key.to_string());
    let registry = SESSION_SNAPSHOTS.get_or_init(|| RwLock::new(HashMap::new()));
    if let Ok(guard) = registry.read() {
        if let Some(snapshot) = guard.get(&key) {
            return snapshot.clone();
        }
    }

    let snapshot = LaputaService::open(workspace)
        .and_then(|service| FrozenCoreSnapshot::capture(&service))
        .unwrap_or_else(|_| FrozenCoreSnapshot {
            captured_at: Utc::now(),
            sections: FROZEN_CORE_SECTIONS
                .iter()
                .cloned()
                .map(|name| (name, String::new()))
                .collect(),
            section_versions: FROZEN_CORE_SECTIONS
                .iter()
                .cloned()
                .map(|name| (name, content_version("")))
                .collect(),
        });

    if let Ok(mut guard) = registry.write() {
        if guard.len() >= 128 {
            if let Some(oldest) = guard
                .iter()
                .min_by_key(|(_, value)| value.captured_at)
                .map(|(key, _)| key.clone())
            {
                guard.remove(&oldest);
            }
        }
        guard.insert(key, snapshot.clone());
    }
    snapshot
}

/// Return the captured projection for a session that has built context.
pub fn session_projection(
    workspace: &Path,
    session_key: &str,
) -> Option<FrozenCoreSessionProjection> {
    let workspace_id = agent_diva_core::workspace_identity::canonical_workspace_id(workspace);
    SESSION_SNAPSHOTS
        .get()
        .and_then(|registry| registry.read().ok())
        .and_then(|guard| guard.get(&(workspace_id, session_key.to_string())).cloned())
        .map(|snapshot| FrozenCoreSessionProjection {
            session_key: session_key.to_string(),
            captured_at: snapshot.captured_at,
            section_versions: snapshot.section_versions,
        })
}

/// Forget a reset/deleted session so the next prompt captures fresh authority.
pub fn release_session_projection(workspace: &Path, session_key: &str) {
    let workspace_id = agent_diva_core::workspace_identity::canonical_workspace_id(workspace);
    if let Some(registry) = SESSION_SNAPSHOTS.get() {
        if let Ok(mut guard) = registry.write() {
            guard.remove(&(workspace_id, session_key.to_string()));
        }
    }
}

/// Stable digest used as the authority revision shown by the persona UI.
pub fn content_version(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    format!("sha256:{digest:x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LaputaStorage;

    fn open_service(temp: &tempfile::TempDir) -> LaputaService {
        LaputaStorage::open(temp.path()).unwrap();
        LaputaService::open(temp.path()).unwrap()
    }

    fn write_identity_section(temp: &tempfile::TempDir, body: &str) {
        let path = temp
            .path()
            .join(".laputa")
            .join("sections")
            .join("identity.json");
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn capture_freezes_sections_for_the_session() {
        let temp = tempfile::tempdir().unwrap();
        let service = open_service(&temp);
        write_identity_section(&temp, r#"{"name":"vivy"}"#);

        let snapshot = FrozenCoreSnapshot::capture(&service).unwrap();
        assert!(snapshot
            .content_of(&LaputaSectionName::Identity)
            .unwrap()
            .contains("vivy"));

        // A governance write lands mid-session; the snapshot must not move.
        write_identity_section(&temp, r#"{"name":"rewritten"}"#);
        assert!(snapshot
            .content_of(&LaputaSectionName::Identity)
            .unwrap()
            .contains("vivy"));

        // The next session's capture sees the applied write.
        let next = FrozenCoreSnapshot::capture(&service).unwrap();
        assert!(next
            .content_of(&LaputaSectionName::Identity)
            .unwrap()
            .contains("rewritten"));
    }

    #[test]
    fn session_projection_tracks_effective_authority_until_release() {
        let temp = tempfile::tempdir().unwrap();
        open_service(&temp);
        write_identity_section(&temp, r#"{"name":"first"}"#);

        let first = capture_for_session(temp.path(), "desktop:test");
        let first_version = first.section_versions[0].1.clone();
        write_identity_section(&temp, r#"{"name":"second"}"#);

        let still_first = capture_for_session(temp.path(), "desktop:test");
        assert_eq!(still_first.section_versions[0].1, first_version);
        assert_eq!(
            session_projection(temp.path(), "desktop:test")
                .unwrap()
                .section_versions[0]
                .1,
            first_version
        );

        release_session_projection(temp.path(), "desktop:test");
        let next = capture_for_session(temp.path(), "desktop:test");
        assert_ne!(next.section_versions[0].1, first_version);
    }

    #[test]
    fn missing_sections_yield_empty_snapshot_after_open() {
        let temp = tempfile::tempdir().unwrap();
        let service = LaputaService::open(temp.path()).unwrap();

        let sections_dir = temp.path().join(".laputa").join("sections");
        for name in [
            LaputaSectionName::Identity,
            LaputaSectionName::Relationship,
            LaputaSectionName::Commitment,
            LaputaSectionName::Preferences,
        ] {
            assert!(
                !sections_dir
                    .join(format!("{}.json", name.as_str()))
                    .exists(),
                "{name:?} must not be seeded on first open"
            );
        }

        let snapshot = FrozenCoreSnapshot::capture(&service).unwrap();
        assert!(snapshot.is_empty());
        assert_eq!(snapshot.render(0), "");
    }

    #[test]
    fn render_honors_budget_and_canonical_order() {
        let temp = tempfile::tempdir().unwrap();
        let service = open_service(&temp);
        let sections = temp.path().join(".laputa").join("sections");
        std::fs::write(sections.join("identity.json"), r#"{"a":1}"#).unwrap();
        std::fs::write(sections.join("preferences.json"), r#"{"b":2}"#).unwrap();

        let snapshot = FrozenCoreSnapshot::capture(&service).unwrap();
        let full = snapshot.render(0);
        let identity_pos = full.find("identity").unwrap();
        let preferences_pos = full.find("preferences").unwrap();
        assert!(identity_pos < preferences_pos, "canonical order");

        // A tiny budget must not include the second section.
        let tight = snapshot.render(full.chars().count() - 1);
        assert!(tight.contains("identity"));
        assert!(!tight.contains("preferences"));
    }
}

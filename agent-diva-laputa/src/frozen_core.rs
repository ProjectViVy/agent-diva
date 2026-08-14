//! Session-frozen projection of the config-rooted Persona Markdown authority.

use crate::persona::{extract_markdown_section, PersonaError, PersonaKind, PersonaService};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::Path,
    sync::{OnceLock, RwLock},
};
use unicode_segmentation::UnicodeSegmentation;

pub const FROZEN_CORE_SECTIONS: [PersonaKind; 6] = PersonaKind::FROZEN;
pub const DEFAULT_FROZEN_CORE_BUDGET: usize = 4000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenCoreSnapshot {
    pub captured_at: DateTime<Utc>,
    pub sections: Vec<(PersonaKind, String)>,
    pub section_versions: Vec<(PersonaKind, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenCoreSessionProjection {
    pub session_key: String,
    pub captured_at: DateTime<Utc>,
    pub section_versions: Vec<(PersonaKind, String)>,
}

static SESSION_SNAPSHOTS: OnceLock<RwLock<HashMap<(String, String), FrozenCoreSnapshot>>> =
    OnceLock::new();

impl FrozenCoreSnapshot {
    pub fn capture(service: &PersonaService) -> Result<Self, PersonaError> {
        service.status().and_then(|status| match status.status {
            crate::PersonaStatus::Ready => Ok(()),
            crate::PersonaStatus::Uninitialized => Err(PersonaError::Uninitialized),
            crate::PersonaStatus::Incomplete => Err(PersonaError::Incomplete),
        })?;
        let mut sections = Vec::new();
        let mut section_versions = Vec::new();
        for kind in FROZEN_CORE_SECTIONS {
            let document = service.get_document(kind)?;
            let source = if kind == PersonaKind::User {
                extract_markdown_section(&document.content, "Preferences")
                    .unwrap_or(&document.content)
            } else {
                &document.content
            };
            let content = truncate_visible(source, kind.frozen_limit().unwrap_or_default());
            if !content.is_empty() {
                section_versions.push((kind, document.content_hash));
                sections.push((kind, content));
            }
        }
        Ok(Self {
            captured_at: Utc::now(),
            sections,
            section_versions,
        })
    }

    pub fn content_of(&self, kind: PersonaKind) -> Option<&str> {
        self.sections
            .iter()
            .find(|(candidate, _)| *candidate == kind)
            .map(|(_, content)| content.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    pub fn render(&self, max_chars: usize) -> String {
        let budget = if max_chars == 0 {
            DEFAULT_FROZEN_CORE_BUDGET
        } else {
            max_chars
        };
        let mut out = String::new();
        let mut used = 0usize;
        for (kind, content) in &self.sections {
            let header = format!("## Frozen Core — {}\n", kind.as_str());
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

pub fn capture_for_session(config_dir: &Path, session_key: &str) -> FrozenCoreSnapshot {
    let key = (root_key(config_dir), session_key.to_string());
    let registry = SESSION_SNAPSHOTS.get_or_init(|| RwLock::new(HashMap::new()));
    if let Ok(guard) = registry.read() {
        if let Some(snapshot) = guard.get(&key) {
            return snapshot.clone();
        }
    }
    let snapshot = PersonaService::open(config_dir)
        .and_then(|service| FrozenCoreSnapshot::capture(&service))
        .unwrap_or_else(|error| {
            tracing::error!(error = %error, "failed to capture Persona Frozen Core");
            FrozenCoreSnapshot {
                captured_at: Utc::now(),
                sections: Vec::new(),
                section_versions: Vec::new(),
            }
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

pub fn session_projection(
    config_dir: &Path,
    session_key: &str,
) -> Option<FrozenCoreSessionProjection> {
    SESSION_SNAPSHOTS
        .get()
        .and_then(|registry| registry.read().ok())
        .and_then(|guard| {
            guard
                .get(&(root_key(config_dir), session_key.to_string()))
                .cloned()
        })
        .map(|snapshot| FrozenCoreSessionProjection {
            session_key: session_key.to_string(),
            captured_at: snapshot.captured_at,
            section_versions: snapshot.section_versions,
        })
}

pub fn release_session_projection(config_dir: &Path, session_key: &str) {
    if let Some(registry) = SESSION_SNAPSHOTS.get() {
        if let Ok(mut guard) = registry.write() {
            guard.remove(&(root_key(config_dir), session_key.to_string()));
        }
    }
}

pub fn content_version(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    format!("sha256:{digest:x}")
}

fn root_key(root: &Path) -> String {
    root.to_string_lossy().to_lowercase()
}

fn truncate_visible(content: &str, limit: usize) -> String {
    let mut visible = 0usize;
    let mut output = String::new();
    for grapheme in UnicodeSegmentation::graphemes(content, true) {
        if !grapheme.chars().all(char::is_whitespace) {
            if visible == limit {
                break;
            }
            visible += 1;
        }
        output.push_str(grapheme);
    }
    output.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PersonaInitialization;

    fn initialized() -> (tempfile::TempDir, PersonaService) {
        let temp = tempfile::tempdir().unwrap();
        let service = PersonaService::open(temp.path()).unwrap();
        service
            .initialize(PersonaInitialization {
                identity: format!("# Identity\n{}", "I".repeat(240)),
                relationship: "# Relationship\nPartner".into(),
                redline: "# Redline\nAsk first".into(),
                user: "Concise".into(),
                world: "# World\nNever inject me".into(),
            })
            .unwrap();
        (temp, service)
    }

    #[test]
    fn capture_uses_markdown_caps_and_excludes_world() {
        let (_temp, service) = initialized();
        let snapshot = FrozenCoreSnapshot::capture(&service).unwrap();
        assert!(snapshot.content_of(PersonaKind::World).is_none());
        assert_eq!(
            crate::persona::visible_len(snapshot.content_of(PersonaKind::Identity).unwrap()),
            200
        );
        assert_eq!(snapshot.content_of(PersonaKind::User), Some("Concise"));
    }

    #[test]
    fn session_projection_is_frozen_until_release() {
        let (temp, service) = initialized();
        let first = capture_for_session(temp.path(), "desktop:test");
        let current = service.get_document(PersonaKind::Identity).unwrap();
        service
            .save_user_document(
                PersonaKind::Identity,
                "# Identity\nRewritten",
                current.revision,
                "edit",
            )
            .unwrap();
        assert_eq!(
            capture_for_session(temp.path(), "desktop:test").sections,
            first.sections
        );
        release_session_projection(temp.path(), "desktop:test");
        assert_ne!(
            capture_for_session(temp.path(), "desktop:test").sections,
            first.sections
        );
    }
}

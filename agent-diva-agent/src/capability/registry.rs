//! v0 placeholder in-memory registry (no download, no signatures, no hot-reload).

use std::collections::HashMap;
use std::sync::RwLock;

use serde::Serialize;

use super::error::{codes, CapabilityManifestError, CapabilityManifestErrors};
use super::validate::{ValidatedCapability, ValidatedManifest};

/// Process-wide placeholder registry.
///
/// **Thread safety:** `std::sync::RwLock` allows concurrent reads; writers exclude readers.
/// Typical use: load once at startup (single-threaded) or infrequent updates from a gateway thread.
#[derive(Debug, Default)]
pub struct PlaceholderCapabilityRegistry {
    inner: RwLock<HashMap<String, ValidatedCapability>>,
}

impl PlaceholderCapabilityRegistry {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// Insert each capability from a validated manifest. Fails if any `id` already exists; **no partial insert**.
    pub fn register(&self, manifest: ValidatedManifest) -> Result<(), CapabilityManifestErrors> {
        let mut errors = CapabilityManifestErrors::default();
        {
            let map = self.inner.read().map_err(|_| {
                CapabilityManifestErrors::new(vec![CapabilityManifestError::file(
                    codes::REGISTRY_LOCK_POISONED,
                    "capability registry lock poisoned",
                )])
            })?;
            for cap in &manifest.capabilities {
                if map.contains_key(&cap.id) {
                    errors.push(CapabilityManifestError::field(
                        codes::DUPLICATE_ID,
                        format!("capability id {:?} already registered", cap.id),
                        format!("/capabilities/{}", cap.id),
                    ));
                }
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let mut map = self.inner.write().map_err(|_| {
            CapabilityManifestErrors::new(vec![CapabilityManifestError::file(
                codes::REGISTRY_LOCK_POISONED,
                "capability registry lock poisoned",
            )])
        })?;
        for cap in manifest.capabilities {
            map.insert(cap.id.clone(), cap);
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.inner.read().map(|m| m.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, id: &str) -> Option<ValidatedCapability> {
        self.inner.read().ok()?.get(id).cloned()
    }

    /// Stable id list (sorted for deterministic summaries).
    pub fn ids(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .inner
            .read()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        v.sort();
        v
    }

    /// Lightweight summary for doctor / Epic 1 follow-ups (`serde` for future `invoke`).
    pub fn summary(&self) -> RegistrySummary {
        let ids = self.ids();
        RegistrySummary {
            count: ids.len(),
            ids,
        }
    }

    /// Clear all entries (tests / dev).
    pub fn clear(&self) {
        if let Ok(mut m) = self.inner.write() {
            m.clear();
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RegistrySummary {
    pub count: usize,
    pub ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::super::validate::parse_and_validate_manifest_from_str;
    use super::*;

    #[test]
    fn register_and_query() {
        let reg = PlaceholderCapabilityRegistry::new();
        let m = parse_and_validate_manifest_from_str(
            r#"{"schema_version":"0","capabilities":[{"id":"a","name":"A"}]}"#,
        )
        .unwrap();
        reg.register(m).unwrap();
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.get("a").unwrap().name.as_deref(), Some("A"));
        let s = reg.summary();
        assert_eq!(s.count, 1);
        assert_eq!(s.ids, vec!["a".to_string()]);
    }

    #[test]
    fn register_duplicate_across_calls_fails() {
        let reg = PlaceholderCapabilityRegistry::new();
        let m1 = parse_and_validate_manifest_from_str(r#"{"capabilities":[{"id":"a"}]}"#).unwrap();
        reg.register(m1).unwrap();
        let m2 = parse_and_validate_manifest_from_str(r#"{"capabilities":[{"id":"a"}]}"#).unwrap();
        let err = reg.register(m2).unwrap_err();
        assert!(err.errors.iter().any(|e| e.code == codes::DUPLICATE_ID));
    }
}

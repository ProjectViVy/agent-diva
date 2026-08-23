//! Agent-facing memory CRUD contract.
//!
//! Agent and management surface for direct BML record operations.
//! Implementations that do not support an operation report `Failed` with an
//! explicit reason instead of silently succeeding.

use std::path::PathBuf;

use crate::evolution::EvidenceRef;

/// Input for a low-risk immediate memory write.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemoryAddRequest {
    /// Content to remember (user-requested fact or preference).
    pub content: String,
    /// Optional evidence references backing this write (B9 partial).
    #[serde(default)]
    pub evidence_refs: Vec<EvidenceRef>,
}

/// Input for listing the applied memory projection.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemoryListRequest {
    /// Maximum number of entries to return.
    pub limit: Option<u32>,
}

/// Input for retrieving one visible long-term record by id.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemoryGetRequest {
    /// Stable record id in the authority.
    pub record_id: String,
}

/// Input for searching applied memory.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemorySearchRequest {
    /// Free-text query matched against applied authority.
    pub query: String,
    /// Maximum number of entries to return.
    pub limit: Option<u32>,
}

/// Input for a compare-and-swap update applied directly to BML.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemoryUpdateRequest {
    /// Target record id in the applied authority.
    pub record_id: String,
    /// Replacement content.
    pub content: String,
    /// Record revision observed by the caller.
    pub base_revision: i64,
    /// Optional evidence references backing the replacement.
    #[serde(default)]
    pub evidence_refs: Vec<EvidenceRef>,
}

/// Input for a compare-and-swap soft deletion applied directly to BML.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemoryRemoveRequest {
    /// Target record id in the applied authority.
    pub record_id: String,
    /// Human-readable reason for the removal.
    pub reason: String,
    /// Record revision observed by the caller.
    pub base_revision: i64,
}

/// Input for proactive experience distillation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemoryDistillRequest {
    /// Skill name for the distilled experience; the skill file is
    /// `<workspace>/skills/<skill_name>/SKILL.md`.
    pub skill_name: String,
    /// Action-verified experience content (minimal patch discipline).
    pub content: String,
    /// Optional session-context evidence backing the distillation (G1: the
    /// Wave 1 minimal contract uses session context; checkpoint evidence is
    /// attached by Wave 2 callers).
    pub evidence: Option<String>,
}

/// A single entry of the applied memory projection.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemoryEntry {
    /// Stable record id in the authority.
    pub id: String,
    /// Rendered content of the record.
    pub content: String,
    /// Trust tier of the record (applied authority for visible entries).
    pub trust: String,
    /// Provenance source label, when available.
    pub provenance: Option<String>,
    /// Evidence references stored with the record.
    #[serde(default)]
    pub evidence_refs: Vec<EvidenceRef>,
    /// Monotonic record revision used for update/delete CAS.
    pub revision: i64,
    /// RFC 3339 creation timestamp.
    pub created_at: String,
    /// RFC 3339 last-effective timestamp.
    pub updated_at: String,
}

/// Deterministic outcome of a memory CRUD operation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum MemoryCrudOutcome {
    /// The write reached the durable authority.
    Applied {
        /// Entry as written, when applicable.
        entry: Option<MemoryEntry>,
        /// Advisory note when the write lacked tool-result evidence (B9 partial).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        evidence_advisory: Option<String>,
    },
    /// A read projection of the applied authority.
    Listed {
        /// Visible entries of the applied authority.
        entries: Vec<MemoryEntry>,
    },
    /// The operation did not succeed.
    Failed {
        /// Stable, payload-free reason.
        reason: String,
    },
}

/// Convenience constructor for the unsupported default.
impl MemoryCrudOutcome {
    pub fn unsupported(operation: &str) -> Self {
        Self::Failed {
            reason: format!("{operation} not supported by this memory provider"),
        }
    }
}

/// Workspace-scoped request shared by all CRUD operations.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemoryCrudContext {
    /// Workspace root for the active agent session.
    pub workspace_root: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_reports_stable_reason() {
        let outcome = MemoryCrudOutcome::unsupported("memory_add");
        assert!(matches!(
            outcome,
            MemoryCrudOutcome::Failed { reason } if reason.contains("memory_add")
        ));
    }

    #[test]
    fn outcome_roundtrip_serde() {
        let applied = MemoryCrudOutcome::Applied {
            entry: Some(MemoryEntry {
                id: "rec-1".to_string(),
                content: "favorite color is blue".to_string(),
                trust: "applied_authority".to_string(),
                provenance: Some("memory_add".to_string()),
                evidence_refs: Vec::new(),
                revision: 1,
                created_at: "2026-08-15T00:00:00Z".to_string(),
                updated_at: "2026-08-15T00:00:00Z".to_string(),
            }),
            evidence_advisory: None,
        };
        let json = serde_json::to_string(&applied).unwrap();
        assert!(json.contains("\"status\":\"applied\""));
        assert!(!json.contains("evidence_advisory"));
        let back: MemoryCrudOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(back, applied);
    }

    #[test]
    fn applied_with_advisory_serializes() {
        let applied = MemoryCrudOutcome::Applied {
            entry: None,
            evidence_advisory: Some("no evidence_refs".to_string()),
        };
        let json = serde_json::to_string(&applied).unwrap();
        assert!(json.contains("evidence_advisory"));
        let back: MemoryCrudOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(back, applied);
    }
}

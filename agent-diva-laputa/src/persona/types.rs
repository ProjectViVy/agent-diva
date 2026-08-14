use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt, path::PathBuf, str::FromStr};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonaKind {
    Identity,
    Relationship,
    Redline,
    User,
    World,
    Dream,
    Dark,
}

impl PersonaKind {
    pub const ALL: [Self; 7] = [
        Self::Identity,
        Self::Relationship,
        Self::Redline,
        Self::User,
        Self::World,
        Self::Dream,
        Self::Dark,
    ];
    pub const REQUIRED: [Self; 5] = [
        Self::Identity,
        Self::Relationship,
        Self::Redline,
        Self::User,
        Self::World,
    ];
    pub const FROZEN: [Self; 6] = [
        Self::Identity,
        Self::Relationship,
        Self::Redline,
        Self::User,
        Self::Dream,
        Self::Dark,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Relationship => "relationship",
            Self::Redline => "redline",
            Self::User => "user",
            Self::World => "world",
            Self::Dream => "dream",
            Self::Dark => "dark",
        }
    }

    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Identity => "IDENTITY.MD",
            Self::Relationship => "RELATIONSHIP.MD",
            Self::Redline => "REDLINE.MD",
            Self::User => "USER.MD",
            Self::World => "WORLD.MD",
            Self::Dream => "DREAM.MD",
            Self::Dark => "DARK.MD",
        }
    }

    pub const fn content_limit(self) -> usize {
        match self {
            Self::Identity => 800,
            Self::Relationship => 600,
            Self::Redline => 400,
            Self::User => 800,
            Self::World => 1000,
            Self::Dream => 40,
            Self::Dark => 300,
        }
    }

    pub const fn frozen_limit(self) -> Option<usize> {
        match self {
            Self::Identity => Some(200),
            Self::Relationship => Some(120),
            Self::Redline => Some(200),
            Self::User => Some(160),
            Self::World => None,
            Self::Dream => Some(10),
            Self::Dark => Some(60),
        }
    }

    pub const fn required_for_ready(self) -> bool {
        matches!(
            self,
            Self::Identity | Self::Relationship | Self::Redline | Self::User | Self::World
        )
    }
}

impl fmt::Display for PersonaKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PersonaKind {
    type Err = PersonaError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str().eq_ignore_ascii_case(value))
            .ok_or_else(|| PersonaError::KindForbidden(value.to_string()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonaStatus {
    Uninitialized,
    Ready,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaFileState {
    pub kind: PersonaKind,
    pub file_name: String,
    pub exists: bool,
    pub valid: bool,
    pub reason: Option<String>,
    pub revision: u64,
    pub updated_at: Option<DateTime<Utc>>,
    pub pending_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaStatusView {
    pub status: PersonaStatus,
    pub files: BTreeMap<PersonaKind, PersonaFileState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaDocument {
    pub kind: PersonaKind,
    pub file_name: String,
    pub exists: bool,
    pub valid: bool,
    pub content: String,
    pub revision: u64,
    pub content_hash: String,
    pub updated_at: Option<DateTime<Utc>>,
    pub pending_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonaWriteSource {
    UserDirect,
    AgentP16,
    AgentP5Accepted,
    AutodreamP5Accepted,
    Init,
    HistoryResave,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaHistoryEntry {
    pub revision: u64,
    pub content_hash: String,
    pub snapshot: String,
    pub diff: String,
    pub actor: String,
    pub source: PersonaWriteSource,
    pub reason: String,
    pub base_revision: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaHistoryRevision {
    #[serde(flatten)]
    pub entry: PersonaHistoryEntry,
    pub content: String,
    pub unified_diff: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaWriteOutcome {
    pub document: PersonaDocument,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaInitialization {
    pub identity: String,
    pub relationship: String,
    pub redline: String,
    pub user: String,
    pub world: String,
}

impl PersonaInitialization {
    pub fn into_map(self) -> BTreeMap<PersonaKind, String> {
        BTreeMap::from([
            (PersonaKind::Identity, self.identity),
            (PersonaKind::Relationship, self.relationship),
            (PersonaKind::Redline, self.redline),
            (PersonaKind::User, self.user),
            (PersonaKind::World, self.world),
        ])
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaRepair {
    pub documents: BTreeMap<PersonaKind, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonaRequestActor {
    Agent,
    Autodream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonaRequestState {
    Pending,
    Accepted,
    Rejected,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaChangeRequest {
    pub id: String,
    pub kind: PersonaKind,
    pub base_revision: u64,
    pub base_hash: String,
    pub proposed_markdown: String,
    pub actor: PersonaRequestActor,
    pub reason: String,
    pub created_at: DateTime<Utc>,
    pub state: PersonaRequestState,
    pub decided_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Error)]
pub enum PersonaError {
    #[error("Persona is not initialized")]
    Uninitialized,
    #[error("Persona is incomplete")]
    Incomplete,
    #[error("Persona revision conflict: expected {expected}, current {current}")]
    RevisionConflict { expected: u64, current: u64 },
    #[error("a pending Persona request already exists for {0}")]
    RequestExists(PersonaKind),
    #[error("Persona request is stale: {0}")]
    RequestStale(String),
    #[error("Persona content exceeds the {limit} character cap for {kind}")]
    CapExceeded { kind: PersonaKind, limit: usize },
    #[error("Persona kind or write scope is forbidden: {0}")]
    KindForbidden(String),
    #[error("WORLD request would overwrite protected user content")]
    WorldProtectedClaim,
    #[error("WORLD request violates the R6 bounded, reviewable claim entry gate")]
    WorldEntryGate,
    #[error("Persona request not found: {0}")]
    RequestNotFound(String),
    #[error("invalid Persona content for {kind}: {reason}")]
    InvalidContent { kind: PersonaKind, reason: String },
    #[error("Persona I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Persona JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl PersonaError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Uninitialized => "persona_uninitialized",
            Self::Incomplete => "persona_incomplete",
            Self::RevisionConflict { .. } => "persona_revision_conflict",
            Self::RequestExists(_) => "persona_request_exists",
            Self::RequestStale(_) => "persona_request_stale",
            Self::CapExceeded { .. } => "persona_cap_exceeded",
            Self::KindForbidden(_) => "persona_kind_forbidden",
            Self::WorldProtectedClaim => "persona_world_protected_claim",
            Self::WorldEntryGate => "persona_world_entry_gate",
            Self::RequestNotFound(_) => "persona_request_not_found",
            Self::InvalidContent { .. } => "persona_invalid_content",
            Self::Io { .. } => "persona_storage_error",
            Self::Json(_) => "persona_storage_error",
        }
    }

    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

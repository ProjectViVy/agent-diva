//! Governed write path for WORLD.MD (LAPUTA-COGNITIVE-SYNC S2, decision Q4).
//!
//! WORLD is a cognitive governance file, not a numbered Laputa section, so
//! its writes run on this dedicated pipeline instead of the section-based
//! [`crate::proposals`] machinery:
//!
//! - `actor == "user"` upserts apply immediately (direct human authority);
//! - any other actor (e.g. AutoDream) lands in a pending-review queue and
//!   only reaches WORLD.MD after explicit approval;
//! - user-confirmed claim protection is enforced again at apply time, so a
//!   claim confirmed by the user between submission and review cannot be
//!   overwritten (only marked stale);
//! - every submit/approve/reject is appended to an append-only JSONL ledger
//!   next to the pending queue (audit surface).

use std::{fs, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    atomic_write,
    cognitive::world::{WorldClaim, WorldError, USER_ACTOR},
    cognitive::WorldStore,
    LaputaError, Result,
};

/// Persisted pending-review queue file name.
pub const WORLD_PROPOSALS_FILE_NAME: &str = "world-proposals.json";
/// Append-only governance ledger file name.
pub const WORLD_LEDGER_FILE_NAME: &str = "world-ledger.jsonl";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldProposalState {
    PendingReview,
    Applied,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldUpsertProposal {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub claim: WorldClaimPayload,
    pub state: WorldProposalState,
    pub decided_at: Option<DateTime<Utc>>,
    pub decision_reason: Option<String>,
}

/// Serializable claim payload (mirrors [`WorldClaim`] for JSON persistence).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldClaimPayload {
    pub domain: String,
    pub title: String,
    pub status: String,
    pub confidence: String,
    pub scopes: Vec<String>,
    pub source: String,
    pub text: String,
}

impl WorldClaimPayload {
    pub fn from_claim(claim: &WorldClaim) -> Self {
        Self {
            domain: claim.domain.clone(),
            title: claim.title.clone(),
            status: claim.status.as_str().to_string(),
            confidence: claim.confidence.clone(),
            scopes: claim.scopes.clone(),
            source: claim.source.clone(),
            text: claim.text.clone(),
        }
    }

    pub fn to_claim(&self, now: DateTime<Utc>) -> std::result::Result<WorldClaim, WorldError> {
        let status =
            crate::cognitive::ClaimStatus::parse(&self.status).ok_or(WorldError::EmptyHeading)?;
        if self.domain.trim().is_empty() || self.title.trim().is_empty() {
            return Err(WorldError::EmptyHeading);
        }
        Ok(WorldClaim {
            domain: self.domain.clone(),
            title: self.title.clone(),
            status,
            confidence: self.confidence.clone(),
            scopes: self.scopes.clone(),
            source: self.source.clone(),
            updated: now,
            text: self.text.clone(),
        })
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WorldGovernanceError {
    #[error("world proposal not found: {id}")]
    NotFound { id: String },
    #[error("world proposal already exists: {id}")]
    AlreadyExists { id: String },
    #[error("world proposal {id} is not pending review")]
    NotPending { id: String },
    #[error("world claim rejected by protection rules: {reason}")]
    Protected { reason: String },
    #[error("world governance storage failure: {reason}")]
    Storage { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorldGovernanceState {
    schema_version: u32,
    proposals: Vec<WorldUpsertProposal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorldLedgerEntry {
    at: DateTime<Utc>,
    actor: String,
    action: String,
    proposal_id: String,
    domain: String,
    title: String,
    result: String,
}

/// Governed WORLD write service over one workspace cognitive directory.
#[derive(Debug, Clone)]
pub struct WorldGovernance {
    world_path: PathBuf,
    proposals_path: PathBuf,
    ledger_path: PathBuf,
    state: WorldGovernanceState,
}

impl WorldGovernance {
    /// Open the governance service for a cognitive directory; loads the
    /// pending queue if present.
    pub fn open(cognitive_dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = cognitive_dir.into();
        let proposals_path = dir.join(WORLD_PROPOSALS_FILE_NAME);
        let state = match fs::read_to_string(&proposals_path) {
            Ok(raw) => serde_json::from_str(&raw)?,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => WorldGovernanceState {
                schema_version: 1,
                proposals: Vec::new(),
            },
            Err(source) => return Err(LaputaError::io(&proposals_path, source)),
        };
        Ok(Self {
            world_path: dir.join(crate::cognitive::WORLD_FILE_NAME),
            proposals_path,
            ledger_path: dir.join(WORLD_LEDGER_FILE_NAME),
            state,
        })
    }

    pub fn proposals(&self) -> &[WorldUpsertProposal] {
        &self.state.proposals
    }

    pub fn pending(&self) -> Vec<&WorldUpsertProposal> {
        self.state
            .proposals
            .iter()
            .filter(|proposal| proposal.state == WorldProposalState::PendingReview)
            .collect()
    }

    /// Submit an upsert. User submissions apply immediately; any other
    /// actor is queued for review.
    pub fn submit(
        &mut self,
        id: &str,
        actor: &str,
        payload: WorldClaimPayload,
        now: DateTime<Utc>,
    ) -> std::result::Result<WorldProposalState, WorldGovernanceError> {
        if self.state.proposals.iter().any(|item| item.id == id) {
            return Err(WorldGovernanceError::AlreadyExists { id: id.into() });
        }

        if actor == USER_ACTOR {
            let claim = payload
                .to_claim(now)
                .map_err(|error| WorldGovernanceError::Protected {
                    reason: error.to_string(),
                })?;
            let mut store = WorldStore::load(&self.world_path).map_err(storage_error)?;
            store.governed_upsert(USER_ACTOR, claim).map_err(|error| {
                WorldGovernanceError::Protected {
                    reason: error.to_string(),
                }
            })?;
            store.save().map_err(storage_error)?;

            self.state.proposals.push(WorldUpsertProposal {
                id: id.to_string(),
                created_at: now,
                created_by: actor.to_string(),
                claim: payload.clone(),
                state: WorldProposalState::Applied,
                decided_at: Some(now),
                decision_reason: Some("user direct edit".to_string()),
            });
            self.persist_state()?;
            self.append_ledger(now, actor, "submit_apply", id, &payload, "applied");
            return Ok(WorldProposalState::Applied);
        }

        self.state.proposals.push(WorldUpsertProposal {
            id: id.to_string(),
            created_at: now,
            created_by: actor.to_string(),
            claim: payload.clone(),
            state: WorldProposalState::PendingReview,
            decided_at: None,
            decision_reason: None,
        });
        self.persist_state()?;
        self.append_ledger(now, actor, "submit", id, &payload, "pending_review");
        Ok(WorldProposalState::PendingReview)
    }

    /// Approve a pending proposal: protection is re-checked against the
    /// current WORLD.MD before writing.
    pub fn apply(
        &mut self,
        id: &str,
        approver: &str,
        now: DateTime<Utc>,
    ) -> std::result::Result<(), WorldGovernanceError> {
        let position = self.position_of(id)?;
        if self.state.proposals[position].state != WorldProposalState::PendingReview {
            return Err(WorldGovernanceError::NotPending { id: id.into() });
        }

        let payload = self.state.proposals[position].claim.clone();
        let created_by = self.state.proposals[position].created_by.clone();
        let claim = payload
            .to_claim(now)
            .map_err(|error| WorldGovernanceError::Protected {
                reason: error.to_string(),
            })?;

        let mut store = WorldStore::load(&self.world_path).map_err(storage_error)?;
        store.governed_upsert(&created_by, claim).map_err(|error| {
            WorldGovernanceError::Protected {
                reason: error.to_string(),
            }
        })?;
        store.save().map_err(storage_error)?;

        let proposal = &mut self.state.proposals[position];
        proposal.state = WorldProposalState::Applied;
        proposal.decided_at = Some(now);
        proposal.decision_reason = Some(format!("approved by {approver}"));
        self.persist_state()?;
        self.append_ledger(now, approver, "apply", id, &payload, "applied");
        Ok(())
    }

    /// Reject a pending proposal without touching WORLD.MD.
    pub fn reject(
        &mut self,
        id: &str,
        approver: &str,
        reason: &str,
        now: DateTime<Utc>,
    ) -> std::result::Result<(), WorldGovernanceError> {
        let position = self.position_of(id)?;
        if self.state.proposals[position].state != WorldProposalState::PendingReview {
            return Err(WorldGovernanceError::NotPending { id: id.into() });
        }

        let payload = self.state.proposals[position].claim.clone();
        let proposal = &mut self.state.proposals[position];
        proposal.state = WorldProposalState::Rejected;
        proposal.decided_at = Some(now);
        proposal.decision_reason = Some(format!("rejected by {approver}: {reason}"));
        self.persist_state()?;
        self.append_ledger(now, approver, "reject", id, &payload, "rejected");
        Ok(())
    }

    fn position_of(&self, id: &str) -> std::result::Result<usize, WorldGovernanceError> {
        self.state
            .proposals
            .iter()
            .position(|item| item.id == id)
            .ok_or_else(|| WorldGovernanceError::NotFound { id: id.into() })
    }

    fn persist_state(&self) -> std::result::Result<(), WorldGovernanceError> {
        crate::atomic_write_json(&self.proposals_path, &self.state).map_err(storage_error)
    }

    fn append_ledger(
        &self,
        now: DateTime<Utc>,
        actor: &str,
        action: &str,
        proposal_id: &str,
        payload: &WorldClaimPayload,
        result: &str,
    ) {
        let entry = WorldLedgerEntry {
            at: now,
            actor: actor.to_string(),
            action: action.to_string(),
            proposal_id: proposal_id.to_string(),
            domain: payload.domain.clone(),
            title: payload.title.clone(),
            result: result.to_string(),
        };
        if let Ok(line) = serde_json::to_string(&entry) {
            let mut raw = fs::read_to_string(&self.ledger_path).unwrap_or_default();
            raw.push_str(&line);
            raw.push('\n');
            let _ = atomic_write(&self.ledger_path, raw.as_bytes());
        }
    }
}

fn storage_error(error: LaputaError) -> WorldGovernanceError {
    WorldGovernanceError::Storage {
        reason: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::world::ClaimStatus;

    fn payload(title: &str) -> WorldClaimPayload {
        WorldClaimPayload {
            domain: "env".to_string(),
            title: title.to_string(),
            status: "observed".to_string(),
            confidence: "medium".to_string(),
            scopes: vec!["dev".to_string()],
            source: "autodream".to_string(),
            text: "observed fact".to_string(),
        }
    }

    fn open(temp: &tempfile::TempDir) -> WorldGovernance {
        let dir = temp.path().join("cognitive");
        fs::create_dir_all(&dir).unwrap();
        WorldGovernance::open(dir).unwrap()
    }

    #[test]
    fn autodream_submission_queues_for_review_and_applies_after_approval() {
        let temp = tempfile::tempdir().unwrap();
        let mut governance = open(&temp);
        let now = Utc::now();

        let state = governance
            .submit("world-1", "autodream", payload("dev machine"), now)
            .unwrap();
        assert_eq!(state, WorldProposalState::PendingReview);

        // Not in WORLD.MD before approval.
        let world_path = temp.path().join("cognitive").join("WORLD.MD");
        let store = WorldStore::load(&world_path).unwrap();
        assert_eq!(store.total(), 0);

        governance.apply("world-1", "human", now).unwrap();
        let store = WorldStore::load(&world_path).unwrap();
        assert_eq!(store.total(), 1);
        assert_eq!(store.claims()[0].title, "dev machine");
        assert_eq!(store.claims()[0].source, "autodream");

        // Ledger recorded both events.
        let ledger =
            fs::read_to_string(temp.path().join("cognitive").join("world-ledger.jsonl")).unwrap();
        assert!(ledger.contains("\"action\":\"submit\""));
        assert!(ledger.contains("\"action\":\"apply\""));

        // Re-applying is refused.
        assert!(matches!(
            governance.apply("world-1", "human", now),
            Err(WorldGovernanceError::NotPending { .. })
        ));
    }

    #[test]
    fn user_submission_applies_immediately() {
        let temp = tempfile::tempdir().unwrap();
        let mut governance = open(&temp);
        let now = Utc::now();

        let mut claim = payload("user fact");
        claim.source = "user".to_string();
        claim.status = "confirmed".to_string();
        let state = governance.submit("world-2", "user", claim, now).unwrap();
        assert_eq!(state, WorldProposalState::Applied);

        let world_path = temp.path().join("cognitive").join("WORLD.MD");
        let store = WorldStore::load(&world_path).unwrap();
        assert_eq!(store.total(), 1);
        assert!(store.claims()[0].is_user_confirmed());
    }

    #[test]
    fn apply_rechecks_user_confirmed_protection() {
        let temp = tempfile::tempdir().unwrap();
        let mut governance = open(&temp);
        let now = Utc::now();
        let world_path = temp.path().join("cognitive").join("WORLD.MD");

        // AutoDream queues a claim, then the user confirms the same claim
        // directly before review happens.
        governance
            .submit("world-3", "autodream", payload("contested"), now)
            .unwrap();
        let mut store = WorldStore::load(&world_path).unwrap();
        store
            .governed_upsert(
                USER_ACTOR,
                WorldClaim {
                    domain: "env".to_string(),
                    title: "contested".to_string(),
                    status: ClaimStatus::Confirmed,
                    confidence: "high".to_string(),
                    scopes: vec!["dev".to_string()],
                    source: USER_ACTOR.to_string(),
                    updated: now,
                    text: "user truth".to_string(),
                },
            )
            .unwrap();
        store.save().unwrap();

        // Approval must fail closed against the now-protected claim.
        assert!(matches!(
            governance.apply("world-3", "human", now),
            Err(WorldGovernanceError::Protected { .. })
        ));

        // Rejection still works and leaves WORLD.MD untouched.
        governance
            .reject("world-3", "human", "superseded by user edit", now)
            .unwrap();
        let store = WorldStore::load(&world_path).unwrap();
        assert_eq!(store.claims()[0].text, "user truth");
    }

    #[test]
    fn duplicate_ids_and_unknown_ids_are_refused() {
        let temp = tempfile::tempdir().unwrap();
        let mut governance = open(&temp);
        let now = Utc::now();

        governance
            .submit("world-4", "autodream", payload("dup"), now)
            .unwrap();
        assert!(matches!(
            governance.submit("world-4", "autodream", payload("dup"), now),
            Err(WorldGovernanceError::AlreadyExists { .. })
        ));
        assert!(matches!(
            governance.apply("missing", "human", now),
            Err(WorldGovernanceError::NotFound { .. })
        ));
    }

    #[test]
    fn pending_queue_survives_reload() {
        let temp = tempfile::tempdir().unwrap();
        let now = Utc::now();
        {
            let mut governance = open(&temp);
            governance
                .submit("world-5", "autodream", payload("durable"), now)
                .unwrap();
        }
        let reopened = open(&temp);
        assert_eq!(reopened.pending().len(), 1);
        assert_eq!(reopened.pending()[0].id, "world-5");
    }
}

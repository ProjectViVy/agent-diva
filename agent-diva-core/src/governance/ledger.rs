//! Append-only SQLite approval ledger and derived state machine.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use super::{
    ApprovalGrant, ApprovalReceipt, ApprovalRequest, AuditCorrelation, Capability, ContentDigest,
    Decision, EvidenceSource, GovernanceSubject, GovernanceSubjectKind, GovernanceValidationError,
    ResourceScope, RiskClass,
};

/// Evidence metadata safe to persist in the approval ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalEvidenceRecord {
    pub id: String,
    pub source: EvidenceSource,
    pub uri: String,
    pub hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Payload-free request metadata persisted by the approval ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub correlation: AuditCorrelation,
    pub subject: GovernanceSubject,
    pub capability: Capability,
    pub resource: ResourceScope,
    pub risk: RiskClass,
    pub content_digest: ContentDigest,
    pub policy_version: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub evidence_refs: Vec<ApprovalEvidenceRecord>,
}

impl ApprovalRecord {
    /// Create a persistence-safe record without retaining the domain payload or excerpts.
    pub fn from_request<P>(
        request: &ApprovalRequest<P>,
    ) -> Result<Self, GovernanceValidationError> {
        request.validate()?;
        Ok(Self {
            correlation: request.correlation.clone(),
            subject: request.subject.clone(),
            capability: request.capability.clone(),
            resource: request.resource.clone(),
            risk: request.risk.clone(),
            content_digest: request.content_digest.clone(),
            policy_version: request.policy_version.clone(),
            created_at: request.created_at,
            expires_at: request.expires_at,
            evidence_refs: request
                .evidence_refs
                .iter()
                .map(|evidence| ApprovalEvidenceRecord {
                    id: evidence.id.clone(),
                    source: evidence.source.clone(),
                    uri: evidence.uri.clone(),
                    hash: evidence.hash.clone(),
                    created_at: evidence.created_at,
                })
                .collect(),
        })
    }

    fn validate(&self) -> Result<(), GovernanceValidationError> {
        let request = ApprovalRequest {
            correlation: self.correlation.clone(),
            subject: self.subject.clone(),
            capability: self.capability.clone(),
            resource: self.resource.clone(),
            risk: self.risk.clone(),
            content_digest: self.content_digest.clone(),
            policy_version: self.policy_version.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
            evidence_refs: Vec::new(),
            payload: (),
        };
        request.validate()
    }

    /// Verify that an approve-once receipt authorizes this persisted request.
    pub fn validate_approve_once(
        &self,
        receipt: &ApprovalReceipt,
    ) -> Result<(), GovernanceValidationError> {
        self.validate()?;
        receipt.validate()?;
        if receipt.decision != Decision::Allow {
            return Err(GovernanceValidationError::DecisionDoesNotAllow);
        }
        if receipt.grant != ApprovalGrant::Once {
            return Err(GovernanceValidationError::GrantIsNotOnce);
        }
        if receipt.request_id != self.correlation.request_id {
            return Err(GovernanceValidationError::RequestIdMismatch);
        }
        if receipt.content_digest != self.content_digest {
            return Err(GovernanceValidationError::ContentDigestMismatch);
        }
        if receipt.policy_version != self.policy_version {
            return Err(GovernanceValidationError::PolicyVersionMismatch);
        }
        if receipt.capability != self.capability {
            return Err(GovernanceValidationError::CapabilityMismatch);
        }
        if receipt.resource != self.resource {
            return Err(GovernanceValidationError::ResourceScopeMismatch);
        }
        Ok(())
    }
}

/// Append-only event category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalLedgerEventKind {
    Requested,
    Allowed,
    Denied,
    Revoked,
    Consumed,
    Expired,
}

/// Immutable event persisted in the ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalLedgerEvent {
    pub event_id: String,
    pub request_id: String,
    pub version: u64,
    pub idempotency_key: String,
    pub operation_fingerprint: String,
    pub occurred_at: DateTime<Utc>,
    pub kind: ApprovalLedgerEventKind,
    pub request: Option<ApprovalRecord>,
    pub receipt: Option<ApprovalReceipt>,
    pub actor: Option<GovernanceSubject>,
}

/// Current status derived exclusively from ledger events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Allowed,
    Denied,
    Revoked,
    Consumed,
    Expired,
}

/// Current approval aggregate derived from the append-only event stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalState {
    pub request: ApprovalRecord,
    pub status: ApprovalStatus,
    pub version: u64,
    pub receipt: Option<ApprovalReceipt>,
}

/// One bounded, request-id ordered page of replayed approval aggregates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalStatePage {
    pub states: Vec<ApprovalState>,
    pub next_cursor: Option<String>,
}

/// One immutable ledger event paired with its durable reconnect cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalEventItem {
    pub cursor: String,
    pub event: ApprovalLedgerEvent,
}

/// One bounded, append-order page of immutable governance events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalEventPage {
    pub events: Vec<ApprovalEventItem>,
    pub next_cursor: Option<String>,
}

/// Stable errors returned by ledger operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ApprovalLedgerError {
    #[error("governance validation failed: {0}")]
    Validation(GovernanceValidationError),
    #[error("approval request not found")]
    NotFound,
    #[error("approval ledger version conflict")]
    VersionConflict,
    #[error("approval ledger idempotency key conflicts with another operation")]
    IdempotencyConflict,
    #[error("approval state transition is not allowed")]
    InvalidTransition,
    #[error("approval request or receipt has expired")]
    Expired,
    #[error("approve-once receipt has already been consumed")]
    AlreadyConsumed,
    #[error("approval ledger persistence failed: {0}")]
    Persistence(String),
}

/// Stable wire-safe classification for [`ApprovalLedgerError`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalLedgerErrorCode {
    Validation,
    NotFound,
    VersionConflict,
    IdempotencyConflict,
    InvalidTransition,
    Expired,
    AlreadyConsumed,
    Persistence,
}

impl ApprovalLedgerError {
    pub fn code(&self) -> ApprovalLedgerErrorCode {
        match self {
            Self::Validation(_) => ApprovalLedgerErrorCode::Validation,
            Self::NotFound => ApprovalLedgerErrorCode::NotFound,
            Self::VersionConflict => ApprovalLedgerErrorCode::VersionConflict,
            Self::IdempotencyConflict => ApprovalLedgerErrorCode::IdempotencyConflict,
            Self::InvalidTransition => ApprovalLedgerErrorCode::InvalidTransition,
            Self::Expired => ApprovalLedgerErrorCode::Expired,
            Self::AlreadyConsumed => ApprovalLedgerErrorCode::AlreadyConsumed,
            Self::Persistence(_) => ApprovalLedgerErrorCode::Persistence,
        }
    }
}

impl From<GovernanceValidationError> for ApprovalLedgerError {
    fn from(value: GovernanceValidationError) -> Self {
        Self::Validation(value)
    }
}

/// Persistent approval ledger behavior.
#[async_trait]
pub trait GovernanceLedger: Send + Sync {
    async fn submit(
        &self,
        request: ApprovalRecord,
        idempotency_key: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<ApprovalState, ApprovalLedgerError>;

    async fn decide(
        &self,
        request_id: &str,
        expected_version: u64,
        idempotency_key: &str,
        receipt: ApprovalReceipt,
    ) -> Result<ApprovalState, ApprovalLedgerError>;

    async fn revoke(
        &self,
        request_id: &str,
        expected_version: u64,
        idempotency_key: &str,
        actor: GovernanceSubject,
        occurred_at: DateTime<Utc>,
    ) -> Result<ApprovalState, ApprovalLedgerError>;

    async fn consume_once(
        &self,
        request_id: &str,
        expected_version: u64,
        idempotency_key: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<ApprovalState, ApprovalLedgerError>;

    async fn expire(
        &self,
        request_id: &str,
        expected_version: u64,
        idempotency_key: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<ApprovalState, ApprovalLedgerError>;

    async fn state(
        &self,
        request_id: &str,
        evaluated_at: DateTime<Utc>,
    ) -> Result<ApprovalState, ApprovalLedgerError>;

    /// Replay a stable page of aggregates at `evaluated_at`.
    async fn states_page(
        &self,
        after_request_id: Option<&str>,
        limit: u32,
        evaluated_at: DateTime<Utc>,
    ) -> Result<ApprovalStatePage, ApprovalLedgerError>;

    /// Read committed events after an opaque durable cursor in append order.
    async fn events_page(
        &self,
        after_cursor: Option<&str>,
        limit: u32,
    ) -> Result<ApprovalEventPage, ApprovalLedgerError>;

    async fn events(
        &self,
        request_id: &str,
    ) -> Result<Vec<ApprovalLedgerEvent>, ApprovalLedgerError>;
}

/// SQLite implementation storing immutable JSON events.
#[derive(Clone)]
pub struct SqliteGovernanceLedger {
    pool: SqlitePool,
}

impl SqliteGovernanceLedger {
    pub async fn new(pool: SqlitePool) -> Result<Self, ApprovalLedgerError> {
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS governance_ledger_events (
                event_id TEXT PRIMARY KEY,
                request_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                idempotency_key TEXT NOT NULL UNIQUE,
                operation_fingerprint TEXT NOT NULL,
                occurred_at TEXT NOT NULL,
                event_json TEXT NOT NULL,
                UNIQUE(request_id, version)
            )"#,
        )
        .execute(&pool)
        .await
        .map_err(persistence)?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS governance_ledger_request_idx \
             ON governance_ledger_events(request_id, version)",
        )
        .execute(&pool)
        .await
        .map_err(persistence)?;
        sqlx::query(
            "CREATE TRIGGER IF NOT EXISTS governance_ledger_no_update \
             BEFORE UPDATE ON governance_ledger_events \
             BEGIN SELECT RAISE(ABORT, 'governance ledger is append-only'); END",
        )
        .execute(&pool)
        .await
        .map_err(persistence)?;
        sqlx::query(
            "CREATE TRIGGER IF NOT EXISTS governance_ledger_no_delete \
             BEFORE DELETE ON governance_ledger_events \
             BEGIN SELECT RAISE(ABORT, 'governance ledger is append-only'); END",
        )
        .execute(&pool)
        .await
        .map_err(persistence)?;
        Ok(Self { pool })
    }

    async fn replay(
        &self,
        request_id: &str,
        evaluated_at: DateTime<Utc>,
    ) -> Result<ApprovalState, ApprovalLedgerError> {
        derive_state(&self.events(request_id).await?, evaluated_at)
    }

    async fn idempotent_event(
        &self,
        key: &str,
        fingerprint: &str,
    ) -> Result<Option<ApprovalLedgerEvent>, ApprovalLedgerError> {
        let row = sqlx::query(
            "SELECT event_json, operation_fingerprint FROM governance_ledger_events \
             WHERE idempotency_key = ?",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(persistence)?;
        let Some(row) = row else {
            return Ok(None);
        };
        if row.get::<String, _>("operation_fingerprint") != fingerprint {
            return Err(ApprovalLedgerError::IdempotencyConflict);
        }
        Ok(Some(decode_event(row.get("event_json"))?))
    }

    async fn append(&self, event: &ApprovalLedgerEvent) -> Result<(), ApprovalLedgerError> {
        let json = serde_json::to_string(event).map_err(|error| {
            ApprovalLedgerError::Persistence(format!("serialize event: {error}"))
        })?;
        let result = sqlx::query(
            "INSERT INTO governance_ledger_events \
             (event_id, request_id, version, idempotency_key, operation_fingerprint, occurred_at, event_json) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&event.event_id)
        .bind(&event.request_id)
        .bind(event.version as i64)
        .bind(&event.idempotency_key)
        .bind(&event.operation_fingerprint)
        .bind(event.occurred_at.to_rfc3339())
        .bind(json)
        .execute(&self.pool)
        .await;
        match result {
            Ok(_) => Ok(()),
            Err(error) if is_unique_violation(&error) => {
                if self
                    .idempotent_event(&event.idempotency_key, &event.operation_fingerprint)
                    .await?
                    .is_some()
                {
                    Ok(())
                } else {
                    Err(ApprovalLedgerError::VersionConflict)
                }
            }
            Err(error) => Err(persistence(error)),
        }
    }

    async fn replay_idempotent(
        &self,
        key: &str,
        fingerprint: &str,
        evaluated_at: DateTime<Utc>,
    ) -> Result<Option<ApprovalState>, ApprovalLedgerError> {
        let Some(event) = self.idempotent_event(key, fingerprint).await? else {
            return Ok(None);
        };
        Ok(Some(self.replay(&event.request_id, evaluated_at).await?))
    }

    async fn events_page(
        &self,
        after_cursor: Option<&str>,
        limit: u32,
    ) -> Result<ApprovalEventPage, ApprovalLedgerError> {
        if limit == 0 || limit > 1_000 {
            return Err(GovernanceValidationError::InvalidPageLimit.into());
        }
        let after_rowid = match after_cursor {
            Some(cursor) => cursor
                .parse::<i64>()
                .map_err(|_| GovernanceValidationError::InvalidCursor)?,
            None => 0,
        };
        let rows = sqlx::query(
            "SELECT rowid AS event_cursor, event_json FROM governance_ledger_events \
             WHERE rowid > ? ORDER BY rowid ASC LIMIT ?",
        )
        .bind(after_rowid)
        .bind(i64::from(limit) + 1)
        .fetch_all(&self.pool)
        .await
        .map_err(persistence)?;
        let has_more = rows.len() > limit as usize;
        let events = rows
            .into_iter()
            .take(limit as usize)
            .map(|row| {
                Ok(ApprovalEventItem {
                    cursor: row.get::<i64, _>("event_cursor").to_string(),
                    event: decode_event(row.get("event_json"))?,
                })
            })
            .collect::<Result<Vec<_>, ApprovalLedgerError>>()?;
        let next_cursor = has_more
            .then(|| events.last().map(|item| item.cursor.clone()))
            .flatten();
        Ok(ApprovalEventPage {
            events,
            next_cursor,
        })
    }
}

#[async_trait]
impl GovernanceLedger for SqliteGovernanceLedger {
    async fn submit(
        &self,
        request: ApprovalRecord,
        idempotency_key: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<ApprovalState, ApprovalLedgerError> {
        request.validate()?;
        require_key(idempotency_key)?;
        if occurred_at < request.created_at || occurred_at >= request.expires_at {
            return Err(ApprovalLedgerError::Expired);
        }
        let fingerprint = operation_fingerprint("submit", &request)?;
        if let Some(state) = self
            .replay_idempotent(idempotency_key, &fingerprint, occurred_at)
            .await?
        {
            return Ok(state);
        }
        let event = ApprovalLedgerEvent {
            event_id: Uuid::new_v4().to_string(),
            request_id: request.correlation.request_id.clone(),
            version: 1,
            idempotency_key: idempotency_key.to_string(),
            operation_fingerprint: fingerprint,
            occurred_at,
            kind: ApprovalLedgerEventKind::Requested,
            request: Some(request),
            receipt: None,
            actor: None,
        };
        self.append(&event).await?;
        self.replay(&event.request_id, occurred_at).await
    }

    async fn decide(
        &self,
        request_id: &str,
        expected_version: u64,
        idempotency_key: &str,
        receipt: ApprovalReceipt,
    ) -> Result<ApprovalState, ApprovalLedgerError> {
        require_key(idempotency_key)?;
        receipt.validate()?;
        if !matches!(receipt.decision, Decision::Allow | Decision::Deny) {
            return Err(ApprovalLedgerError::InvalidTransition);
        }
        let fingerprint =
            operation_fingerprint("decide", &(request_id, expected_version, &receipt))?;
        if let Some(state) = self
            .replay_idempotent(idempotency_key, &fingerprint, receipt.decided_at)
            .await?
        {
            return Ok(state);
        }
        let current = self.replay(request_id, receipt.decided_at).await?;
        require_version(&current, expected_version)?;
        if current.status == ApprovalStatus::Expired {
            return Err(ApprovalLedgerError::Expired);
        }
        if current.status != ApprovalStatus::Pending {
            return Err(ApprovalLedgerError::InvalidTransition);
        }
        validate_receipt_binding(&current.request, &receipt)?;
        if receipt.decided_at < current.request.created_at
            || receipt.decided_at >= current.request.expires_at
            || receipt.expires_at <= receipt.decided_at
        {
            return Err(ApprovalLedgerError::Expired);
        }
        let kind = if receipt.decision == Decision::Allow {
            ApprovalLedgerEventKind::Allowed
        } else {
            ApprovalLedgerEventKind::Denied
        };
        let event = ApprovalLedgerEvent {
            event_id: Uuid::new_v4().to_string(),
            request_id: request_id.to_string(),
            version: expected_version + 1,
            idempotency_key: idempotency_key.to_string(),
            operation_fingerprint: fingerprint,
            occurred_at: receipt.decided_at,
            kind,
            request: None,
            actor: Some(receipt.decided_by.clone()),
            receipt: Some(receipt),
        };
        self.append(&event).await?;
        self.replay(request_id, event.occurred_at).await
    }

    async fn revoke(
        &self,
        request_id: &str,
        expected_version: u64,
        idempotency_key: &str,
        actor: GovernanceSubject,
        occurred_at: DateTime<Utc>,
    ) -> Result<ApprovalState, ApprovalLedgerError> {
        require_key(idempotency_key)?;
        let fingerprint = operation_fingerprint(
            "revoke",
            &(request_id, expected_version, &actor, occurred_at),
        )?;
        if let Some(state) = self
            .replay_idempotent(idempotency_key, &fingerprint, occurred_at)
            .await?
        {
            return Ok(state);
        }
        validate_actor(&actor)?;
        let current = self.replay(request_id, occurred_at).await?;
        require_version(&current, expected_version)?;
        if !matches!(
            current.status,
            ApprovalStatus::Pending | ApprovalStatus::Allowed
        ) {
            return if current.status == ApprovalStatus::Consumed {
                Err(ApprovalLedgerError::AlreadyConsumed)
            } else {
                Err(ApprovalLedgerError::InvalidTransition)
            };
        }
        self.append(&ApprovalLedgerEvent {
            event_id: Uuid::new_v4().to_string(),
            request_id: request_id.to_string(),
            version: expected_version + 1,
            idempotency_key: idempotency_key.to_string(),
            operation_fingerprint: fingerprint,
            occurred_at,
            kind: ApprovalLedgerEventKind::Revoked,
            request: None,
            receipt: None,
            actor: Some(actor),
        })
        .await?;
        self.replay(request_id, occurred_at).await
    }

    async fn consume_once(
        &self,
        request_id: &str,
        expected_version: u64,
        idempotency_key: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<ApprovalState, ApprovalLedgerError> {
        require_key(idempotency_key)?;
        let fingerprint =
            operation_fingerprint("consume_once", &(request_id, expected_version, occurred_at))?;
        if let Some(state) = self
            .replay_idempotent(idempotency_key, &fingerprint, occurred_at)
            .await?
        {
            return Ok(state);
        }
        let current = self.replay(request_id, occurred_at).await?;
        require_version(&current, expected_version)?;
        if current.status == ApprovalStatus::Consumed {
            return Err(ApprovalLedgerError::AlreadyConsumed);
        }
        if current.status == ApprovalStatus::Expired {
            return Err(ApprovalLedgerError::Expired);
        }
        let receipt = current
            .receipt
            .as_ref()
            .ok_or(ApprovalLedgerError::InvalidTransition)?;
        if current.status != ApprovalStatus::Allowed || receipt.grant != ApprovalGrant::Once {
            return Err(ApprovalLedgerError::InvalidTransition);
        }
        validate_receipt_binding(&current.request, receipt)?;
        self.append(&ApprovalLedgerEvent {
            event_id: Uuid::new_v4().to_string(),
            request_id: request_id.to_string(),
            version: expected_version + 1,
            idempotency_key: idempotency_key.to_string(),
            operation_fingerprint: fingerprint,
            occurred_at,
            kind: ApprovalLedgerEventKind::Consumed,
            request: None,
            receipt: None,
            actor: None,
        })
        .await?;
        self.replay(request_id, occurred_at).await
    }

    async fn expire(
        &self,
        request_id: &str,
        expected_version: u64,
        idempotency_key: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<ApprovalState, ApprovalLedgerError> {
        require_key(idempotency_key)?;
        let fingerprint =
            operation_fingerprint("expire", &(request_id, expected_version, occurred_at))?;
        if let Some(state) = self
            .replay_idempotent(idempotency_key, &fingerprint, occurred_at)
            .await?
        {
            return Ok(state);
        }
        let current = self.replay(request_id, occurred_at).await?;
        require_version(&current, expected_version)?;
        if current.status != ApprovalStatus::Expired {
            return Err(ApprovalLedgerError::InvalidTransition);
        }
        self.append(&ApprovalLedgerEvent {
            event_id: Uuid::new_v4().to_string(),
            request_id: request_id.to_string(),
            version: expected_version + 1,
            idempotency_key: idempotency_key.to_string(),
            operation_fingerprint: fingerprint,
            occurred_at,
            kind: ApprovalLedgerEventKind::Expired,
            request: None,
            receipt: None,
            actor: None,
        })
        .await?;
        self.replay(request_id, occurred_at).await
    }

    async fn state(
        &self,
        request_id: &str,
        evaluated_at: DateTime<Utc>,
    ) -> Result<ApprovalState, ApprovalLedgerError> {
        self.replay(request_id, evaluated_at).await
    }

    async fn states_page(
        &self,
        after_request_id: Option<&str>,
        limit: u32,
        evaluated_at: DateTime<Utc>,
    ) -> Result<ApprovalStatePage, ApprovalLedgerError> {
        if limit == 0 || limit > 1_000 {
            return Err(GovernanceValidationError::InvalidPageLimit.into());
        }
        let cursor = after_request_id.unwrap_or("");
        let rows = sqlx::query(
            "SELECT request_id FROM governance_ledger_events \
             WHERE version = 1 AND request_id > ? \
             ORDER BY request_id ASC LIMIT ?",
        )
        .bind(cursor)
        .bind(i64::from(limit) + 1)
        .fetch_all(&self.pool)
        .await
        .map_err(persistence)?;
        let has_more = rows.len() > limit as usize;
        let ids: Vec<String> = rows
            .into_iter()
            .take(limit as usize)
            .map(|row| row.get("request_id"))
            .collect();
        let next_cursor = has_more.then(|| ids.last().cloned()).flatten();
        let mut states = Vec::with_capacity(ids.len());
        for request_id in ids {
            states.push(self.replay(&request_id, evaluated_at).await?);
        }
        Ok(ApprovalStatePage {
            states,
            next_cursor,
        })
    }

    async fn events_page(
        &self,
        after_cursor: Option<&str>,
        limit: u32,
    ) -> Result<ApprovalEventPage, ApprovalLedgerError> {
        SqliteGovernanceLedger::events_page(self, after_cursor, limit).await
    }

    async fn events(
        &self,
        request_id: &str,
    ) -> Result<Vec<ApprovalLedgerEvent>, ApprovalLedgerError> {
        let rows = sqlx::query(
            "SELECT event_json FROM governance_ledger_events \
             WHERE request_id = ? ORDER BY version ASC",
        )
        .bind(request_id)
        .fetch_all(&self.pool)
        .await
        .map_err(persistence)?;
        rows.into_iter()
            .map(|row| decode_event(row.get("event_json")))
            .collect()
    }
}

fn derive_state(
    events: &[ApprovalLedgerEvent],
    evaluated_at: DateTime<Utc>,
) -> Result<ApprovalState, ApprovalLedgerError> {
    let first = events.first().ok_or(ApprovalLedgerError::NotFound)?;
    if first.version != 1 || first.kind != ApprovalLedgerEventKind::Requested {
        return Err(ApprovalLedgerError::Persistence(
            "ledger does not begin with requested version 1".into(),
        ));
    }
    let request = first.request.clone().ok_or_else(|| {
        ApprovalLedgerError::Persistence("requested event has no request record".into())
    })?;
    request.validate().map_err(|error| {
        ApprovalLedgerError::Persistence(format!("stored request is invalid: {error}"))
    })?;
    if first.request_id != request.correlation.request_id {
        return Err(ApprovalLedgerError::Persistence(
            "requested event id does not match its request record".into(),
        ));
    }
    let mut state = ApprovalState {
        request,
        status: ApprovalStatus::Pending,
        version: 1,
        receipt: None,
    };
    let mut previous_at = first.occurred_at;
    for event in &events[1..] {
        if event.request_id != state.request.correlation.request_id {
            return Err(ApprovalLedgerError::Persistence(
                "ledger event request id does not match aggregate".into(),
            ));
        }
        if event.version != state.version + 1 {
            return Err(ApprovalLedgerError::Persistence(
                "ledger event versions are not contiguous".into(),
            ));
        }
        if event.occurred_at < previous_at {
            return Err(ApprovalLedgerError::Persistence(
                "ledger event timestamps are not monotonic".into(),
            ));
        }
        previous_at = event.occurred_at;
        state.version = event.version;
        match event.kind {
            ApprovalLedgerEventKind::Allowed => {
                require_replay_status(&state, &[ApprovalStatus::Pending])?;
                validate_stored_receipt(&state.request, event, Decision::Allow)?;
                state.status = ApprovalStatus::Allowed;
                state.receipt = event.receipt.clone();
            }
            ApprovalLedgerEventKind::Denied => {
                require_replay_status(&state, &[ApprovalStatus::Pending])?;
                validate_stored_receipt(&state.request, event, Decision::Deny)?;
                state.status = ApprovalStatus::Denied;
                state.receipt = event.receipt.clone();
            }
            ApprovalLedgerEventKind::Revoked => {
                require_replay_status(&state, &[ApprovalStatus::Pending, ApprovalStatus::Allowed])?;
                state.status = ApprovalStatus::Revoked;
            }
            ApprovalLedgerEventKind::Consumed => {
                require_replay_status(&state, &[ApprovalStatus::Allowed])?;
                state.status = ApprovalStatus::Consumed;
            }
            ApprovalLedgerEventKind::Expired => {
                require_replay_status(&state, &[ApprovalStatus::Pending, ApprovalStatus::Allowed])?;
                state.status = ApprovalStatus::Expired;
            }
            ApprovalLedgerEventKind::Requested => {
                return Err(ApprovalLedgerError::Persistence(
                    "duplicate requested event".into(),
                ))
            }
        }
    }
    if matches!(
        state.status,
        ApprovalStatus::Pending | ApprovalStatus::Allowed
    ) {
        let receipt_expired = state
            .receipt
            .as_ref()
            .is_some_and(|receipt| evaluated_at >= receipt.expires_at);
        if evaluated_at >= state.request.expires_at || receipt_expired {
            state.status = ApprovalStatus::Expired;
        }
    }
    Ok(state)
}

fn validate_stored_receipt(
    request: &ApprovalRecord,
    event: &ApprovalLedgerEvent,
    expected_decision: Decision,
) -> Result<(), ApprovalLedgerError> {
    let receipt = event
        .receipt
        .as_ref()
        .ok_or_else(|| ApprovalLedgerError::Persistence("decision event has no receipt".into()))?;
    receipt.validate().map_err(|error| {
        ApprovalLedgerError::Persistence(format!("stored receipt is invalid: {error}"))
    })?;
    validate_receipt_binding(request, receipt).map_err(|error| {
        ApprovalLedgerError::Persistence(format!("stored receipt binding is invalid: {error}"))
    })?;
    if receipt.decision != expected_decision || receipt.decided_at != event.occurred_at {
        return Err(ApprovalLedgerError::Persistence(
            "decision event does not match its receipt".into(),
        ));
    }
    Ok(())
}

fn require_replay_status(
    state: &ApprovalState,
    allowed: &[ApprovalStatus],
) -> Result<(), ApprovalLedgerError> {
    if allowed.contains(&state.status) {
        Ok(())
    } else {
        Err(ApprovalLedgerError::Persistence(
            "ledger contains an illegal state transition".into(),
        ))
    }
}

fn validate_receipt_binding(
    request: &ApprovalRecord,
    receipt: &ApprovalReceipt,
) -> Result<(), ApprovalLedgerError> {
    if receipt.request_id != request.correlation.request_id {
        return Err(GovernanceValidationError::RequestIdMismatch.into());
    }
    if receipt.content_digest != request.content_digest {
        return Err(GovernanceValidationError::ContentDigestMismatch.into());
    }
    if receipt.policy_version != request.policy_version {
        return Err(GovernanceValidationError::PolicyVersionMismatch.into());
    }
    if receipt.capability != request.capability {
        return Err(GovernanceValidationError::CapabilityMismatch.into());
    }
    if receipt.resource != request.resource {
        return Err(GovernanceValidationError::ResourceScopeMismatch.into());
    }
    Ok(())
}

fn validate_actor(actor: &GovernanceSubject) -> Result<(), ApprovalLedgerError> {
    if actor.kind == GovernanceSubjectKind::Unknown {
        return Err(GovernanceValidationError::UnknownSubject.into());
    }
    if actor.id.trim().is_empty() {
        return Err(GovernanceValidationError::MissingRequiredField("subject.id").into());
    }
    Ok(())
}

fn require_version(state: &ApprovalState, expected: u64) -> Result<(), ApprovalLedgerError> {
    if state.version == expected {
        Ok(())
    } else {
        Err(ApprovalLedgerError::VersionConflict)
    }
}

fn require_key(key: &str) -> Result<(), ApprovalLedgerError> {
    if key.trim().is_empty() {
        Err(ApprovalLedgerError::Validation(
            GovernanceValidationError::MissingRequiredField("idempotency_key"),
        ))
    } else {
        Ok(())
    }
}

fn operation_fingerprint<T: Serialize>(
    operation: &str,
    value: &T,
) -> Result<String, ApprovalLedgerError> {
    serde_json::to_string(&(operation, value))
        .map_err(|error| ApprovalLedgerError::Persistence(format!("serialize operation: {error}")))
}

fn decode_event(json: String) -> Result<ApprovalLedgerEvent, ApprovalLedgerError> {
    serde_json::from_str(&json)
        .map_err(|error| ApprovalLedgerError::Persistence(format!("decode event: {error}")))
}

fn persistence(error: sqlx::Error) -> ApprovalLedgerError {
    ApprovalLedgerError::Persistence(error.to_string())
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database) if database.is_unique_violation())
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::evolution::{EvidenceRef, EvidenceSource};
    use crate::governance::{DigestAlgorithm, GovernanceSubjectKind, ResourceKind};

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 29, 13, 0, 0)
            .single()
            .unwrap()
    }

    fn request(id: &str) -> ApprovalRequest<String> {
        ApprovalRequest {
            correlation: AuditCorrelation {
                request_id: id.into(),
                turn_id: "turn-1".into(),
                session_id: "session-1".into(),
                trace_id: None,
            },
            subject: GovernanceSubject {
                kind: GovernanceSubjectKind::Agent,
                id: "agent-1".into(),
            },
            capability: Capability::CommandExecute,
            resource: ResourceScope {
                workspace_id: "workspace-1".into(),
                session_id: Some("session-1".into()),
                kind: ResourceKind::Command,
                resource_id: "command-1".into(),
                boundary: Some("workspace".into()),
            },
            risk: RiskClass::High,
            content_digest: ContentDigest {
                algorithm: DigestAlgorithm::Sha256,
                value: "digest-1".into(),
            },
            policy_version: "policy-v1".into(),
            created_at: now(),
            expires_at: now() + Duration::minutes(5),
            evidence_refs: vec![EvidenceRef {
                id: "evidence-1".into(),
                source: EvidenceSource::UserInput,
                uri: "session://1".into(),
                excerpt: Some("secret command".into()),
                hash: Some("evidence-hash".into()),
                created_at: now(),
            }],
            payload: "raw secret payload".into(),
        }
    }

    fn receipt(record: &ApprovalRecord, decision: Decision) -> ApprovalReceipt {
        ApprovalReceipt {
            request_id: record.correlation.request_id.clone(),
            content_digest: record.content_digest.clone(),
            policy_version: record.policy_version.clone(),
            capability: record.capability.clone(),
            resource: record.resource.clone(),
            decision,
            decided_by: GovernanceSubject {
                kind: GovernanceSubjectKind::User,
                id: "user-1".into(),
            },
            decided_at: now() + Duration::seconds(10),
            expires_at: now() + Duration::minutes(2),
            grant: ApprovalGrant::Once,
        }
    }

    async fn memory_ledger() -> SqliteGovernanceLedger {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        SqliteGovernanceLedger::new(pool).await.unwrap()
    }

    async fn file_ledger(path: &std::path::Path) -> SqliteGovernanceLedger {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(path)
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        SqliteGovernanceLedger::new(pool).await.unwrap()
    }

    #[test]
    fn approval_record_drops_payload_and_evidence_excerpt() {
        let record = ApprovalRecord::from_request(&request("request-1")).unwrap();
        let json = serde_json::to_string(&record).unwrap();
        assert!(!json.contains("raw secret payload"));
        assert!(!json.contains("secret command"));
        assert!(json.contains("evidence-hash"));
    }

    #[test]
    fn fixed_json_contract_locks_status_and_event_names() {
        let record = ApprovalRecord::from_request(&request("request-1")).unwrap();
        let event = ApprovalLedgerEvent {
            event_id: "event-1".into(),
            request_id: "request-1".into(),
            version: 1,
            idempotency_key: "idem-1".into(),
            operation_fingerprint: "fingerprint".into(),
            occurred_at: now(),
            kind: ApprovalLedgerEventKind::Requested,
            request: Some(record),
            receipt: None,
            actor: None,
        };
        let value = serde_json::to_value(event).unwrap();
        assert_eq!(value["kind"], "requested");
        assert_eq!(
            serde_json::to_value(ApprovalStatus::Pending).unwrap(),
            "pending"
        );
        assert!(value["request"].get("payload").is_none());
        assert!(value["request"]["evidence_refs"][0]
            .get("excerpt")
            .is_none());
        assert_eq!(
            serde_json::to_value(ApprovalLedgerError::VersionConflict.code()).unwrap(),
            "version_conflict"
        );
    }

    #[tokio::test]
    async fn submit_decide_consume_replays_across_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("governance.db");
        let ledger = file_ledger(&path).await;
        let record = ApprovalRecord::from_request(&request("request-1")).unwrap();
        let pending = ledger
            .submit(record.clone(), "submit-1", now() + Duration::seconds(1))
            .await
            .unwrap();
        let allowed = ledger
            .decide(
                "request-1",
                pending.version,
                "allow-1",
                receipt(&record, Decision::Allow),
            )
            .await
            .unwrap();
        ledger
            .consume_once(
                "request-1",
                allowed.version,
                "consume-1",
                now() + Duration::seconds(20),
            )
            .await
            .unwrap();
        drop(ledger);

        let reopened = file_ledger(&path).await;
        let state = reopened
            .state("request-1", now() + Duration::seconds(30))
            .await
            .unwrap();
        assert_eq!(state.status, ApprovalStatus::Consumed);
        assert_eq!(state.version, 3);
        assert_eq!(reopened.events("request-1").await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn idempotency_replays_same_operation_and_rejects_changed_content() {
        let ledger = memory_ledger().await;
        let record = ApprovalRecord::from_request(&request("request-1")).unwrap();
        let first = ledger
            .submit(record.clone(), "idem-1", now() + Duration::seconds(1))
            .await
            .unwrap();
        let replay = ledger
            .submit(record.clone(), "idem-1", now() + Duration::seconds(1))
            .await
            .unwrap();
        assert_eq!(first, replay);

        let mut changed = record;
        changed.content_digest.value = "different".into();
        assert_eq!(
            ledger
                .submit(changed, "idem-1", now() + Duration::seconds(1))
                .await,
            Err(ApprovalLedgerError::IdempotencyConflict)
        );
    }

    #[tokio::test]
    async fn concurrent_allow_uses_version_cas_first_winner() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = file_ledger(&dir.path().join("governance.db")).await;
        let record = ApprovalRecord::from_request(&request("request-1")).unwrap();
        ledger
            .submit(record.clone(), "submit-1", now() + Duration::seconds(1))
            .await
            .unwrap();
        let left = ledger.clone();
        let right = ledger.clone();
        let left_receipt = receipt(&record, Decision::Allow);
        let right_receipt = left_receipt.clone();
        let (left_result, right_result) = tokio::join!(
            left.decide("request-1", 1, "allow-left", left_receipt),
            right.decide("request-1", 1, "allow-right", right_receipt)
        );
        assert_eq!(
            usize::from(left_result.is_ok()) + usize::from(right_result.is_ok()),
            1
        );
        let error = left_result.err().or_else(|| right_result.err()).unwrap();
        assert_eq!(error, ApprovalLedgerError::VersionConflict);
    }

    #[tokio::test]
    async fn concurrent_allow_and_deny_commit_exactly_one_decision() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = file_ledger(&dir.path().join("governance.db")).await;
        let record = ApprovalRecord::from_request(&request("request-1")).unwrap();
        ledger
            .submit(record.clone(), "submit-1", now() + Duration::seconds(1))
            .await
            .unwrap();
        let left = ledger.clone();
        let right = ledger.clone();
        let (allow, deny) = tokio::join!(
            left.decide("request-1", 1, "allow-1", receipt(&record, Decision::Allow)),
            right.decide("request-1", 1, "deny-1", receipt(&record, Decision::Deny))
        );
        assert_eq!(usize::from(allow.is_ok()) + usize::from(deny.is_ok()), 1);
        let state = ledger
            .state("request-1", now() + Duration::seconds(20))
            .await
            .unwrap();
        assert!(matches!(
            state.status,
            ApprovalStatus::Allowed | ApprovalStatus::Denied
        ));
        assert_eq!(state.version, 2);
    }

    #[tokio::test]
    async fn denial_and_revocation_are_terminal_and_block_consumption() {
        let ledger = memory_ledger().await;
        let record = ApprovalRecord::from_request(&request("denied")).unwrap();
        ledger
            .submit(
                record.clone(),
                "submit-denied",
                now() + Duration::seconds(1),
            )
            .await
            .unwrap();
        let denied = ledger
            .decide("denied", 1, "deny-1", receipt(&record, Decision::Deny))
            .await
            .unwrap();
        assert_eq!(denied.status, ApprovalStatus::Denied);
        assert_eq!(
            ledger
                .decide("denied", 2, "allow-late", receipt(&record, Decision::Allow))
                .await,
            Err(ApprovalLedgerError::InvalidTransition)
        );

        let record = ApprovalRecord::from_request(&request("revoked")).unwrap();
        ledger
            .submit(
                record.clone(),
                "submit-revoked",
                now() + Duration::seconds(1),
            )
            .await
            .unwrap();
        ledger
            .decide(
                "revoked",
                1,
                "allow-revoked",
                receipt(&record, Decision::Allow),
            )
            .await
            .unwrap();
        let revoked = ledger
            .revoke(
                "revoked",
                2,
                "revoke-1",
                GovernanceSubject {
                    kind: GovernanceSubjectKind::User,
                    id: "user-1".into(),
                },
                now() + Duration::seconds(20),
            )
            .await
            .unwrap();
        assert_eq!(revoked.status, ApprovalStatus::Revoked);
        assert_eq!(
            ledger
                .consume_once(
                    "revoked",
                    3,
                    "consume-revoked",
                    now() + Duration::seconds(21)
                )
                .await,
            Err(ApprovalLedgerError::InvalidTransition)
        );
    }

    #[tokio::test]
    async fn once_consumption_is_idempotent_but_cannot_be_consumed_twice() {
        let ledger = memory_ledger().await;
        let record = ApprovalRecord::from_request(&request("request-1")).unwrap();
        ledger
            .submit(record.clone(), "submit-1", now() + Duration::seconds(1))
            .await
            .unwrap();
        ledger
            .decide("request-1", 1, "allow-1", receipt(&record, Decision::Allow))
            .await
            .unwrap();
        let first = ledger
            .consume_once("request-1", 2, "consume-1", now() + Duration::seconds(20))
            .await
            .unwrap();
        let replay = ledger
            .consume_once("request-1", 2, "consume-1", now() + Duration::seconds(20))
            .await
            .unwrap();
        assert_eq!(first, replay);
        assert_eq!(
            ledger
                .consume_once("request-1", 3, "consume-2", now() + Duration::seconds(21),)
                .await,
            Err(ApprovalLedgerError::AlreadyConsumed)
        );
    }

    #[tokio::test]
    async fn sqlite_triggers_reject_update_and_delete() {
        let ledger = memory_ledger().await;
        let record = ApprovalRecord::from_request(&request("request-1")).unwrap();
        ledger
            .submit(record, "submit-1", now() + Duration::seconds(1))
            .await
            .unwrap();
        assert!(sqlx::query(
            "UPDATE governance_ledger_events SET version = 9 WHERE request_id = 'request-1'"
        )
        .execute(&ledger.pool)
        .await
        .is_err());
        assert!(
            sqlx::query("DELETE FROM governance_ledger_events WHERE request_id = 'request-1'")
                .execute(&ledger.pool)
                .await
                .is_err()
        );
        assert_eq!(ledger.events("request-1").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn ttl_is_derived_without_mutating_event_history() {
        let ledger = memory_ledger().await;
        let record = ApprovalRecord::from_request(&request("request-1")).unwrap();
        ledger
            .submit(record.clone(), "submit-1", now() + Duration::seconds(1))
            .await
            .unwrap();
        let state = ledger.state("request-1", record.expires_at).await.unwrap();
        assert_eq!(state.status, ApprovalStatus::Expired);
        assert_eq!(state.version, 1);
        assert_eq!(ledger.events("request-1").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn tampered_receipt_and_future_decision_fail_closed() {
        let ledger = memory_ledger().await;
        let record = ApprovalRecord::from_request(&request("request-1")).unwrap();
        ledger
            .submit(record.clone(), "submit-1", now() + Duration::seconds(1))
            .await
            .unwrap();
        let mut tampered = receipt(&record, Decision::Allow);
        tampered.content_digest.value = "tampered".into();
        assert_eq!(
            ledger.decide("request-1", 1, "allow-1", tampered).await,
            Err(ApprovalLedgerError::Validation(
                GovernanceValidationError::ContentDigestMismatch
            ))
        );

        let mut future = receipt(&record, Decision::Allow);
        future.decided_at = record.expires_at;
        future.expires_at = record.expires_at + Duration::minutes(1);
        assert_eq!(
            ledger.decide("request-1", 1, "allow-2", future).await,
            Err(ApprovalLedgerError::Expired)
        );
    }

    #[tokio::test]
    async fn state_transition_table_rejects_terminal_reentry() {
        let terminal_decisions = [("denied", Decision::Deny), ("allowed", Decision::Allow)];
        for (id, decision) in terminal_decisions {
            let ledger = memory_ledger().await;
            let record = ApprovalRecord::from_request(&request(id)).unwrap();
            ledger
                .submit(
                    record.clone(),
                    &format!("submit-{id}"),
                    now() + Duration::seconds(1),
                )
                .await
                .unwrap();
            let decided = ledger
                .decide(
                    id,
                    1,
                    &format!("decide-{id}"),
                    receipt(&record, decision.clone()),
                )
                .await
                .unwrap();
            if decision == Decision::Allow {
                ledger
                    .consume_once(
                        id,
                        decided.version,
                        &format!("consume-{id}"),
                        now() + Duration::seconds(20),
                    )
                    .await
                    .unwrap();
            }
            assert!(matches!(
                ledger
                    .revoke(
                        id,
                        if decision == Decision::Allow { 3 } else { 2 },
                        &format!("revoke-{id}"),
                        GovernanceSubject {
                            kind: GovernanceSubjectKind::User,
                            id: "user-1".into(),
                        },
                        now() + Duration::seconds(30),
                    )
                    .await,
                Err(ApprovalLedgerError::InvalidTransition)
                    | Err(ApprovalLedgerError::AlreadyConsumed)
            ));
        }
    }

    #[tokio::test]
    async fn state_pages_use_stable_request_id_cursor_and_replay_time() {
        let ledger = memory_ledger().await;
        for id in ["request-c", "request-a", "request-b"] {
            ledger
                .submit(
                    ApprovalRecord::from_request(&request(id)).unwrap(),
                    &format!("submit-{id}"),
                    now(),
                )
                .await
                .unwrap();
        }

        let first = ledger.states_page(None, 2, now()).await.unwrap();
        assert_eq!(
            first
                .states
                .iter()
                .map(|state| state.request.correlation.request_id.as_str())
                .collect::<Vec<_>>(),
            ["request-a", "request-b"]
        );
        assert_eq!(first.next_cursor.as_deref(), Some("request-b"));

        let second = ledger
            .states_page(first.next_cursor.as_deref(), 2, now())
            .await
            .unwrap();
        assert_eq!(second.states.len(), 1);
        assert_eq!(second.states[0].request.correlation.request_id, "request-c");
        assert!(second.next_cursor.is_none());

        let expired = ledger
            .states_page(None, 3, now() + Duration::minutes(6))
            .await
            .unwrap();
        assert!(expired
            .states
            .iter()
            .all(|state| state.status == ApprovalStatus::Expired));
        assert_eq!(
            ledger.states_page(None, 0, now()).await,
            Err(ApprovalLedgerError::Validation(
                GovernanceValidationError::InvalidPageLimit
            ))
        );
    }

    #[tokio::test]
    async fn event_pages_replay_append_order_after_durable_cursor() {
        let ledger = memory_ledger().await;
        for id in ["request-b", "request-a"] {
            ledger
                .submit(
                    ApprovalRecord::from_request(&request(id)).unwrap(),
                    &format!("submit-{id}"),
                    now(),
                )
                .await
                .unwrap();
        }

        let first = ledger.events_page(None, 1).await.unwrap();
        assert_eq!(first.events.len(), 1);
        assert_eq!(first.events[0].event.request_id, "request-b");
        assert_eq!(first.next_cursor, Some(first.events[0].cursor.clone()));

        let second = ledger
            .events_page(first.next_cursor.as_deref(), 1)
            .await
            .unwrap();
        assert_eq!(second.events.len(), 1);
        assert_eq!(second.events[0].event.request_id, "request-a");
        assert!(second.next_cursor.is_none());

        assert!(ledger.events_page(Some("not-a-cursor"), 1).await.is_err());
    }
}

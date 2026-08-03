//! Domain-neutral coordination over governance policy and durable approvals.

use std::sync::Arc;

use chrono::{DateTime, Utc};

use super::{
    evaluate_policy, ApprovalLedgerError, ApprovalReceipt, ApprovalRecord, ApprovalRequest,
    ApprovalState, ApprovalStatePage, Decision, GovernanceLedger, GovernanceSubject, PolicyContext,
    PolicyEvaluation,
};

/// Result of coordinating one domain-owned request.
///
/// Only [`Self::Pending`] implies that a durable approval record was appended.
/// Safe policy allows and fail-closed policy denials remain pure evaluations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinatedApproval {
    /// Policy permits the operation without creating a human approval request.
    Allowed { evaluation: PolicyEvaluation },
    /// Policy rejects the operation without creating a human approval request.
    Denied { evaluation: PolicyEvaluation },
    /// Human approval is required and the request is durably pending.
    Pending {
        evaluation: PolicyEvaluation,
        state: Box<ApprovalState>,
    },
}

impl CoordinatedApproval {
    /// Return the policy evaluation shared by every coordination outcome.
    pub fn evaluation(&self) -> &PolicyEvaluation {
        match self {
            Self::Allowed { evaluation }
            | Self::Denied { evaluation }
            | Self::Pending { evaluation, .. } => evaluation,
        }
    }

    /// Return the durable state when this request entered human review.
    pub fn pending_state(&self) -> Option<&ApprovalState> {
        match self {
            Self::Pending { state, .. } => Some(state.as_ref()),
            Self::Allowed { .. } | Self::Denied { .. } => None,
        }
    }
}

/// Shared Plan, Sandbox, and Memory approval coordination boundary.
///
/// Domain payloads are evaluated in memory and reduced to [`ApprovalRecord`]
/// before persistence. Receipt consumption intentionally remains an executor
/// responsibility until GMH-30B.
#[derive(Clone)]
pub struct ApprovalCoordinator {
    ledger: Arc<dyn GovernanceLedger>,
}

impl std::fmt::Debug for ApprovalCoordinator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApprovalCoordinator")
            .finish_non_exhaustive()
    }
}

impl ApprovalCoordinator {
    /// Create a coordinator over the durable governance authority.
    pub fn new(ledger: Arc<dyn GovernanceLedger>) -> Self {
        Self { ledger }
    }

    /// Evaluate a request and durably enqueue it only when a human is required.
    pub async fn coordinate<P>(
        &self,
        request: &ApprovalRequest<P>,
        context: &PolicyContext,
        idempotency_key: &str,
    ) -> Result<CoordinatedApproval, ApprovalLedgerError> {
        let evaluation = evaluate_policy(request, context);
        match evaluation.decision {
            Decision::Allow => Ok(CoordinatedApproval::Allowed { evaluation }),
            Decision::RequireHuman => {
                let record = ApprovalRecord::from_request(request)?;
                let state = self
                    .ledger
                    .submit(record, idempotency_key, context.evaluated_at)
                    .await?;
                Ok(CoordinatedApproval::Pending {
                    evaluation,
                    state: Box::new(state),
                })
            }
            Decision::Deny | Decision::Unknown => Ok(CoordinatedApproval::Denied { evaluation }),
        }
    }

    /// Commit one human allow or deny decision using ledger CAS semantics.
    pub async fn decide(
        &self,
        request_id: &str,
        expected_version: u64,
        idempotency_key: &str,
        receipt: ApprovalReceipt,
    ) -> Result<ApprovalState, ApprovalLedgerError> {
        self.ledger
            .decide(request_id, expected_version, idempotency_key, receipt)
            .await
    }

    /// Read current durable state, deriving expiry at the supplied time.
    pub async fn state(
        &self,
        request_id: &str,
        evaluated_at: DateTime<Utc>,
    ) -> Result<ApprovalState, ApprovalLedgerError> {
        self.ledger.state(request_id, evaluated_at).await
    }

    /// Consume an approve-once receipt before its protected side effect runs.
    pub async fn consume_once(
        &self,
        request_id: &str,
        expected_version: u64,
        idempotency_key: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<ApprovalState, ApprovalLedgerError> {
        self.ledger
            .consume_once(request_id, expected_version, idempotency_key, occurred_at)
            .await
    }

    /// Revoke a pending or unconsumed allowed approval.
    pub async fn revoke(
        &self,
        request_id: &str,
        expected_version: u64,
        idempotency_key: &str,
        actor: GovernanceSubject,
        occurred_at: DateTime<Utc>,
    ) -> Result<ApprovalState, ApprovalLedgerError> {
        self.ledger
            .revoke(
                request_id,
                expected_version,
                idempotency_key,
                actor,
                occurred_at,
            )
            .await
    }

    /// Persist explicit expiry for a pending or unconsumed allowed approval.
    pub async fn expire(
        &self,
        request_id: &str,
        expected_version: u64,
        idempotency_key: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<ApprovalState, ApprovalLedgerError> {
        self.ledger
            .expire(request_id, expected_version, idempotency_key, occurred_at)
            .await
    }

    /// Read a bounded stable page of states replayed at one instant.
    pub async fn states_page(
        &self,
        after_request_id: Option<&str>,
        limit: u32,
        evaluated_at: DateTime<Utc>,
    ) -> Result<ApprovalStatePage, ApprovalLedgerError> {
        self.ledger
            .states_page(after_request_id, limit, evaluated_at)
            .await
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::governance::{
        ApprovalGrant, ApprovalLedgerError, ApprovalStatus, AuditCorrelation, AutonomyLevel,
        Capability, ContentDigest, Decision, DigestAlgorithm, GovernanceSubject,
        GovernanceSubjectKind, PolicyReasonCode, ResourceKind, ResourceScope, RiskClass,
        SqliteGovernanceLedger,
    };

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 3, 12, 0, 0).single().unwrap()
    }

    async fn setup() -> (ApprovalCoordinator, Arc<SqliteGovernanceLedger>) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        let ledger = Arc::new(SqliteGovernanceLedger::new(pool).await.unwrap());
        (ApprovalCoordinator::new(ledger.clone()), ledger)
    }

    fn request(
        id: &str,
        capability: Capability,
        kind: ResourceKind,
        risk: RiskClass,
    ) -> ApprovalRequest<String> {
        ApprovalRequest {
            correlation: AuditCorrelation {
                request_id: id.into(),
                turn_id: format!("turn-{id}"),
                session_id: "session-1".into(),
                trace_id: None,
            },
            subject: GovernanceSubject {
                kind: GovernanceSubjectKind::Agent,
                id: "agent-1".into(),
            },
            capability,
            resource: ResourceScope {
                workspace_id: "workspace-1".into(),
                session_id: Some("session-1".into()),
                kind,
                resource_id: format!("resource-{id}"),
                boundary: None,
            },
            risk,
            content_digest: ContentDigest {
                algorithm: DigestAlgorithm::Sha256,
                value: format!("digest-{id}"),
            },
            policy_version: "policy-v1".into(),
            created_at: now(),
            expires_at: now() + Duration::minutes(5),
            evidence_refs: vec![],
            payload: "secret domain payload".into(),
        }
    }

    fn context(autonomy: AutonomyLevel) -> PolicyContext {
        PolicyContext {
            evaluated_at: now() + Duration::seconds(1),
            autonomy,
            explicit_user_decision: None,
            restrictions: vec![],
            authorizations: vec![],
        }
    }

    fn receipt(request: &ApprovalRequest<String>) -> ApprovalReceipt {
        ApprovalReceipt {
            request_id: request.correlation.request_id.clone(),
            content_digest: request.content_digest.clone(),
            policy_version: request.policy_version.clone(),
            capability: request.capability.clone(),
            resource: request.resource.clone(),
            decision: Decision::Allow,
            decided_by: GovernanceSubject {
                kind: GovernanceSubjectKind::User,
                id: "reviewer".into(),
            },
            decided_at: now() + Duration::seconds(2),
            expires_at: now() + Duration::minutes(2),
            grant: ApprovalGrant::Once,
        }
    }

    #[tokio::test]
    async fn plan_sandbox_and_memory_share_one_pending_boundary() {
        let cases = [
            ("plan", Capability::PlanExecute, ResourceKind::Plan),
            ("sandbox", Capability::CommandExecute, ResourceKind::Command),
            ("memory", Capability::MemoryApply, ResourceKind::Memory),
        ];
        for (id, capability, kind) in cases {
            let (coordinator, _) = setup().await;
            let outcome = coordinator
                .coordinate(
                    &request(id, capability, kind, RiskClass::High),
                    &context(AutonomyLevel::L1),
                    &format!("submit-{id}"),
                )
                .await
                .unwrap();
            assert_eq!(
                outcome.evaluation().reason,
                PolicyReasonCode::HumanApprovalRequired
            );
            assert_eq!(
                outcome.pending_state().unwrap().status,
                ApprovalStatus::Pending
            );
        }
    }

    #[tokio::test]
    async fn safe_allow_and_hard_deny_do_not_create_approval_records() {
        let (coordinator, _) = setup().await;
        let safe = request(
            "safe",
            Capability::PlanMutate,
            ResourceKind::Plan,
            RiskClass::Low,
        );
        assert!(matches!(
            coordinator
                .coordinate(&safe, &context(AutonomyLevel::L1), "")
                .await
                .unwrap(),
            CoordinatedApproval::Allowed { .. }
        ));
        assert_eq!(
            coordinator.state("safe", now()).await,
            Err(ApprovalLedgerError::NotFound)
        );

        let prohibited = request(
            "prohibited",
            Capability::MemoryApply,
            ResourceKind::Memory,
            RiskClass::Prohibited,
        );
        assert!(matches!(
            coordinator
                .coordinate(&prohibited, &context(AutonomyLevel::L1), "")
                .await
                .unwrap(),
            CoordinatedApproval::Denied { .. }
        ));
        assert_eq!(
            coordinator.state("prohibited", now()).await,
            Err(ApprovalLedgerError::NotFound)
        );
    }

    #[tokio::test]
    async fn pending_submission_and_decision_are_idempotent_and_payload_free() {
        let (coordinator, ledger) = setup().await;
        let command = request(
            "command",
            Capability::CommandExecute,
            ResourceKind::Command,
            RiskClass::High,
        );
        let first = coordinator
            .coordinate(&command, &context(AutonomyLevel::L1), "submit-command")
            .await
            .unwrap();
        let replay = coordinator
            .coordinate(&command, &context(AutonomyLevel::L1), "submit-command")
            .await
            .unwrap();
        assert_eq!(first, replay);
        let pending = first.pending_state().unwrap();
        let allowed = coordinator
            .decide(
                "command",
                pending.version,
                "allow-command",
                receipt(&command),
            )
            .await
            .unwrap();
        assert_eq!(allowed.status, ApprovalStatus::Allowed);
        assert_eq!(
            coordinator
                .decide(
                    "command",
                    pending.version,
                    "allow-command",
                    receipt(&command)
                )
                .await
                .unwrap(),
            allowed
        );

        let json = serde_json::to_string(&ledger.events("command").await.unwrap()).unwrap();
        assert!(!json.contains("secret domain payload"));
    }

    #[tokio::test]
    async fn changed_request_cannot_reuse_submission_key() {
        let (coordinator, _) = setup().await;
        let first = request(
            "first",
            Capability::CommandExecute,
            ResourceKind::Command,
            RiskClass::High,
        );
        coordinator
            .coordinate(&first, &context(AutonomyLevel::L1), "same-key")
            .await
            .unwrap();
        let changed = request(
            "changed",
            Capability::CommandExecute,
            ResourceKind::Command,
            RiskClass::High,
        );
        assert_eq!(
            coordinator
                .coordinate(&changed, &context(AutonomyLevel::L1), "same-key")
                .await,
            Err(ApprovalLedgerError::IdempotencyConflict)
        );
    }
}

//! Explicit compatibility adapters for legacy Plan approvals.
//!
//! These functions are not wired into the production planning store.

use chrono::{DateTime, Utc};

use crate::governance::{
    ApprovalGrant, ApprovalReceipt as GovernanceReceipt, ApprovalRequest as GovernanceRequest,
    AuditCorrelation, Capability, ContentDigest, Decision, EvidenceRef, GovernanceSubject,
    ResourceKind, ResourceScope, RiskClass,
};

use super::{ApprovalReceipt, ApprovalRequest, PlanId};

/// Caller-owned governance metadata absent from the legacy Plan contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanGovernanceContext {
    pub correlation: AuditCorrelation,
    pub subject: GovernanceSubject,
    pub workspace_id: String,
    pub session_id: Option<String>,
    pub content_digest: ContentDigest,
    pub policy_version: String,
    pub risk: RiskClass,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub evidence_refs: Vec<EvidenceRef>,
}

/// Wrap a legacy Plan request in the generic governance envelope.
pub fn adapt_plan_approval_request(
    plan_id: &PlanId,
    request: ApprovalRequest,
    context: PlanGovernanceContext,
) -> GovernanceRequest<ApprovalRequest> {
    GovernanceRequest {
        correlation: context.correlation,
        subject: context.subject,
        capability: Capability::PlanExecute,
        resource: ResourceScope {
            workspace_id: context.workspace_id,
            session_id: context.session_id,
            kind: ResourceKind::Plan,
            resource_id: plan_id.to_string(),
            boundary: Some(format!("revision:{}", request.expected_revision)),
        },
        risk: context.risk,
        content_digest: context.content_digest,
        policy_version: context.policy_version,
        created_at: context.created_at,
        expires_at: context.expires_at,
        evidence_refs: context.evidence_refs,
        payload: request,
    }
}

/// Convert an existing successful Plan receipt without changing its wire type.
pub fn adapt_plan_approval_receipt(
    receipt: &ApprovalReceipt,
    request: &GovernanceRequest<ApprovalRequest>,
    expires_at: DateTime<Utc>,
) -> GovernanceReceipt {
    GovernanceReceipt {
        request_id: request.correlation.request_id.clone(),
        content_digest: request.content_digest.clone(),
        policy_version: request.policy_version.clone(),
        capability: request.capability.clone(),
        resource: request.resource.clone(),
        decision: Decision::Allow,
        decided_by: GovernanceSubject {
            kind: crate::governance::GovernanceSubjectKind::User,
            id: receipt.approved_by.clone(),
        },
        decided_at: receipt.approved_at,
        expires_at,
        grant: ApprovalGrant::Once,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};

    use super::*;
    use crate::governance::{DigestAlgorithm, GovernanceSubjectKind};
    use crate::planning::TodoPolicy;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 29, 14, 0, 0)
            .single()
            .unwrap()
    }

    #[test]
    fn adapter_preserves_legacy_payload_and_binds_plan_revision() {
        let legacy = ApprovalRequest {
            expected_revision: 7,
            approved_by: "desktop-ui".into(),
            todo_policy: TodoPolicy::Optional,
            materialize_todos: true,
        };
        let request = adapt_plan_approval_request(
            &PlanId("plan-1".into()),
            legacy.clone(),
            PlanGovernanceContext {
                correlation: AuditCorrelation {
                    request_id: "request-1".into(),
                    turn_id: "turn-1".into(),
                    session_id: "session-1".into(),
                    trace_id: None,
                },
                subject: GovernanceSubject {
                    kind: GovernanceSubjectKind::User,
                    id: "user-1".into(),
                },
                workspace_id: "workspace-1".into(),
                session_id: Some("session-1".into()),
                content_digest: ContentDigest {
                    algorithm: DigestAlgorithm::Sha256,
                    value: "digest-1".into(),
                },
                policy_version: "policy-v1".into(),
                risk: RiskClass::High,
                created_at: now(),
                expires_at: now() + Duration::minutes(5),
                evidence_refs: vec![],
            },
        );
        assert_eq!(request.payload, legacy);
        assert_eq!(request.resource.boundary.as_deref(), Some("revision:7"));
        assert_eq!(request.capability, Capability::PlanExecute);
        request.validate().unwrap();
    }
}

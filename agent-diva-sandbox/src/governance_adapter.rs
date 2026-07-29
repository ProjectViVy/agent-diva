//! Explicit compatibility adapters for command approvals.
//!
//! The adapters do not register the governance ledger with the coordinator.

use chrono::{DateTime, Utc};

use agent_diva_core::governance::{
    ApprovalGrant, ApprovalReceipt, ApprovalRequest as GovernanceRequest, AuditCorrelation,
    Capability, ContentDigest, Decision, EvidenceRef, GovernanceSubject, ResourceKind,
    ResourceScope, RiskClass,
};

use crate::{ApprovalDecision, CommandApprovalRequest};

/// Caller-owned governance metadata absent from the legacy Sandbox request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxGovernanceContext {
    pub correlation: AuditCorrelation,
    pub subject: GovernanceSubject,
    pub workspace_id: String,
    pub content_digest: ContentDigest,
    pub policy_version: String,
    pub risk: RiskClass,
    pub expires_at: DateTime<Utc>,
    pub evidence_refs: Vec<EvidenceRef>,
}

/// Wrap a command approval request without changing the coordinator contract.
pub fn adapt_command_approval_request(
    request: CommandApprovalRequest,
    context: SandboxGovernanceContext,
) -> GovernanceRequest<CommandApprovalRequest> {
    GovernanceRequest {
        correlation: context.correlation,
        subject: context.subject,
        capability: Capability::CommandExecute,
        resource: ResourceScope {
            workspace_id: context.workspace_id,
            session_id: Some(request.scope.session_key.clone()),
            kind: ResourceKind::Command,
            resource_id: request.approval_id.clone(),
            boundary: Some(request.cwd.display().to_string()),
        },
        risk: context.risk,
        content_digest: context.content_digest,
        policy_version: context.policy_version,
        created_at: request.created_at,
        expires_at: context.expires_at,
        evidence_refs: context.evidence_refs,
        payload: request,
    }
}

/// Convert a legacy command decision into a governance receipt.
pub fn adapt_command_approval_decision(
    request: &GovernanceRequest<CommandApprovalRequest>,
    decision: ApprovalDecision,
    decided_by: GovernanceSubject,
    decided_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> ApprovalReceipt {
    let (decision, grant) = match decision {
        ApprovalDecision::ApproveOnce => (Decision::Allow, ApprovalGrant::Once),
        ApprovalDecision::ApproveSession => (Decision::Allow, ApprovalGrant::Session),
        ApprovalDecision::ApproveGlobal => (Decision::Allow, ApprovalGrant::Rule),
        ApprovalDecision::Reject => (Decision::Deny, ApprovalGrant::Once),
    };
    ApprovalReceipt {
        request_id: request.correlation.request_id.clone(),
        content_digest: request.content_digest.clone(),
        policy_version: request.policy_version.clone(),
        capability: request.capability.clone(),
        resource: request.resource.clone(),
        decision,
        decided_by,
        decided_at,
        expires_at,
        grant,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::{Duration, TimeZone};

    use super::*;
    use crate::CommandApprovalScope;
    use agent_diva_core::governance::{DigestAlgorithm, GovernanceSubjectKind};

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 29, 15, 0, 0)
            .single()
            .unwrap()
    }

    fn context() -> SandboxGovernanceContext {
        SandboxGovernanceContext {
            correlation: AuditCorrelation {
                request_id: "request-1".into(),
                turn_id: "turn-1".into(),
                session_id: "session-1".into(),
                trace_id: None,
            },
            subject: GovernanceSubject {
                kind: GovernanceSubjectKind::Agent,
                id: "agent-1".into(),
            },
            workspace_id: "workspace-1".into(),
            content_digest: ContentDigest {
                algorithm: DigestAlgorithm::Sha256,
                value: "digest-1".into(),
            },
            policy_version: "policy-v1".into(),
            risk: RiskClass::High,
            expires_at: now() + Duration::minutes(5),
            evidence_refs: vec![],
        }
    }

    #[test]
    fn adapter_preserves_command_payload_and_maps_scope() {
        let legacy = CommandApprovalRequest {
            approval_id: "approval-1".into(),
            command: "cargo test".into(),
            cwd: PathBuf::from("workspace"),
            reason: "test".into(),
            scope: CommandApprovalScope {
                channel: "api".into(),
                chat_id: "chat-1".into(),
                session_key: "session-1".into(),
            },
            created_at: now(),
            timeout_seconds: 300,
            suggested_prefix: None,
        };
        let request = adapt_command_approval_request(legacy, context());
        assert_eq!(request.payload.command, "cargo test");
        assert_eq!(request.resource.kind, ResourceKind::Command);
        assert_eq!(request.resource.session_id.as_deref(), Some("session-1"));
        request.validate().unwrap();
    }

    #[test]
    fn adapter_maps_all_legacy_decisions() {
        let legacy = CommandApprovalRequest {
            approval_id: "approval-1".into(),
            command: "cargo test".into(),
            cwd: PathBuf::from("workspace"),
            reason: "test".into(),
            scope: CommandApprovalScope {
                channel: "api".into(),
                chat_id: "chat-1".into(),
                session_key: "session-1".into(),
            },
            created_at: now(),
            timeout_seconds: 300,
            suggested_prefix: None,
        };
        let request = adapt_command_approval_request(legacy, context());
        let cases = [
            (
                ApprovalDecision::ApproveOnce,
                Decision::Allow,
                ApprovalGrant::Once,
            ),
            (
                ApprovalDecision::ApproveSession,
                Decision::Allow,
                ApprovalGrant::Session,
            ),
            (
                ApprovalDecision::ApproveGlobal,
                Decision::Allow,
                ApprovalGrant::Rule,
            ),
            (
                ApprovalDecision::Reject,
                Decision::Deny,
                ApprovalGrant::Once,
            ),
        ];
        for (legacy, expected_decision, expected_grant) in cases {
            let receipt = adapt_command_approval_decision(
                &request,
                legacy,
                GovernanceSubject {
                    kind: GovernanceSubjectKind::User,
                    id: "user-1".into(),
                },
                now() + Duration::seconds(1),
                now() + Duration::minutes(2),
            );
            assert_eq!(receipt.decision, expected_decision);
            assert_eq!(receipt.grant, expected_grant);
            receipt.validate().unwrap();
        }
    }
}

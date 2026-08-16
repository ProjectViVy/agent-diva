//! Deterministic, side-effect-free governance policy evaluation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{
    ApprovalGrant, ApprovalReceipt, ApprovalRequest, Capability, Decision, EvidenceRef,
    GovernanceValidationError, ResourceKind, RiskClass,
};

/// Product autonomy level applied to a governance evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyLevel {
    L0,
    L1,
    L2,
    L3,
    L4,
    #[serde(other)]
    Unknown,
}

/// Restriction category supplied by the owning runtime domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestrictionKind {
    Resource,
    Mode,
    #[serde(other)]
    Unknown,
}

/// A domain-owned restriction already resolved to an allow/deny fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyRestriction {
    pub kind: RestrictionKind,
    pub allowed: bool,
    pub code: String,
}

/// Context required by the pure policy evaluator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyContext {
    pub evaluated_at: DateTime<Utc>,
    pub autonomy: AutonomyLevel,
    pub explicit_user_decision: Option<Decision>,
    pub restrictions: Vec<PolicyRestriction>,
    pub authorizations: Vec<ApprovalReceipt>,
}

/// Stable, machine-matchable explanation for a policy result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyReasonCode {
    HardProhibited,
    ExplicitUserDenial,
    InvalidRequest,
    InvalidContext,
    ResourceCapabilityMismatch,
    ResourceRestricted,
    ModeRestricted,
    InvalidAuthorization,
    AuthorizedOnce,
    AuthorizedSession,
    AuthorizedRule,
    SafeDefault,
    HumanApprovalRequired,
}

/// Constraint attached to an allow or require-human result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyConstraintKind {
    Request,
    Session,
    Resource,
    PolicyVersion,
    Expiry,
}

/// Auditable boundary that qualifies a policy decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyConstraint {
    pub kind: PolicyConstraintKind,
    pub value: String,
}

/// Complete result returned by the pure evaluator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyEvaluation {
    pub decision: Decision,
    pub reason: PolicyReasonCode,
    pub constraints: Vec<PolicyConstraint>,
    pub evidence_refs: Vec<EvidenceRef>,
}

/// Evaluate one request using stable precedence and no I/O or mutable state.
pub fn evaluate_policy<P>(
    request: &ApprovalRequest<P>,
    context: &PolicyContext,
) -> PolicyEvaluation {
    if request.validate().is_err()
        || context.evaluated_at < request.created_at
        || context.evaluated_at >= request.expires_at
    {
        return result(
            request,
            Decision::Deny,
            PolicyReasonCode::InvalidRequest,
            vec![],
        );
    }
    if request.risk == RiskClass::Prohibited || context.autonomy == AutonomyLevel::L4 {
        return result(
            request,
            Decision::Deny,
            PolicyReasonCode::HardProhibited,
            vec![],
        );
    }
    if matches!(context.explicit_user_decision, Some(Decision::Deny)) {
        return result(
            request,
            Decision::Deny,
            PolicyReasonCode::ExplicitUserDenial,
            vec![],
        );
    }
    if context.autonomy == AutonomyLevel::Unknown
        || matches!(context.explicit_user_decision, Some(Decision::Unknown))
    {
        return result(
            request,
            Decision::Deny,
            PolicyReasonCode::InvalidContext,
            vec![],
        );
    }
    if !capability_matches_resource(&request.capability, &request.resource.kind) {
        return result(
            request,
            Decision::Deny,
            PolicyReasonCode::ResourceCapabilityMismatch,
            vec![],
        );
    }
    for restriction in &context.restrictions {
        if restriction.kind == RestrictionKind::Unknown || restriction.code.trim().is_empty() {
            return result(
                request,
                Decision::Deny,
                PolicyReasonCode::InvalidContext,
                vec![],
            );
        }
        if !restriction.allowed {
            let reason = match restriction.kind {
                RestrictionKind::Resource => PolicyReasonCode::ResourceRestricted,
                RestrictionKind::Mode => PolicyReasonCode::ModeRestricted,
                RestrictionKind::Unknown => PolicyReasonCode::InvalidContext,
            };
            return result(request, Decision::Deny, reason, vec![]);
        }
    }
    if context.autonomy == AutonomyLevel::L0 && request.capability != Capability::Inspect {
        return result(
            request,
            Decision::Deny,
            PolicyReasonCode::ModeRestricted,
            vec![],
        );
    }

    match matching_authorization(request, context) {
        Err(_) => {
            return result(
                request,
                Decision::Deny,
                PolicyReasonCode::InvalidAuthorization,
                vec![],
            )
        }
        Ok(Some(receipt)) if authorization_satisfies_level(receipt, request, context) => {
            let (reason, constraints) = authorized_result(receipt, request);
            return result(request, Decision::Allow, reason, constraints);
        }
        Ok(_) => {}
    }

    if safe_default_allows(request, context) {
        return result(
            request,
            Decision::Allow,
            PolicyReasonCode::SafeDefault,
            resource_constraints(request),
        );
    }
    result(
        request,
        Decision::RequireHuman,
        PolicyReasonCode::HumanApprovalRequired,
        resource_constraints(request),
    )
}

fn matching_authorization<'a, P>(
    request: &ApprovalRequest<P>,
    context: &'a PolicyContext,
) -> Result<Option<&'a ApprovalReceipt>, GovernanceValidationError> {
    let mut matching = None;
    for receipt in &context.authorizations {
        receipt.validate()?;
        if receipt.decided_at > context.evaluated_at {
            return Err(GovernanceValidationError::InvalidExpiry);
        }
        if receipt.decision != Decision::Allow || receipt.expires_at <= context.evaluated_at {
            continue;
        }
        if receipt.policy_version != request.policy_version
            || receipt.capability != request.capability
            || receipt.resource != request.resource
        {
            continue;
        }
        match receipt.grant {
            ApprovalGrant::Once => {
                receipt.validate_approve_once(request)?;
            }
            ApprovalGrant::Session => {
                if request.resource.session_id.as_deref()
                    != Some(request.correlation.session_id.as_str())
                {
                    continue;
                }
            }
            ApprovalGrant::Rule => {
                if receipt.content_digest != request.content_digest {
                    continue;
                }
            }
            ApprovalGrant::Unknown => {
                return Err(GovernanceValidationError::UnknownApprovalGrant);
            }
        }
        matching = Some(receipt);
        break;
    }
    Ok(matching)
}

fn authorization_satisfies_level<P>(
    receipt: &ApprovalReceipt,
    request: &ApprovalRequest<P>,
    context: &PolicyContext,
) -> bool {
    if request.risk == RiskClass::Critical {
        return receipt.grant == ApprovalGrant::Once;
    }
    match context.autonomy {
        AutonomyLevel::L0 => request.capability == Capability::Inspect,
        AutonomyLevel::L1 => true,
        AutonomyLevel::L2 => {
            matches!(receipt.grant, ApprovalGrant::Session | ApprovalGrant::Rule)
        }
        AutonomyLevel::L3 => receipt.grant == ApprovalGrant::Once,
        AutonomyLevel::L4 | AutonomyLevel::Unknown => false,
    }
}

fn safe_default_allows<P>(request: &ApprovalRequest<P>, context: &PolicyContext) -> bool {
    match context.autonomy {
        AutonomyLevel::L0 => request.capability == Capability::Inspect,
        AutonomyLevel::L1 => {
            request.risk == RiskClass::Low
                && matches!(
                    request.capability,
                    Capability::Inspect | Capability::PlanMutate
                )
        }
        AutonomyLevel::L2 | AutonomyLevel::L3 | AutonomyLevel::L4 | AutonomyLevel::Unknown => false,
    }
}

fn capability_matches_resource(capability: &Capability, resource: &ResourceKind) -> bool {
    match capability {
        Capability::Inspect => true,
        Capability::WorkspaceWrite => {
            matches!(
                resource,
                ResourceKind::Workspace | ResourceKind::WorkspacePath
            )
        }
        Capability::CommandExecute => resource == &ResourceKind::Command,
        Capability::NetworkAccess => resource == &ResourceKind::Network,
        Capability::McpInvoke => resource == &ResourceKind::McpServer,
        Capability::Spawn => resource == &ResourceKind::Agent,
        Capability::Schedule => resource == &ResourceKind::Schedule,
        Capability::PlanMutate | Capability::PlanExecute => resource == &ResourceKind::Plan,
        Capability::PolicyManage => resource == &ResourceKind::Policy,
        Capability::Unknown => false,
    }
}

fn authorized_result<P>(
    receipt: &ApprovalReceipt,
    request: &ApprovalRequest<P>,
) -> (PolicyReasonCode, Vec<PolicyConstraint>) {
    let reason = match receipt.grant {
        ApprovalGrant::Once => PolicyReasonCode::AuthorizedOnce,
        ApprovalGrant::Session => PolicyReasonCode::AuthorizedSession,
        ApprovalGrant::Rule => PolicyReasonCode::AuthorizedRule,
        ApprovalGrant::Unknown => PolicyReasonCode::InvalidAuthorization,
    };
    let mut constraints = resource_constraints(request);
    constraints.push(PolicyConstraint {
        kind: PolicyConstraintKind::Expiry,
        value: receipt.expires_at.to_rfc3339(),
    });
    if receipt.grant == ApprovalGrant::Once {
        constraints.push(PolicyConstraint {
            kind: PolicyConstraintKind::Request,
            value: request.correlation.request_id.clone(),
        });
    }
    if receipt.grant == ApprovalGrant::Session {
        constraints.push(PolicyConstraint {
            kind: PolicyConstraintKind::Session,
            value: request.correlation.session_id.clone(),
        });
    }
    (reason, constraints)
}

fn resource_constraints<P>(request: &ApprovalRequest<P>) -> Vec<PolicyConstraint> {
    vec![
        PolicyConstraint {
            kind: PolicyConstraintKind::Resource,
            value: request.resource.resource_id.clone(),
        },
        PolicyConstraint {
            kind: PolicyConstraintKind::PolicyVersion,
            value: request.policy_version.clone(),
        },
    ]
}

fn result<P>(
    request: &ApprovalRequest<P>,
    decision: Decision,
    reason: PolicyReasonCode,
    constraints: Vec<PolicyConstraint>,
) -> PolicyEvaluation {
    PolicyEvaluation {
        decision,
        reason,
        constraints,
        evidence_refs: request.evidence_refs.clone(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};
    use serde_json::json;

    use super::*;
    use crate::governance::{
        AuditCorrelation, ContentDigest, DigestAlgorithm, GovernanceSubject, GovernanceSubjectKind,
        ResourceScope,
    };

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 29, 12, 0, 0)
            .single()
            .unwrap()
    }

    fn request(
        capability: Capability,
        resource_kind: ResourceKind,
        risk: RiskClass,
    ) -> ApprovalRequest<()> {
        ApprovalRequest {
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
            capability,
            resource: ResourceScope {
                workspace_id: "workspace-1".into(),
                session_id: Some("session-1".into()),
                kind: resource_kind,
                resource_id: "resource-1".into(),
                boundary: None,
            },
            risk,
            content_digest: ContentDigest {
                algorithm: DigestAlgorithm::Sha256,
                value: "digest-1".into(),
            },
            policy_version: "policy-v1".into(),
            created_at: now(),
            expires_at: now() + Duration::minutes(5),
            evidence_refs: vec![],
            payload: (),
        }
    }

    fn context(autonomy: AutonomyLevel) -> PolicyContext {
        PolicyContext {
            evaluated_at: now() + Duration::seconds(10),
            autonomy,
            explicit_user_decision: None,
            restrictions: vec![],
            authorizations: vec![],
        }
    }

    fn receipt(request: &ApprovalRequest<()>, grant: ApprovalGrant) -> ApprovalReceipt {
        ApprovalReceipt {
            request_id: request.correlation.request_id.clone(),
            content_digest: request.content_digest.clone(),
            policy_version: request.policy_version.clone(),
            capability: request.capability.clone(),
            resource: request.resource.clone(),
            decision: Decision::Allow,
            decided_by: GovernanceSubject {
                kind: GovernanceSubjectKind::User,
                id: "user-1".into(),
            },
            decided_at: now() + Duration::seconds(1),
            expires_at: now() + Duration::minutes(2),
            grant,
        }
    }

    #[test]
    fn precedence_is_hard_deny_then_rejection_then_restriction_then_authorization() {
        let prohibited = request(
            Capability::Inspect,
            ResourceKind::Workspace,
            RiskClass::Prohibited,
        );
        let mut ctx = context(AutonomyLevel::L1);
        ctx.explicit_user_decision = Some(Decision::Deny);
        assert_eq!(
            evaluate_policy(&prohibited, &ctx).reason,
            PolicyReasonCode::HardProhibited
        );

        let command = request(
            Capability::CommandExecute,
            ResourceKind::Command,
            RiskClass::High,
        );
        ctx.authorizations
            .push(receipt(&command, ApprovalGrant::Once));
        assert_eq!(
            evaluate_policy(&command, &ctx).reason,
            PolicyReasonCode::ExplicitUserDenial
        );

        ctx.explicit_user_decision = None;
        ctx.restrictions.push(PolicyRestriction {
            kind: RestrictionKind::Mode,
            allowed: false,
            code: "plan_awaiting_approval".into(),
        });
        assert_eq!(
            evaluate_policy(&command, &ctx).reason,
            PolicyReasonCode::ModeRestricted
        );
    }

    #[test]
    fn domain_matrix_covers_plan_memory_and_side_effect_capabilities() {
        let cases = [
            (Capability::WorkspaceWrite, ResourceKind::WorkspacePath),
            (Capability::CommandExecute, ResourceKind::Command),
            (Capability::NetworkAccess, ResourceKind::Network),
            (Capability::McpInvoke, ResourceKind::McpServer),
            (Capability::Spawn, ResourceKind::Agent),
            (Capability::Schedule, ResourceKind::Schedule),
            (Capability::PlanExecute, ResourceKind::Plan),
        ];
        for (capability, resource) in cases {
            let request = request(capability, resource, RiskClass::High);
            assert_eq!(
                evaluate_policy(&request, &context(AutonomyLevel::L1)).decision,
                Decision::RequireHuman
            );
        }
    }

    #[test]
    fn every_capability_resource_pair_is_fail_closed_except_declared_matrix() {
        let capabilities = [
            Capability::Inspect,
            Capability::WorkspaceWrite,
            Capability::CommandExecute,
            Capability::NetworkAccess,
            Capability::McpInvoke,
            Capability::Spawn,
            Capability::Schedule,
            Capability::PlanMutate,
            Capability::PlanExecute,
            Capability::PolicyManage,
        ];
        let resources = [
            ResourceKind::Workspace,
            ResourceKind::WorkspacePath,
            ResourceKind::Command,
            ResourceKind::Network,
            ResourceKind::McpServer,
            ResourceKind::Agent,
            ResourceKind::Schedule,
            ResourceKind::Plan,
            ResourceKind::Policy,
        ];
        for capability in capabilities {
            for resource in &resources {
                let expected = capability_matches_resource(&capability, resource);
                let evaluation = evaluate_policy(
                    &request(capability.clone(), resource.clone(), RiskClass::Low),
                    &context(AutonomyLevel::L1),
                );
                assert_eq!(
                    evaluation.reason != PolicyReasonCode::ResourceCapabilityMismatch,
                    expected,
                    "{capability:?} / {resource:?}"
                );
            }
        }
    }

    #[test]
    fn autonomy_levels_require_their_frozen_authorization_scope() {
        let command = request(
            Capability::CommandExecute,
            ResourceKind::Command,
            RiskClass::High,
        );
        let cases = [
            (AutonomyLevel::L0, ApprovalGrant::Once, Decision::Deny),
            (AutonomyLevel::L1, ApprovalGrant::Once, Decision::Allow),
            (AutonomyLevel::L2, ApprovalGrant::Session, Decision::Allow),
            (
                AutonomyLevel::L2,
                ApprovalGrant::Once,
                Decision::RequireHuman,
            ),
            (AutonomyLevel::L3, ApprovalGrant::Once, Decision::Allow),
            (
                AutonomyLevel::L3,
                ApprovalGrant::Session,
                Decision::RequireHuman,
            ),
            (AutonomyLevel::L4, ApprovalGrant::Once, Decision::Deny),
        ];
        for (level, grant, expected) in cases {
            let mut ctx = context(level);
            ctx.authorizations.push(receipt(&command, grant));
            assert_eq!(evaluate_policy(&command, &ctx).decision, expected);
        }
    }

    #[test]
    fn stale_tampered_and_unknown_authorizations_fail_closed() {
        let command = request(
            Capability::CommandExecute,
            ResourceKind::Command,
            RiskClass::High,
        );
        let mut ctx = context(AutonomyLevel::L3);
        let mut approval = receipt(&command, ApprovalGrant::Once);
        approval.content_digest.value = "tampered".into();
        ctx.authorizations.push(approval);
        assert_eq!(
            evaluate_policy(&command, &ctx).reason,
            PolicyReasonCode::InvalidAuthorization
        );

        ctx.authorizations[0] = receipt(&command, ApprovalGrant::Once);
        ctx.authorizations[0].expires_at = ctx.evaluated_at;
        assert_eq!(
            evaluate_policy(&command, &ctx).decision,
            Decision::RequireHuman
        );

        ctx.autonomy = AutonomyLevel::Unknown;
        assert_eq!(
            evaluate_policy(&command, &ctx).reason,
            PolicyReasonCode::InvalidContext
        );
    }

    #[test]
    fn expired_requests_and_future_dated_receipts_do_not_authorize() {
        let mut command = request(
            Capability::CommandExecute,
            ResourceKind::Command,
            RiskClass::High,
        );
        let ctx = context(AutonomyLevel::L3);
        command.expires_at = ctx.evaluated_at;
        assert_eq!(
            evaluate_policy(&command, &ctx).reason,
            PolicyReasonCode::InvalidRequest
        );

        command.expires_at = now() + Duration::minutes(5);
        let mut ctx = context(AutonomyLevel::L3);
        let mut approval = receipt(&command, ApprovalGrant::Once);
        approval.decided_at = ctx.evaluated_at + Duration::seconds(1);
        ctx.authorizations.push(approval);
        assert_eq!(
            evaluate_policy(&command, &ctx).reason,
            PolicyReasonCode::InvalidAuthorization
        );
    }

    #[test]
    fn safe_defaults_allow_low_risk_inspect_and_plan_draft_only() {
        let allowed = [
            (Capability::Inspect, ResourceKind::Workspace),
            (Capability::PlanMutate, ResourceKind::Plan),
        ];
        for (capability, resource) in allowed {
            let evaluation = evaluate_policy(
                &request(capability, resource, RiskClass::Low),
                &context(AutonomyLevel::L1),
            );
            assert_eq!(evaluation.decision, Decision::Allow);
            assert_eq!(evaluation.reason, PolicyReasonCode::SafeDefault);
        }
        let network_access = request(
            Capability::NetworkAccess,
            ResourceKind::Network,
            RiskClass::Low,
        );
        assert_eq!(
            evaluate_policy(&network_access, &context(AutonomyLevel::L1)).decision,
            Decision::RequireHuman
        );
    }

    #[test]
    fn serialized_output_contract_is_stable() {
        let evaluation = evaluate_policy(
            &request(
                Capability::NetworkAccess,
                ResourceKind::Network,
                RiskClass::High,
            ),
            &context(AutonomyLevel::L2),
        );
        assert_eq!(
            serde_json::to_value(evaluation).unwrap(),
            json!({
                "decision": "require_human",
                "reason": "human_approval_required",
                "constraints": [
                    {"kind": "resource", "value": "resource-1"},
                    {"kind": "policy_version", "value": "policy-v1"}
                ],
                "evidence_refs": []
            })
        );
    }
}

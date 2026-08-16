//! Stable governance request, receipt, and validation types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::EvidenceRef;

/// Kind of actor asking for or making a governance decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceSubjectKind {
    User,
    Agent,
    System,
    Service,
    #[serde(other)]
    Unknown,
}

/// Stable identity of an actor participating in governance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceSubject {
    pub kind: GovernanceSubjectKind,
    pub id: String,
}

/// Governed capability families shared by runtime domains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Inspect,
    WorkspaceWrite,
    CommandExecute,
    NetworkAccess,
    McpInvoke,
    Spawn,
    Schedule,
    PlanMutate,
    PlanExecute,
    PolicyManage,
    #[serde(other)]
    Unknown,
}

/// Kind of resource targeted by a governed capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Workspace,
    WorkspacePath,
    Command,
    Network,
    McpServer,
    Agent,
    Schedule,
    Plan,
    Policy,
    #[serde(other)]
    Unknown,
}

/// Stable resource boundary to which a request or receipt applies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceScope {
    pub workspace_id: String,
    pub session_id: Option<String>,
    pub kind: ResourceKind,
    pub resource_id: String,
    pub boundary: Option<String>,
}

/// Risk classification used by the governance policy layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskClass {
    Low,
    Moderate,
    High,
    Critical,
    Prohibited,
    #[serde(other)]
    Unknown,
}

/// Result of a governance evaluation or human decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Allow,
    Deny,
    RequireHuman,
    #[serde(other)]
    Unknown,
}

/// Digest algorithm used to bind approval to immutable content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DigestAlgorithm {
    Sha256,
    #[serde(other)]
    Unknown,
}

/// Non-secret digest of the exact content covered by a decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentDigest {
    pub algorithm: DigestAlgorithm,
    pub value: String,
}

/// Identifiers used to correlate governance activity with runtime audit data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditCorrelation {
    pub request_id: String,
    pub turn_id: String,
    pub session_id: String,
    pub trace_id: Option<String>,
}

/// Domain-neutral approval request carrying a domain-owned payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest<P> {
    pub correlation: AuditCorrelation,
    pub subject: GovernanceSubject,
    pub capability: Capability,
    pub resource: ResourceScope,
    pub risk: RiskClass,
    pub content_digest: ContentDigest,
    pub policy_version: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub evidence_refs: Vec<EvidenceRef>,
    pub payload: P,
}

/// Lifetime and reuse boundary granted by an approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalGrant {
    Once,
    Session,
    Rule,
    #[serde(other)]
    Unknown,
}

/// Immutable decision record bound to one request revision and resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalReceipt {
    pub request_id: String,
    pub content_digest: ContentDigest,
    pub policy_version: String,
    pub capability: Capability,
    pub resource: ResourceScope,
    pub decision: Decision,
    pub decided_by: GovernanceSubject,
    pub decided_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub grant: ApprovalGrant,
}

/// Stable reasons why a governance contract is invalid or cannot authorize use.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GovernanceValidationError {
    #[error("missing required field: {0}")]
    MissingRequiredField(&'static str),
    #[error("unknown governance subject kind")]
    UnknownSubject,
    #[error("unknown capability")]
    UnknownCapability,
    #[error("unknown resource kind")]
    UnknownResource,
    #[error("unknown risk class")]
    UnknownRisk,
    #[error("unknown digest algorithm")]
    UnknownDigestAlgorithm,
    #[error("unknown decision")]
    UnknownDecision,
    #[error("unknown approval grant")]
    UnknownApprovalGrant,
    #[error("expiry must be later than creation or decision time")]
    InvalidExpiry,
    #[error("approval receipt request id does not match")]
    RequestIdMismatch,
    #[error("approval receipt content digest does not match")]
    ContentDigestMismatch,
    #[error("approval receipt policy version does not match")]
    PolicyVersionMismatch,
    #[error("approval receipt capability does not match")]
    CapabilityMismatch,
    #[error("approval receipt resource scope does not match")]
    ResourceScopeMismatch,
    #[error("approval receipt is not an allow decision")]
    DecisionDoesNotAllow,
    #[error("approval receipt is not an approve-once grant")]
    GrantIsNotOnce,
    #[error("governance cursor is invalid")]
    InvalidCursor,
    #[error("governance page limit is invalid")]
    InvalidPageLimit,
}

impl<P> ApprovalRequest<P> {
    /// Validate the domain-neutral request envelope.
    pub fn validate(&self) -> Result<(), GovernanceValidationError> {
        require_non_empty("request_id", &self.correlation.request_id)?;
        require_non_empty("turn_id", &self.correlation.turn_id)?;
        require_non_empty("session_id", &self.correlation.session_id)?;
        validate_subject(&self.subject)?;
        if self.capability == Capability::Unknown {
            return Err(GovernanceValidationError::UnknownCapability);
        }
        validate_resource(&self.resource)?;
        if self.risk == RiskClass::Unknown {
            return Err(GovernanceValidationError::UnknownRisk);
        }
        validate_digest(&self.content_digest)?;
        require_non_empty("policy_version", &self.policy_version)?;
        if self.expires_at <= self.created_at {
            return Err(GovernanceValidationError::InvalidExpiry);
        }
        Ok(())
    }
}

impl ApprovalReceipt {
    /// Validate the receipt independently of the request it may authorize.
    pub fn validate(&self) -> Result<(), GovernanceValidationError> {
        require_non_empty("request_id", &self.request_id)?;
        validate_digest(&self.content_digest)?;
        require_non_empty("policy_version", &self.policy_version)?;
        if self.capability == Capability::Unknown {
            return Err(GovernanceValidationError::UnknownCapability);
        }
        validate_resource(&self.resource)?;
        validate_subject(&self.decided_by)?;
        if self.decision == Decision::Unknown {
            return Err(GovernanceValidationError::UnknownDecision);
        }
        if self.grant == ApprovalGrant::Unknown {
            return Err(GovernanceValidationError::UnknownApprovalGrant);
        }
        if self.expires_at <= self.decided_at {
            return Err(GovernanceValidationError::InvalidExpiry);
        }
        Ok(())
    }

    /// Verify that an approve-once receipt authorizes this exact request.
    pub fn validate_approve_once<P>(
        &self,
        request: &ApprovalRequest<P>,
    ) -> Result<(), GovernanceValidationError> {
        request.validate()?;
        self.validate()?;
        if self.decision != Decision::Allow {
            return Err(GovernanceValidationError::DecisionDoesNotAllow);
        }
        if self.grant != ApprovalGrant::Once {
            return Err(GovernanceValidationError::GrantIsNotOnce);
        }
        if self.request_id != request.correlation.request_id {
            return Err(GovernanceValidationError::RequestIdMismatch);
        }
        if self.content_digest != request.content_digest {
            return Err(GovernanceValidationError::ContentDigestMismatch);
        }
        if self.policy_version != request.policy_version {
            return Err(GovernanceValidationError::PolicyVersionMismatch);
        }
        if self.capability != request.capability {
            return Err(GovernanceValidationError::CapabilityMismatch);
        }
        if self.resource != request.resource {
            return Err(GovernanceValidationError::ResourceScopeMismatch);
        }
        Ok(())
    }
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), GovernanceValidationError> {
    if value.trim().is_empty() {
        Err(GovernanceValidationError::MissingRequiredField(field))
    } else {
        Ok(())
    }
}

fn validate_subject(subject: &GovernanceSubject) -> Result<(), GovernanceValidationError> {
    if subject.kind == GovernanceSubjectKind::Unknown {
        return Err(GovernanceValidationError::UnknownSubject);
    }
    require_non_empty("subject.id", &subject.id)
}

fn validate_resource(resource: &ResourceScope) -> Result<(), GovernanceValidationError> {
    require_non_empty("resource.workspace_id", &resource.workspace_id)?;
    require_non_empty("resource.resource_id", &resource.resource_id)?;
    if resource.kind == ResourceKind::Unknown {
        return Err(GovernanceValidationError::UnknownResource);
    }
    Ok(())
}

fn validate_digest(digest: &ContentDigest) -> Result<(), GovernanceValidationError> {
    if digest.algorithm == DigestAlgorithm::Unknown {
        return Err(GovernanceValidationError::UnknownDigestAlgorithm);
    }
    require_non_empty("content_digest.value", &digest.value)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use serde_json::json;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct DomainPayload {
        operation: String,
    }

    fn ts(second: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 29, 12, 0, second)
            .single()
            .unwrap()
    }

    fn request(capability: Capability, kind: ResourceKind) -> ApprovalRequest<DomainPayload> {
        ApprovalRequest {
            correlation: AuditCorrelation {
                request_id: "req-1".into(),
                turn_id: "turn-1".into(),
                session_id: "session-1".into(),
                trace_id: Some("trace-1".into()),
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
                resource_id: "resource-1".into(),
                boundary: None,
            },
            risk: RiskClass::High,
            content_digest: ContentDigest {
                algorithm: DigestAlgorithm::Sha256,
                value: "abc123".into(),
            },
            policy_version: "policy-v1".into(),
            created_at: ts(0),
            expires_at: ts(30),
            evidence_refs: Vec::new(),
            payload: DomainPayload {
                operation: "execute".into(),
            },
        }
    }

    fn receipt(request: &ApprovalRequest<DomainPayload>) -> ApprovalReceipt {
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
            decided_at: ts(1),
            expires_at: ts(20),
            grant: ApprovalGrant::Once,
        }
    }

    #[test]
    fn fixed_json_fixture_locks_generic_envelope_schema() {
        let request = request(Capability::CommandExecute, ResourceKind::Command);
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            json!({
                "correlation": {
                    "request_id": "req-1",
                    "turn_id": "turn-1",
                    "session_id": "session-1",
                    "trace_id": "trace-1"
                },
                "subject": {"kind": "agent", "id": "agent-1"},
                "capability": "command_execute",
                "resource": {
                    "workspace_id": "workspace-1",
                    "session_id": "session-1",
                    "kind": "command",
                    "resource_id": "resource-1",
                    "boundary": null
                },
                "risk": "high",
                "content_digest": {"algorithm": "sha256", "value": "abc123"},
                "policy_version": "policy-v1",
                "created_at": "2026-07-29T12:00:00Z",
                "expires_at": "2026-07-29T12:00:30Z",
                "evidence_refs": [],
                "payload": {"operation": "execute"}
            })
        );
        let json = serde_json::to_string(&request).unwrap();
        let decoded: ApprovalRequest<DomainPayload> = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, request);
    }

    #[test]
    fn generic_envelope_validates_plan_sandbox_and_memory_payloads() {
        let cases = [
            (Capability::PlanExecute, ResourceKind::Plan),
            (Capability::CommandExecute, ResourceKind::Command),
        ];
        for (capability, kind) in cases {
            assert_eq!(request(capability, kind).validate(), Ok(()));
        }
    }

    #[test]
    fn request_validation_fails_closed_for_missing_unknown_and_expired_fields() {
        let mut value = request(Capability::Inspect, ResourceKind::Workspace);
        value.correlation.request_id = " ".into();
        assert_eq!(
            value.validate(),
            Err(GovernanceValidationError::MissingRequiredField(
                "request_id"
            ))
        );

        let value = request(Capability::Unknown, ResourceKind::Workspace);
        assert_eq!(
            value.validate(),
            Err(GovernanceValidationError::UnknownCapability)
        );

        let value = request(Capability::Inspect, ResourceKind::Unknown);
        assert_eq!(
            value.validate(),
            Err(GovernanceValidationError::UnknownResource)
        );

        let mut value = request(Capability::Inspect, ResourceKind::Workspace);
        value.subject.kind = GovernanceSubjectKind::Unknown;
        assert_eq!(
            value.validate(),
            Err(GovernanceValidationError::UnknownSubject)
        );

        let mut value = request(Capability::Inspect, ResourceKind::Workspace);
        value.risk = RiskClass::Unknown;
        assert_eq!(
            value.validate(),
            Err(GovernanceValidationError::UnknownRisk)
        );

        let mut value = request(Capability::Inspect, ResourceKind::Workspace);
        value.content_digest.algorithm = DigestAlgorithm::Unknown;
        assert_eq!(
            value.validate(),
            Err(GovernanceValidationError::UnknownDigestAlgorithm)
        );

        let mut value = request(Capability::Inspect, ResourceKind::Workspace);
        value.expires_at = value.created_at;
        assert_eq!(
            value.validate(),
            Err(GovernanceValidationError::InvalidExpiry)
        );
    }

    #[test]
    fn approve_once_receipt_is_bound_to_exact_request() {
        let request = request(Capability::CommandExecute, ResourceKind::Command);
        let receipt = receipt(&request);
        assert_eq!(receipt.validate_approve_once(&request), Ok(()));

        let mut changed = request.clone();
        changed.content_digest.value = "tampered".into();
        assert_eq!(
            receipt.validate_approve_once(&changed),
            Err(GovernanceValidationError::ContentDigestMismatch)
        );

        let mut changed = request.clone();
        changed.policy_version = "policy-v2".into();
        assert_eq!(
            receipt.validate_approve_once(&changed),
            Err(GovernanceValidationError::PolicyVersionMismatch)
        );

        let mut changed = request.clone();
        changed.resource.resource_id = "other".into();
        assert_eq!(
            receipt.validate_approve_once(&changed),
            Err(GovernanceValidationError::ResourceScopeMismatch)
        );
    }

    #[test]
    fn unknown_wire_values_deserialize_but_cannot_authorize() {
        let mut value =
            serde_json::to_value(request(Capability::CommandExecute, ResourceKind::Command))
                .unwrap();
        value["capability"] = json!("future_capability");
        let decoded: ApprovalRequest<DomainPayload> = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.capability, Capability::Unknown);
        assert_eq!(
            decoded.validate(),
            Err(GovernanceValidationError::UnknownCapability)
        );
    }

    #[test]
    fn receipt_rejects_non_allow_and_non_once_decisions() {
        let request = request(Capability::CommandExecute, ResourceKind::Command);
        let mut value = receipt(&request);
        value.decision = Decision::Deny;
        assert_eq!(
            value.validate_approve_once(&request),
            Err(GovernanceValidationError::DecisionDoesNotAllow)
        );

        let mut value = receipt(&request);
        value.grant = ApprovalGrant::Session;
        assert_eq!(
            value.validate_approve_once(&request),
            Err(GovernanceValidationError::GrantIsNotOnce)
        );
    }

    #[test]
    fn existing_evidence_contract_is_reexported_without_duplication() {
        let source = super::super::EvidenceSource::UserInput;
        assert_eq!(serde_json::to_string(&source).unwrap(), "\"user_input\"");
    }
}

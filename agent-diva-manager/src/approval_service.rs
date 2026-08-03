//! Unified, payload-free approval projection over the three domain authorities.

use agent_diva_core::governance::{
    ApprovalCoordinator, ApprovalGrant, ApprovalLedgerError, ApprovalLedgerEventKind,
    ApprovalReceipt, ApprovalState, ApprovalStatus, Capability, Decision, GovernanceSubject,
    GovernanceSubjectKind,
};
use agent_diva_laputa::{MemoryGovernanceDecision, MemoryGovernanceError};
use agent_diva_sandbox::{ApprovalDecision, ApprovalResolveError};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDomain {
    Command,
    Plan,
    Memory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalReasonCode {
    ApprovalNotFound,
    ApprovalVersionConflict,
    ApprovalIdempotencyConflict,
    ApprovalExpired,
    ApprovalAlreadyResolved,
    ApprovalAlreadyConsumed,
    ApprovalDigestMismatch,
    ApprovalInvalidGrant,
    ApprovalInvalidTransition,
    ApprovalPayloadUnavailable,
    ApprovalPersistenceFailed,
    ApprovalRequiredNoninteractive,
    ApprovalQueueUnavailable,
    ApprovalOutcomeUnknown,
    ApprovalInvalidCursor,
    ApprovalInvalidBody,
    ApprovalInvalidQuery,
    ApprovalDenied,
    ApprovalRevoked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalView {
    pub request_id: String,
    pub version: u64,
    pub domain: ApprovalDomain,
    pub capability: Capability,
    pub resource: agent_diva_core::governance::ResourceScope,
    pub risk: agent_diva_core::governance::RiskClass,
    pub status: ApprovalStatus,
    pub created_at: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
    pub subject: agent_diva_core::governance::GovernanceSubject,
    pub evidence: Vec<agent_diva_core::governance::ApprovalEvidenceRecord>,
    pub receipt: Option<ApprovalReceipt>,
    pub reason_code: Option<ApprovalReasonCode>,
    pub actions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApprovalListQuery {
    pub domain: Option<ApprovalDomain>,
    pub status: Option<ApprovalStatus>,
    pub session: Option<String>,
    pub cursor: Option<String>,
    #[serde(default = "default_page_limit")]
    pub limit: u32,
}

fn default_page_limit() -> u32 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalListPage {
    pub approvals: Vec<ApprovalView>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalEventView {
    pub event_id: String,
    pub cursor: String,
    pub request_id: String,
    pub version: u64,
    pub domain: ApprovalDomain,
    pub status: ApprovalStatus,
    pub reason_code: Option<ApprovalReasonCode>,
    pub reason: Option<ApprovalReasonCode>,
    pub occurred_at: chrono::DateTime<Utc>,
    pub correlation: Option<agent_diva_core::governance::AuditCorrelation>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UnifiedDecisionBody {
    pub expected_version: u64,
    pub idempotency_key: String,
    pub decision: Decision,
    pub grant: ApprovalGrant,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UnifiedCancelBody {
    pub expected_version: u64,
    pub idempotency_key: String,
}

#[derive(Debug)]
pub enum ApprovalServiceError {
    Unavailable,
    PayloadUnavailable,
    InvalidGrant,
    Ledger(ApprovalLedgerError),
    Command(ApprovalResolveError),
    Memory(MemoryGovernanceError),
    Plan(String),
}

impl std::fmt::Display for ApprovalServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("approval runtime is unavailable"),
            Self::PayloadUnavailable => formatter.write_str("approval payload is unavailable"),
            Self::InvalidGrant => formatter.write_str("approval grant is invalid for this domain"),
            Self::Ledger(error) => write!(formatter, "{error}"),
            Self::Command(error) => write!(formatter, "command approval failed: {error:?}"),
            Self::Memory(error) => write!(formatter, "memory approval failed: {error}"),
            Self::Plan(error) => write!(formatter, "plan approval failed: {error}"),
        }
    }
}

impl From<ApprovalLedgerError> for ApprovalServiceError {
    fn from(error: ApprovalLedgerError) -> Self {
        Self::Ledger(error)
    }
}

impl From<MemoryGovernanceError> for ApprovalServiceError {
    fn from(error: MemoryGovernanceError) -> Self {
        Self::Memory(error)
    }
}

#[derive(Clone)]
pub struct ApprovalService {
    state: AppState,
    governance: ApprovalCoordinator,
}

impl ApprovalService {
    pub fn from_state(state: &AppState) -> Result<Self, ApprovalServiceError> {
        let governance = state
            .governance
            .clone()
            .ok_or(ApprovalServiceError::Unavailable)?;
        Ok(Self {
            state: state.clone(),
            governance,
        })
    }

    pub async fn list(
        &self,
        query: &ApprovalListQuery,
    ) -> Result<ApprovalListPage, ApprovalServiceError> {
        self.materialize_expired().await?;
        let page = self
            .governance
            .states_page(query.cursor.as_deref(), query.limit, Utc::now())
            .await?;
        let mut approvals = Vec::new();
        for state in page.states {
            let domain = domain_for(&state)?;
            if query.domain.is_some_and(|filter| filter != domain)
                || query
                    .status
                    .as_ref()
                    .is_some_and(|filter| filter != &state.status)
                || query.session.as_ref().is_some_and(|session| {
                    state.request.resource.session_id.as_ref() != Some(session)
                })
            {
                continue;
            }
            approvals.push(self.assemble(state, false).await?);
        }
        Ok(ApprovalListPage {
            approvals,
            next_cursor: page.next_cursor,
        })
    }

    pub async fn detail(&self, request_id: &str) -> Result<ApprovalView, ApprovalServiceError> {
        let mut state = self.governance.state(request_id, Utc::now()).await?;
        if state.status == ApprovalStatus::Expired {
            state = self
                .governance
                .expire(
                    request_id,
                    state.version,
                    &format!("manager-expire-{request_id}-v{}", state.version),
                    state.request.expires_at,
                )
                .await?;
        }
        self.assemble(state, true).await
    }

    pub async fn cancel(
        &self,
        request_id: &str,
        body: &UnifiedCancelBody,
    ) -> Result<ApprovalView, ApprovalServiceError> {
        let current = self.governance.state(request_id, Utc::now()).await?;
        if domain_for(&current)? == ApprovalDomain::Command {
            self.state
                .command_approvals
                .cancel_request(
                    request_id,
                    body.expected_version,
                    &body.idempotency_key,
                    local_user(),
                )
                .await
                .map_err(ApprovalServiceError::Command)?;
            return self.detail(request_id).await;
        }
        let state = self
            .governance
            .revoke(
                request_id,
                body.expected_version,
                &body.idempotency_key,
                local_user(),
                Utc::now(),
            )
            .await?;
        self.assemble(state, false).await
    }

    pub async fn decide(
        &self,
        request_id: &str,
        body: &UnifiedDecisionBody,
    ) -> Result<ApprovalView, ApprovalServiceError> {
        let current = self.governance.state(request_id, Utc::now()).await?;
        let domain = domain_for(&current)?;
        if domain != ApprovalDomain::Command
            && matches!(
                current.status,
                ApprovalStatus::Allowed | ApprovalStatus::Denied | ApprovalStatus::Consumed
            )
        {
            let receipt = current
                .receipt
                .clone()
                .ok_or(ApprovalServiceError::PayloadUnavailable)?;
            if receipt.decision != body.decision || receipt.grant != body.grant {
                return Err(ApprovalLedgerError::IdempotencyConflict.into());
            }
            self.governance
                .decide(
                    request_id,
                    body.expected_version,
                    &body.idempotency_key,
                    receipt,
                )
                .await?;
            return self.detail(request_id).await;
        }
        match domain {
            ApprovalDomain::Command => {
                let decision = command_decision(body)?;
                self.state
                    .command_approvals
                    .resolve_with_preconditions(
                        request_id,
                        decision,
                        Some(body.expected_version),
                        Some(&body.idempotency_key),
                    )
                    .await
                    .map_err(ApprovalServiceError::Command)?;
            }
            ApprovalDomain::Plan if body.decision == Decision::Allow => {
                if body.grant != ApprovalGrant::Once {
                    return Err(ApprovalServiceError::InvalidGrant);
                }
                self.state
                    .planning_service
                    .as_ref()
                    .ok_or(ApprovalServiceError::Unavailable)?
                    .approve_governed_request(
                        request_id,
                        body.expected_version,
                        &body.idempotency_key,
                    )
                    .await
                    .map_err(map_plan_error)?;
            }
            ApprovalDomain::Plan => {
                decide_ledger(&self.governance, &current, body).await?;
            }
            ApprovalDomain::Memory => {
                let proposal = self
                    .state
                    .laputa
                    .get_proposal(&current.request.resource.resource_id)
                    .map_err(|_| ApprovalServiceError::PayloadUnavailable)?;
                self.state
                    .memory_governance
                    .decide(
                        &proposal,
                        body.expected_version,
                        MemoryGovernanceDecision {
                            decision: body.decision.clone(),
                            grant: body.grant.clone(),
                            actor: local_user(),
                            idempotency_key: &body.idempotency_key,
                            decided_at: Utc::now(),
                        },
                    )
                    .await
                    .map_err(map_memory_error)?;
                let desired = if body.decision == Decision::Allow {
                    agent_diva_core::evolution::ProposalState::Approved
                } else {
                    agent_diva_core::evolution::ProposalState::Rejected
                };
                if proposal.state != desired {
                    self.state
                        .laputa
                        .transition_proposal(&proposal.id, desired, Utc::now())
                        .map_err(|_| ApprovalServiceError::PayloadUnavailable)?;
                }
            }
        }
        self.detail(request_id).await
    }

    pub async fn events(
        &self,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<Vec<ApprovalEventView>, ApprovalServiceError> {
        self.materialize_expired().await?;
        let page = self.governance.events_page(cursor, limit).await?;
        let mut events = Vec::with_capacity(page.events.len());
        for item in page.events {
            let status = event_status(&item.event.kind);
            let aggregate = self
                .governance
                .state(&item.event.request_id, item.event.occurred_at)
                .await?;
            events.push(ApprovalEventView {
                event_id: item.event.event_id,
                cursor: item.cursor,
                request_id: item.event.request_id,
                version: item.event.version,
                domain: domain_for_capability(&aggregate.request.capability)?,
                status: status.clone(),
                reason_code: reason_for_status(&status),
                reason: reason_for_status(&status),
                occurred_at: item.event.occurred_at,
                correlation: Some(aggregate.request.correlation),
            });
        }
        Ok(events)
    }

    async fn materialize_expired(&self) -> Result<(), ApprovalServiceError> {
        let evaluated_at = Utc::now();
        let mut cursor = None;
        loop {
            let page = self
                .governance
                .states_page(cursor.as_deref(), 100, evaluated_at)
                .await?;
            for state in page.states {
                if state.status == ApprovalStatus::Expired {
                    let request_id = &state.request.correlation.request_id;
                    match self
                        .governance
                        .expire(
                            request_id,
                            state.version,
                            &format!("manager-expire-{request_id}-v{}", state.version),
                            state.request.expires_at,
                        )
                        .await
                    {
                        Ok(_) | Err(ApprovalLedgerError::VersionConflict) => {}
                        Err(error) => return Err(error.into()),
                    }
                }
            }
            let Some(next) = page.next_cursor else { break };
            cursor = Some(next);
        }
        Ok(())
    }

    async fn assemble(
        &self,
        state: ApprovalState,
        include_presentation: bool,
    ) -> Result<ApprovalView, ApprovalServiceError> {
        let domain = domain_for(&state)?;
        let presentation = if include_presentation {
            self.presentation(domain, &state).await?
        } else {
            None
        };
        Ok(ApprovalView {
            request_id: state.request.correlation.request_id.clone(),
            version: state.version,
            domain,
            capability: state.request.capability.clone(),
            resource: state.request.resource.clone(),
            risk: state.request.risk.clone(),
            status: state.status.clone(),
            created_at: state.request.created_at,
            expires_at: state.request.expires_at,
            subject: state.request.subject.clone(),
            evidence: state.request.evidence_refs.clone(),
            receipt: state.receipt,
            reason_code: reason_for_status(&state.status),
            actions: actions(domain, &state.status),
            presentation,
        })
    }

    async fn presentation(
        &self,
        domain: ApprovalDomain,
        state: &ApprovalState,
    ) -> Result<Option<serde_json::Value>, ApprovalServiceError> {
        match domain {
            ApprovalDomain::Command => Ok(self
                .state
                .command_approvals
                .pending(None)
                .await
                .into_iter()
                .find(|request| request.approval_id == state.request.correlation.request_id)
                .map(|request| {
                    serde_json::json!({
                        "title": "Command execution",
                        "command": request.command,
                        "cwd": request.cwd,
                        "reason": request.reason,
                        "session_key": request.scope.session_key,
                    })
                })),
            ApprovalDomain::Plan => {
                let reports = self
                    .state
                    .planning_service
                    .as_ref()
                    .ok_or(ApprovalServiceError::Unavailable)?
                    .list_reports()
                    .await
                    .map_err(|error| ApprovalServiceError::Plan(error.to_string()))?;
                Ok(reports
                    .into_iter()
                    .find(|report| report.report.id.0 == state.request.resource.resource_id)
                    .filter(|report| {
                        agent_diva_core::planning::revision_hash(&report.revision.markdown)
                            == state.request.content_digest.value
                    })
                    .map(|report| {
                        serde_json::json!({
                        "title": report.revision.title,
                            "revision": report.revision.revision,
                            "session_key": report.report.session_key,
                        })
                    }))
            }
            ApprovalDomain::Memory => {
                let proposal = self
                    .state
                    .laputa
                    .get_proposal(&state.request.resource.resource_id)
                    .map_err(|_| ApprovalServiceError::PayloadUnavailable)?;
                if agent_diva_laputa::proposal_digest(&proposal) != state.request.content_digest {
                    return Err(ApprovalServiceError::PayloadUnavailable);
                }
                Ok(Some(serde_json::json!({
                    "title": "Memory change",
                    "proposal_id": proposal.id,
                    "target_section": proposal.target_section,
                    "proposal_type": proposal.proposal_type,
                    "evidence_count": proposal.evidence_refs.len(),
                })))
            }
        }
    }
}

fn map_plan_error(error: anyhow::Error) -> ApprovalServiceError {
    match error.downcast::<ApprovalLedgerError>() {
        Ok(error) => ApprovalServiceError::Ledger(error),
        Err(error) if error.to_string() == "approval payload is unavailable" => {
            ApprovalServiceError::PayloadUnavailable
        }
        Err(error) => ApprovalServiceError::Plan(error.to_string()),
    }
}

fn map_memory_error(error: MemoryGovernanceError) -> ApprovalServiceError {
    match error {
        MemoryGovernanceError::Ledger(error) => ApprovalServiceError::Ledger(error),
        MemoryGovernanceError::StaleProposal => ApprovalServiceError::PayloadUnavailable,
        MemoryGovernanceError::InvalidGrant => ApprovalServiceError::InvalidGrant,
        error => ApprovalServiceError::Memory(error),
    }
}

async fn decide_ledger(
    governance: &ApprovalCoordinator,
    current: &ApprovalState,
    body: &UnifiedDecisionBody,
) -> Result<ApprovalState, ApprovalServiceError> {
    if body.grant != ApprovalGrant::Once {
        return Err(ApprovalServiceError::InvalidGrant);
    }
    let now = Utc::now();
    Ok(governance
        .decide(
            &current.request.correlation.request_id,
            body.expected_version,
            &body.idempotency_key,
            ApprovalReceipt {
                request_id: current.request.correlation.request_id.clone(),
                content_digest: current.request.content_digest.clone(),
                policy_version: current.request.policy_version.clone(),
                capability: current.request.capability.clone(),
                resource: current.request.resource.clone(),
                decision: body.decision.clone(),
                decided_by: local_user(),
                decided_at: now,
                expires_at: current.request.expires_at,
                grant: body.grant.clone(),
            },
        )
        .await?)
}

fn command_decision(body: &UnifiedDecisionBody) -> Result<ApprovalDecision, ApprovalServiceError> {
    match (&body.decision, &body.grant) {
        (Decision::Deny, ApprovalGrant::Once) => Ok(ApprovalDecision::Reject),
        (Decision::Allow, ApprovalGrant::Once) => Ok(ApprovalDecision::ApproveOnce),
        (Decision::Allow, ApprovalGrant::Session) => Ok(ApprovalDecision::ApproveSession),
        (Decision::Allow, ApprovalGrant::Rule) => Ok(ApprovalDecision::ApproveGlobal),
        _ => Err(ApprovalServiceError::InvalidGrant),
    }
}

fn local_user() -> GovernanceSubject {
    GovernanceSubject {
        kind: GovernanceSubjectKind::User,
        id: "local-user".into(),
    }
}

fn domain_for(state: &ApprovalState) -> Result<ApprovalDomain, ApprovalServiceError> {
    domain_for_capability(&state.request.capability)
}

fn domain_for_capability(capability: &Capability) -> Result<ApprovalDomain, ApprovalServiceError> {
    match capability {
        Capability::CommandExecute => Ok(ApprovalDomain::Command),
        Capability::PlanExecute => Ok(ApprovalDomain::Plan),
        Capability::MemoryApply => Ok(ApprovalDomain::Memory),
        _ => Err(ApprovalServiceError::PayloadUnavailable),
    }
}

fn event_status(kind: &ApprovalLedgerEventKind) -> ApprovalStatus {
    match kind {
        ApprovalLedgerEventKind::Requested => ApprovalStatus::Pending,
        ApprovalLedgerEventKind::Allowed => ApprovalStatus::Allowed,
        ApprovalLedgerEventKind::Denied => ApprovalStatus::Denied,
        ApprovalLedgerEventKind::Revoked => ApprovalStatus::Revoked,
        ApprovalLedgerEventKind::Consumed => ApprovalStatus::Consumed,
        ApprovalLedgerEventKind::Expired => ApprovalStatus::Expired,
    }
}

fn reason_for_status(status: &ApprovalStatus) -> Option<ApprovalReasonCode> {
    match status {
        ApprovalStatus::Pending | ApprovalStatus::Allowed => None,
        ApprovalStatus::Denied => Some(ApprovalReasonCode::ApprovalDenied),
        ApprovalStatus::Revoked => Some(ApprovalReasonCode::ApprovalRevoked),
        ApprovalStatus::Consumed => Some(ApprovalReasonCode::ApprovalAlreadyConsumed),
        ApprovalStatus::Expired => Some(ApprovalReasonCode::ApprovalExpired),
    }
}

fn actions(domain: ApprovalDomain, status: &ApprovalStatus) -> Vec<String> {
    match status {
        ApprovalStatus::Pending => match domain {
            ApprovalDomain::Memory => vec!["allow", "deny", "cancel", "edit"],
            ApprovalDomain::Plan => vec!["allow", "deny", "cancel", "edit"],
            ApprovalDomain::Command => vec!["allow", "deny", "cancel"],
        },
        ApprovalStatus::Allowed if domain == ApprovalDomain::Memory => vec!["apply"],
        _ => Vec::new(),
    }
    .into_iter()
    .map(str::to_string)
    .collect()
}

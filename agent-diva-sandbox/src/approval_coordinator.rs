//! Async coordination for recoverable command approvals.

use crate::command_rules::{safe_prefix_suggestion, CommandRuleStore, SafePrefixSuggestion};
use crate::governance_adapter::{
    adapt_command_approval_decision, adapt_command_approval_request, SandboxGovernanceContext,
};
use agent_diva_core::governance::{
    ApprovalCoordinator, ApprovalState, AuditCorrelation, AutonomyLevel, ContentDigest,
    CoordinatedApproval, DigestAlgorithm, GovernanceSubject, GovernanceSubjectKind, PolicyContext,
    RiskClass,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, oneshot, Mutex};
use uuid::Uuid;

fn emit_approval_audit(approval_id: &str, decision: &str, context: String) {
    agent_diva_core::audit::emit(agent_diva_core::audit::AuditEvent::DecisionPoint {
        agent_id: approval_id.to_string(),
        decision: decision.to_string(),
        context,
    });
}

/// Default lifetime for a pending command approval.
pub const DEFAULT_APPROVAL_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandApprovalScope {
    pub channel: String,
    pub chat_id: String,
    pub session_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandApprovalRequest {
    pub approval_id: String,
    pub command: String,
    pub cwd: PathBuf,
    pub reason: String,
    pub scope: CommandApprovalScope,
    pub created_at: DateTime<Utc>,
    pub timeout_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_prefix: Option<SafePrefixSuggestion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    ApproveOnce,
    ApproveSession,
    ApproveGlobal,
    Reject,
}

impl ApprovalDecision {
    pub fn allows_execution(self) -> bool {
        matches!(
            self,
            Self::ApproveOnce | Self::ApproveSession | Self::ApproveGlobal
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandApprovalStatus {
    Approved,
    Rejected,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveApprovalResponse {
    pub approval_id: String,
    pub decision: ApprovalDecision,
    pub status: CommandApprovalStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalResolveError {
    NotFound,
    AlreadyResolved,
    VersionConflict,
    IdempotencyConflict,
    Expired,
    AlreadyConsumed,
    InvalidTransition,
    InvalidGlobalApproval,
    Persistence,
}

struct PendingApproval {
    request: CommandApprovalRequest,
    response_tx: oneshot::Sender<ApprovalDecision>,
    governance_state: Option<ApprovalState>,
}

struct SessionApproval {
    key: (CommandApprovalScope, String, PathBuf),
    expires_at: DateTime<Utc>,
}

#[derive(Default)]
struct CoordinatorState {
    pending: HashMap<String, PendingApproval>,
    resolved: HashSet<String>,
    session_approvals: Vec<SessionApproval>,
}

/// Shared, process-local approval coordinator used by tools and transports.
#[derive(Clone)]
pub struct CommandApprovalCoordinator {
    state: Arc<Mutex<CoordinatorState>>,
    request_tx: broadcast::Sender<CommandApprovalRequest>,
    timeout: Duration,
    command_rules: Option<Arc<CommandRuleStore>>,
    governance: Option<ApprovalCoordinator>,
    workspace_id: String,
}

impl Default for CommandApprovalCoordinator {
    fn default() -> Self {
        Self::new(DEFAULT_APPROVAL_TIMEOUT)
    }
}

impl CommandApprovalCoordinator {
    pub fn new(timeout: Duration) -> Self {
        let (request_tx, _) = broadcast::channel(128);
        Self {
            state: Arc::new(Mutex::new(CoordinatorState::default())),
            request_tx,
            timeout,
            command_rules: None,
            governance: None,
            workspace_id: "default".into(),
        }
    }

    /// Attach the durable governance authority used for all human decisions.
    pub fn governed(
        mut self,
        governance: ApprovalCoordinator,
        workspace_id: impl Into<String>,
    ) -> Self {
        self.governance = Some(governance);
        self.workspace_id = workspace_id.into();
        self
    }

    /// Revoke command approvals that cannot safely be resumed after restart.
    pub async fn recover_incomplete(
        &self,
    ) -> Result<usize, agent_diva_core::governance::ApprovalLedgerError> {
        let Some(governance) = &self.governance else {
            return Ok(0);
        };
        let evaluated_at = Utc::now();
        let mut cursor = None;
        let mut revoked = 0;
        loop {
            let page = governance
                .states_page(cursor.as_deref(), 100, evaluated_at)
                .await?;
            for state in page.states {
                if state.request.capability
                    != agent_diva_core::governance::Capability::CommandExecute
                    || state.request.resource.workspace_id != self.workspace_id
                    || !matches!(
                        state.status,
                        agent_diva_core::governance::ApprovalStatus::Pending
                            | agent_diva_core::governance::ApprovalStatus::Allowed
                    )
                {
                    continue;
                }
                let request_id = &state.request.correlation.request_id;
                governance
                    .revoke(
                        request_id,
                        state.version,
                        &format!("sandbox-restart-revoke-{request_id}-v{}", state.version),
                        GovernanceSubject {
                            kind: GovernanceSubjectKind::System,
                            id: "manager-restart".into(),
                        },
                        evaluated_at,
                    )
                    .await?;
                revoked += 1;
            }
            let Some(next) = page.next_cursor else {
                break;
            };
            cursor = Some(next);
        }
        Ok(revoked)
    }

    pub fn with_command_rules(mut self, command_rules: Arc<CommandRuleStore>) -> Self {
        self.command_rules = Some(command_rules);
        self
    }

    pub fn command_rules(&self) -> Option<&Arc<CommandRuleStore>> {
        self.command_rules.as_ref()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CommandApprovalRequest> {
        self.request_tx.subscribe()
    }

    pub async fn request(
        &self,
        command: String,
        cwd: PathBuf,
        reason: String,
        scope: CommandApprovalScope,
    ) -> (String, CommandApprovalStatus) {
        if self
            .command_rules
            .as_ref()
            .is_some_and(|rules| rules.allows(&command))
        {
            emit_approval_audit(
                "",
                "command_rule_reused",
                format!("session={}", scope.session_key),
            );
            return (String::new(), CommandApprovalStatus::Approved);
        }
        let session_key = (scope.clone(), command.clone(), cwd.clone());
        let now = Utc::now();
        let mut state = self.state.lock().await;
        state
            .session_approvals
            .retain(|approval| approval.expires_at > now);
        if state
            .session_approvals
            .iter()
            .any(|approval| approval.key == session_key)
        {
            return (String::new(), CommandApprovalStatus::Approved);
        }
        drop(state);

        let approval_id = Uuid::new_v4().to_string();
        let suggested_prefix = safe_prefix_suggestion(&command);
        let request = CommandApprovalRequest {
            approval_id: approval_id.clone(),
            command,
            cwd,
            reason,
            scope,
            created_at: Utc::now(),
            timeout_seconds: self.timeout.as_secs(),
            suggested_prefix,
        };
        let (response_tx, response_rx) = oneshot::channel();
        let governance_state = if let Some(governance) = &self.governance {
            let digest = ContentDigest {
                algorithm: DigestAlgorithm::Sha256,
                value: format!(
                    "{:x}",
                    Sha256::digest(
                        format!("{}\0{}", request.command, request.cwd.display()).as_bytes()
                    )
                ),
            };
            let governed = adapt_command_approval_request(
                request.clone(),
                SandboxGovernanceContext {
                    correlation: AuditCorrelation {
                        request_id: approval_id.clone(),
                        turn_id: approval_id.clone(),
                        session_id: request.scope.session_key.clone(),
                        trace_id: None,
                    },
                    subject: GovernanceSubject {
                        kind: GovernanceSubjectKind::Agent,
                        id: "sandbox".into(),
                    },
                    workspace_id: self.workspace_id.clone(),
                    content_digest: digest,
                    policy_version: "sandbox-exec-v1".into(),
                    risk: RiskClass::High,
                    expires_at: request.created_at
                        + chrono::Duration::from_std(self.timeout)
                            .unwrap_or_else(|_| chrono::Duration::minutes(5)),
                    evidence_refs: vec![],
                },
            );
            let context = PolicyContext {
                evaluated_at: request.created_at,
                autonomy: AutonomyLevel::L3,
                explicit_user_decision: None,
                restrictions: vec![],
                authorizations: vec![],
            };
            match governance
                .coordinate(
                    &governed,
                    &context,
                    &format!("sandbox-submit-{approval_id}"),
                )
                .await
            {
                Ok(CoordinatedApproval::Pending { state, .. }) => Some(*state),
                _ => return (approval_id, CommandApprovalStatus::Cancelled),
            }
        } else {
            None
        };
        self.state.lock().await.pending.insert(
            approval_id.clone(),
            PendingApproval {
                request: request.clone(),
                response_tx,
                governance_state,
            },
        );
        emit_approval_audit(
            &approval_id,
            "command_approval_requested",
            format!("session={}", request.scope.session_key),
        );
        let _ = self.request_tx.send(request);

        let status = match tokio::time::timeout(self.timeout, response_rx).await {
            Ok(Ok(decision)) if decision.allows_execution() => CommandApprovalStatus::Approved,
            Ok(Ok(_)) => CommandApprovalStatus::Rejected,
            Ok(Err(_)) => CommandApprovalStatus::Cancelled,
            Err(_) => {
                if let Some(governance) = &self.governance {
                    let governed = self
                        .state
                        .lock()
                        .await
                        .pending
                        .get(&approval_id)
                        .and_then(|pending| pending.governance_state.clone());
                    if let Some(governed) = governed {
                        let occurred_at = governed.request.expires_at;
                        let _ = governance
                            .expire(
                                &approval_id,
                                governed.version,
                                &format!("sandbox-expire-{approval_id}"),
                                occurred_at,
                            )
                            .await;
                    }
                }
                CommandApprovalStatus::Expired
            }
        };
        let mut state = self.state.lock().await;
        state.pending.remove(&approval_id);
        state.resolved.insert(approval_id.clone());
        emit_approval_audit(
            &approval_id,
            match status {
                CommandApprovalStatus::Approved => "command_approval_approved",
                CommandApprovalStatus::Rejected => "command_approval_rejected",
                CommandApprovalStatus::Expired => "command_approval_expired",
                CommandApprovalStatus::Cancelled => "command_approval_cancelled",
            },
            "command approval terminal state".to_string(),
        );
        (approval_id, status)
    }

    pub async fn resolve(
        &self,
        approval_id: &str,
        decision: ApprovalDecision,
    ) -> Result<ResolveApprovalResponse, ApprovalResolveError> {
        self.resolve_with_preconditions(approval_id, decision, None, None)
            .await
    }

    /// Resolve through the durable coordinator with caller-supplied CAS and
    /// idempotency preconditions. Legacy callers may omit both via [`Self::resolve`].
    pub async fn resolve_with_preconditions(
        &self,
        approval_id: &str,
        decision: ApprovalDecision,
        expected_version: Option<u64>,
        idempotency_key: Option<&str>,
    ) -> Result<ResolveApprovalResponse, ApprovalResolveError> {
        let state = self.state.lock().await;
        let Some(pending) = state.pending.get(approval_id) else {
            drop(state);
            if let (Some(governance), Some(expected_version), Some(idempotency_key)) =
                (&self.governance, expected_version, idempotency_key)
            {
                if let Ok(current) = governance.state(approval_id, Utc::now()).await {
                    if let Some(receipt) = current.receipt.clone() {
                        let matches = match decision {
                            ApprovalDecision::ApproveOnce => {
                                receipt.decision == agent_diva_core::governance::Decision::Allow
                                    && receipt.grant
                                        == agent_diva_core::governance::ApprovalGrant::Once
                            }
                            ApprovalDecision::ApproveSession => {
                                receipt.decision == agent_diva_core::governance::Decision::Allow
                                    && receipt.grant
                                        == agent_diva_core::governance::ApprovalGrant::Session
                            }
                            ApprovalDecision::ApproveGlobal => {
                                receipt.decision == agent_diva_core::governance::Decision::Allow
                                    && receipt.grant
                                        == agent_diva_core::governance::ApprovalGrant::Rule
                            }
                            ApprovalDecision::Reject => {
                                receipt.decision == agent_diva_core::governance::Decision::Deny
                            }
                        };
                        if matches {
                            governance
                                .decide(approval_id, expected_version, idempotency_key, receipt)
                                .await
                                .map_err(map_governance_error)?;
                            return Ok(ResolveApprovalResponse {
                                approval_id: approval_id.to_string(),
                                decision,
                                status: if decision.allows_execution() {
                                    CommandApprovalStatus::Approved
                                } else {
                                    CommandApprovalStatus::Rejected
                                },
                            });
                        }
                    }
                }
            }
            let state = self.state.lock().await;
            return if state.resolved.contains(approval_id) {
                Err(ApprovalResolveError::AlreadyResolved)
            } else {
                Err(ApprovalResolveError::NotFound)
            };
        };
        let request = pending.request.clone();
        let governed_state = pending.governance_state.clone();
        drop(state);
        let decided_at = Utc::now();
        let receipt_expires_at = if decision == ApprovalDecision::ApproveSession {
            decided_at + chrono::Duration::minutes(5)
        } else {
            request.created_at
                + chrono::Duration::from_std(self.timeout)
                    .unwrap_or_else(|_| chrono::Duration::minutes(5))
        };
        let mut decided_durable_state = None;
        if let (Some(governance), Some(current)) = (&self.governance, &governed_state) {
            if expected_version.is_some_and(|expected| expected != current.version) {
                return Err(ApprovalResolveError::VersionConflict);
            }
            let governed_request = adapt_command_approval_request(
                request.clone(),
                SandboxGovernanceContext {
                    correlation: current.request.correlation.clone(),
                    subject: current.request.subject.clone(),
                    workspace_id: current.request.resource.workspace_id.clone(),
                    content_digest: current.request.content_digest.clone(),
                    policy_version: current.request.policy_version.clone(),
                    risk: current.request.risk.clone(),
                    expires_at: current.request.expires_at,
                    evidence_refs: vec![],
                },
            );
            let receipt = adapt_command_approval_decision(
                &governed_request,
                decision,
                GovernanceSubject {
                    kind: GovernanceSubjectKind::User,
                    id: "command-approval-user".into(),
                },
                decided_at,
                receipt_expires_at,
            );
            let decided = governance
                .decide(
                    approval_id,
                    current.version,
                    idempotency_key
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("sandbox-decide-{approval_id}"))
                        .as_str(),
                    receipt,
                )
                .await
                .map_err(map_governance_error)?;
            decided_durable_state = Some(if decision == ApprovalDecision::ApproveOnce {
                governance
                    .consume_once(
                        approval_id,
                        decided.version,
                        &format!("sandbox-consume-{approval_id}"),
                        Utc::now(),
                    )
                    .await
                    .map_err(map_governance_error)?
            } else {
                decided
            });
        }
        if decision == ApprovalDecision::ApproveGlobal {
            let state = self.state.lock().await;
            let pending = state
                .pending
                .get(approval_id)
                .ok_or(ApprovalResolveError::AlreadyResolved)?;
            let suggestion = pending
                .request
                .suggested_prefix
                .as_ref()
                .ok_or(ApprovalResolveError::InvalidGlobalApproval)?;
            let rules = self
                .command_rules
                .as_ref()
                .ok_or(ApprovalResolveError::InvalidGlobalApproval)?;
            let persistence = rules
                .add_suggestion(suggestion)
                .map_err(|error| match error {
                    crate::command_rules::CommandRuleError::UnsafeSuggestion => {
                        ApprovalResolveError::InvalidGlobalApproval
                    }
                    _ => ApprovalResolveError::Persistence,
                });
            drop(state);
            if let Err(error) = persistence {
                if let (Some(governance), Some(durable)) =
                    (&self.governance, &decided_durable_state)
                {
                    let _ = governance
                        .revoke(
                            approval_id,
                            durable.version,
                            &format!("sandbox-rule-persist-revoke-{approval_id}"),
                            GovernanceSubject {
                                kind: GovernanceSubjectKind::System,
                                id: "rule-persistence".into(),
                            },
                            Utc::now(),
                        )
                        .await;
                }
                return Err(error);
            }
        }
        let mut state = self.state.lock().await;
        let pending = state
            .pending
            .remove(approval_id)
            .ok_or(ApprovalResolveError::NotFound)?;
        if decision == ApprovalDecision::ApproveSession {
            state.session_approvals.push(SessionApproval {
                key: (
                    pending.request.scope.clone(),
                    pending.request.command.clone(),
                    pending.request.cwd.clone(),
                ),
                expires_at: receipt_expires_at,
            });
        }
        state.resolved.insert(approval_id.to_string());
        let _ = pending.response_tx.send(decision);
        emit_approval_audit(
            approval_id,
            "command_approval_resolved",
            format!("decision={decision:?}"),
        );
        Ok(ResolveApprovalResponse {
            approval_id: approval_id.to_string(),
            decision,
            status: if decision.allows_execution() {
                CommandApprovalStatus::Approved
            } else {
                CommandApprovalStatus::Rejected
            },
        })
    }

    pub async fn pending(
        &self,
        scope: Option<&CommandApprovalScope>,
    ) -> Vec<CommandApprovalRequest> {
        let state = self.state.lock().await;
        let mut requests: Vec<_> = state
            .pending
            .values()
            .filter(|pending| scope.map_or(true, |scope| &pending.request.scope == scope))
            .map(|pending| pending.request.clone())
            .collect();
        requests.sort_by_key(|request| request.created_at);
        requests
    }

    /// Cancel one pending command through durable CAS and wake its waiter as
    /// cancelled by dropping the response channel.
    pub async fn cancel_request(
        &self,
        approval_id: &str,
        expected_version: u64,
        idempotency_key: &str,
        actor: GovernanceSubject,
    ) -> Result<(), ApprovalResolveError> {
        let state = self.state.lock().await;
        let pending = state.pending.get(approval_id).ok_or_else(|| {
            if state.resolved.contains(approval_id) {
                ApprovalResolveError::AlreadyResolved
            } else {
                ApprovalResolveError::NotFound
            }
        })?;
        let governed = pending.governance_state.clone();
        drop(state);
        if let (Some(governance), Some(_current)) = (&self.governance, governed) {
            governance
                .revoke(
                    approval_id,
                    expected_version,
                    idempotency_key,
                    actor,
                    Utc::now(),
                )
                .await
                .map_err(map_governance_error)?;
        }
        let mut state = self.state.lock().await;
        state
            .pending
            .remove(approval_id)
            .ok_or(ApprovalResolveError::AlreadyResolved)?;
        state.resolved.insert(approval_id.to_string());
        Ok(())
    }

    pub async fn cancel_scope(&self, scope: &CommandApprovalScope) -> usize {
        let mut state = self.state.lock().await;
        let ids: Vec<_> = state
            .pending
            .iter()
            .filter(|(_, pending)| &pending.request.scope == scope)
            .map(|(id, _)| id.clone())
            .collect();
        let pending_states: Vec<_> = ids
            .iter()
            .filter_map(|id| {
                state
                    .pending
                    .get(id)
                    .and_then(|pending| pending.governance_state.clone())
                    .map(|governed| (id.clone(), governed))
            })
            .collect();
        for id in &ids {
            state.pending.remove(id);
            state.resolved.insert(id.clone());
        }
        state
            .session_approvals
            .retain(|approval| &approval.key.0 != scope);
        drop(state);
        if let Some(governance) = &self.governance {
            for (id, governed) in pending_states {
                let _ = governance
                    .revoke(
                        &id,
                        governed.version,
                        &format!("sandbox-revoke-{id}"),
                        GovernanceSubject {
                            kind: GovernanceSubjectKind::System,
                            id: "scope-cancel".into(),
                        },
                        Utc::now(),
                    )
                    .await;
            }
        }
        for id in &ids {
            emit_approval_audit(
                id,
                "command_approval_cancelled",
                format!("session={}", scope.session_key),
            );
        }
        ids.len()
    }
}

fn map_governance_error(
    error: agent_diva_core::governance::ApprovalLedgerError,
) -> ApprovalResolveError {
    use agent_diva_core::governance::ApprovalLedgerError;
    match error {
        ApprovalLedgerError::NotFound => ApprovalResolveError::NotFound,
        ApprovalLedgerError::VersionConflict => ApprovalResolveError::VersionConflict,
        ApprovalLedgerError::IdempotencyConflict => ApprovalResolveError::IdempotencyConflict,
        ApprovalLedgerError::Expired => ApprovalResolveError::Expired,
        ApprovalLedgerError::AlreadyConsumed => ApprovalResolveError::AlreadyConsumed,
        ApprovalLedgerError::InvalidTransition => ApprovalResolveError::InvalidTransition,
        ApprovalLedgerError::Validation(_) | ApprovalLedgerError::Persistence(_) => {
            ApprovalResolveError::Persistence
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_rules::CommandRuleStore;
    use agent_diva_core::governance::{ApprovalStatus, SqliteGovernanceLedger};
    use serde_json::json;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn governed_coordinator() -> (CommandApprovalCoordinator, ApprovalCoordinator) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        let governance =
            ApprovalCoordinator::new(Arc::new(SqliteGovernanceLedger::new(pool).await.unwrap()));
        (
            CommandApprovalCoordinator::new(Duration::from_secs(2))
                .governed(governance.clone(), "workspace-1"),
            governance,
        )
    }

    async fn wait_for_pending(coordinator: &CommandApprovalCoordinator) -> CommandApprovalRequest {
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(request) = coordinator.pending(None).await.into_iter().next() {
                    return request;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("governed request becomes pending")
    }

    fn scope(chat: &str) -> CommandApprovalScope {
        CommandApprovalScope {
            channel: "api".into(),
            chat_id: chat.into(),
            session_key: format!("api:{chat}"),
        }
    }

    #[test]
    fn legacy_command_approval_request_json_contract_is_unchanged() {
        let request = CommandApprovalRequest {
            approval_id: "approval-1".into(),
            command: "echo ok".into(),
            cwd: PathBuf::from("."),
            reason: "sandbox denied".into(),
            scope: scope("one"),
            created_at: DateTime::parse_from_rfc3339("2026-07-29T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            timeout_seconds: 300,
            suggested_prefix: None,
        };

        assert_eq!(
            serde_json::to_value(request).unwrap(),
            json!({
                "approval_id": "approval-1",
                "command": "echo ok",
                "cwd": ".",
                "reason": "sandbox denied",
                "scope": {
                    "channel": "api",
                    "chat_id": "one",
                    "session_key": "api:one"
                },
                "created_at": "2026-07-29T12:00:00Z",
                "timeout_seconds": 300
            })
        );
    }

    #[tokio::test]
    async fn global_approval_persists_and_reuses_across_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let rules = Arc::new(CommandRuleStore::open(dir.path().join("execpolicy.toml")).unwrap());
        let coordinator = CommandApprovalCoordinator::new(Duration::from_secs(2))
            .with_command_rules(rules.clone());
        let task = tokio::spawn({
            let coordinator = coordinator.clone();
            async move {
                coordinator
                    .request(
                        "git status".into(),
                        PathBuf::from("."),
                        "sandbox denied".into(),
                        scope("first"),
                    )
                    .await
            }
        });
        tokio::task::yield_now().await;
        let pending = coordinator.pending(None).await;
        assert!(pending[0].suggested_prefix.is_some());
        coordinator
            .resolve(&pending[0].approval_id, ApprovalDecision::ApproveGlobal)
            .await
            .unwrap();
        assert_eq!(task.await.unwrap().1, CommandApprovalStatus::Approved);
        assert_eq!(
            coordinator
                .request(
                    "git status".into(),
                    PathBuf::from("elsewhere"),
                    "sandbox denied".into(),
                    scope("second"),
                )
                .await
                .1,
            CommandApprovalStatus::Approved
        );
        assert!(coordinator.pending(None).await.is_empty());
        assert_eq!(rules.list().len(), 1);
    }

    #[tokio::test]
    async fn global_approval_rejects_commands_without_safe_suggestion() {
        let dir = tempfile::tempdir().unwrap();
        let rules = Arc::new(CommandRuleStore::open(dir.path().join("execpolicy.toml")).unwrap());
        let coordinator =
            CommandApprovalCoordinator::new(Duration::from_secs(2)).with_command_rules(rules);
        let task = tokio::spawn({
            let coordinator = coordinator.clone();
            async move {
                coordinator
                    .request(
                        "cargo build".into(),
                        PathBuf::from("."),
                        "sandbox denied".into(),
                        scope("unsafe"),
                    )
                    .await
            }
        });
        tokio::task::yield_now().await;
        let pending = coordinator.pending(None).await;
        assert!(pending[0].suggested_prefix.is_none());
        assert!(matches!(
            coordinator
                .resolve(&pending[0].approval_id, ApprovalDecision::ApproveGlobal)
                .await,
            Err(ApprovalResolveError::InvalidGlobalApproval)
        ));
        coordinator
            .resolve(&pending[0].approval_id, ApprovalDecision::Reject)
            .await
            .unwrap();
        assert_eq!(task.await.unwrap().1, CommandApprovalStatus::Rejected);
    }

    #[tokio::test]
    async fn approve_once_resumes_exact_request_and_is_consumed() {
        let coordinator = CommandApprovalCoordinator::new(Duration::from_secs(1));
        let task = tokio::spawn({
            let coordinator = coordinator.clone();
            async move {
                coordinator
                    .request("echo ok".into(), ".".into(), "denied".into(), scope("one"))
                    .await
            }
        });
        let request = coordinator.subscribe();
        drop(request);
        tokio::task::yield_now().await;
        let pending = coordinator.pending(None).await;
        assert_eq!(pending.len(), 1);
        coordinator
            .resolve(&pending[0].approval_id, ApprovalDecision::ApproveOnce)
            .await
            .expect("resolve");
        assert_eq!(task.await.expect("join").1, CommandApprovalStatus::Approved);
        assert!(coordinator.pending(None).await.is_empty());
    }

    #[tokio::test]
    async fn session_approval_is_scoped_by_session_command_and_cwd() {
        let coordinator = CommandApprovalCoordinator::new(Duration::from_secs(1));
        let first = tokio::spawn({
            let coordinator = coordinator.clone();
            async move {
                coordinator
                    .request("echo ok".into(), ".".into(), "denied".into(), scope("one"))
                    .await
            }
        });
        tokio::task::yield_now().await;
        let pending = coordinator.pending(None).await;
        coordinator
            .resolve(&pending[0].approval_id, ApprovalDecision::ApproveSession)
            .await
            .expect("resolve");
        assert_eq!(
            first.await.expect("join").1,
            CommandApprovalStatus::Approved
        );
        assert_eq!(
            coordinator
                .request("echo ok".into(), ".".into(), "denied".into(), scope("one"))
                .await
                .1,
            CommandApprovalStatus::Approved
        );
        assert!(coordinator.pending(Some(&scope("two"))).await.is_empty());
    }

    #[tokio::test]
    async fn timeout_and_cancel_leave_no_pending_requests() {
        let coordinator = CommandApprovalCoordinator::new(Duration::from_millis(20));
        let (_, status) = coordinator
            .request("echo ok".into(), ".".into(), "denied".into(), scope("one"))
            .await;
        assert_eq!(status, CommandApprovalStatus::Expired);
        assert!(coordinator.pending(None).await.is_empty());

        let task = tokio::spawn({
            let coordinator = coordinator.clone();
            async move {
                coordinator
                    .request(
                        "echo later".into(),
                        ".".into(),
                        "denied".into(),
                        scope("one"),
                    )
                    .await
            }
        });
        tokio::task::yield_now().await;
        assert_eq!(coordinator.cancel_scope(&scope("one")).await, 1);
        assert_eq!(
            task.await.expect("join").1,
            CommandApprovalStatus::Cancelled
        );
    }

    #[tokio::test]
    async fn governed_approve_once_is_consumed_before_waiter_runs() {
        let (coordinator, governance) = governed_coordinator().await;
        let waiter = tokio::spawn({
            let coordinator = coordinator.clone();
            async move {
                coordinator
                    .request(
                        "echo once".into(),
                        ".".into(),
                        "denied".into(),
                        scope("one"),
                    )
                    .await
            }
        });
        let id = wait_for_pending(&coordinator).await.approval_id;
        coordinator
            .resolve(&id, ApprovalDecision::ApproveOnce)
            .await
            .unwrap();
        assert_eq!(waiter.await.unwrap().1, CommandApprovalStatus::Approved);

        let durable = governance.state(&id, Utc::now()).await.unwrap();
        assert_eq!(durable.status, ApprovalStatus::Consumed);
        let persisted = serde_json::to_string(&durable).unwrap();
        assert!(!persisted.contains("echo once"));
        assert!(!persisted.contains("denied"));
    }

    #[tokio::test]
    async fn governed_timeout_materializes_expired_event() {
        let (base, governance) = governed_coordinator().await;
        let coordinator = CommandApprovalCoordinator {
            timeout: Duration::from_millis(20),
            ..base
        };
        let (id, status) = coordinator
            .request(
                "echo timeout".into(),
                ".".into(),
                "denied".into(),
                scope("timeout"),
            )
            .await;
        assert_eq!(status, CommandApprovalStatus::Expired);
        let durable = governance.state(&id, Utc::now()).await.unwrap();
        assert_eq!(durable.status, ApprovalStatus::Expired);
        assert_eq!(durable.version, 2);
        let events = governance.events_page(None, 10).await.unwrap();
        assert_eq!(
            events
                .events
                .iter()
                .filter(|item| item.event.request_id == id)
                .count(),
            2
        );
    }

    #[tokio::test]
    async fn governed_scope_cancel_revokes_without_execution() {
        let (coordinator, governance) = governed_coordinator().await;
        let waiter = tokio::spawn({
            let coordinator = coordinator.clone();
            async move {
                coordinator
                    .request(
                        "echo never".into(),
                        ".".into(),
                        "denied".into(),
                        scope("one"),
                    )
                    .await
            }
        });
        let id = wait_for_pending(&coordinator).await.approval_id;
        assert_eq!(coordinator.cancel_scope(&scope("one")).await, 1);
        assert_eq!(waiter.await.unwrap().1, CommandApprovalStatus::Cancelled);
        assert_eq!(
            governance.state(&id, Utc::now()).await.unwrap().status,
            ApprovalStatus::Revoked
        );
    }

    #[tokio::test]
    async fn restart_recovery_revokes_pending_and_unconsumed_allowed() {
        let (coordinator, governance) = governed_coordinator().await;
        let pending_waiter = tokio::spawn({
            let coordinator = coordinator.clone();
            async move {
                coordinator
                    .request(
                        "echo pending".into(),
                        ".".into(),
                        "denied".into(),
                        scope("pending"),
                    )
                    .await
            }
        });
        let pending_id = wait_for_pending(&coordinator).await.approval_id;
        let recovery = CommandApprovalCoordinator::new(Duration::from_secs(2))
            .governed(governance.clone(), "workspace-1");
        assert_eq!(recovery.recover_incomplete().await.unwrap(), 1);
        assert_eq!(
            governance
                .state(&pending_id, Utc::now())
                .await
                .unwrap()
                .status,
            ApprovalStatus::Revoked
        );
        coordinator.cancel_scope(&scope("pending")).await;
        assert_eq!(
            pending_waiter.await.unwrap().1,
            CommandApprovalStatus::Cancelled
        );

        let allowed_waiter = tokio::spawn({
            let coordinator = coordinator.clone();
            async move {
                coordinator
                    .request(
                        "echo allowed".into(),
                        ".".into(),
                        "denied".into(),
                        scope("allowed"),
                    )
                    .await
            }
        });
        let allowed_id = wait_for_pending(&coordinator).await.approval_id;
        coordinator
            .resolve(&allowed_id, ApprovalDecision::ApproveSession)
            .await
            .unwrap();
        assert_eq!(
            allowed_waiter.await.unwrap().1,
            CommandApprovalStatus::Approved
        );
        assert_eq!(recovery.recover_incomplete().await.unwrap(), 1);
        assert_eq!(
            governance
                .state(&allowed_id, Utc::now())
                .await
                .unwrap()
                .status,
            ApprovalStatus::Revoked
        );
    }
}

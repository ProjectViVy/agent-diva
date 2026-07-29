//! Async coordination for recoverable command approvals.

use crate::command_rules::{safe_prefix_suggestion, CommandRuleStore, SafePrefixSuggestion};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
    InvalidGlobalApproval,
    Persistence,
}

struct PendingApproval {
    request: CommandApprovalRequest,
    response_tx: oneshot::Sender<ApprovalDecision>,
}

#[derive(Default)]
struct CoordinatorState {
    pending: HashMap<String, PendingApproval>,
    resolved: HashSet<String>,
    session_approvals: HashSet<(CommandApprovalScope, String, PathBuf)>,
}

/// Shared, process-local approval coordinator used by tools and transports.
#[derive(Clone)]
pub struct CommandApprovalCoordinator {
    state: Arc<Mutex<CoordinatorState>>,
    request_tx: broadcast::Sender<CommandApprovalRequest>,
    timeout: Duration,
    command_rules: Option<Arc<CommandRuleStore>>,
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
        }
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
        if self
            .state
            .lock()
            .await
            .session_approvals
            .contains(&session_key)
        {
            return (String::new(), CommandApprovalStatus::Approved);
        }

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
        self.state.lock().await.pending.insert(
            approval_id.clone(),
            PendingApproval {
                request: request.clone(),
                response_tx,
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
            Err(_) => CommandApprovalStatus::Expired,
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
        let mut state = self.state.lock().await;
        let Some(pending) = state.pending.get(approval_id) else {
            return if state.resolved.contains(approval_id) {
                Err(ApprovalResolveError::AlreadyResolved)
            } else {
                Err(ApprovalResolveError::NotFound)
            };
        };
        if decision == ApprovalDecision::ApproveGlobal {
            let suggestion = pending
                .request
                .suggested_prefix
                .as_ref()
                .ok_or(ApprovalResolveError::InvalidGlobalApproval)?;
            let rules = self
                .command_rules
                .as_ref()
                .ok_or(ApprovalResolveError::InvalidGlobalApproval)?;
            rules
                .add_suggestion(suggestion)
                .map_err(|error| match error {
                    crate::command_rules::CommandRuleError::UnsafeSuggestion => {
                        ApprovalResolveError::InvalidGlobalApproval
                    }
                    _ => ApprovalResolveError::Persistence,
                })?;
        }
        let pending = state
            .pending
            .remove(approval_id)
            .ok_or(ApprovalResolveError::NotFound)?;
        if decision == ApprovalDecision::ApproveSession {
            state.session_approvals.insert((
                pending.request.scope.clone(),
                pending.request.command.clone(),
                pending.request.cwd.clone(),
            ));
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

    pub async fn cancel_scope(&self, scope: &CommandApprovalScope) -> usize {
        let mut state = self.state.lock().await;
        let ids: Vec<_> = state
            .pending
            .iter()
            .filter(|(_, pending)| &pending.request.scope == scope)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &ids {
            state.pending.remove(id);
            state.resolved.insert(id.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_rules::CommandRuleStore;
    use serde_json::json;

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
}

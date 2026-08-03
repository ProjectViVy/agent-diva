//! Process-local projection of session-scoped PLAN runtime state.
//!
//! Durable plan revisions and prepared execution contexts live in the canonical
//! planning store. This registry is rebuilt from those records after restart and
//! never acts as approval authority.

use anyhow::anyhow;
use chrono::Utc;
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};
use tokio::sync::RwLock;

use super::{
    assert_report_ready_for_approval, revision_hash, ExecutionContextBoundary,
    ExecutionContextPolicy, ExecutionInitializationStatus, ExecutionSession,
    ExecutionSessionStatus, ExecutionTodo, PlanId, PlanReport, PlanReportStatus, PlanRevision,
    PlanRevisionApproval, PlanRevisionAuthor,
};
use super::{PlanPhase, PlanStatus, TodoPriority, TodoStatus};
use crate::bus::{PlanRuntimeState, PlanRuntimeTodo};

/// The draft currently available to a single session during this process.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanReportDetail {
    pub report: PlanReport,
    pub revision: PlanRevision,
}

#[derive(Clone)]
pub struct EphemeralPlanRegistry {
    sessions: Arc<RwLock<HashMap<String, SessionPlanState>>>,
}

#[derive(Clone)]
struct SessionPlanState {
    draft: Option<PlanReportDetail>,
    execution: Option<ExecutionState>,
}

#[derive(Clone)]
struct ExecutionState {
    session: ExecutionSession,
    markdown: String,
    todos: Vec<ExecutionTodo>,
}

impl Default for EphemeralPlanRegistry {
    /// Always share the process-wide registry.
    ///
    /// `#[derive(Default)]` would allocate a private empty map, which breaks
    /// Manager approve/execution against drafts created by the agent loop
    /// (`plan draft not found for session`).
    fn default() -> Self {
        Self::new()
    }
}

impl EphemeralPlanRegistry {
    pub fn new() -> Self {
        // Components are constructed independently (agent loop, gateway, and
        // desktop bridge), but they run in one backend process.  They share
        // this registry instance while retaining strict session-key isolation.
        //
        // Both `new()` and `Default` must return handles to this same map;
        // Manager `PlanningService` historically constructed via Default while
        // the agent loop uses `new()`.
        static PROCESS_REGISTRY: OnceLock<Arc<RwLock<HashMap<String, SessionPlanState>>>> =
            OnceLock::new();
        Self {
            sessions: PROCESS_REGISTRY
                .get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
                .clone(),
        }
    }

    /// Replace any existing draft/execution state for this session.
    pub async fn create_report(
        &self,
        session_key: &str,
        title: &str,
        markdown: &str,
        author: PlanRevisionAuthor,
    ) -> anyhow::Result<PlanReportDetail> {
        let now = Utc::now();
        let report = PlanReport {
            id: PlanId::new(),
            session_key: session_key.to_string(),
            current_revision: 1,
            status: PlanReportStatus::AwaitingApproval,
            created_at: now,
            updated_at: now,
        };
        let detail = PlanReportDetail {
            revision: PlanRevision {
                report_id: report.id.clone(),
                revision: 1,
                title: title.to_string(),
                markdown: markdown.to_string(),
                author,
                created_at: now,
            },
            report,
        };
        self.sessions.write().await.insert(
            session_key.to_string(),
            SessionPlanState {
                draft: Some(detail.clone()),
                execution: None,
            },
        );
        Ok(detail)
    }

    pub async fn append_revision(
        &self,
        session_key: &str,
        report_id: &PlanId,
        expected_revision: i64,
        title: &str,
        markdown: &str,
        author: PlanRevisionAuthor,
    ) -> anyhow::Result<PlanReportDetail> {
        let mut sessions = self.sessions.write().await;
        let state = sessions
            .get_mut(session_key)
            .ok_or_else(|| anyhow!("plan draft not found for session"))?;
        let draft = state
            .draft
            .as_mut()
            .filter(|detail| detail.report.id == *report_id)
            .ok_or_else(|| anyhow!("plan draft is not available for this session"))?;
        if draft.report.current_revision != expected_revision {
            return Err(anyhow!("plan report revision conflict"));
        }
        let now = Utc::now();
        let revision = expected_revision + 1;
        draft.report.current_revision = revision;
        draft.report.status = PlanReportStatus::AwaitingApproval;
        draft.report.updated_at = now;
        draft.revision = PlanRevision {
            report_id: report_id.clone(),
            revision,
            title: title.to_string(),
            markdown: markdown.to_string(),
            author,
            created_at: now,
        };
        Ok(draft.clone())
    }

    /// Restores a persisted, still-pending report into the process-local
    /// content cache. This is used after restart; the caller must source the
    /// revision and phase from the canonical store.
    pub async fn restore_report(
        &self,
        session_key: &str,
        detail: PlanReportDetail,
    ) -> anyhow::Result<()> {
        if detail.report.session_key != session_key
            || detail.report.id != detail.revision.report_id
            || detail.report.current_revision != detail.revision.revision
            || detail.report.status != PlanReportStatus::AwaitingApproval
        {
            return Err(anyhow!("invalid persisted plan report projection"));
        }
        self.sessions.write().await.insert(
            session_key.to_string(),
            SessionPlanState {
                draft: Some(detail),
                execution: None,
            },
        );
        Ok(())
    }

    /// Approve only the draft currently resident in `session_key`.
    pub async fn approve_revision(
        &self,
        session_key: &str,
        report_id: &PlanId,
        revision: i64,
        expected_hash: &str,
        context_policy: ExecutionContextPolicy,
        compacted_context: Option<&str>,
    ) -> anyhow::Result<(PlanRevisionApproval, ExecutionSession)> {
        self.approve_revision_with_execution_id(
            session_key,
            report_id,
            revision,
            expected_hash,
            context_policy,
            compacted_context,
            uuid::Uuid::new_v4().to_string(),
        )
        .await
    }

    /// Approve a revision using an execution ID already durably prepared by governance.
    #[allow(clippy::too_many_arguments)]
    pub async fn approve_revision_with_execution_id(
        &self,
        session_key: &str,
        report_id: &PlanId,
        revision: i64,
        expected_hash: &str,
        context_policy: ExecutionContextPolicy,
        compacted_context: Option<&str>,
        execution_id: String,
    ) -> anyhow::Result<(PlanRevisionApproval, ExecutionSession)> {
        let mut sessions = self.sessions.write().await;
        let state = sessions
            .get_mut(session_key)
            .ok_or_else(|| anyhow!("plan draft not found for session"))?;
        let draft = state
            .draft
            .as_ref()
            .filter(|detail| detail.report.id == *report_id)
            .ok_or_else(|| anyhow!("plan draft is not available for this session"))?;
        if draft.report.current_revision != revision
            || draft.report.status != PlanReportStatus::AwaitingApproval
        {
            return Err(anyhow!("plan report is not awaiting approval"));
        }
        let actual_hash = revision_hash(&draft.revision.markdown);
        if actual_hash != expected_hash {
            return Err(anyhow!("plan report revision conflict"));
        }
        assert_report_ready_for_approval(&draft.revision.markdown)
            .map_err(|error| anyhow!(error))?;
        let now = Utc::now();
        let approval = PlanRevisionApproval {
            report_id: report_id.clone(),
            revision,
            revision_hash: actual_hash,
            context_policy,
            approved_at: now,
        };
        let session = ExecutionSession {
            id: execution_id,
            report_id: report_id.clone(),
            revision,
            context_policy,
            status: ExecutionSessionStatus::Executing,
            compacted_context: compacted_context.map(ToOwned::to_owned),
            boundary: None,
            initialization_status: ExecutionInitializationStatus::Pending,
            initialization_error: None,
            created_at: now,
            updated_at: now,
        };
        let markdown = draft.revision.markdown.clone();
        state.draft = None;
        state.execution = Some(ExecutionState {
            session: session.clone(),
            markdown,
            todos: vec![],
        });
        Ok((approval, session))
    }

    pub async fn update_execution_context(
        &self,
        execution_id: &str,
        boundary: Option<ExecutionContextBoundary>,
        compacted_context: Option<String>,
        initialization_status: ExecutionInitializationStatus,
        initialization_error: Option<String>,
    ) -> anyhow::Result<ExecutionSession> {
        let mut sessions = self.sessions.write().await;
        let execution = sessions
            .values_mut()
            .filter_map(|state| state.execution.as_mut())
            .find(|execution| execution.session.id == execution_id)
            .ok_or_else(|| anyhow!("execution session not found"))?;
        execution.session.boundary = boundary;
        execution.session.compacted_context = compacted_context;
        execution.session.initialization_status = initialization_status;
        execution.session.initialization_error = initialization_error;
        execution.session.updated_at = Utc::now();
        Ok(execution.session.clone())
    }

    pub async fn active_execution_for_session(
        &self,
        session_key: &str,
    ) -> Option<ExecutionSession> {
        self.sessions
            .read()
            .await
            .get(session_key)
            .and_then(|state| state.execution.as_ref())
            .map(|execution| execution.session.clone())
    }

    pub async fn restore_execution(
        &self,
        session_key: &str,
        session: ExecutionSession,
        markdown: String,
    ) -> anyhow::Result<()> {
        if session.revision <= 0 || markdown.trim().is_empty() {
            return Err(anyhow!("invalid persisted execution projection"));
        }
        self.sessions.write().await.insert(
            session_key.to_string(),
            SessionPlanState {
                draft: None,
                execution: Some(ExecutionState {
                    session,
                    markdown,
                    todos: Vec::new(),
                }),
            },
        );
        Ok(())
    }

    /// Projects the session-scoped report state into the canonical runtime
    /// shape used by capability policy. The report registry remains a content
    /// and execution-context cache; lifecycle authorization consumes this
    /// projection rather than inferring state from request mode.
    pub async fn runtime_state_for_session(&self, session_key: &str) -> Option<PlanRuntimeState> {
        let sessions = self.sessions.read().await;
        let state = sessions.get(session_key)?;
        if let Some(draft) = state.draft.as_ref() {
            let goal = draft
                .revision
                .markdown
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty() && !line.starts_with('#'))
                .unwrap_or(&draft.revision.title)
                .to_string();
            return Some(PlanRuntimeState {
                plan_id: draft.report.id.0.clone(),
                revision: Some(draft.revision.revision),
                title: draft.revision.title.clone(),
                goal,
                phase: PlanPhase::AwaitingApproval,
                status: PlanStatus::Pending,
                strategy: Some(draft.revision.markdown.clone()),
                summary: draft.revision.markdown.clone(),
                steps: Vec::new(),
                todos: Vec::new(),
                created_at: draft.report.created_at,
                updated_at: draft.report.updated_at,
            });
        }
        let execution = state.execution.as_ref()?;
        let (phase, status) = match execution.session.status {
            ExecutionSessionStatus::Executing => (PlanPhase::Execute, PlanStatus::InProgress),
            ExecutionSessionStatus::Verifying => (PlanPhase::Verify, PlanStatus::InProgress),
            ExecutionSessionStatus::Completed => (PlanPhase::Completed, PlanStatus::Completed),
            ExecutionSessionStatus::Failed => (PlanPhase::Failed, PlanStatus::Failed),
            ExecutionSessionStatus::Partial => (PlanPhase::Partial, PlanStatus::Partial),
        };
        Some(PlanRuntimeState {
            plan_id: execution.session.report_id.0.clone(),
            revision: Some(execution.session.revision),
            title: "Approved plan".to_string(),
            goal: "Execute the approved plan".to_string(),
            phase,
            status,
            strategy: Some(execution.markdown.clone()),
            summary: execution.markdown.clone(),
            steps: Vec::new(),
            todos: execution
                .todos
                .iter()
                .map(|todo| PlanRuntimeTodo {
                    id: todo.id.clone(),
                    plan_step_id: None,
                    title: todo.title.clone(),
                    detail: todo.detail.clone(),
                    status: match todo.status {
                        super::ExecutionTodoStatus::Pending => TodoStatus::Pending,
                        super::ExecutionTodoStatus::InProgress => TodoStatus::InProgress,
                        super::ExecutionTodoStatus::Blocked => TodoStatus::Blocked,
                        super::ExecutionTodoStatus::Completed => TodoStatus::Completed,
                        super::ExecutionTodoStatus::Canceled => TodoStatus::Canceled,
                    },
                    priority: match todo.priority {
                        super::ExecutionTodoPriority::Low => TodoPriority::Low,
                        super::ExecutionTodoPriority::Normal => TodoPriority::Normal,
                        super::ExecutionTodoPriority::High => TodoPriority::High,
                    },
                    evidence_ref: todo.evidence_ref.clone(),
                    block_reason: todo.block_reason.clone(),
                    updated_at: todo.updated_at,
                })
                .collect(),
            created_at: execution.session.created_at,
            updated_at: execution.session.updated_at,
        })
    }

    pub async fn execution_markdown(&self, execution_session_id: &str) -> Option<String> {
        self.sessions.read().await.values().find_map(|state| {
            state
                .execution
                .as_ref()
                .filter(|execution| execution.session.id == execution_session_id)
                .map(|execution| execution.markdown.clone())
        })
    }

    pub async fn replace_execution_todos(
        &self,
        execution_session_id: &str,
        todos: &[ExecutionTodo],
    ) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        let execution = find_execution_mut(&mut sessions, execution_session_id)?;
        execution.todos = todos.to_vec();
        Ok(())
    }

    pub async fn execution_todos(
        &self,
        execution_session_id: &str,
    ) -> anyhow::Result<Vec<ExecutionTodo>> {
        let sessions = self.sessions.read().await;
        Ok(find_execution(&sessions, execution_session_id)?
            .todos
            .clone())
    }

    pub async fn update_execution_todo(&self, todo: &ExecutionTodo) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        let execution = find_execution_mut(&mut sessions, &todo.execution_session_id)?;
        let item = execution
            .todos
            .iter_mut()
            .find(|item| item.id == todo.id)
            .ok_or_else(|| anyhow!("execution todo not found"))?;
        *item = todo.clone();
        Ok(())
    }

    /// Discard every draft and execution state for a deleted/reset session.
    pub async fn discard_session(&self, session_key: &str) {
        self.sessions.write().await.remove(session_key);
    }
}

fn find_execution<'a>(
    sessions: &'a HashMap<String, SessionPlanState>,
    id: &str,
) -> anyhow::Result<&'a ExecutionState> {
    sessions
        .values()
        .find_map(|state| {
            state
                .execution
                .as_ref()
                .filter(|execution| execution.session.id == id)
        })
        .ok_or_else(|| anyhow!("execution session is not active"))
}

fn find_execution_mut<'a>(
    sessions: &'a mut HashMap<String, SessionPlanState>,
    id: &str,
) -> anyhow::Result<&'a mut ExecutionState> {
    sessions
        .values_mut()
        .find_map(|state| {
            state
                .execution
                .as_mut()
                .filter(|execution| execution.session.id == id)
        })
        .ok_or_else(|| anyhow!("execution session is not active"))
}

#[cfg(test)]
mod tests {
    use super::*;
    const PLAN: &str = "# Plan\n\nDo the work.\n";

    #[tokio::test]
    async fn drafts_and_execution_are_isolated_by_session_and_replaced() {
        let registry = EphemeralPlanRegistry::new();
        let a = registry
            .create_report("session-isolation-a", "A", PLAN, PlanRevisionAuthor::Agent)
            .await
            .unwrap();
        registry
            .create_report("session-isolation-b", "B", PLAN, PlanRevisionAuthor::Agent)
            .await
            .unwrap();
        assert!(registry
            .active_execution_for_session("session-isolation-b")
            .await
            .is_none());
        let hash = revision_hash(PLAN);
        let (_, execution) = registry
            .approve_revision(
                "session-isolation-a",
                &a.report.id,
                1,
                &hash,
                ExecutionContextPolicy::Retain,
                None,
            )
            .await
            .unwrap();
        assert_eq!(
            registry.execution_markdown(&execution.id).await.as_deref(),
            Some(PLAN)
        );
        assert!(registry
            .create_report(
                "session-isolation-a",
                "new",
                PLAN,
                PlanRevisionAuthor::Agent,
            )
            .await
            .is_ok());
        assert!(registry.execution_markdown(&execution.id).await.is_none());
        registry.discard_session("session-isolation-a").await;
        registry.discard_session("session-isolation-b").await;
    }

    /// Regression: agent loop creates via `new()`, manager approve used
    /// `Default`/`PlanningService::new()`. They must see the same draft.
    #[tokio::test]
    async fn default_and_new_share_process_registry_for_approve() {
        let agent_side = EphemeralPlanRegistry::new();
        let manager_side = EphemeralPlanRegistry::default();
        let session_key = "gui:plan-approve-registry-share";
        agent_side.discard_session(session_key).await;
        manager_side.discard_session(session_key).await;

        let draft = agent_side
            .create_report(session_key, "Shared", PLAN, PlanRevisionAuthor::Agent)
            .await
            .unwrap();
        let hash = revision_hash(PLAN);
        let result = manager_side
            .approve_revision(
                session_key,
                &draft.report.id,
                1,
                &hash,
                ExecutionContextPolicy::Compact,
                None,
            )
            .await;
        assert!(
            result.is_ok(),
            "manager Default registry must see agent new() draft: {:?}",
            result.err().map(|e| e.to_string())
        );
        let (_, execution) = result.unwrap();
        assert_eq!(
            manager_side
                .execution_markdown(&execution.id)
                .await
                .as_deref(),
            Some(PLAN)
        );
        agent_side.discard_session(session_key).await;
    }

    #[tokio::test]
    async fn runtime_projection_keeps_awaiting_approval_fail_closed_then_enters_execute() {
        let registry = EphemeralPlanRegistry::new();
        let session_key = "gui:runtime-policy-projection";
        registry.discard_session(session_key).await;
        let draft = registry
            .create_report(session_key, "Projected", PLAN, PlanRevisionAuthor::Agent)
            .await
            .unwrap();
        let pending = registry
            .runtime_state_for_session(session_key)
            .await
            .unwrap();
        assert_eq!(pending.phase, PlanPhase::AwaitingApproval);
        assert_eq!(pending.revision, Some(1));

        registry
            .approve_revision(
                session_key,
                &draft.report.id,
                1,
                &revision_hash(PLAN),
                ExecutionContextPolicy::Compact,
                None,
            )
            .await
            .unwrap();
        let executing = registry
            .runtime_state_for_session(session_key)
            .await
            .unwrap();
        assert_eq!(executing.phase, PlanPhase::Execute);
        registry.discard_session(session_key).await;
    }
}

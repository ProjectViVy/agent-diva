//! Process-local, session-scoped PLAN runtime state.
//!
//! PLAN drafts, approvals, execution context, and execution TODOs deliberately
//! live only for the lifetime of this process.  This module has no database
//! dependency and never restores plan state from session history.

use anyhow::anyhow;
use chrono::Utc;
use std::{collections::HashMap, sync::{Arc, OnceLock}};
use tokio::sync::RwLock;

use super::{
    assert_report_ready_for_approval, revision_hash, ExecutionContextPolicy, ExecutionSession,
    ExecutionSessionStatus, ExecutionTodo, PlanId, PlanReport,
    PlanReportStatus, PlanRevision, PlanRevisionApproval, PlanRevisionAuthor,
};

/// The draft currently available to a single session during this process.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanReportDetail {
    pub report: PlanReport,
    pub revision: PlanRevision,
}

#[derive(Clone, Default)]
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

impl EphemeralPlanRegistry {
    pub fn new() -> Self {
        // Components are constructed independently (agent loop, gateway, and
        // desktop bridge), but they run in one backend process.  They share
        // this registry instance while retaining strict session-key isolation.
        static PROCESS_REGISTRY: OnceLock<Arc<RwLock<HashMap<String, SessionPlanState>>>> = OnceLock::new();
        Self { sessions: PROCESS_REGISTRY.get_or_init(|| Arc::new(RwLock::new(HashMap::new()))).clone() }
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
        assert_report_ready_for_approval(&draft.revision.markdown).map_err(|error| anyhow!(error))?;
        let now = Utc::now();
        let approval = PlanRevisionApproval {
            report_id: report_id.clone(), revision, revision_hash: actual_hash,
            context_policy, approved_at: now,
        };
        let session = ExecutionSession {
            id: uuid::Uuid::new_v4().to_string(), report_id: report_id.clone(), revision,
            context_policy, status: ExecutionSessionStatus::Executing,
            compacted_context: compacted_context.map(ToOwned::to_owned),
            created_at: now, updated_at: now,
        };
        let markdown = draft.revision.markdown.clone();
        state.draft = None;
        state.execution = Some(ExecutionState { session: session.clone(), markdown, todos: vec![] });
        Ok((approval, session))
    }

    pub async fn active_execution_for_session(&self, session_key: &str) -> Option<ExecutionSession> {
        self.sessions.read().await.get(session_key).and_then(|state| state.execution.as_ref())
            .map(|execution| execution.session.clone())
    }

    pub async fn execution_markdown(&self, execution_session_id: &str) -> Option<String> {
        self.sessions.read().await.values().find_map(|state| state.execution.as_ref()
            .filter(|execution| execution.session.id == execution_session_id)
            .map(|execution| execution.markdown.clone()))
    }

    pub async fn replace_execution_todos(&self, execution_session_id: &str, todos: &[ExecutionTodo]) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        let execution = find_execution_mut(&mut sessions, execution_session_id)?;
        execution.todos = todos.to_vec();
        Ok(())
    }

    pub async fn execution_todos(&self, execution_session_id: &str) -> anyhow::Result<Vec<ExecutionTodo>> {
        let sessions = self.sessions.read().await;
        Ok(find_execution(&sessions, execution_session_id)?.todos.clone())
    }

    pub async fn update_execution_todo(&self, todo: &ExecutionTodo) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        let execution = find_execution_mut(&mut sessions, &todo.execution_session_id)?;
        let item = execution.todos.iter_mut().find(|item| item.id == todo.id)
            .ok_or_else(|| anyhow!("execution todo not found"))?;
        *item = todo.clone();
        Ok(())
    }

    /// Discard every draft and execution state for a deleted/reset session.
    pub async fn discard_session(&self, session_key: &str) {
        self.sessions.write().await.remove(session_key);
    }
}

fn find_execution<'a>(sessions: &'a HashMap<String, SessionPlanState>, id: &str) -> anyhow::Result<&'a ExecutionState> {
    sessions.values().find_map(|state| state.execution.as_ref().filter(|execution| execution.session.id == id))
        .ok_or_else(|| anyhow!("execution session is not active"))
}

fn find_execution_mut<'a>(sessions: &'a mut HashMap<String, SessionPlanState>, id: &str) -> anyhow::Result<&'a mut ExecutionState> {
    sessions.values_mut().find_map(|state| state.execution.as_mut().filter(|execution| execution.session.id == id))
        .ok_or_else(|| anyhow!("execution session is not active"))
}

#[cfg(test)]
mod tests {
    use super::*;
    const PLAN: &str = "# Plan\n\nDo the work.\n";

    #[tokio::test]
    async fn drafts_and_execution_are_isolated_by_session_and_replaced() {
        let registry = EphemeralPlanRegistry::new();
        let a = registry.create_report("a", "A", PLAN, PlanRevisionAuthor::Agent).await.unwrap();
        registry.create_report("b", "B", PLAN, PlanRevisionAuthor::Agent).await.unwrap();
        assert!(registry.active_execution_for_session("b").await.is_none());
        let hash = revision_hash(PLAN);
        let (_, execution) = registry.approve_revision("a", &a.report.id, 1, &hash, ExecutionContextPolicy::Retain, None).await.unwrap();
        assert_eq!(registry.execution_markdown(&execution.id).await.as_deref(), Some(PLAN));
        assert!(registry.create_report("a", "new", PLAN, PlanRevisionAuthor::Agent).await.is_ok());
        assert!(registry.execution_markdown(&execution.id).await.is_none());
    }
}

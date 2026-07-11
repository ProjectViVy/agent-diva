//! Session-scoped, process-local PLAN service for the manager API.

use agent_diva_core::planning::{
    EphemeralPlanRegistry, ExecutionContextPolicy, ExecutionSession, ExecutionTodo,
    ExecutionTodoPriority, ExecutionTodoStatus, PlanId, PlanReportDetail, PlanRevisionAuthor,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePlanReportRequest { pub session_key: String, pub title: String, pub markdown: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendPlanReportRevisionRequest { pub session_key: String, pub expected_revision: i64, pub title: String, pub markdown: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovePlanReportRequest {
    pub session_key: String, pub revision: i64, pub revision_hash: String,
    pub context_policy: ExecutionContextPolicy, pub compacted_context: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateExecutionTodoRequest {
    pub status: Option<ExecutionTodoStatus>, pub title: Option<String>, pub detail: Option<String>,
    pub priority: Option<ExecutionTodoPriority>, pub evidence_ref: Option<String>, pub block_reason: Option<String>,
}

#[derive(Clone)]
pub struct PlanningService {
    registry: Arc<EphemeralPlanRegistry>,
}

impl Default for PlanningService {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanningService {
    /// Construct a service handle on the process-wide plan registry.
    ///
    /// Must use [`EphemeralPlanRegistry::new`] (not a private empty map) so
    /// gateway approve/execution sees drafts created by the agent loop.
    pub fn new() -> Self {
        Self {
            registry: Arc::new(EphemeralPlanRegistry::new()),
        }
    }
    pub fn registry(&self) -> Arc<EphemeralPlanRegistry> { self.registry.clone() }
    pub async fn create_report(&self, session_key: &str, title: &str, markdown: &str, author: PlanRevisionAuthor) -> anyhow::Result<PlanReportDetail> {
        self.registry.create_report(session_key, title, markdown, author).await
    }
    pub async fn append_report_revision(&self, request: &AppendPlanReportRevisionRequest, report_id: &str) -> anyhow::Result<PlanReportDetail> {
        self.registry.append_revision(&request.session_key, &PlanId(report_id.to_string()), request.expected_revision, &request.title, &request.markdown, PlanRevisionAuthor::User).await
    }
    pub async fn approve_report_revision(&self, request: &ApprovePlanReportRequest, report_id: &str) -> anyhow::Result<ExecutionSession> {
        self.registry.approve_revision(&request.session_key, &PlanId(report_id.to_string()), request.revision, &request.revision_hash, request.context_policy, request.compacted_context.as_deref()).await.map(|(_, session)| session)
    }
    pub async fn active_execution(&self, session_key: &str) -> Option<ExecutionSession> { self.registry.active_execution_for_session(session_key).await }
    pub async fn execution_todos(&self, execution_id: &str) -> anyhow::Result<Vec<ExecutionTodo>> { self.registry.execution_todos(execution_id).await }
    pub async fn update_execution_todo(&self, execution_id: &str, todo_id: &str, request: UpdateExecutionTodoRequest) -> anyhow::Result<ExecutionTodo> {
        let mut todos = self.registry.execution_todos(execution_id).await?;
        let todo = todos.iter_mut().find(|todo| todo.id == todo_id).ok_or_else(|| anyhow::anyhow!("execution todo not found: {todo_id}"))?;
        if let Some(value) = request.status { todo.status = value; }
        if let Some(value) = request.title { if value.trim().is_empty() { return Err(anyhow::anyhow!("execution todo title cannot be empty")); } todo.title = value; }
        if let Some(value) = request.detail { todo.detail = non_empty(value); }
        if let Some(value) = request.priority { todo.priority = value; }
        if let Some(value) = request.evidence_ref { todo.evidence_ref = non_empty(value); }
        if let Some(value) = request.block_reason { todo.block_reason = non_empty(value); }
        if todo.status == ExecutionTodoStatus::Blocked && todo.block_reason.is_none() { return Err(anyhow::anyhow!("blocked execution todo requires block_reason")); }
        todo.updated_at = Utc::now();
        let updated = todo.clone(); self.registry.update_execution_todo(&updated).await?; Ok(updated)
    }
    pub async fn discard_session(&self, session_key: &str) { self.registry.discard_session(session_key).await; }
}
fn non_empty(value: String) -> Option<String> { let value = value.trim(); (!value.is_empty()).then(|| value.to_string()) }

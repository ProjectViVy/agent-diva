//! Planning service for the manager HTTP API.
//!
//! Provides CRUD operations for plans via the [`PlanningStore`] trait,
//! along with DTO types suitable for JSON serialization.

use agent_diva_core::planning::ids::PlanId;
use agent_diva_core::planning::model::{Plan, PlanPhase, PlanStatus, TodoStatus};
use agent_diva_core::planning::store::{PlanningStore, SqlitePlanningStore};
use agent_diva_core::planning::{
    ExecutionContextPolicy, PlanReportDetail, PlanRevisionAuthor, SqlitePlanReportStore,
};
use anyhow::Context;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

/// Lightweight plan summary for list views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSummary {
    pub id: String,
    pub title: String,
    pub goal: String,
    pub phase: String,
    pub status: String,
    pub todo_count: usize,
    pub todo_completed: usize,
    pub is_active: bool,
}

/// Full plan detail including steps and todos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanDetail {
    pub id: String,
    pub revision: Option<i64>,
    pub title: String,
    pub goal: String,
    pub phase: String,
    pub status: String,
    pub strategy: Option<String>,
    pub assumptions: Vec<String>,
    pub risks: Vec<String>,
    pub open_questions: Vec<String>,
    pub verification_verdict: Option<String>,
    pub steps: Vec<StepDetail>,
    pub todos: Vec<TodoDetail>,
    pub created_at: String,
    pub updated_at: String,
}

/// Detail view of a single plan step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepDetail {
    pub id: String,
    pub plan_id: String,
    pub ordinal: i32,
    pub title: String,
    pub rationale: Option<String>,
    pub expected_output: Option<String>,
    pub status: String,
    pub evidence_ref: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Detail view of a single todo item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoDetail {
    pub id: String,
    pub plan_step_id: Option<String>,
    pub title: String,
    pub detail: Option<String>,
    pub status: String,
    pub priority: String,
    pub evidence_ref: Option<String>,
    pub block_reason: Option<String>,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Request DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePlanRequest {
    pub title: String,
    pub goal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePlanRequest {
    pub title: Option<String>,
    pub goal: Option<String>,
    pub strategy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePlanReportRequest {
    pub session_key: String,
    pub title: String,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendPlanReportRevisionRequest {
    pub expected_revision: i64,
    pub title: String,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovePlanReportRequest {
    pub revision: i64,
    pub revision_hash: String,
    pub context_policy: ExecutionContextPolicy,
    pub compacted_context: Option<String>,
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/// Planning service that wraps the planning store for use by the manager.
#[derive(Clone)]
pub struct PlanningService {
    store: Arc<SqlitePlanningStore>,
    report_store: Arc<SqlitePlanReportStore>,
}

impl PlanningService {
    pub async fn new(store: Arc<SqlitePlanningStore>) -> anyhow::Result<Self> {
        let report_store = SqlitePlanReportStore::new(store.pool().clone())
            .await
            .context("failed to create plan report store")?;
        Ok(Self {
            store,
            report_store: Arc::new(report_store),
        })
    }

    /// Create a new PlanningService from a raw SqlitePool, auto-migrating the schema.
    pub async fn new_from_pool(pool: SqlitePool) -> anyhow::Result<Self> {
        let store = SqlitePlanningStore::new(pool.clone())
            .await
            .context("failed to create planning store")?;
        let report_store = SqlitePlanReportStore::new(pool)
            .await
            .context("failed to create plan report store")?;
        Ok(Self {
            store: Arc::new(store),
            report_store: Arc::new(report_store),
        })
    }

    pub async fn create_report(
        &self,
        session_key: &str,
        title: &str,
        markdown: &str,
        author: PlanRevisionAuthor,
    ) -> anyhow::Result<PlanReportDetail> {
        self.report_store
            .create_report(session_key, title, markdown, author)
            .await
    }

    pub async fn append_report_revision(
        &self,
        report_id: &str,
        expected_revision: i64,
        title: &str,
        markdown: &str,
        author: PlanRevisionAuthor,
    ) -> anyhow::Result<PlanReportDetail> {
        self.report_store
            .append_revision(
                &PlanId(report_id.to_string()),
                expected_revision,
                title,
                markdown,
                author,
            )
            .await
    }

    pub async fn approve_report_revision(
        &self,
        report_id: &str,
        revision: i64,
        revision_hash: &str,
        context_policy: ExecutionContextPolicy,
        compacted_context: Option<&str>,
    ) -> anyhow::Result<agent_diva_core::planning::ExecutionSession> {
        self.report_store
            .approve_revision(
                &PlanId(report_id.to_string()),
                revision,
                revision_hash,
                context_policy,
                compacted_context,
            )
            .await
            .map(|(_, session)| session)
    }

    pub async fn list_reports(&self) -> anyhow::Result<Vec<PlanReportDetail>> {
        self.report_store.list_reports().await
    }

    pub async fn report_detail(
        &self,
        report_id: &str,
        revision: i64,
    ) -> anyhow::Result<PlanReportDetail> {
        self.report_store
            .get_detail(&PlanId(report_id.to_string()), revision)
            .await
    }

    /// List all plans as lightweight summaries.
    pub async fn list_plans(&self) -> anyhow::Result<Vec<PlanSummary>> {
        self.store
            .delete_expired_plans()
            .await
            .context("failed to clean up expired plans before listing")?;

        let plans = self
            .store
            .list_plans()
            .await
            .context("failed to list plans")?;

        let active_plan_id = self.store.get_active_plan().await.ok().map(|id| id.0);

        let mut summaries = Vec::with_capacity(plans.len());
        for plan in plans {
            let todos = self
                .store
                .get_todos(&plan.id)
                .await
                .context("failed to get todos for plan")?;

            let todo_count = todos.items.len();
            let todo_completed = todos
                .items
                .iter()
                .filter(|t| t.status == TodoStatus::Completed)
                .count();

            summaries.push(PlanSummary {
                id: plan.id.0.clone(),
                title: plan.title,
                goal: plan.goal,
                phase: plan.phase.to_string(),
                status: plan.status.to_string(),
                todo_count,
                todo_completed,
                is_active: active_plan_id.as_deref() == Some(&plan.id.0),
            });
        }

        Ok(summaries)
    }

    /// Get full plan detail by ID.
    pub async fn get_plan(&self, id: &str) -> anyhow::Result<Option<PlanDetail>> {
        let plan_id = PlanId(id.to_string());
        let plan = match self.store.get_plan(&plan_id).await {
            Ok(plan) => plan,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("not found") || msg.contains("Not found") {
                    return Ok(None);
                }
                return Err(e).context("failed to get plan");
            }
        };

        let steps = self
            .store
            .get_steps(&plan.id)
            .await
            .context("failed to get steps")?;

        let todos = self
            .store
            .get_todos(&plan.id)
            .await
            .context("failed to get todos")?;
        let revision = self
            .store
            .get_plan_revision(&plan.id)
            .await
            .context("failed to get plan revision")?;

        Ok(Some(self.to_plan_detail(
            &plan,
            revision,
            &steps,
            &todos.items,
        )))
    }

    /// Create a new plan.
    pub async fn create_plan(&self, title: &str, goal: &str) -> anyhow::Result<Plan> {
        let now = Utc::now();
        let plan = Plan {
            id: PlanId::new(),
            title: title.to_string(),
            goal: goal.to_string(),
            phase: PlanPhase::Explore,
            status: PlanStatus::Pending,
            strategy: None,
            assumptions: vec![],
            risks: vec![],
            open_questions: vec![],
            verification_verdict: None,
            created_at: now,
            updated_at: now,
        };

        self.store
            .create_plan(&plan)
            .await
            .context("failed to create plan")?;

        Ok(plan)
    }

    /// Update an existing plan's mutable fields.
    pub async fn update_plan(
        &self,
        id: &str,
        title: Option<&str>,
        goal: Option<&str>,
        strategy: Option<&str>,
    ) -> anyhow::Result<Plan> {
        let plan_id = PlanId(id.to_string());
        let mut plan = self
            .store
            .get_plan(&plan_id)
            .await
            .context("failed to get plan for update")?;

        if let Some(t) = title {
            plan.title = t.to_string();
        }
        if let Some(g) = goal {
            plan.goal = g.to_string();
        }
        if let Some(s) = strategy {
            plan.strategy = Some(s.to_string());
        }
        plan.updated_at = Utc::now();

        self.store
            .update_plan(&plan)
            .await
            .context("failed to update plan")?;

        Ok(plan)
    }

    /// Delete a plan by ID.
    pub async fn delete_plan(&self, id: &str) -> anyhow::Result<()> {
        let plan_id = PlanId(id.to_string());
        self.store
            .delete_plan(&plan_id)
            .await
            .context("failed to delete plan")?;
        self.store
            .clear_active_plan(&plan_id)
            .await
            .context("failed to clear deleted active plan")?;
        Ok(())
    }

    pub async fn delete_todo(&self, plan_id: &str, todo_id: &str) -> anyhow::Result<()> {
        self.set_todo_status(plan_id, todo_id, TodoStatus::Canceled)
            .await
    }

    pub async fn restore_todo(&self, plan_id: &str, todo_id: &str) -> anyhow::Result<()> {
        self.set_todo_status(plan_id, todo_id, TodoStatus::Pending)
            .await
    }

    async fn set_todo_status(
        &self,
        plan_id: &str,
        todo_id: &str,
        status: TodoStatus,
    ) -> anyhow::Result<()> {
        let plan_id = PlanId(plan_id.to_string());
        let mut todos = self.store.get_todos(&plan_id).await?.items;
        let todo = todos
            .iter_mut()
            .find(|todo| todo.id.0 == todo_id)
            .ok_or_else(|| anyhow::anyhow!("todo not found: {todo_id}"))?;
        todo.status = status;
        todo.updated_at = Utc::now();
        self.store.update_todo(&plan_id, todo).await?;
        Ok(())
    }

    // -- helpers --

    fn to_plan_detail(
        &self,
        plan: &Plan,
        revision: Option<i64>,
        steps: &[agent_diva_core::planning::model::PlanStep],
        todos: &[agent_diva_core::planning::model::TodoItem],
    ) -> PlanDetail {
        PlanDetail {
            id: plan.id.0.clone(),
            revision,
            title: plan.title.clone(),
            goal: plan.goal.clone(),
            phase: plan.phase.to_string(),
            status: plan.status.to_string(),
            strategy: plan.strategy.clone(),
            assumptions: plan.assumptions.clone(),
            risks: plan.risks.clone(),
            open_questions: plan.open_questions.clone(),
            verification_verdict: plan.verification_verdict.as_ref().map(|v| v.to_string()),
            steps: steps
                .iter()
                .map(|s| StepDetail {
                    id: s.id.clone(),
                    plan_id: s.plan_id.0.clone(),
                    ordinal: s.ordinal,
                    title: s.title.clone(),
                    rationale: s.rationale.clone(),
                    expected_output: s.expected_output.clone(),
                    status: s.status.to_string(),
                    evidence_ref: s.evidence_ref.clone(),
                    created_at: s.created_at.to_rfc3339(),
                    updated_at: s.updated_at.to_rfc3339(),
                })
                .collect(),
            todos: todos
                .iter()
                .map(|t| TodoDetail {
                    id: t.id.0.clone(),
                    plan_step_id: t.plan_step_id.clone(),
                    title: t.title.clone(),
                    detail: t.detail.clone(),
                    status: t.status.to_string(),
                    priority: t.priority.to_string(),
                    evidence_ref: t.evidence_ref.clone(),
                    block_reason: t.block_reason.clone(),
                    updated_at: t.updated_at.to_rfc3339(),
                })
                .collect(),
            created_at: plan.created_at.to_rfc3339(),
            updated_at: plan.updated_at.to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::planning::store::SqlitePlanningStore;
    use sqlx::SqlitePool;

    async fn setup_service() -> PlanningService {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("failed to open in-memory sqlite");
        let store = SqlitePlanningStore::new(pool)
            .await
            .expect("failed to create planning store");
        PlanningService::new(Arc::new(store))
            .await
            .expect("failed to create planning service")
    }

    #[tokio::test]
    async fn create_and_list_plans() {
        let svc = setup_service().await;
        let plan = svc.create_plan("Test", "Goal").await.unwrap();
        assert_eq!(plan.title, "Test");

        let plans = svc.list_plans().await.unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].title, "Test");
        assert_eq!(plans[0].phase, "Explore");
    }

    #[tokio::test]
    async fn get_plan_returns_detail() {
        let svc = setup_service().await;
        let plan = svc.create_plan("Detail Test", "Goal").await.unwrap();

        let detail = svc.get_plan(&plan.id.0).await.unwrap();
        assert!(detail.is_some());
        let detail = detail.unwrap();
        assert_eq!(detail.title, "Detail Test");
        assert!(detail.steps.is_empty());
        assert!(detail.todos.is_empty());
    }

    #[tokio::test]
    async fn get_plan_returns_none_for_missing() {
        let svc = setup_service().await;
        let result = svc.get_plan("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn update_plan_modifies_fields() {
        let svc = setup_service().await;
        let plan = svc.create_plan("Old Title", "Old Goal").await.unwrap();

        let updated = svc
            .update_plan(&plan.id.0, Some("New Title"), None, Some("strategy"))
            .await
            .unwrap();

        assert_eq!(updated.title, "New Title");
        assert_eq!(updated.goal, "Old Goal");
        assert_eq!(updated.strategy, Some("strategy".to_string()));
    }

    #[tokio::test]
    async fn delete_plan_removes_it() {
        let svc = setup_service().await;
        let plan = svc.create_plan("To Delete", "Goal").await.unwrap();

        svc.delete_plan(&plan.id.0).await.unwrap();

        let result = svc.get_plan(&plan.id.0).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn list_plans_cleans_up_expired_plans() {
        let svc = setup_service().await;
        let mut expired = svc.create_plan("Expired", "Old goal").await.unwrap();
        expired.updated_at = Utc::now() - chrono::Duration::days(31);
        svc.store.update_plan(&expired).await.unwrap();
        svc.create_plan("Recent", "Keep goal").await.unwrap();

        let plans = svc.list_plans().await.unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].title, "Recent");
        assert!(svc.get_plan(&expired.id.0).await.unwrap().is_none());
    }
}

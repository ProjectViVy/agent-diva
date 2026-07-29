//! Session-scoped, process-local PLAN service for the manager API.

use agent_diva_core::planning::store::{PlanningStore, SqlitePlanningStore};
use agent_diva_core::planning::{
    validate_report_markdown, ApprovalRequest, EphemeralPlanRegistry, ExecutionContextPolicy,
    ExecutionSession, ExecutionTodo, ExecutionTodoPriority, ExecutionTodoStatus, Plan, PlanId,
    PlanPhase, PlanReport, PlanReportDetail, PlanReportStatus, PlanRevision, PlanRevisionAuthor,
    PlanStatus, PlanStep, PlanSubmission, TodoPolicy,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::OnceCell;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePlanReportRequest {
    pub session_key: String,
    pub title: String,
    pub markdown: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendPlanReportRevisionRequest {
    pub session_key: String,
    pub expected_revision: i64,
    pub title: String,
    pub markdown: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovePlanReportRequest {
    pub session_key: String,
    pub revision: i64,
    pub revision_hash: String,
    pub context_policy: ExecutionContextPolicy,
    pub compacted_context: Option<String>,
    #[serde(default = "default_todo_policy")]
    pub todo_policy: TodoPolicy,
    #[serde(default)]
    pub materialize_todos: bool,
}

fn default_todo_policy() -> TodoPolicy {
    TodoPolicy::Optional
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateExecutionTodoRequest {
    pub status: Option<ExecutionTodoStatus>,
    pub title: Option<String>,
    pub detail: Option<String>,
    pub priority: Option<ExecutionTodoPriority>,
    pub evidence_ref: Option<String>,
    pub block_reason: Option<String>,
}

#[derive(Clone)]
pub struct PlanningService {
    registry: Arc<EphemeralPlanRegistry>,
    canonical_store: Arc<OnceCell<Arc<SqlitePlanningStore>>>,
    workspace: PathBuf,
}

impl Default for PlanningService {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

impl PlanningService {
    /// Construct a service handle on the process-wide plan registry.
    ///
    /// Must use [`EphemeralPlanRegistry::new`] (not a private empty map) so
    /// gateway approve/execution sees drafts created by the agent loop.
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            registry: Arc::new(EphemeralPlanRegistry::new()),
            canonical_store: Arc::new(OnceCell::new()),
            workspace,
        }
    }
    pub fn registry(&self) -> Arc<EphemeralPlanRegistry> {
        self.registry.clone()
    }
    pub async fn list_reports(&self) -> anyhow::Result<Vec<PlanReportDetail>> {
        let store = self.canonical_store().await?;
        let plan_id = match store.get_active_plan().await {
            Ok(plan_id) => plan_id,
            Err(_) => return Ok(Vec::new()),
        };
        let plan = store.get_plan(&plan_id).await?;
        if plan.phase != PlanPhase::AwaitingApproval {
            return Ok(Vec::new());
        }
        let Some(revision) = store.get_plan_revision(&plan_id).await? else {
            return Ok(Vec::new());
        };
        Ok(vec![plan_report_projection(
            &plan,
            revision,
            "restored:active",
        )?])
    }
    pub async fn create_report(
        &self,
        session_key: &str,
        title: &str,
        markdown: &str,
        author: PlanRevisionAuthor,
    ) -> anyhow::Result<PlanReportDetail> {
        validate_report_markdown(markdown)?;
        ParsedPlanMarkdown::parse(markdown)?;
        let detail = self
            .registry
            .create_report(session_key, title, markdown, author)
            .await?;
        self.sync_report_to_canonical_store(&detail).await?;
        Ok(detail)
    }
    pub async fn append_report_revision(
        &self,
        request: &AppendPlanReportRevisionRequest,
        report_id: &str,
    ) -> anyhow::Result<PlanReportDetail> {
        validate_report_markdown(&request.markdown)?;
        ParsedPlanMarkdown::parse(&request.markdown)?;
        let detail = self
            .registry
            .append_revision(
                &request.session_key,
                &PlanId(report_id.to_string()),
                request.expected_revision,
                &request.title,
                &request.markdown,
                PlanRevisionAuthor::User,
            )
            .await?;
        self.sync_report_to_canonical_store(&detail).await?;
        Ok(detail)
    }
    pub async fn approve_report_revision(
        &self,
        request: &ApprovePlanReportRequest,
        report_id: &str,
    ) -> anyhow::Result<ExecutionSession> {
        let plan_id = PlanId(report_id.to_string());
        let store = self.canonical_store().await?;
        if self
            .registry
            .runtime_state_for_session(&request.session_key)
            .await
            .is_none()
        {
            let plan = store.get_plan(&plan_id).await?;
            let revision = store
                .get_plan_revision(&plan_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("plan is not awaiting approval"))?;
            self.registry
                .restore_report(
                    &request.session_key,
                    plan_report_projection(&plan, revision, &request.session_key)?,
                )
                .await?;
        }
        let receipt = store
            .approve_plan(
                &plan_id,
                &ApprovalRequest {
                    expected_revision: request.revision,
                    approved_by: "desktop-ui".to_string(),
                    todo_policy: request.todo_policy,
                    materialize_todos: request.materialize_todos,
                },
            )
            .await?;
        let (_, session) = self
            .registry
            .approve_revision(
                &request.session_key,
                &PlanId(report_id.to_string()),
                request.revision,
                &request.revision_hash,
                request.context_policy,
                request.compacted_context.as_deref(),
            )
            .await?;
        if receipt.revision != session.revision {
            return Err(anyhow::anyhow!(
                "canonical approval revision does not match execution session"
            ));
        }
        if request.todo_policy.materializes(request.materialize_todos) {
            let markdown = self
                .registry
                .execution_markdown(&session.id)
                .await
                .ok_or_else(|| anyhow::anyhow!("approved execution markdown is unavailable"))?;
            let steps = execution_steps_from_markdown(&markdown);
            if steps.is_empty() {
                return Err(anyhow::anyhow!(
                    "approved plan has no materializable execution steps"
                ));
            }
            let now = Utc::now();
            let todos = steps
                .into_iter()
                .enumerate()
                .map(|(index, title)| ExecutionTodo {
                    id: format!("{}-todo-{}", session.id, index + 1),
                    execution_session_id: session.id.clone(),
                    title,
                    detail: None,
                    status: ExecutionTodoStatus::Pending,
                    priority: ExecutionTodoPriority::Normal,
                    evidence_ref: None,
                    block_reason: None,
                    updated_at: now,
                })
                .collect::<Vec<_>>();
            self.registry
                .replace_execution_todos(&session.id, &todos)
                .await?;
        }
        Ok(session)
    }
    pub async fn active_execution(&self, session_key: &str) -> Option<ExecutionSession> {
        self.registry
            .active_execution_for_session(session_key)
            .await
    }
    pub async fn execution_todos(&self, execution_id: &str) -> anyhow::Result<Vec<ExecutionTodo>> {
        self.registry.execution_todos(execution_id).await
    }
    pub async fn update_execution_todo(
        &self,
        execution_id: &str,
        todo_id: &str,
        request: UpdateExecutionTodoRequest,
    ) -> anyhow::Result<ExecutionTodo> {
        let mut todos = self.registry.execution_todos(execution_id).await?;
        let todo = todos
            .iter_mut()
            .find(|todo| todo.id == todo_id)
            .ok_or_else(|| anyhow::anyhow!("execution todo not found: {todo_id}"))?;
        if let Some(value) = request.status {
            todo.status = value;
        }
        if let Some(value) = request.title {
            if value.trim().is_empty() {
                return Err(anyhow::anyhow!("execution todo title cannot be empty"));
            }
            todo.title = value;
        }
        if let Some(value) = request.detail {
            todo.detail = non_empty(value);
        }
        if let Some(value) = request.priority {
            todo.priority = value;
        }
        if let Some(value) = request.evidence_ref {
            todo.evidence_ref = non_empty(value);
        }
        if let Some(value) = request.block_reason {
            todo.block_reason = non_empty(value);
        }
        if todo.status == ExecutionTodoStatus::Blocked && todo.block_reason.is_none() {
            return Err(anyhow::anyhow!(
                "blocked execution todo requires block_reason"
            ));
        }
        todo.updated_at = Utc::now();
        let updated = todo.clone();
        self.registry.update_execution_todo(&updated).await?;
        Ok(updated)
    }
    pub async fn discard_session(&self, session_key: &str) {
        self.registry.discard_session(session_key).await;
    }

    async fn canonical_store(&self) -> anyhow::Result<Arc<SqlitePlanningStore>> {
        self.canonical_store
            .get_or_try_init(|| async {
                let data_dir = self.workspace.join(".agent-diva").join("data");
                std::fs::create_dir_all(&data_dir)?;
                let options = SqliteConnectOptions::new()
                    .filename(data_dir.join("planning.sqlite3"))
                    .create_if_missing(true);
                let pool = SqlitePoolOptions::new().connect_with(options).await?;
                Ok::<_, anyhow::Error>(Arc::new(SqlitePlanningStore::new(pool).await?))
            })
            .await
            .cloned()
    }

    async fn sync_report_to_canonical_store(
        &self,
        detail: &PlanReportDetail,
    ) -> anyhow::Result<()> {
        validate_report_markdown(&detail.revision.markdown)?;
        let parsed = ParsedPlanMarkdown::parse(&detail.revision.markdown)?;
        let store = self.canonical_store().await?;
        let plan_id = detail.report.id.clone();
        let now = Utc::now();
        let plan = Plan {
            id: plan_id.clone(),
            title: detail.revision.title.clone(),
            goal: parsed.goal,
            phase: PlanPhase::Plan,
            status: PlanStatus::Pending,
            strategy: Some(detail.revision.markdown.clone()),
            assumptions: parsed.assumptions,
            risks: parsed.risks,
            open_questions: Vec::new(),
            verification_verdict: None,
            created_at: detail.report.created_at,
            updated_at: now,
        };
        let existing = store.get_plan(&plan_id).await.ok();
        if existing.is_some() {
            store.update_plan(&plan).await?;
        } else {
            store.create_plan(&plan).await?;
        }
        let steps = parsed
            .steps
            .into_iter()
            .enumerate()
            .map(|(index, title)| PlanStep {
                id: format!("{}-step-{}", plan_id.0, index + 1),
                plan_id: plan_id.clone(),
                ordinal: (index + 1) as i32,
                title,
                rationale: None,
                expected_output: None,
                status: PlanStatus::Pending,
                evidence_ref: None,
                created_at: now,
                updated_at: now,
            })
            .collect::<Vec<_>>();
        store.replace_steps(&plan_id, &steps).await?;
        let revision = store
            .submit_plan(
                &plan_id,
                &PlanSubmission {
                    scope: parsed.scope,
                    verification_method: parsed.verification,
                    open_question_handling: "No unresolved questions.".to_string(),
                },
            )
            .await?;
        if revision != detail.revision.revision {
            return Err(anyhow::anyhow!(
                "canonical revision {revision} does not match report revision {}",
                detail.revision.revision
            ));
        }
        store.set_active_plan(&plan_id).await?;
        Ok(())
    }
}
fn non_empty(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn execution_steps_from_markdown(markdown: &str) -> Vec<String> {
    let mut in_steps = false;
    let mut steps = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            in_steps = matches!(
                trimmed.trim_start_matches("## ").trim(),
                "计划步骤" | "步骤" | "实现步骤"
            );
            continue;
        }
        if !in_steps || trimmed.is_empty() {
            continue;
        }
        let candidate = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| {
                let (prefix, rest) = trimmed.split_once(". ")?;
                prefix.chars().all(|ch| ch.is_ascii_digit()).then_some(rest)
            });
        if let Some(title) = candidate.map(str::trim).filter(|value| !value.is_empty()) {
            steps.push(title.to_string());
        }
    }
    steps
}

struct ParsedPlanMarkdown {
    goal: String,
    scope: String,
    steps: Vec<String>,
    assumptions: Vec<String>,
    risks: Vec<String>,
    verification: String,
}

impl ParsedPlanMarkdown {
    fn parse(markdown: &str) -> anyhow::Result<Self> {
        let goal = section_body(markdown, "目标");
        let scope = section_body(markdown, "范围");
        let risk_text = section_body(markdown, "风险与假设");
        let verification = section_body(markdown, "验证方法");
        let steps = execution_steps_from_markdown(markdown);
        if [
            goal.as_str(),
            scope.as_str(),
            risk_text.as_str(),
            verification.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
            || steps.is_empty()
        {
            return Err(anyhow::anyhow!(
                "plan report sections must contain actionable content"
            ));
        }
        Ok(Self {
            goal,
            scope,
            steps,
            assumptions: vec![risk_text.clone()],
            risks: vec![risk_text],
            verification,
        })
    }
}

fn section_body(markdown: &str, section: &str) -> String {
    let heading = format!("## {section}");
    let mut active = false;
    let mut body = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            if active {
                break;
            }
            active = trimmed == heading;
            continue;
        }
        if active && !trimmed.is_empty() {
            body.push(trimmed);
        }
    }
    body.join("\n")
}

fn plan_report_projection(
    plan: &Plan,
    revision: i64,
    session_key: &str,
) -> anyhow::Result<PlanReportDetail> {
    let markdown = plan
        .strategy
        .clone()
        .ok_or_else(|| anyhow::anyhow!("persisted plan has no markdown strategy"))?;
    Ok(PlanReportDetail {
        report: PlanReport {
            id: plan.id.clone(),
            session_key: session_key.to_string(),
            current_revision: revision,
            status: PlanReportStatus::AwaitingApproval,
            created_at: plan.created_at,
            updated_at: plan.updated_at,
        },
        revision: PlanRevision {
            report_id: plan.id.clone(),
            revision,
            title: plan.title.clone(),
            markdown,
            author: PlanRevisionAuthor::Agent,
            created_at: plan.updated_at,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::planning::{revision_hash, PlanPhase};

    #[test]
    fn extracts_materializable_plan_steps_only() {
        let markdown = "# Plan\n\n## 计划步骤\n1. First\n- Second\n\n## 验证方法\nRun tests";
        assert_eq!(
            execution_steps_from_markdown(markdown),
            vec!["First".to_string(), "Second".to_string()]
        );
    }

    #[tokio::test]
    async fn report_projection_uses_canonical_store_for_revision_approval() {
        let temp = tempfile::tempdir().unwrap();
        let service = PlanningService::new(temp.path().to_path_buf());
        let markdown = [
            "# Canonical plan",
            "",
            "## 目标",
            "Ship the change.",
            "",
            "## 范围",
            "Planning runtime.",
            "",
            "## 计划步骤",
            "1. Implement the gate.",
            "2. Verify the flow.",
            "",
            "## 风险与假设",
            "Assume local storage is writable.",
            "",
            "## 验证方法",
            "Run focused tests.",
        ]
        .join("\n");
        let detail = service
            .create_report(
                "gui:canonical",
                "Canonical plan",
                &markdown,
                PlanRevisionAuthor::Agent,
            )
            .await
            .unwrap();
        let store = service.canonical_store().await.unwrap();
        assert_eq!(
            store.get_plan(&detail.report.id).await.unwrap().phase,
            PlanPhase::AwaitingApproval
        );

        service.registry.discard_session("gui:canonical").await;
        let restarted = PlanningService::new(temp.path().to_path_buf());
        let restored = restarted.list_reports().await.unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].report.id, detail.report.id);

        let session = restarted
            .approve_report_revision(
                &ApprovePlanReportRequest {
                    session_key: "gui:canonical".to_string(),
                    revision: 1,
                    revision_hash: revision_hash(&markdown),
                    context_policy: ExecutionContextPolicy::Compact,
                    compacted_context: Some("approved summary".to_string()),
                    todo_policy: TodoPolicy::Always,
                    materialize_todos: true,
                },
                &detail.report.id.0,
            )
            .await
            .unwrap();
        assert_eq!(
            store.get_plan(&detail.report.id).await.unwrap().phase,
            PlanPhase::Execute
        );
        assert_eq!(
            restarted.execution_todos(&session.id).await.unwrap().len(),
            2
        );
    }
}

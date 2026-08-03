//! Durable governed PLAN service with a process-local runtime projection.

use agent_diva_core::governance::{
    ApprovalCoordinator, ApprovalGrant, ApprovalLedgerError, ApprovalReceipt as GovernanceReceipt,
    ApprovalRequest as GovernanceRequest, ApprovalState, ApprovalStatus, AuditCorrelation,
    AutonomyLevel, Capability, ContentDigest, Decision, DigestAlgorithm, GovernanceSubject,
    GovernanceSubjectKind, PolicyContext, ResourceKind, ResourceScope, RiskClass,
    SqliteGovernanceLedger,
};
use agent_diva_core::planning::store::{PlanningStore, SqlitePlanningStore};
use agent_diva_core::planning::{
    validate_report_markdown, ApprovalRequest, EphemeralPlanRegistry, ExecutionContextPolicy,
    ExecutionSession, ExecutionTodo, ExecutionTodoPriority, ExecutionTodoStatus,
    PersistedExecutionContext, Plan, PlanId, PlanPhase, PlanReport, PlanReportDetail,
    PlanReportStatus, PlanRevision, PlanRevisionAuthor, PlanStatus, PlanStep, PlanSubmission,
    TodoPolicy,
};
use chrono::{Duration, Utc};
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
    governance: Option<ApprovalCoordinator>,
    fallback_governance: Arc<OnceCell<ApprovalCoordinator>>,
    workspace_id: String,
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
        let workspace_id = agent_diva_core::workspace_identity::canonical_workspace_id(&workspace);
        Self {
            registry: Arc::new(EphemeralPlanRegistry::new()),
            canonical_store: Arc::new(OnceCell::new()),
            workspace,
            governance: None,
            fallback_governance: Arc::new(OnceCell::new()),
            workspace_id,
        }
    }

    /// Construct production planning over the process-wide governance coordinator.
    pub fn governed(
        workspace: PathBuf,
        governance: ApprovalCoordinator,
        workspace_id: String,
    ) -> Self {
        Self {
            registry: Arc::new(EphemeralPlanRegistry::new()),
            canonical_store: Arc::new(OnceCell::new()),
            workspace,
            governance: Some(governance),
            fallback_governance: Arc::new(OnceCell::new()),
            workspace_id,
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
        let session_key = self
            .plan_session_key(&plan_id, revision)
            .await?
            .unwrap_or_else(|| "restored:active".to_string());
        let detail = plan_report_projection(&plan, revision, &session_key)?;
        self.ensure_pending_governance(&detail, Utc::now()).await?;
        Ok(vec![detail])
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
        self.ensure_pending_governance(&detail, Utc::now()).await?;
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
        self.ensure_pending_governance(&detail, Utc::now()).await?;
        Ok(detail)
    }
    pub async fn approve_report_revision(
        &self,
        request: &ApprovePlanReportRequest,
        report_id: &str,
    ) -> anyhow::Result<ExecutionSession> {
        let plan_id = PlanId(report_id.to_string());
        let store = self.canonical_store().await?;
        self.initialize_governance_mapping(&store).await?;
        self.recover_incomplete().await?;
        let existing_context = store
            .get_execution_context(&plan_id, request.revision)
            .await?;
        if let Some(context) = existing_context.as_ref() {
            if context.initialization_status
                == agent_diva_core::planning::ExecutionInitializationStatus::Ready
            {
                return Ok(execution_session_from_context(context));
            }
        }
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
        let plan = store.get_plan(&plan_id).await?;
        let revision = store
            .get_plan_revision(&plan_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("plan is not awaiting approval"))?;
        let detail = plan_report_projection(&plan, revision, &request.session_key)?;
        if request.revision != revision
            || request.revision_hash
                != agent_diva_core::planning::revision_hash(&detail.revision.markdown)
        {
            return Err(anyhow::anyhow!("plan report revision conflict"));
        }
        let governance = self.governance().await?;
        let mut state = self.ensure_pending_governance(&detail, Utc::now()).await?;
        if state.status == ApprovalStatus::Pending {
            let receipt = GovernanceReceipt {
                request_id: state.request.correlation.request_id.clone(),
                content_digest: state.request.content_digest.clone(),
                policy_version: state.request.policy_version.clone(),
                capability: Capability::PlanExecute,
                resource: state.request.resource.clone(),
                decision: Decision::Allow,
                decided_by: GovernanceSubject {
                    kind: GovernanceSubjectKind::User,
                    id: "desktop-ui".to_string(),
                },
                decided_at: Utc::now(),
                expires_at: state.request.expires_at,
                grant: ApprovalGrant::Once,
            };
            state = governance
                .decide(
                    &state.request.correlation.request_id,
                    state.version,
                    &format!("plan-allow:{}", state.request.correlation.request_id),
                    receipt,
                )
                .await?;
        }
        let mut persisted_context = match existing_context {
            Some(context) => context,
            None if state.status == ApprovalStatus::Allowed => {
                let execution_id = format!("plan-execution:{}", PlanId::new().0);
                let now = Utc::now();
                let context = PersistedExecutionContext {
                    plan_id: plan_id.clone(),
                    revision: request.revision,
                    session_key: request.session_key.clone(),
                    execution_id: execution_id.clone(),
                    context_policy: request.context_policy,
                    boundary: None,
                    compacted_context: request.compacted_context.clone(),
                    initialization_status:
                        agent_diva_core::planning::ExecutionInitializationStatus::Pending,
                    initialization_error: None,
                    created_at: now,
                    updated_at: now,
                };
                store.create_execution_context(&context).await?;
                sqlx::query(
                    "UPDATE plan_governance SET execution_id = ?, context_json = ?, operation_json = ?, updated_at = ?
                     WHERE plan_id = ? AND revision = ? AND request_id = ?",
                )
                .bind(&execution_id)
                .bind(serde_json::to_string(&context)?)
                .bind(serde_json::to_string(request)?)
                .bind(now.to_rfc3339())
                .bind(&plan_id.0)
                .bind(request.revision)
                .bind(&state.request.correlation.request_id)
                .execute(store.pool())
                .await?;
                context
            }
            None => {
                return Err(anyhow::anyhow!(
                    "consumed plan approval has no prepared execution context"
                ));
            }
        };
        if state.status == ApprovalStatus::Allowed {
            let consumed = governance
                .consume_once(
                    &state.request.correlation.request_id,
                    state.version,
                    &format!("plan-consume:{}", state.request.correlation.request_id),
                    Utc::now(),
                )
                .await?;
            if consumed.status != ApprovalStatus::Consumed {
                return Err(anyhow::anyhow!("plan approval receipt was not consumed"));
            }
            state = consumed;
        }
        if state.status != ApprovalStatus::Consumed {
            return Err(anyhow::anyhow!("plan approval is not consumable"));
        }
        let execution_id = persisted_context.execution_id.clone();
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
            .approve_revision_with_execution_id(
                &request.session_key,
                &plan_id,
                request.revision,
                &request.revision_hash,
                request.context_policy,
                request.compacted_context.as_deref(),
                execution_id,
            )
            .await?;
        if receipt.revision != session.revision {
            return Err(anyhow::anyhow!(
                "canonical approval revision does not match execution session"
            ));
        }
        persisted_context.initialization_status =
            agent_diva_core::planning::ExecutionInitializationStatus::Ready;
        persisted_context.updated_at = Utc::now();
        store.update_execution_context(&persisted_context).await?;
        self.registry
            .update_execution_context(
                &session.id,
                session.boundary.clone(),
                session.compacted_context.clone(),
                agent_diva_core::planning::ExecutionInitializationStatus::Ready,
                None,
            )
            .await?;
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

    pub async fn revoke_approval(
        &self,
        plan_id: &PlanId,
        revision: i64,
        expected_version: u64,
        idempotency_key: &str,
        actor: GovernanceSubject,
    ) -> anyhow::Result<ApprovalState> {
        let store = self.canonical_store().await?;
        self.initialize_governance_mapping(&store).await?;
        let request_id: String = sqlx::query_scalar(
            "SELECT request_id FROM plan_governance WHERE plan_id = ? AND revision = ?",
        )
        .bind(&plan_id.0)
        .bind(revision)
        .fetch_one(store.pool())
        .await?;
        Ok(self
            .governance()
            .await?
            .revoke(
                &request_id,
                expected_version,
                idempotency_key,
                actor,
                Utc::now(),
            )
            .await?)
    }

    pub async fn expire_approval(
        &self,
        plan_id: &PlanId,
        revision: i64,
        expected_version: u64,
        idempotency_key: &str,
        now: chrono::DateTime<Utc>,
    ) -> anyhow::Result<ApprovalState> {
        let store = self.canonical_store().await?;
        self.initialize_governance_mapping(&store).await?;
        let request_id: String = sqlx::query_scalar(
            "SELECT request_id FROM plan_governance WHERE plan_id = ? AND revision = ?",
        )
        .bind(&plan_id.0)
        .bind(revision)
        .fetch_one(store.pool())
        .await?;
        Ok(self
            .governance()
            .await?
            .expire(&request_id, expected_version, idempotency_key, now)
            .await?)
    }

    /// Recover durable Plan approval and execution initialization state.
    pub async fn recover_incomplete(&self) -> anyhow::Result<usize> {
        let store = self.canonical_store().await?;
        self.initialize_governance_mapping(&store).await?;
        let rows = sqlx::query_as::<_, (String, i64, String, String, Option<String>)>(
            "SELECT plan_id, revision, request_id, session_key, operation_json
             FROM plan_governance ORDER BY plan_id, revision",
        )
        .fetch_all(store.pool())
        .await?;
        let governance = self.governance().await?;
        let now = Utc::now();
        let mut recovered = 0;
        for (plan_id, revision, request_id, session_key, operation_json) in rows {
            let plan_id = PlanId(plan_id);
            let state = match governance.state(&request_id, now).await {
                Ok(state) => state,
                Err(ApprovalLedgerError::NotFound) => continue,
                Err(error) => return Err(error.into()),
            };
            let context = store.get_execution_context(&plan_id, revision).await?;
            match (state.status, context) {
                (ApprovalStatus::Pending, _) => {}
                (ApprovalStatus::Allowed, None) => {
                    governance
                        .revoke(
                            &request_id,
                            state.version,
                            &format!("plan-recovery-revoke:{request_id}"),
                            GovernanceSubject {
                                kind: GovernanceSubjectKind::System,
                                id: "planning-recovery".to_string(),
                            },
                            now,
                        )
                        .await?;
                    sqlx::query("DELETE FROM plan_governance WHERE plan_id = ? AND revision = ?")
                        .bind(&plan_id.0)
                        .bind(revision)
                        .execute(store.pool())
                        .await?;
                    let plan = store.get_plan(&plan_id).await?;
                    if plan.phase == PlanPhase::AwaitingApproval {
                        let detail = plan_report_projection(&plan, revision, &session_key)?;
                        self.ensure_pending_governance(&detail, now).await?;
                    }
                    recovered += 1;
                }
                (ApprovalStatus::Allowed, Some(context)) => {
                    let consumed = governance
                        .consume_once(
                            &request_id,
                            state.version,
                            &format!("plan-consume:{request_id}"),
                            now,
                        )
                        .await?;
                    if consumed.status == ApprovalStatus::Consumed {
                        let operation: ApprovePlanReportRequest =
                            serde_json::from_str(operation_json.as_deref().ok_or_else(|| {
                                anyhow::anyhow!("prepared plan operation is missing")
                            })?)?;
                        self.complete_prepared_plan(&store, &operation, context)
                            .await?;
                        recovered += 1;
                    }
                }
                (ApprovalStatus::Consumed, Some(context)) => {
                    if context.initialization_status
                        != agent_diva_core::planning::ExecutionInitializationStatus::Ready
                    {
                        let operation: ApprovePlanReportRequest =
                            serde_json::from_str(operation_json.as_deref().ok_or_else(|| {
                                anyhow::anyhow!("prepared plan operation is missing")
                            })?)?;
                        self.complete_prepared_plan(&store, &operation, context)
                            .await?;
                        recovered += 1;
                    }
                }
                (ApprovalStatus::Consumed, None) => {
                    return Err(anyhow::anyhow!(
                        "consumed plan approval {request_id} has no prepared execution context"
                    ));
                }
                _ => {}
            }
        }
        Ok(recovered)
    }

    async fn complete_prepared_plan(
        &self,
        store: &SqlitePlanningStore,
        request: &ApprovePlanReportRequest,
        mut context: PersistedExecutionContext,
    ) -> anyhow::Result<ExecutionSession> {
        let plan = store.get_plan(&context.plan_id).await?;
        let markdown = plan
            .strategy
            .clone()
            .ok_or_else(|| anyhow::anyhow!("prepared plan has no persisted markdown"))?;
        store
            .approve_plan(
                &context.plan_id,
                &ApprovalRequest {
                    expected_revision: context.revision,
                    approved_by: "desktop-ui".to_string(),
                    todo_policy: request.todo_policy,
                    materialize_todos: request.materialize_todos,
                },
            )
            .await?;
        context.initialization_status =
            agent_diva_core::planning::ExecutionInitializationStatus::Ready;
        context.initialization_error = None;
        context.updated_at = Utc::now();
        store.update_execution_context(&context).await?;
        let session = execution_session_from_context(&context);
        self.registry
            .restore_execution(&context.session_key, session.clone(), markdown)
            .await?;
        Ok(session)
    }

    async fn governance(&self) -> anyhow::Result<ApprovalCoordinator> {
        if let Some(governance) = self.governance.as_ref() {
            return Ok(governance.clone());
        }
        self.fallback_governance
            .get_or_try_init(|| async {
                let governance_dir = self.workspace.join(".laputa");
                std::fs::create_dir_all(&governance_dir)?;
                let pool = SqlitePoolOptions::new()
                    .max_connections(4)
                    .connect_with(
                        SqliteConnectOptions::new()
                            .filename(governance_dir.join("governance.db"))
                            .create_if_missing(true),
                    )
                    .await?;
                Ok::<_, anyhow::Error>(ApprovalCoordinator::new(Arc::new(
                    SqliteGovernanceLedger::new(pool).await?,
                )))
            })
            .await
            .cloned()
    }

    async fn initialize_governance_mapping(
        &self,
        store: &SqlitePlanningStore,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS plan_governance (
                plan_id TEXT NOT NULL,
                revision INTEGER NOT NULL,
                request_id TEXT NOT NULL UNIQUE,
                content_digest TEXT NOT NULL,
                session_key TEXT NOT NULL,
                execution_id TEXT,
                context_json TEXT,
                operation_json TEXT,
                updated_at TEXT NOT NULL,
                PRIMARY KEY(plan_id, revision)
            )",
        )
        .execute(store.pool())
        .await?;
        Ok(())
    }

    async fn plan_session_key(
        &self,
        plan_id: &PlanId,
        revision: i64,
    ) -> anyhow::Result<Option<String>> {
        let store = self.canonical_store().await?;
        self.initialize_governance_mapping(&store).await?;
        Ok(sqlx::query_scalar(
            "SELECT session_key FROM plan_governance WHERE plan_id = ? AND revision = ?",
        )
        .bind(&plan_id.0)
        .bind(revision)
        .fetch_optional(store.pool())
        .await?)
    }

    fn governance_request(
        &self,
        detail: &PlanReportDetail,
        request_id: String,
        now: chrono::DateTime<Utc>,
    ) -> GovernanceRequest<()> {
        GovernanceRequest {
            correlation: AuditCorrelation {
                request_id,
                turn_id: format!("plan:{}:{}", detail.report.id.0, detail.revision.revision),
                session_id: detail.report.session_key.clone(),
                trace_id: None,
            },
            subject: GovernanceSubject {
                kind: GovernanceSubjectKind::Agent,
                id: "planning-service".to_string(),
            },
            capability: Capability::PlanExecute,
            resource: ResourceScope {
                workspace_id: self.workspace_id.clone(),
                session_id: Some(detail.report.session_key.clone()),
                kind: ResourceKind::Plan,
                resource_id: detail.report.id.0.clone(),
                boundary: Some(format!("revision:{}", detail.revision.revision)),
            },
            risk: RiskClass::High,
            content_digest: ContentDigest {
                algorithm: DigestAlgorithm::Sha256,
                value: agent_diva_core::planning::revision_hash(&detail.revision.markdown),
            },
            policy_version: "plan-execute-v1".to_string(),
            created_at: now,
            expires_at: now + Duration::hours(24),
            evidence_refs: Vec::new(),
            payload: (),
        }
    }

    async fn ensure_pending_governance(
        &self,
        detail: &PlanReportDetail,
        now: chrono::DateTime<Utc>,
    ) -> anyhow::Result<ApprovalState> {
        let store = self.canonical_store().await?;
        self.initialize_governance_mapping(&store).await?;
        let governance = self.governance().await?;
        let digest = agent_diva_core::planning::revision_hash(&detail.revision.markdown);
        let existing: Option<(String, String)> = sqlx::query_as(
            "SELECT request_id, content_digest FROM plan_governance
             WHERE plan_id = ? AND revision = ?",
        )
        .bind(&detail.report.id.0)
        .bind(detail.revision.revision)
        .fetch_optional(store.pool())
        .await?;
        if let Some((request_id, mapped_digest)) = existing {
            if mapped_digest == digest {
                let state = governance.state(&request_id, now).await?;
                if matches!(
                    state.status,
                    ApprovalStatus::Pending | ApprovalStatus::Allowed | ApprovalStatus::Consumed
                ) {
                    return Ok(state);
                }
            }
        }

        let stale: Vec<String> = sqlx::query_scalar(
            "SELECT request_id FROM plan_governance WHERE plan_id = ? AND revision != ?",
        )
        .bind(&detail.report.id.0)
        .bind(detail.revision.revision)
        .fetch_all(store.pool())
        .await?;
        for request_id in stale {
            if let Ok(state) = governance.state(&request_id, now).await {
                if matches!(
                    state.status,
                    ApprovalStatus::Pending | ApprovalStatus::Allowed
                ) {
                    governance
                        .revoke(
                            &request_id,
                            state.version,
                            &format!("plan-revision-revoke:{request_id}"),
                            GovernanceSubject {
                                kind: GovernanceSubjectKind::System,
                                id: "planning-service".to_string(),
                            },
                            now,
                        )
                        .await?;
                }
            }
        }

        let request_id = format!(
            "plan-execute:{}:{}:{}",
            detail.report.id.0,
            detail.revision.revision,
            PlanId::new().0
        );
        let request = self.governance_request(detail, request_id.clone(), now);
        let outcome = governance
            .coordinate(
                &request,
                &PolicyContext {
                    evaluated_at: now,
                    autonomy: AutonomyLevel::L1,
                    explicit_user_decision: None,
                    restrictions: Vec::new(),
                    authorizations: Vec::new(),
                },
                &format!("plan-submit:{request_id}"),
            )
            .await?;
        let state = outcome
            .pending_state()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("plan execution did not enter human approval"))?;
        sqlx::query(
            "INSERT INTO plan_governance(
                plan_id, revision, request_id, content_digest, session_key, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(plan_id, revision) DO UPDATE SET
                request_id=excluded.request_id,
                content_digest=excluded.content_digest,
                session_key=excluded.session_key,
                execution_id=NULL,
                context_json=NULL,
                operation_json=NULL,
                updated_at=excluded.updated_at",
        )
        .bind(&detail.report.id.0)
        .bind(detail.revision.revision)
        .bind(&request_id)
        .bind(&digest)
        .bind(&detail.report.session_key)
        .bind(now.to_rfc3339())
        .execute(store.pool())
        .await?;
        Ok(state)
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

fn execution_session_from_context(context: &PersistedExecutionContext) -> ExecutionSession {
    ExecutionSession {
        id: context.execution_id.clone(),
        report_id: context.plan_id.clone(),
        revision: context.revision,
        context_policy: context.context_policy,
        status: agent_diva_core::planning::ExecutionSessionStatus::Executing,
        compacted_context: context.compacted_context.clone(),
        boundary: context.boundary.clone(),
        initialization_status: context.initialization_status,
        initialization_error: context.initialization_error.clone(),
        created_at: context.created_at,
        updated_at: context.updated_at,
    }
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

    fn plan_markdown() -> String {
        [
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
        .join("\n")
    }

    fn allow_receipt(state: &ApprovalState) -> GovernanceReceipt {
        GovernanceReceipt {
            request_id: state.request.correlation.request_id.clone(),
            content_digest: state.request.content_digest.clone(),
            policy_version: state.request.policy_version.clone(),
            capability: Capability::PlanExecute,
            resource: state.request.resource.clone(),
            decision: Decision::Allow,
            decided_by: GovernanceSubject {
                kind: GovernanceSubjectKind::User,
                id: "reviewer".to_string(),
            },
            decided_at: Utc::now(),
            expires_at: state.request.expires_at,
            grant: ApprovalGrant::Once,
        }
    }

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
        let markdown = plan_markdown();
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
        let governance_pool = SqlitePoolOptions::new()
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(temp.path().join(".laputa").join("governance.db")),
            )
            .await
            .unwrap();
        let ledger_json: Vec<String> =
            sqlx::query_scalar("SELECT event_json FROM governance_ledger_events")
                .fetch_all(&governance_pool)
                .await
                .unwrap();
        assert!(!ledger_json.join("\n").contains(&markdown));

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
        let governance_state = restarted
            .ensure_pending_governance(&detail, Utc::now())
            .await
            .unwrap();
        assert_eq!(governance_state.status, ApprovalStatus::Consumed);
    }

    #[tokio::test]
    async fn restart_revokes_dangling_plan_allow_and_creates_new_pending() {
        let temp = tempfile::tempdir().unwrap();
        let service = PlanningService::new(temp.path().to_path_buf());
        let markdown = plan_markdown();
        let detail = service
            .create_report(
                "gui:dangling",
                "Dangling plan",
                &markdown,
                PlanRevisionAuthor::Agent,
            )
            .await
            .unwrap();
        let pending = service
            .ensure_pending_governance(&detail, Utc::now())
            .await
            .unwrap();
        let governance = service.governance().await.unwrap();
        let allowed = governance
            .decide(
                &pending.request.correlation.request_id,
                pending.version,
                "allow-dangling-plan",
                allow_receipt(&pending),
            )
            .await
            .unwrap();
        assert_eq!(allowed.status, ApprovalStatus::Allowed);

        let restarted = PlanningService::new(temp.path().to_path_buf());
        assert_eq!(restarted.recover_incomplete().await.unwrap(), 1);
        assert_eq!(
            governance
                .state(&pending.request.correlation.request_id, Utc::now())
                .await
                .unwrap()
                .status,
            ApprovalStatus::Revoked
        );
        let store = restarted.canonical_store().await.unwrap();
        let new_request: String = sqlx::query_scalar(
            "SELECT request_id FROM plan_governance WHERE plan_id = ? AND revision = 1",
        )
        .bind(&detail.report.id.0)
        .fetch_one(store.pool())
        .await
        .unwrap();
        assert_ne!(new_request, pending.request.correlation.request_id);
        assert_eq!(
            governance
                .state(&new_request, Utc::now())
                .await
                .unwrap()
                .status,
            ApprovalStatus::Pending
        );
    }

    #[tokio::test]
    async fn consumed_prepared_plan_recovers_exactly_once() {
        let temp = tempfile::tempdir().unwrap();
        let service = PlanningService::new(temp.path().to_path_buf());
        let markdown = plan_markdown();
        let detail = service
            .create_report(
                "gui:prepared",
                "Prepared plan",
                &markdown,
                PlanRevisionAuthor::Agent,
            )
            .await
            .unwrap();
        let request = ApprovePlanReportRequest {
            session_key: "gui:prepared".to_string(),
            revision: 1,
            revision_hash: revision_hash(&markdown),
            context_policy: ExecutionContextPolicy::Compact,
            compacted_context: Some("prepared summary".to_string()),
            todo_policy: TodoPolicy::Always,
            materialize_todos: true,
        };
        let pending = service
            .ensure_pending_governance(&detail, Utc::now())
            .await
            .unwrap();
        let governance = service.governance().await.unwrap();
        let allowed = governance
            .decide(
                &pending.request.correlation.request_id,
                pending.version,
                "allow-prepared-plan",
                allow_receipt(&pending),
            )
            .await
            .unwrap();
        let store = service.canonical_store().await.unwrap();
        let now = Utc::now();
        let context = PersistedExecutionContext {
            plan_id: detail.report.id.clone(),
            revision: 1,
            session_key: request.session_key.clone(),
            execution_id: "execution-prepared".to_string(),
            context_policy: request.context_policy,
            boundary: None,
            compacted_context: request.compacted_context.clone(),
            initialization_status:
                agent_diva_core::planning::ExecutionInitializationStatus::Pending,
            initialization_error: None,
            created_at: now,
            updated_at: now,
        };
        store.create_execution_context(&context).await.unwrap();
        sqlx::query(
            "UPDATE plan_governance SET execution_id = ?, context_json = ?, operation_json = ?
             WHERE plan_id = ? AND revision = 1",
        )
        .bind(&context.execution_id)
        .bind(serde_json::to_string(&context).unwrap())
        .bind(serde_json::to_string(&request).unwrap())
        .bind(&detail.report.id.0)
        .execute(store.pool())
        .await
        .unwrap();
        governance
            .consume_once(
                &pending.request.correlation.request_id,
                allowed.version,
                "plan-consume-prepared",
                Utc::now(),
            )
            .await
            .unwrap();

        let restarted = PlanningService::new(temp.path().to_path_buf());
        assert_eq!(restarted.recover_incomplete().await.unwrap(), 1);
        assert_eq!(restarted.recover_incomplete().await.unwrap(), 0);
        let store = restarted.canonical_store().await.unwrap();
        assert_eq!(
            store.get_plan(&detail.report.id).await.unwrap().phase,
            PlanPhase::Execute
        );
        assert_eq!(
            store
                .get_todos(&detail.report.id)
                .await
                .unwrap()
                .items
                .len(),
            2
        );
        assert_eq!(
            store
                .get_execution_context(&detail.report.id, 1)
                .await
                .unwrap()
                .unwrap()
                .initialization_status,
            agent_diva_core::planning::ExecutionInitializationStatus::Ready
        );
    }

    #[tokio::test]
    async fn plan_revision_edit_revokes_old_request_and_rebinds_digest() {
        let temp = tempfile::tempdir().unwrap();
        let service = PlanningService::new(temp.path().to_path_buf());
        let markdown = plan_markdown();
        let detail = service
            .create_report(
                "gui:revision",
                "Revision plan",
                &markdown,
                PlanRevisionAuthor::Agent,
            )
            .await
            .unwrap();
        let old = service
            .ensure_pending_governance(&detail, Utc::now())
            .await
            .unwrap();
        let revised_markdown = markdown.replace("Ship the change.", "Ship the revised change.");
        let revised = service
            .append_report_revision(
                &AppendPlanReportRevisionRequest {
                    session_key: "gui:revision".to_string(),
                    expected_revision: 1,
                    title: "Revision plan".to_string(),
                    markdown: revised_markdown,
                },
                &detail.report.id.0,
            )
            .await
            .unwrap();
        let governance = service.governance().await.unwrap();
        assert_eq!(
            governance
                .state(&old.request.correlation.request_id, Utc::now())
                .await
                .unwrap()
                .status,
            ApprovalStatus::Revoked
        );
        let current = service
            .ensure_pending_governance(&revised, Utc::now())
            .await
            .unwrap();
        assert_eq!(current.status, ApprovalStatus::Pending);
        assert_ne!(current.request.content_digest, old.request.content_digest);
    }

    #[tokio::test]
    async fn revoked_and_expired_plan_receipts_never_execute() {
        let temp = tempfile::tempdir().unwrap();
        let service = PlanningService::new(temp.path().to_path_buf());
        let markdown = plan_markdown();
        let revoked_detail = service
            .create_report(
                "gui:revoke",
                "Revoked plan",
                &markdown,
                PlanRevisionAuthor::Agent,
            )
            .await
            .unwrap();
        let revoked_pending = service
            .ensure_pending_governance(&revoked_detail, Utc::now())
            .await
            .unwrap();
        let revoked = service
            .revoke_approval(
                &revoked_detail.report.id,
                1,
                revoked_pending.version,
                "revoke-plan-test",
                GovernanceSubject {
                    kind: GovernanceSubjectKind::User,
                    id: "reviewer".to_string(),
                },
            )
            .await
            .unwrap();
        assert_eq!(revoked.status, ApprovalStatus::Revoked);

        let expired_detail = service
            .create_report(
                "gui:expire",
                "Expired plan",
                &markdown,
                PlanRevisionAuthor::Agent,
            )
            .await
            .unwrap();
        let expired_pending = service
            .ensure_pending_governance(&expired_detail, Utc::now())
            .await
            .unwrap();
        let expired = service
            .expire_approval(
                &expired_detail.report.id,
                1,
                expired_pending.version,
                "expire-plan-test",
                expired_pending.request.expires_at + Duration::seconds(1),
            )
            .await
            .unwrap();
        assert_eq!(expired.status, ApprovalStatus::Expired);
        let store = service.canonical_store().await.unwrap();
        assert_eq!(
            store
                .get_plan(&revoked_detail.report.id)
                .await
                .unwrap()
                .phase,
            PlanPhase::AwaitingApproval
        );
        assert_eq!(
            store
                .get_plan(&expired_detail.report.id)
                .await
                .unwrap()
                .phase,
            PlanPhase::AwaitingApproval
        );
    }
}

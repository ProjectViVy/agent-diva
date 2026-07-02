//! Persistent storage for the planning domain.
//!
//! Provides the [`PlanningStore`] trait and a SQLite-backed implementation
//! ([`SqlitePlanningStore`]) that auto-creates the required schema.

use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;

use super::events::{PlanEvent, TodoEvent};
use super::ids::{PlanId, TodoId};
use super::model::{Plan, PlanStep, TodoItem, TodoList};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Planning-specific error variants.
#[derive(Debug, thiserror::Error)]
pub enum PlanningError {
    #[error("Plan not found: {0}")]
    PlanNotFound(String),
    #[error("Todo not found: {0}")]
    TodoNotFound(String),
    #[error("No active plan")]
    NoActivePlan,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<PlanningError> for crate::Error {
    fn from(e: PlanningError) -> Self {
        crate::Error::Internal(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Trait abstracting plan/step/todo persistence and event logging.
#[async_trait]
pub trait PlanningStore: Send + Sync {
    async fn create_plan(&self, plan: &Plan) -> crate::Result<()>;
    async fn get_plan(&self, id: &PlanId) -> crate::Result<Plan>;
    async fn update_plan(&self, plan: &Plan) -> crate::Result<()>;
    async fn delete_plan(&self, id: &PlanId) -> crate::Result<()>;
    async fn list_plans(&self) -> crate::Result<Vec<Plan>>;

    async fn create_step(&self, step: &PlanStep) -> crate::Result<()>;
    async fn get_steps(&self, plan_id: &PlanId) -> crate::Result<Vec<PlanStep>>;
    async fn update_step(&self, step: &PlanStep) -> crate::Result<()>;

    async fn create_todo(&self, plan_id: &PlanId, todo: &TodoItem) -> crate::Result<()>;
    async fn get_todos(&self, plan_id: &PlanId) -> crate::Result<TodoList>;
    async fn update_todo(&self, plan_id: &PlanId, todo: &TodoItem) -> crate::Result<()>;
    async fn delete_todos(&self, plan_id: &PlanId) -> crate::Result<()>;
    async fn replace_todos(
        &self,
        plan_id: &PlanId,
        todos: &[TodoItem],
        event: &TodoEvent,
    ) -> crate::Result<()>;

    async fn append_event(&self, plan_id: &PlanId, event: &PlanEvent) -> crate::Result<()>;
    async fn append_todo_event(&self, plan_id: &PlanId, event: &TodoEvent) -> crate::Result<()>;
    async fn get_events(&self, plan_id: &PlanId) -> crate::Result<Vec<PlanEvent>>;

    async fn set_active_plan(&self, plan_id: &PlanId) -> crate::Result<()>;
    async fn get_active_plan(&self) -> crate::Result<PlanId>;
}

// ---------------------------------------------------------------------------
// SQLite Implementation
// ---------------------------------------------------------------------------

/// SQLite-backed implementation of [`PlanningStore`].
pub struct SqlitePlanningStore {
    pool: SqlitePool,
}

impl SqlitePlanningStore {
    /// Create a new store, running migrations to ensure all tables exist.
    pub async fn new(pool: SqlitePool) -> Result<Self, PlanningError> {
        configure_sqlite_connection(&pool).await?;

        let mut tx = pool.begin().await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS plans (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                goal TEXT NOT NULL,
                phase TEXT NOT NULL,
                status TEXT NOT NULL,
                strategy TEXT,
                assumptions TEXT,
                risks TEXT,
                open_questions TEXT,
                verification_verdict TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )"#,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS plan_steps (
                id TEXT PRIMARY KEY,
                plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
                ordinal INTEGER NOT NULL,
                title TEXT NOT NULL,
                rationale TEXT,
                expected_output TEXT,
                status TEXT NOT NULL,
                evidence_ref TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )"#,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS todo_items (
                id TEXT PRIMARY KEY,
                plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
                plan_step_id TEXT,
                title TEXT NOT NULL,
                detail TEXT,
                status TEXT NOT NULL,
                priority TEXT NOT NULL,
                evidence_ref TEXT,
                block_reason TEXT,
                updated_at TEXT NOT NULL
            )"#,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS planning_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
                event_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                created_at TEXT NOT NULL
            )"#,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS active_plan (
                singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE
            )"#,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Self { pool })
    }
}

async fn configure_sqlite_connection(pool: &SqlitePool) -> Result<(), PlanningError> {
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(pool)
        .await?;
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(pool)
        .await?;
    sqlx::query("PRAGMA busy_timeout = 5000")
        .execute(pool)
        .await?;
    Ok(())
}

#[async_trait]
impl PlanningStore for SqlitePlanningStore {
    async fn create_plan(&self, plan: &Plan) -> crate::Result<()> {
        let assumptions = serde_json::to_string(&plan.assumptions)?;
        let risks = serde_json::to_string(&plan.risks)?;
        let open_questions = serde_json::to_string(&plan.open_questions)?;

        sqlx::query(
            r#"INSERT INTO plans (id, title, goal, phase, status, strategy, assumptions, risks, open_questions, verification_verdict, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&plan.id.0)
        .bind(&plan.title)
        .bind(&plan.goal)
        .bind(plan.phase.to_string())
        .bind(plan.status.to_string())
        .bind(&plan.strategy)
        .bind(assumptions)
        .bind(risks)
        .bind(open_questions)
        .bind(plan.verification_verdict.as_ref().map(|v| v.to_string()))
        .bind(plan.created_at.to_rfc3339())
        .bind(plan.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_plan(&self, id: &PlanId) -> crate::Result<Plan> {
        let row = sqlx::query_as::<_, PlanRow>("SELECT * FROM plans WHERE id = ?")
            .bind(&id.0)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(r) => Ok(r.into_plan()?),
            None => Err(PlanningError::PlanNotFound(id.0.clone()).into()),
        }
    }

    async fn update_plan(&self, plan: &Plan) -> crate::Result<()> {
        let assumptions = serde_json::to_string(&plan.assumptions)?;
        let risks = serde_json::to_string(&plan.risks)?;
        let open_questions = serde_json::to_string(&plan.open_questions)?;

        let result = sqlx::query(
            r#"UPDATE plans SET title = ?, goal = ?, phase = ?, status = ?, strategy = ?,
               assumptions = ?, risks = ?, open_questions = ?, verification_verdict = ?,
               updated_at = ? WHERE id = ?"#,
        )
        .bind(&plan.title)
        .bind(&plan.goal)
        .bind(plan.phase.to_string())
        .bind(plan.status.to_string())
        .bind(&plan.strategy)
        .bind(assumptions)
        .bind(risks)
        .bind(open_questions)
        .bind(plan.verification_verdict.as_ref().map(|v| v.to_string()))
        .bind(plan.updated_at.to_rfc3339())
        .bind(&plan.id.0)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(PlanningError::PlanNotFound(plan.id.0.clone()).into());
        }
        Ok(())
    }

    async fn delete_plan(&self, id: &PlanId) -> crate::Result<()> {
        sqlx::query("DELETE FROM plans WHERE id = ?")
            .bind(&id.0)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_plans(&self) -> crate::Result<Vec<Plan>> {
        let rows = sqlx::query_as::<_, PlanRow>("SELECT * FROM plans ORDER BY created_at")
            .fetch_all(&self.pool)
            .await?;

        let mut plans = Vec::with_capacity(rows.len());
        for r in rows {
            plans.push(r.into_plan()?);
        }
        Ok(plans)
    }

    async fn create_step(&self, step: &PlanStep) -> crate::Result<()> {
        sqlx::query(
            r#"INSERT INTO plan_steps (id, plan_id, ordinal, title, rationale, expected_output, status, evidence_ref, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&step.id)
        .bind(&step.plan_id.0)
        .bind(step.ordinal)
        .bind(&step.title)
        .bind(&step.rationale)
        .bind(&step.expected_output)
        .bind(step.status.to_string())
        .bind(&step.evidence_ref)
        .bind(step.created_at.to_rfc3339())
        .bind(step.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_steps(&self, plan_id: &PlanId) -> crate::Result<Vec<PlanStep>> {
        let rows = sqlx::query_as::<_, StepRow>(
            "SELECT * FROM plan_steps WHERE plan_id = ? ORDER BY ordinal",
        )
        .bind(&plan_id.0)
        .fetch_all(&self.pool)
        .await?;

        let mut steps = Vec::with_capacity(rows.len());
        for r in rows {
            steps.push(r.into_step()?);
        }
        Ok(steps)
    }

    async fn update_step(&self, step: &PlanStep) -> crate::Result<()> {
        let result = sqlx::query(
            r#"UPDATE plan_steps SET ordinal = ?, title = ?, rationale = ?, expected_output = ?,
               status = ?, evidence_ref = ?, updated_at = ? WHERE id = ?"#,
        )
        .bind(step.ordinal)
        .bind(&step.title)
        .bind(&step.rationale)
        .bind(&step.expected_output)
        .bind(step.status.to_string())
        .bind(&step.evidence_ref)
        .bind(step.updated_at.to_rfc3339())
        .bind(&step.id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::NotFound(format!(
                "Step not found: {}",
                step.id
            )));
        }
        Ok(())
    }

    async fn create_todo(&self, plan_id: &PlanId, todo: &TodoItem) -> crate::Result<()> {
        sqlx::query(
            r#"INSERT INTO todo_items (id, plan_id, plan_step_id, title, detail, status, priority, evidence_ref, block_reason, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&todo.id.0)
        .bind(&plan_id.0)
        .bind(&todo.plan_step_id)
        .bind(&todo.title)
        .bind(&todo.detail)
        .bind(todo.status.to_string())
        .bind(todo.priority.to_string())
        .bind(&todo.evidence_ref)
        .bind(&todo.block_reason)
        .bind(todo.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_todos(&self, plan_id: &PlanId) -> crate::Result<TodoList> {
        let rows = sqlx::query_as::<_, TodoRow>(
            "SELECT * FROM todo_items WHERE plan_id = ? ORDER BY rowid",
        )
        .bind(&plan_id.0)
        .fetch_all(&self.pool)
        .await?;

        let mut items = Vec::with_capacity(rows.len());
        for r in rows {
            items.push(r.into_todo()?);
        }

        // Determine revision from the number of planning_events with event_type containing "TodoRevised" or just count of todos-related events
        let revision: i32 = sqlx::query_scalar::<_, i32>(
            "SELECT COALESCE(MAX(CAST(SUBSTR(payload, 12, 4) AS INTEGER)), 1) FROM planning_events WHERE plan_id = ? AND event_type = 'TodoEvent'",
        )
        .bind(&plan_id.0)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(1);

        Ok(TodoList {
            plan_id: plan_id.clone(),
            revision,
            items,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    async fn update_todo(&self, plan_id: &PlanId, todo: &TodoItem) -> crate::Result<()> {
        let result = sqlx::query(
            r#"UPDATE todo_items SET plan_step_id = ?, title = ?, detail = ?, status = ?,
               priority = ?, evidence_ref = ?, block_reason = ?, updated_at = ?
               WHERE id = ? AND plan_id = ?"#,
        )
        .bind(&todo.plan_step_id)
        .bind(&todo.title)
        .bind(&todo.detail)
        .bind(todo.status.to_string())
        .bind(todo.priority.to_string())
        .bind(&todo.evidence_ref)
        .bind(&todo.block_reason)
        .bind(todo.updated_at.to_rfc3339())
        .bind(&todo.id.0)
        .bind(&plan_id.0)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(PlanningError::TodoNotFound(todo.id.0.clone()).into());
        }
        Ok(())
    }

    async fn delete_todos(&self, plan_id: &PlanId) -> crate::Result<()> {
        sqlx::query("DELETE FROM todo_items WHERE plan_id = ?")
            .bind(&plan_id.0)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn replace_todos(
        &self,
        plan_id: &PlanId,
        todos: &[TodoItem],
        event: &TodoEvent,
    ) -> crate::Result<()> {
        let payload = serde_json::to_string(event)?;
        let now = Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM todo_items WHERE plan_id = ?")
            .bind(&plan_id.0)
            .execute(&mut *tx)
            .await?;

        for todo in todos {
            sqlx::query(
                r#"INSERT INTO todo_items (id, plan_id, plan_step_id, title, detail, status, priority, evidence_ref, block_reason, updated_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&todo.id.0)
            .bind(&plan_id.0)
            .bind(&todo.plan_step_id)
            .bind(&todo.title)
            .bind(&todo.detail)
            .bind(todo.status.to_string())
            .bind(todo.priority.to_string())
            .bind(&todo.evidence_ref)
            .bind(&todo.block_reason)
            .bind(todo.updated_at.to_rfc3339())
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            r#"INSERT INTO planning_events (plan_id, event_type, payload, created_at) VALUES (?, ?, ?, ?)"#,
        )
        .bind(&plan_id.0)
        .bind("TodoEvent")
        .bind(payload)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn append_event(&self, plan_id: &PlanId, event: &PlanEvent) -> crate::Result<()> {
        let event_type = plan_event_type(event);
        let payload = serde_json::to_string(event)?;

        sqlx::query(
            r#"INSERT INTO planning_events (plan_id, event_type, payload, created_at) VALUES (?, ?, ?, ?)"#,
        )
        .bind(&plan_id.0)
        .bind(event_type)
        .bind(payload)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn append_todo_event(&self, plan_id: &PlanId, event: &TodoEvent) -> crate::Result<()> {
        let event_type = "TodoEvent";
        let payload = serde_json::to_string(event)?;

        sqlx::query(
            r#"INSERT INTO planning_events (plan_id, event_type, payload, created_at) VALUES (?, ?, ?, ?)"#,
        )
        .bind(&plan_id.0)
        .bind(event_type)
        .bind(payload)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_events(&self, plan_id: &PlanId) -> crate::Result<Vec<PlanEvent>> {
        let rows = sqlx::query_as::<_, EventRow>(
            "SELECT * FROM planning_events WHERE plan_id = ? AND event_type != 'TodoEvent' ORDER BY id",
        )
        .bind(&plan_id.0)
        .fetch_all(&self.pool)
        .await?;

        let mut events = Vec::with_capacity(rows.len());
        for r in rows {
            events.push(r.into_event()?);
        }
        Ok(events)
    }

    async fn set_active_plan(&self, plan_id: &PlanId) -> crate::Result<()> {
        // Upsert the singleton row
        sqlx::query(
            r#"INSERT INTO active_plan (singleton, plan_id) VALUES (1, ?)
               ON CONFLICT(singleton) DO UPDATE SET plan_id = excluded.plan_id"#,
        )
        .bind(&plan_id.0)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_active_plan(&self) -> crate::Result<PlanId> {
        let row =
            sqlx::query_scalar::<_, String>("SELECT plan_id FROM active_plan WHERE singleton = 1")
                .fetch_optional(&self.pool)
                .await?;

        match row {
            Some(id) => Ok(PlanId(id)),
            None => Err(PlanningError::NoActivePlan.into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Internal row types
// ---------------------------------------------------------------------------

fn plan_event_type(event: &PlanEvent) -> &'static str {
    match event {
        PlanEvent::Created { .. } => "PlanEvent::Created",
        PlanEvent::Drafted { .. } => "PlanEvent::Drafted",
        PlanEvent::PhaseTransition { .. } => "PlanEvent::PhaseTransition",
        PlanEvent::StatusChanged { .. } => "PlanEvent::StatusChanged",
        PlanEvent::VerificationRecorded { .. } => "PlanEvent::VerificationRecorded",
        PlanEvent::Completed { .. } => "PlanEvent::Completed",
        PlanEvent::Failed { .. } => "PlanEvent::Failed",
        PlanEvent::Partial { .. } => "PlanEvent::Partial",
    }
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct PlanRow {
    id: String,
    title: String,
    goal: String,
    phase: String,
    status: String,
    strategy: Option<String>,
    assumptions: String,
    risks: String,
    open_questions: String,
    verification_verdict: Option<String>,
    created_at: String,
    updated_at: String,
}

impl PlanRow {
    fn into_plan(self) -> Result<Plan, PlanningError> {
        Ok(Plan {
            id: PlanId(self.id),
            title: self.title,
            goal: self.goal,
            phase: parse_plan_phase(&self.phase)?,
            status: parse_plan_status(&self.status)?,
            strategy: self.strategy,
            assumptions: serde_json::from_str(&self.assumptions)?,
            risks: serde_json::from_str(&self.risks)?,
            open_questions: serde_json::from_str(&self.open_questions)?,
            verification_verdict: self
                .verification_verdict
                .as_deref()
                .map(parse_verdict)
                .transpose()?,
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .map_err(|e| {
                    PlanningError::Serialization(serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })?
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&self.updated_at)
                .map_err(|e| {
                    PlanningError::Serialization(serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })?
                .with_timezone(&chrono::Utc),
        })
    }
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct StepRow {
    id: String,
    plan_id: String,
    ordinal: i32,
    title: String,
    rationale: Option<String>,
    expected_output: Option<String>,
    status: String,
    evidence_ref: Option<String>,
    created_at: String,
    updated_at: String,
}

impl StepRow {
    fn into_step(self) -> Result<PlanStep, PlanningError> {
        Ok(PlanStep {
            id: self.id,
            plan_id: PlanId(self.plan_id),
            ordinal: self.ordinal,
            title: self.title,
            rationale: self.rationale,
            expected_output: self.expected_output,
            status: parse_plan_status(&self.status)?,
            evidence_ref: self.evidence_ref,
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .map_err(|e| {
                    PlanningError::Serialization(serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })?
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&self.updated_at)
                .map_err(|e| {
                    PlanningError::Serialization(serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })?
                .with_timezone(&chrono::Utc),
        })
    }
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct TodoRow {
    id: String,
    plan_id: String,
    plan_step_id: Option<String>,
    title: String,
    detail: Option<String>,
    status: String,
    priority: String,
    evidence_ref: Option<String>,
    block_reason: Option<String>,
    updated_at: String,
}

impl TodoRow {
    fn into_todo(self) -> Result<TodoItem, PlanningError> {
        Ok(TodoItem {
            id: TodoId(self.id),
            plan_step_id: self.plan_step_id,
            title: self.title,
            detail: self.detail,
            status: parse_todo_status(&self.status)?,
            priority: parse_todo_priority(&self.priority)?,
            evidence_ref: self.evidence_ref,
            block_reason: self.block_reason,
            updated_at: chrono::DateTime::parse_from_rfc3339(&self.updated_at)
                .map_err(|e| {
                    PlanningError::Serialization(serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })?
                .with_timezone(&chrono::Utc),
        })
    }
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct EventRow {
    id: i64,
    plan_id: String,
    event_type: String,
    payload: String,
    created_at: String,
}

impl EventRow {
    fn into_event(self) -> Result<PlanEvent, PlanningError> {
        Ok(serde_json::from_str(&self.payload)?)
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_plan_phase(s: &str) -> Result<super::model::PlanPhase, PlanningError> {
    match s {
        "Explore" => Ok(super::model::PlanPhase::Explore),
        "Plan" => Ok(super::model::PlanPhase::Plan),
        "AwaitingApproval" => Ok(super::model::PlanPhase::AwaitingApproval),
        "Execute" => Ok(super::model::PlanPhase::Execute),
        "Verify" => Ok(super::model::PlanPhase::Verify),
        "Completed" => Ok(super::model::PlanPhase::Completed),
        "Failed" => Ok(super::model::PlanPhase::Failed),
        "Partial" => Ok(super::model::PlanPhase::Partial),
        other => Err(PlanningError::Serialization(serde_json::Error::io(
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unknown PlanPhase: {other}"),
            ),
        ))),
    }
}

fn parse_plan_status(s: &str) -> Result<super::model::PlanStatus, PlanningError> {
    match s {
        "Pending" => Ok(super::model::PlanStatus::Pending),
        "InProgress" => Ok(super::model::PlanStatus::InProgress),
        "Blocked" => Ok(super::model::PlanStatus::Blocked),
        "Completed" => Ok(super::model::PlanStatus::Completed),
        "Failed" => Ok(super::model::PlanStatus::Failed),
        "Partial" => Ok(super::model::PlanStatus::Partial),
        "Canceled" => Ok(super::model::PlanStatus::Canceled),
        other => Err(PlanningError::Serialization(serde_json::Error::io(
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unknown PlanStatus: {other}"),
            ),
        ))),
    }
}

fn parse_todo_status(s: &str) -> Result<super::model::TodoStatus, PlanningError> {
    match s {
        "Pending" => Ok(super::model::TodoStatus::Pending),
        "InProgress" => Ok(super::model::TodoStatus::InProgress),
        "Blocked" => Ok(super::model::TodoStatus::Blocked),
        "Completed" => Ok(super::model::TodoStatus::Completed),
        "Canceled" => Ok(super::model::TodoStatus::Canceled),
        other => Err(PlanningError::Serialization(serde_json::Error::io(
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unknown TodoStatus: {other}"),
            ),
        ))),
    }
}

fn parse_todo_priority(s: &str) -> Result<super::model::TodoPriority, PlanningError> {
    match s {
        "Low" => Ok(super::model::TodoPriority::Low),
        "Normal" => Ok(super::model::TodoPriority::Normal),
        "High" => Ok(super::model::TodoPriority::High),
        other => Err(PlanningError::Serialization(serde_json::Error::io(
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unknown TodoPriority: {other}"),
            ),
        ))),
    }
}

fn parse_verdict(s: &str) -> Result<super::model::VerificationVerdict, PlanningError> {
    match s {
        "Pass" => Ok(super::model::VerificationVerdict::Pass),
        "Fail" => Ok(super::model::VerificationVerdict::Fail),
        "Partial" => Ok(super::model::VerificationVerdict::Partial),
        other => Err(PlanningError::Serialization(serde_json::Error::io(
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unknown VerificationVerdict: {other}"),
            ),
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planning::ids::*;
    use crate::planning::model::*;
    use chrono::Utc;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn test_store() -> SqlitePlanningStore {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        SqlitePlanningStore::new(pool).await.unwrap()
    }

    fn make_plan(id: &str, title: &str) -> Plan {
        let now = Utc::now();
        Plan {
            id: PlanId(id.to_string()),
            title: title.to_string(),
            goal: "Test goal".to_string(),
            phase: PlanPhase::Explore,
            status: PlanStatus::Pending,
            strategy: None,
            assumptions: vec![],
            risks: vec![],
            open_questions: vec![],
            verification_verdict: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn test_create_and_get_plan() {
        let store = test_store().await;
        let plan = make_plan("p1", "My Plan");

        store.create_plan(&plan).await.unwrap();
        let fetched = store.get_plan(&PlanId("p1".to_string())).await.unwrap();

        assert_eq!(fetched.id.0, "p1");
        assert_eq!(fetched.title, "My Plan");
        assert_eq!(fetched.phase, PlanPhase::Explore);
        assert_eq!(fetched.status, PlanStatus::Pending);
    }

    #[tokio::test]
    async fn test_update_plan() {
        let store = test_store().await;
        let mut plan = make_plan("p1", "Original");
        store.create_plan(&plan).await.unwrap();

        plan.title = "Updated".to_string();
        plan.status = PlanStatus::InProgress;
        plan.updated_at = Utc::now();
        store.update_plan(&plan).await.unwrap();

        let fetched = store.get_plan(&PlanId("p1".to_string())).await.unwrap();
        assert_eq!(fetched.title, "Updated");
        assert_eq!(fetched.status, PlanStatus::InProgress);
    }

    #[tokio::test]
    async fn test_list_plans() {
        let store = test_store().await;
        store.create_plan(&make_plan("p1", "A")).await.unwrap();
        store.create_plan(&make_plan("p2", "B")).await.unwrap();

        let plans = store.list_plans().await.unwrap();
        assert_eq!(plans.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_plan() {
        let store = test_store().await;
        store.create_plan(&make_plan("p1", "Doomed")).await.unwrap();
        store.delete_plan(&PlanId("p1".to_string())).await.unwrap();

        let result = store.get_plan(&PlanId("p1".to_string())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sqlite_pragmas_are_configured() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("planning.sqlite");
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        let store = SqlitePlanningStore::new(pool).await.unwrap();

        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
            .fetch_one(&store.pool)
            .await
            .unwrap();

        assert_eq!(foreign_keys, 1);
        assert_eq!(journal_mode, "wal");
        assert_eq!(busy_timeout, 5000);
    }

    #[tokio::test]
    async fn test_delete_plan_cascades_children() {
        let store = test_store().await;
        let plan_id = PlanId("p1".to_string());
        store
            .create_plan(&make_plan(&plan_id.0, "Cascading"))
            .await
            .unwrap();

        let now = Utc::now();
        let step = PlanStep {
            id: "s1".to_string(),
            plan_id: plan_id.clone(),
            ordinal: 0,
            title: "Step".to_string(),
            rationale: None,
            expected_output: None,
            status: PlanStatus::Pending,
            evidence_ref: None,
            created_at: now,
            updated_at: now,
        };
        store.create_step(&step).await.unwrap();

        let todo = TodoItem {
            id: TodoId("t1".to_string()),
            plan_step_id: Some("s1".to_string()),
            title: "Todo".to_string(),
            detail: None,
            status: TodoStatus::Pending,
            priority: TodoPriority::Normal,
            evidence_ref: None,
            block_reason: None,
            updated_at: now,
        };
        store.create_todo(&plan_id, &todo).await.unwrap();
        store
            .append_event(
                &plan_id,
                &PlanEvent::Created {
                    plan_id: plan_id.clone(),
                    title: "Cascading".to_string(),
                    goal: "Test goal".to_string(),
                },
            )
            .await
            .unwrap();
        store.set_active_plan(&plan_id).await.unwrap();

        store.delete_plan(&plan_id).await.unwrap();

        let steps: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plan_steps WHERE plan_id = ?")
            .bind(&plan_id.0)
            .fetch_one(&store.pool)
            .await
            .unwrap();
        let todos: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM todo_items WHERE plan_id = ?")
            .bind(&plan_id.0)
            .fetch_one(&store.pool)
            .await
            .unwrap();
        let events: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM planning_events WHERE plan_id = ?")
                .bind(&plan_id.0)
                .fetch_one(&store.pool)
                .await
                .unwrap();
        let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM active_plan WHERE plan_id = ?")
            .bind(&plan_id.0)
            .fetch_one(&store.pool)
            .await
            .unwrap();

        assert_eq!(steps, 0);
        assert_eq!(todos, 0);
        assert_eq!(events, 0);
        assert_eq!(active, 0);
    }

    #[tokio::test]
    async fn test_get_plan_not_found() {
        let store = test_store().await;
        let result = store.get_plan(&PlanId("nope".to_string())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_crud_steps() {
        let store = test_store().await;
        store.create_plan(&make_plan("p1", "P")).await.unwrap();

        let now = Utc::now();
        let step = PlanStep {
            id: "s1".to_string(),
            plan_id: PlanId("p1".to_string()),
            ordinal: 0,
            title: "Step One".to_string(),
            rationale: Some("Because".to_string()),
            expected_output: Some("Output".to_string()),
            status: PlanStatus::Pending,
            evidence_ref: None,
            created_at: now,
            updated_at: now,
        };

        store.create_step(&step).await.unwrap();
        let steps = store.get_steps(&PlanId("p1".to_string())).await.unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].title, "Step One");

        // Update
        let mut updated = step.clone();
        updated.title = "Updated Step".to_string();
        updated.status = PlanStatus::InProgress;
        store.update_step(&updated).await.unwrap();

        let steps = store.get_steps(&PlanId("p1".to_string())).await.unwrap();
        assert_eq!(steps[0].title, "Updated Step");
        assert_eq!(steps[0].status, PlanStatus::InProgress);
    }

    #[tokio::test]
    async fn test_crud_todos() {
        let store = test_store().await;
        store.create_plan(&make_plan("p1", "P")).await.unwrap();

        let now = Utc::now();
        let todo = TodoItem {
            id: TodoId("t1".to_string()),
            plan_step_id: Some("s1".to_string()),
            title: "Do thing".to_string(),
            detail: Some("Details".to_string()),
            status: TodoStatus::Pending,
            priority: TodoPriority::High,
            evidence_ref: None,
            block_reason: None,
            updated_at: now,
        };

        store
            .create_todo(&PlanId("p1".to_string()), &todo)
            .await
            .unwrap();
        let list = store.get_todos(&PlanId("p1".to_string())).await.unwrap();
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].title, "Do thing");
        assert_eq!(list.items[0].priority, TodoPriority::High);

        // Update
        let mut updated = todo.clone();
        updated.status = TodoStatus::Completed;
        updated.evidence_ref = Some("ref-abc".to_string());
        store
            .update_todo(&PlanId("p1".to_string()), &updated)
            .await
            .unwrap();

        let list = store.get_todos(&PlanId("p1".to_string())).await.unwrap();
        assert_eq!(list.items[0].status, TodoStatus::Completed);
        assert_eq!(list.items[0].evidence_ref, Some("ref-abc".to_string()));

        // Delete
        store.delete_todos(&PlanId("p1".to_string())).await.unwrap();
        let list = store.get_todos(&PlanId("p1".to_string())).await.unwrap();
        assert!(list.items.is_empty());
    }

    #[tokio::test]
    async fn test_replace_todos_rolls_back_on_insert_failure() {
        let store = test_store().await;
        let plan_id = PlanId("p1".to_string());
        store
            .create_plan(&make_plan(&plan_id.0, "P"))
            .await
            .unwrap();

        let now = Utc::now();
        let old_todo = TodoItem {
            id: TodoId("old".to_string()),
            plan_step_id: None,
            title: "Old todo".to_string(),
            detail: None,
            status: TodoStatus::Pending,
            priority: TodoPriority::Normal,
            evidence_ref: None,
            block_reason: None,
            updated_at: now,
        };
        store.create_todo(&plan_id, &old_todo).await.unwrap();

        let duplicate_todo = TodoItem {
            id: TodoId("dup".to_string()),
            plan_step_id: None,
            title: "Duplicate todo".to_string(),
            detail: None,
            status: TodoStatus::Pending,
            priority: TodoPriority::Normal,
            evidence_ref: None,
            block_reason: None,
            updated_at: now,
        };
        let result = store
            .replace_todos(
                &plan_id,
                &[duplicate_todo.clone(), duplicate_todo],
                &TodoEvent::Revised {
                    plan_id: plan_id.clone(),
                    revision: 2,
                },
            )
            .await;

        assert!(result.is_err());

        let list = store.get_todos(&plan_id).await.unwrap();
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].id.0, "old");

        let todo_events: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM planning_events WHERE plan_id = ? AND event_type = 'TodoEvent'",
        )
        .bind(&plan_id.0)
        .fetch_one(&store.pool)
        .await
        .unwrap();
        assert_eq!(todo_events, 0);
    }

    #[tokio::test]
    async fn test_active_plan_singleton() {
        let store = test_store().await;
        store.create_plan(&make_plan("p1", "First")).await.unwrap();
        store.create_plan(&make_plan("p2", "Second")).await.unwrap();

        store
            .set_active_plan(&PlanId("p1".to_string()))
            .await
            .unwrap();
        let active = store.get_active_plan().await.unwrap();
        assert_eq!(active.0, "p1");

        // Replace
        store
            .set_active_plan(&PlanId("p2".to_string()))
            .await
            .unwrap();
        let active = store.get_active_plan().await.unwrap();
        assert_eq!(active.0, "p2");
    }

    #[tokio::test]
    async fn test_event_append_and_query() {
        let store = test_store().await;
        store.create_plan(&make_plan("p1", "P")).await.unwrap();

        let event1 = PlanEvent::Created {
            plan_id: PlanId("p1".to_string()),
            title: "P".to_string(),
            goal: "G".to_string(),
        };
        let event2 = PlanEvent::Drafted {
            plan_id: PlanId("p1".to_string()),
        };

        store
            .append_event(&PlanId("p1".to_string()), &event1)
            .await
            .unwrap();
        store
            .append_event(&PlanId("p1".to_string()), &event2)
            .await
            .unwrap();

        let events = store.get_events(&PlanId("p1".to_string())).await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], event1);
        assert_eq!(events[1], event2);
    }

    #[tokio::test]
    async fn test_no_active_plan() {
        let store = test_store().await;
        let result = store.get_active_plan().await;
        assert!(result.is_err());
    }
}

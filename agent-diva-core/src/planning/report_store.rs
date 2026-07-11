//! SQLite persistence for immutable Markdown plan reports.

use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use super::ids::PlanId;
use super::report::{
    assert_report_ready_for_approval, revision_hash, ExecutionContextPolicy, ExecutionSession,
    ExecutionSessionStatus, ExecutionTodo, ExecutionTodoPriority, ExecutionTodoStatus, PlanReport,
    PlanReportStatus, PlanRevision, PlanRevisionApproval, PlanRevisionAuthor,
};

#[derive(Clone)]
pub struct SqlitePlanReportStore {
    pool: SqlitePool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanReportDetail {
    pub report: PlanReport,
    pub revision: PlanRevision,
}

#[derive(Debug, FromRow)]
struct ReportRow {
    id: String,
    session_key: String,
    current_revision: i64,
    status: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, FromRow)]
struct RevisionRow {
    report_id: String,
    revision: i64,
    title: String,
    markdown: String,
    author: String,
    created_at: String,
}

impl SqlitePlanReportStore {
    pub async fn new(pool: SqlitePool) -> anyhow::Result<Self> {
        let mut tx = pool.begin().await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS plan_reports (id TEXT PRIMARY KEY, session_key TEXT NOT NULL, current_revision INTEGER NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS plan_report_revisions (report_id TEXT NOT NULL REFERENCES plan_reports(id) ON DELETE CASCADE, revision INTEGER NOT NULL, title TEXT NOT NULL, markdown TEXT NOT NULL, revision_hash TEXT NOT NULL, author TEXT NOT NULL, created_at TEXT NOT NULL, PRIMARY KEY(report_id, revision))",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS plan_report_approvals (report_id TEXT NOT NULL REFERENCES plan_reports(id) ON DELETE CASCADE, revision INTEGER NOT NULL, revision_hash TEXT NOT NULL, context_policy TEXT NOT NULL, approved_at TEXT NOT NULL, PRIMARY KEY(report_id, revision))",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS plan_execution_sessions (id TEXT PRIMARY KEY, report_id TEXT NOT NULL REFERENCES plan_reports(id) ON DELETE CASCADE, revision INTEGER NOT NULL, context_policy TEXT NOT NULL, status TEXT NOT NULL, compacted_context TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS execution_todos (id TEXT PRIMARY KEY, execution_session_id TEXT NOT NULL REFERENCES plan_execution_sessions(id) ON DELETE CASCADE, title TEXT NOT NULL, detail TEXT, status TEXT NOT NULL, priority TEXT NOT NULL, evidence_ref TEXT, block_reason TEXT, updated_at TEXT NOT NULL)",
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Self { pool })
    }

    pub async fn create_report(
        &self,
        session_key: &str,
        title: &str,
        markdown: &str,
        author: PlanRevisionAuthor,
    ) -> anyhow::Result<PlanReportDetail> {
        let report = PlanReport {
            id: PlanId::new(),
            session_key: session_key.to_string(),
            current_revision: 1,
            status: PlanReportStatus::AwaitingApproval,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let revision = PlanRevision {
            report_id: report.id.clone(),
            revision: 1,
            title: title.to_string(),
            markdown: markdown.to_string(),
            author,
            created_at: report.created_at,
        };
        self.insert_report(&report, &revision).await?;
        Ok(PlanReportDetail { report, revision })
    }

    pub async fn append_revision(
        &self,
        report_id: &PlanId,
        expected_revision: i64,
        title: &str,
        markdown: &str,
        author: PlanRevisionAuthor,
    ) -> anyhow::Result<PlanReportDetail> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query_as::<_, ReportRow>("SELECT * FROM plan_reports WHERE id = ?")
            .bind(&report_id.0)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow!("plan report not found"))?;
        if row.current_revision != expected_revision {
            return Err(anyhow!("plan report revision conflict"));
        }
        if row.status == "Approved" || row.status == "Closed" {
            return Err(anyhow!("approved or closed reports are immutable"));
        }
        let revision_number = expected_revision + 1;
        let now = Utc::now();
        sqlx::query("INSERT INTO plan_report_revisions (report_id, revision, title, markdown, revision_hash, author, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(&report_id.0).bind(revision_number).bind(title).bind(markdown)
            .bind(revision_hash(markdown)).bind(author_name(author)).bind(now.to_rfc3339())
            .execute(&mut *tx).await?;
        sqlx::query("UPDATE plan_reports SET current_revision = ?, status = 'AwaitingApproval', updated_at = ? WHERE id = ?")
            .bind(revision_number).bind(now.to_rfc3339()).bind(&report_id.0)
            .execute(&mut *tx).await?;
        tx.commit().await?;
        self.get_detail(report_id, revision_number).await
    }

    pub async fn approve_revision(
        &self,
        report_id: &PlanId,
        revision: i64,
        expected_hash: &str,
        context_policy: ExecutionContextPolicy,
        compacted_context: Option<&str>,
    ) -> anyhow::Result<(PlanRevisionApproval, ExecutionSession)> {
        let mut tx = self.pool.begin().await?;
        let report = sqlx::query_as::<_, ReportRow>("SELECT * FROM plan_reports WHERE id = ?")
            .bind(&report_id.0)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow!("plan report not found"))?;
        if report.current_revision != revision || report.status != "AwaitingApproval" {
            return Err(anyhow!("plan report is not awaiting approval"));
        }
        let row = sqlx::query_as::<_, RevisionRow>("SELECT report_id, revision, title, markdown, author, created_at FROM plan_report_revisions WHERE report_id = ? AND revision = ?")
            .bind(&report_id.0).bind(revision).fetch_optional(&mut *tx).await?
            .ok_or_else(|| anyhow!("plan report revision not found"))?;
        let actual_hash = revision_hash(&row.markdown);
        if actual_hash != expected_hash {
            return Err(anyhow!("plan report revision conflict"));
        }
        assert_report_ready_for_approval(&row.markdown).map_err(|error| anyhow!(error))?;
        let now = Utc::now();
        let approval = PlanRevisionApproval {
            report_id: report_id.clone(),
            revision,
            revision_hash: actual_hash.clone(),
            context_policy,
            approved_at: now,
        };
        let session = ExecutionSession {
            id: uuid::Uuid::new_v4().to_string(),
            report_id: report_id.clone(),
            revision,
            context_policy,
            status: ExecutionSessionStatus::Executing,
            compacted_context: compacted_context.map(ToOwned::to_owned),
            created_at: now,
            updated_at: now,
        };
        sqlx::query("INSERT INTO plan_report_approvals (report_id, revision, revision_hash, context_policy, approved_at) VALUES (?, ?, ?, ?, ?)")
            .bind(&report_id.0).bind(revision).bind(&approval.revision_hash).bind(context_policy_name(context_policy)).bind(now.to_rfc3339()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO plan_execution_sessions (id, report_id, revision, context_policy, status, compacted_context, created_at, updated_at) VALUES (?, ?, ?, ?, 'Executing', ?, ?, ?)")
            .bind(&session.id).bind(&report_id.0).bind(revision).bind(context_policy_name(context_policy)).bind(&session.compacted_context).bind(now.to_rfc3339()).bind(now.to_rfc3339()).execute(&mut *tx).await?;
        sqlx::query("UPDATE plan_reports SET status = 'Approved', updated_at = ? WHERE id = ?")
            .bind(now.to_rfc3339())
            .bind(&report_id.0)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok((approval, session))
    }

    pub async fn list_reports(&self) -> anyhow::Result<Vec<PlanReportDetail>> {
        let rows =
            sqlx::query_as::<_, ReportRow>("SELECT * FROM plan_reports ORDER BY updated_at DESC")
                .fetch_all(&self.pool)
                .await?;
        let mut reports = Vec::with_capacity(rows.len());
        for row in rows {
            reports.push(
                self.get_detail(&PlanId(row.id), row.current_revision)
                    .await?,
            );
        }
        Ok(reports)
    }

    pub async fn active_execution_for_session(
        &self,
        session_key: &str,
    ) -> anyhow::Result<Option<ExecutionSession>> {
        let row = sqlx::query_as::<_, ExecutionSessionRow>(
            "SELECT execution.id, execution.report_id, execution.revision, execution.context_policy, execution.status, execution.compacted_context, execution.created_at, execution.updated_at FROM plan_execution_sessions execution INNER JOIN plan_reports report ON report.id = execution.report_id WHERE report.session_key = ? AND execution.status IN ('Executing', 'Verifying') ORDER BY execution.updated_at DESC LIMIT 1",
        )
        .bind(session_key)
        .fetch_optional(&self.pool)
        .await?;
        row.map(execution_session_from_row).transpose()
    }

    pub async fn replace_execution_todos(
        &self,
        execution_session_id: &str,
        todos: &[ExecutionTodo],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        let active: Option<String> = sqlx::query_scalar(
            "SELECT id FROM plan_execution_sessions WHERE id = ? AND status IN ('Executing', 'Verifying')",
        )
        .bind(execution_session_id)
        .fetch_optional(&mut *tx)
        .await?;
        if active.is_none() {
            return Err(anyhow!("execution session is not active"));
        }
        sqlx::query("DELETE FROM execution_todos WHERE execution_session_id = ?")
            .bind(execution_session_id)
            .execute(&mut *tx)
            .await?;
        for todo in todos {
            sqlx::query("INSERT INTO execution_todos (id, execution_session_id, title, detail, status, priority, evidence_ref, block_reason, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(&todo.id).bind(execution_session_id).bind(&todo.title).bind(&todo.detail).bind(execution_todo_status_name(todo.status)).bind(execution_todo_priority_name(todo.priority)).bind(&todo.evidence_ref).bind(&todo.block_reason).bind(todo.updated_at.to_rfc3339()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn execution_todos(
        &self,
        execution_session_id: &str,
    ) -> anyhow::Result<Vec<ExecutionTodo>> {
        let rows = sqlx::query_as::<_, ExecutionTodoRow>(
            "SELECT * FROM execution_todos WHERE execution_session_id = ? ORDER BY rowid",
        )
        .bind(execution_session_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(execution_todo_from_row).collect()
    }

    /// Update one execution TODO while its owning execution session is active.
    pub async fn update_execution_todo(&self, todo: &ExecutionTodo) -> anyhow::Result<()> {
        let result = sqlx::query(
            "UPDATE execution_todos SET title = ?, detail = ?, status = ?, priority = ?, evidence_ref = ?, block_reason = ?, updated_at = ? WHERE id = ? AND execution_session_id = ? AND EXISTS (SELECT 1 FROM plan_execution_sessions WHERE id = ? AND status IN ('Executing', 'Verifying'))",
        )
        .bind(&todo.title)
        .bind(&todo.detail)
        .bind(execution_todo_status_name(todo.status))
        .bind(execution_todo_priority_name(todo.priority))
        .bind(&todo.evidence_ref)
        .bind(&todo.block_reason)
        .bind(todo.updated_at.to_rfc3339())
        .bind(&todo.id)
        .bind(&todo.execution_session_id)
        .bind(&todo.execution_session_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() != 1 {
            return Err(anyhow!(
                "execution todo is unavailable or its session is not active"
            ));
        }
        Ok(())
    }

    pub async fn get_detail(
        &self,
        report_id: &PlanId,
        revision: i64,
    ) -> anyhow::Result<PlanReportDetail> {
        let report = sqlx::query_as::<_, ReportRow>("SELECT * FROM plan_reports WHERE id = ?")
            .bind(&report_id.0)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow!("plan report not found"))?;
        let revision = sqlx::query_as::<_, RevisionRow>("SELECT report_id, revision, title, markdown, author, created_at FROM plan_report_revisions WHERE report_id = ? AND revision = ?").bind(&report_id.0).bind(revision).fetch_optional(&self.pool).await?.ok_or_else(|| anyhow!("plan report revision not found"))?;
        Ok(PlanReportDetail {
            report: report_from_row(report)?,
            revision: revision_from_row(revision)?,
        })
    }

    /// Load the current revision body for a report id, if it exists.
    pub async fn get_current_detail(
        &self,
        report_id: &PlanId,
    ) -> anyhow::Result<Option<PlanReportDetail>> {
        let row = sqlx::query_as::<_, ReportRow>("SELECT * FROM plan_reports WHERE id = ?")
            .bind(&report_id.0)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(report) => Ok(Some(self.get_detail(report_id, report.current_revision).await?)),
            None => Ok(None),
        }
    }

    async fn insert_report(
        &self,
        report: &PlanReport,
        revision: &PlanRevision,
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO plan_reports (id, session_key, current_revision, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&report.id.0).bind(&report.session_key).bind(report.current_revision).bind(status_name(report.status)).bind(report.created_at.to_rfc3339()).bind(report.updated_at.to_rfc3339()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO plan_report_revisions (report_id, revision, title, markdown, revision_hash, author, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(&report.id.0).bind(revision.revision).bind(&revision.title).bind(&revision.markdown).bind(revision_hash(&revision.markdown)).bind(author_name(revision.author)).bind(revision.created_at.to_rfc3339()).execute(&mut *tx).await?;
        tx.commit().await.context("failed to persist plan report")
    }
}

#[derive(Debug, FromRow)]
struct ExecutionSessionRow {
    id: String,
    report_id: String,
    revision: i64,
    context_policy: String,
    status: String,
    compacted_context: Option<String>,
    created_at: String,
    updated_at: String,
}
#[derive(Debug, FromRow)]
struct ExecutionTodoRow {
    id: String,
    execution_session_id: String,
    title: String,
    detail: Option<String>,
    status: String,
    priority: String,
    evidence_ref: Option<String>,
    block_reason: Option<String>,
    updated_at: String,
}

fn author_name(author: PlanRevisionAuthor) -> &'static str {
    match author {
        PlanRevisionAuthor::Agent => "Agent",
        PlanRevisionAuthor::User => "User",
    }
}
fn status_name(status: PlanReportStatus) -> &'static str {
    match status {
        PlanReportStatus::Draft => "Draft",
        PlanReportStatus::AwaitingApproval => "AwaitingApproval",
        PlanReportStatus::Approved => "Approved",
        PlanReportStatus::Closed => "Closed",
    }
}
fn context_policy_name(policy: ExecutionContextPolicy) -> &'static str {
    match policy {
        ExecutionContextPolicy::Retain => "Retain",
        ExecutionContextPolicy::Compact => "Compact",
        ExecutionContextPolicy::Clear => "Clear",
    }
}
fn parse_time(value: String) -> anyhow::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|time| time.with_timezone(&Utc))
        .context("invalid report timestamp")
}
fn report_status(value: &str) -> anyhow::Result<PlanReportStatus> {
    match value {
        "Draft" => Ok(PlanReportStatus::Draft),
        "AwaitingApproval" => Ok(PlanReportStatus::AwaitingApproval),
        "Approved" => Ok(PlanReportStatus::Approved),
        "Closed" => Ok(PlanReportStatus::Closed),
        _ => Err(anyhow!("invalid report status")),
    }
}
fn revision_author(value: &str) -> anyhow::Result<PlanRevisionAuthor> {
    match value {
        "Agent" => Ok(PlanRevisionAuthor::Agent),
        "User" => Ok(PlanRevisionAuthor::User),
        _ => Err(anyhow!("invalid report revision author")),
    }
}
fn execution_todo_status_name(status: ExecutionTodoStatus) -> &'static str {
    match status {
        ExecutionTodoStatus::Pending => "Pending",
        ExecutionTodoStatus::InProgress => "InProgress",
        ExecutionTodoStatus::Blocked => "Blocked",
        ExecutionTodoStatus::Completed => "Completed",
        ExecutionTodoStatus::Canceled => "Canceled",
    }
}
fn execution_todo_priority_name(priority: ExecutionTodoPriority) -> &'static str {
    match priority {
        ExecutionTodoPriority::Low => "Low",
        ExecutionTodoPriority::Normal => "Normal",
        ExecutionTodoPriority::High => "High",
    }
}
fn execution_todo_status(value: &str) -> anyhow::Result<ExecutionTodoStatus> {
    match value {
        "Pending" => Ok(ExecutionTodoStatus::Pending),
        "InProgress" => Ok(ExecutionTodoStatus::InProgress),
        "Blocked" => Ok(ExecutionTodoStatus::Blocked),
        "Completed" => Ok(ExecutionTodoStatus::Completed),
        "Canceled" => Ok(ExecutionTodoStatus::Canceled),
        _ => Err(anyhow!("invalid execution todo status")),
    }
}
fn execution_todo_priority(value: &str) -> anyhow::Result<ExecutionTodoPriority> {
    match value {
        "Low" => Ok(ExecutionTodoPriority::Low),
        "Normal" => Ok(ExecutionTodoPriority::Normal),
        "High" => Ok(ExecutionTodoPriority::High),
        _ => Err(anyhow!("invalid execution todo priority")),
    }
}
fn execution_context_policy(value: &str) -> anyhow::Result<ExecutionContextPolicy> {
    match value {
        "Retain" => Ok(ExecutionContextPolicy::Retain),
        "Compact" => Ok(ExecutionContextPolicy::Compact),
        "Clear" => Ok(ExecutionContextPolicy::Clear),
        _ => Err(anyhow!("invalid execution context policy")),
    }
}
fn execution_session_status(value: &str) -> anyhow::Result<ExecutionSessionStatus> {
    match value {
        "Executing" => Ok(ExecutionSessionStatus::Executing),
        "Verifying" => Ok(ExecutionSessionStatus::Verifying),
        "Completed" => Ok(ExecutionSessionStatus::Completed),
        "Failed" => Ok(ExecutionSessionStatus::Failed),
        "Partial" => Ok(ExecutionSessionStatus::Partial),
        _ => Err(anyhow!("invalid execution session status")),
    }
}
fn execution_session_from_row(row: ExecutionSessionRow) -> anyhow::Result<ExecutionSession> {
    Ok(ExecutionSession {
        id: row.id,
        report_id: PlanId(row.report_id),
        revision: row.revision,
        context_policy: execution_context_policy(&row.context_policy)?,
        status: execution_session_status(&row.status)?,
        compacted_context: row.compacted_context,
        created_at: parse_time(row.created_at)?,
        updated_at: parse_time(row.updated_at)?,
    })
}
fn execution_todo_from_row(row: ExecutionTodoRow) -> anyhow::Result<ExecutionTodo> {
    Ok(ExecutionTodo {
        id: row.id,
        execution_session_id: row.execution_session_id,
        title: row.title,
        detail: row.detail,
        status: execution_todo_status(&row.status)?,
        priority: execution_todo_priority(&row.priority)?,
        evidence_ref: row.evidence_ref,
        block_reason: row.block_reason,
        updated_at: parse_time(row.updated_at)?,
    })
}
fn report_from_row(row: ReportRow) -> anyhow::Result<PlanReport> {
    Ok(PlanReport {
        id: PlanId(row.id),
        session_key: row.session_key,
        current_revision: row.current_revision,
        status: report_status(&row.status)?,
        created_at: parse_time(row.created_at)?,
        updated_at: parse_time(row.updated_at)?,
    })
}
fn revision_from_row(row: RevisionRow) -> anyhow::Result<PlanRevision> {
    Ok(PlanRevision {
        report_id: PlanId(row.report_id),
        revision: row.revision,
        title: row.title,
        markdown: row.markdown,
        author: revision_author(&row.author)?,
        created_at: parse_time(row.created_at)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPORT: &str = "# 迁移\n\n## 目标\n完成\n\n## 范围\n服务\n\n## 计划步骤\n1. 替换\n\n## 风险与假设\n- 备份\n\n## 验证方法\n测试\n";

    async fn store() -> SqlitePlanReportStore {
        SqlitePlanReportStore::new(SqlitePool::connect("sqlite::memory:").await.unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn editing_creates_an_immutable_next_revision() {
        let store = store().await;
        let created = store
            .create_report("chat", "迁移", REPORT, PlanRevisionAuthor::Agent)
            .await
            .unwrap();
        let edited = store
            .append_revision(
                &created.report.id,
                1,
                "迁移",
                &format!("{REPORT}\n## 未决问题\n无"),
                PlanRevisionAuthor::User,
            )
            .await
            .unwrap();
        assert_eq!(edited.revision.revision, 2);
        assert_eq!(
            store
                .get_detail(&created.report.id, 1)
                .await
                .unwrap()
                .revision
                .markdown,
            REPORT
        );
    }

    #[tokio::test]
    async fn approval_is_hash_bound_and_creates_execution_session() {
        let store = store().await;
        let created = store
            .create_report("chat", "迁移", REPORT, PlanRevisionAuthor::Agent)
            .await
            .unwrap();
        let (approval, session) = store
            .approve_revision(
                &created.report.id,
                1,
                &revision_hash(REPORT),
                ExecutionContextPolicy::Clear,
                None,
            )
            .await
            .unwrap();
        assert_eq!(approval.revision, 1);
        assert_eq!(session.context_policy, ExecutionContextPolicy::Clear);
        store
            .replace_execution_todos(
                &session.id,
                &[ExecutionTodo {
                    id: "todo-1".to_string(),
                    execution_session_id: session.id.clone(),
                    title: "Run verification".to_string(),
                    detail: None,
                    status: ExecutionTodoStatus::Pending,
                    priority: ExecutionTodoPriority::Normal,
                    evidence_ref: None,
                    block_reason: None,
                    updated_at: Utc::now(),
                }],
            )
            .await
            .unwrap();
        assert_eq!(store.execution_todos(&session.id).await.unwrap().len(), 1);
        assert!(store
            .approve_revision(
                &created.report.id,
                1,
                "old",
                ExecutionContextPolicy::Retain,
                None
            )
            .await
            .is_err());
    }

    #[tokio::test]
    async fn execution_todo_updates_are_bound_to_active_sessions() {
        let store = store().await;
        let created = store
            .create_report("chat", "杩佺Щ", REPORT, PlanRevisionAuthor::Agent)
            .await
            .unwrap();
        let (_, session) = store
            .approve_revision(
                &created.report.id,
                1,
                &revision_hash(REPORT),
                ExecutionContextPolicy::Retain,
                None,
            )
            .await
            .unwrap();
        let mut todo = ExecutionTodo {
            id: "todo-1".to_string(),
            execution_session_id: session.id.clone(),
            title: "Run verification".to_string(),
            detail: None,
            status: ExecutionTodoStatus::Pending,
            priority: ExecutionTodoPriority::Normal,
            evidence_ref: None,
            block_reason: None,
            updated_at: Utc::now(),
        };
        store
            .replace_execution_todos(&session.id, &[todo.clone()])
            .await
            .unwrap();

        todo.status = ExecutionTodoStatus::Completed;
        todo.evidence_ref = Some("cargo test".to_string());
        store.update_execution_todo(&todo).await.unwrap();

        let todos = store.execution_todos(&session.id).await.unwrap();
        assert_eq!(todos[0].status, ExecutionTodoStatus::Completed);
        assert_eq!(todos[0].evidence_ref.as_deref(), Some("cargo test"));
    }
}

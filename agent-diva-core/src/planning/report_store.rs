//! SQLite persistence for immutable Markdown plan reports.

use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use super::ids::PlanId;
use super::report::{
    revision_hash, validate_report_markdown, ExecutionContextPolicy, ExecutionSession,
    ExecutionSessionStatus, PlanReport, PlanReportStatus, PlanRevision, PlanRevisionApproval,
    PlanRevisionAuthor,
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
        validate_report_markdown(&row.markdown).map_err(|error| anyhow!(error))?;
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
}

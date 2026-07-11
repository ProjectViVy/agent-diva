//! Immutable Markdown plan-report contracts.
//!
//! A plan report is a review artifact, not a tool-maintained task graph.  The
//! report body is canonical Markdown; revisions are append-only and approval
//! is bound to one exact revision.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::ids::PlanId;

/// The actor that produced a report revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanRevisionAuthor {
    Agent,
    User,
}

/// A durable report identity. Its body lives only in [`PlanRevision`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanReport {
    pub id: PlanId,
    pub session_key: String,
    pub current_revision: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// One immutable Markdown revision of a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanRevision {
    pub report_id: PlanId,
    pub revision: i64,
    pub title: String,
    pub markdown: String,
    pub author: PlanRevisionAuthor,
    pub created_at: DateTime<Utc>,
}

/// The execution-context boundary selected when approving a report revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionContextPolicy {
    Retain,
    Compact,
    Clear,
}

/// Immutable record that starts one execution from one approved revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanRevisionApproval {
    pub report_id: PlanId,
    pub revision: i64,
    pub revision_hash: String,
    pub context_policy: ExecutionContextPolicy,
    pub approved_at: DateTime<Utc>,
}

/// User-visible report validation failures. These are never tool protocol
/// failures: drafts remain editable and only approval is blocked.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlanReportValidationError {
    #[error("计划报告缺少标题")]
    MissingTitle,
    #[error("计划报告缺少章节：{0}")]
    MissingSection(&'static str),
}

const REQUIRED_SECTIONS: [&str; 5] = ["目标", "范围", "计划步骤", "风险与假设", "验证方法"];

/// Validate the canonical report format before approval.
pub fn validate_report_markdown(markdown: &str) -> Result<(), PlanReportValidationError> {
    if !markdown.lines().any(|line| line.trim_start().starts_with("# ")) {
        return Err(PlanReportValidationError::MissingTitle);
    }

    for section in REQUIRED_SECTIONS {
        let heading = format!("## {section}");
        if !markdown.lines().any(|line| line.trim() == heading) {
            return Err(PlanReportValidationError::MissingSection(section));
        }
    }
    Ok(())
}

/// Return a deterministic revision hash for optimistic approval.
pub fn revision_hash(markdown: &str) -> String {
    // The standard library has no stable hashing contract, so use the
    // persisted bytes directly with a simple, deterministic FNV-1a digest.
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in markdown.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPLETE_REPORT: &str = "# 迁移计划\n\n## 目标\n完成迁移\n\n## 范围\n网关\n\n## 计划步骤\n1. 替换\n\n## 风险与假设\n- 有备份\n\n## 验证方法\n运行测试\n";

    #[test]
    fn complete_markdown_is_approvable() {
        assert_eq!(validate_report_markdown(COMPLETE_REPORT), Ok(()));
    }

    #[test]
    fn incomplete_markdown_remains_a_user_visible_validation_error() {
        assert_eq!(
            validate_report_markdown("# Draft\n\n## 目标\nOnly one section"),
            Err(PlanReportValidationError::MissingSection("范围"))
        );
    }

    #[test]
    fn revision_hash_is_stable_and_content_sensitive() {
        assert_eq!(revision_hash(COMPLETE_REPORT), revision_hash(COMPLETE_REPORT));
        assert_ne!(revision_hash(COMPLETE_REPORT), revision_hash("# changed"));
    }
}

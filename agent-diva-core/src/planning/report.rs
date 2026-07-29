//! Immutable Markdown plan-report contracts.
//!
//! A plan report is a review artifact, not a tool-maintained task graph. The
//! report body is canonical Markdown; revisions are append-only and approval
//! is bound to one exact revision.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::ids::PlanId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanRevisionAuthor {
    Agent,
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanReportStatus {
    Draft,
    AwaitingApproval,
    Approved,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanReport {
    pub id: PlanId,
    pub session_key: String,
    pub current_revision: i64,
    pub status: PlanReportStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionSessionStatus {
    Executing,
    Verifying,
    Completed,
    Failed,
    Partial,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionSession {
    pub id: String,
    pub report_id: PlanId,
    pub revision: i64,
    pub context_policy: ExecutionContextPolicy,
    pub status: ExecutionSessionStatus,
    pub compacted_context: Option<String>,
    #[serde(default)]
    pub boundary: Option<ExecutionContextBoundary>,
    #[serde(default)]
    pub initialization_status: ExecutionInitializationStatus,
    #[serde(default)]
    pub initialization_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionContextBoundary {
    pub message_count: usize,
    pub initialized_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionInitializationStatus {
    #[default]
    Pending,
    Ready,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedExecutionContext {
    pub plan_id: PlanId,
    pub revision: i64,
    pub session_key: String,
    pub execution_id: String,
    pub context_policy: ExecutionContextPolicy,
    pub boundary: Option<ExecutionContextBoundary>,
    pub compacted_context: Option<String>,
    pub initialization_status: ExecutionInitializationStatus,
    pub initialization_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionTodo {
    pub id: String,
    pub execution_session_id: String,
    pub title: String,
    pub detail: Option<String>,
    pub status: ExecutionTodoStatus,
    pub priority: ExecutionTodoPriority,
    pub evidence_ref: Option<String>,
    pub block_reason: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionTodoStatus {
    Pending,
    InProgress,
    Blocked,
    Completed,
    Canceled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionTodoPriority {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanRevision {
    pub report_id: PlanId,
    pub revision: i64,
    pub title: String,
    pub markdown: String,
    pub author: PlanRevisionAuthor,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionContextPolicy {
    Retain,
    Compact,
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanRevisionApproval {
    pub report_id: PlanId,
    pub revision: i64,
    pub revision_hash: String,
    pub context_policy: ExecutionContextPolicy,
    pub approved_at: DateTime<Utc>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlanReportValidationError {
    #[error("计划报告缺少标题")]
    MissingTitle,
    #[error("计划报告缺少章节：{0}")]
    MissingSection(&'static str),
    #[error("计划报告正文为空")]
    EmptyBody,
}

/// Result of demuxing a plan-mode assistant response for report persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedPlan {
    /// Markdown body inside `<proposed_plan>` (or freeform candidate).
    pub markdown: String,
    /// True when the body came from explicit `<proposed_plan>` tags.
    pub tagged: bool,
}

const REQUIRED_SECTIONS: [&str; 5] = ["目标", "范围", "计划步骤", "风险与假设", "验证方法"];

const SECTION_ALIASES: &[(&str, &str)] = &[
    ("目标", "目标"),
    ("范围", "范围"),
    ("计划步骤", "计划步骤"),
    ("步骤", "计划步骤"),
    ("实现步骤", "计划步骤"),
    ("风险与假设", "风险与假设"),
    ("风险", "风险与假设"),
    ("假设", "风险与假设"),
    ("验证方法", "验证方法"),
    ("验证", "验证方法"),
    ("测试计划", "验证方法"),
];

const PREAMBLE_PREFIXES: &[&str] = &[
    "好的，以下是正式计划报告：",
    "好的，以下是计划报告：",
    "以下是正式计划报告：",
    "以下是计划报告：",
    "这是正式计划报告：",
    "这是计划报告：",
];

/// Soft completeness check: missing title / sections (does not block emit).
pub fn report_validation_issues(markdown: &str) -> Vec<PlanReportValidationError> {
    let mut issues = Vec::new();
    if !markdown
        .lines()
        .any(|line| line.trim_start().starts_with("# "))
    {
        issues.push(PlanReportValidationError::MissingTitle);
    }
    for section in REQUIRED_SECTIONS {
        let heading = format!("## {section}");
        if !markdown.lines().any(|line| line.trim() == heading) {
            issues.push(PlanReportValidationError::MissingSection(section));
        }
    }
    issues
}

/// Full section-completeness check retained for callers that want a hard Result.
pub fn validate_report_markdown(markdown: &str) -> Result<(), PlanReportValidationError> {
    match report_validation_issues(markdown).into_iter().next() {
        Some(issue) => Err(issue),
        None => Ok(()),
    }
}

/// Minimum gate for approval: H1 title plus non-empty body after the title.
pub fn assert_report_ready_for_approval(markdown: &str) -> Result<(), PlanReportValidationError> {
    let mut lines = markdown.lines();
    let has_title = lines.any(|line| line.trim_start().starts_with("# "));
    if !has_title {
        return Err(PlanReportValidationError::MissingTitle);
    }
    let body_non_empty = markdown
        .lines()
        .skip_while(|line| !line.trim_start().starts_with("# "))
        .skip(1)
        .any(|line| !line.trim().is_empty());
    if !body_non_empty {
        return Err(PlanReportValidationError::EmptyBody);
    }
    Ok(())
}

/// Extract a Codex-style `<proposed_plan>` block (line-oriented tags).
///
/// Opening and closing tags must be alone on their lines. Unclosed or missing
/// tags return `None`.
pub fn extract_proposed_plan(text: &str) -> Option<ExtractedPlan> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines
        .iter()
        .position(|line| line.trim() == "<proposed_plan>")?;
    let end_rel = lines[start + 1..]
        .iter()
        .position(|line| line.trim() == "</proposed_plan>")?;
    let end = start + 1 + end_rel;
    let inner = lines[start + 1..end].join("\n");
    if inner.trim().is_empty() {
        return None;
    }
    Some(ExtractedPlan {
        markdown: inner,
        tagged: true,
    })
}

/// Strip the first complete `<proposed_plan>…</proposed_plan>` block from text.
/// Returns the original text when no complete block is present.
pub fn strip_proposed_plan_block(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines
        .iter()
        .position(|line| line.trim() == "<proposed_plan>")
    else {
        return text.to_string();
    };
    let Some(end_rel) = lines[start + 1..]
        .iter()
        .position(|line| line.trim() == "</proposed_plan>")
    else {
        return text.to_string();
    };
    let end = start + 1 + end_rel;
    let mut kept: Vec<&str> = Vec::with_capacity(lines.len());
    kept.extend_from_slice(&lines[..start]);
    if end + 1 < lines.len() {
        kept.extend_from_slice(&lines[end + 1..]);
    }
    let stripped = kept.join("\n");
    // Collapse blank runs introduced by tag removal to a single empty line.
    let mut out = String::new();
    let mut blank_run = 0usize;
    for line in stripped.lines() {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(line);
    }
    out.trim().to_string()
}

/// Freeform fallback: true when the reply looks like a plan report candidate.
///
/// Requires at least two recognizable section labels (with or without `##`), or
/// a plan-like title signal plus one section label.
pub fn looks_like_plan_report(text: &str) -> bool {
    let mut section_hits = 0usize;
    let mut seen_canonical = std::collections::BTreeSet::new();
    for line in text.lines() {
        if let Some(canonical) = match_section_label(line) {
            if seen_canonical.insert(canonical) {
                section_hits += 1;
            }
        }
    }
    if section_hits >= 2 {
        return true;
    }
    let title_signal = text.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("# ")
            || trimmed == "计划报告"
            || trimmed.starts_with("计划报告")
            || trimmed.contains("正式计划报告")
    });
    title_signal && section_hits >= 1
}

fn strip_replacement_chars(text: &str) -> String {
    text.chars().filter(|ch| *ch != '\u{FFFD}').collect()
}

/// Best-effort normalization so freeform model output becomes approvable Markdown.
pub fn normalize_report_markdown(text: &str) -> String {
    // Drop UTF-8 replacement characters early so titles do not become "计��报告".
    let cleaned = strip_replacement_chars(text);
    let mut lines: Vec<String> = cleaned
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect();

    // Drop common assistant preambles.
    while lines
        .first()
        .is_some_and(|line| PREAMBLE_PREFIXES.iter().any(|p| line.trim() == *p))
    {
        lines.remove(0);
        while lines.first().is_some_and(|line| line.trim().is_empty()) {
            lines.remove(0);
        }
    }

    // Promote bare / bold section labels to `## {canonical}`.
    for line in &mut lines {
        if let Some(canonical) = match_section_label(line) {
            *line = format!("## {canonical}");
        }
    }

    // Ensure an H1 title exists.
    let has_h1 = lines
        .iter()
        .any(|line| line.trim_start().starts_with("# ") && !line.trim_start().starts_with("## "));
    if !has_h1 {
        let mut title = lines
            .iter()
            .find(|line| {
                let t = line.trim();
                !t.is_empty() && !t.starts_with("## ")
            })
            .map(|line| line.trim().trim_start_matches('#').trim().to_string())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "计划报告".to_string());
        // Recover obviously broken titles after replacement-char stripping.
        if title.contains('报') && title.chars().count() < 4 {
            title = "计划报告".to_string();
        }
        // If the first content line became the title source and is not a heading, replace it.
        if let Some(first_idx) = lines.iter().position(|line| !line.trim().is_empty()) {
            let first = lines[first_idx].trim();
            if !first.starts_with("## ") && !first.starts_with("# ") {
                lines[first_idx] = format!("# {title}");
            } else {
                lines.insert(first_idx, format!("# {title}"));
            }
        } else {
            lines.push(format!("# {title}"));
        }
    } else {
        // Clean existing H1 lines that contain replacement residue.
        for line in &mut lines {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("# ") {
                if !trimmed.starts_with("## ") {
                    let cleaned_title = strip_replacement_chars(rest).trim().to_string();
                    let title = if cleaned_title.contains('报') && cleaned_title.chars().count() < 4
                    {
                        "计划报告".to_string()
                    } else if cleaned_title.is_empty() {
                        "计划报告".to_string()
                    } else {
                        cleaned_title
                    };
                    *line = format!("# {title}");
                    break;
                }
            }
        }
    }

    // Collapse leading blank lines.
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }

    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Resolve a freeform plan body from plan-mode final content.
///
/// Prefer tagged `<proposed_plan>`; otherwise accept freeform that looks like a plan.
pub fn resolve_plan_report_body(text: &str) -> Option<ExtractedPlan> {
    if let Some(extracted) = extract_proposed_plan(text) {
        return Some(extracted);
    }
    if looks_like_plan_report(text) {
        return Some(ExtractedPlan {
            markdown: text.to_string(),
            tagged: false,
        });
    }
    None
}

fn match_section_label(line: &str) -> Option<&'static str> {
    let mut trimmed = line.trim();
    if trimmed.starts_with("## ") {
        trimmed = trimmed[3..].trim();
    } else if trimmed.starts_with("# ") {
        return None;
    }
    // Strip simple bold wrappers: **目标** or __目标__
    if (trimmed.starts_with("**") && trimmed.ends_with("**") && trimmed.len() > 4)
        || (trimmed.starts_with("__") && trimmed.ends_with("__") && trimmed.len() > 4)
    {
        trimmed = &trimmed[2..trimmed.len() - 2];
        trimmed = trimmed.trim();
    }
    // Strip trailing colon.
    let trimmed = trimmed.trim_end_matches([':', '：']).trim();
    for (alias, canonical) in SECTION_ALIASES {
        if trimmed == *alias {
            // Return the static str of the canonical name from REQUIRED_SECTIONS.
            return REQUIRED_SECTIONS.iter().copied().find(|s| *s == *canonical);
        }
    }
    None
}

pub fn revision_hash(markdown: &str) -> String {
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
        assert_eq!(assert_report_ready_for_approval(COMPLETE_REPORT), Ok(()));
        assert!(report_validation_issues(COMPLETE_REPORT).is_empty());
    }

    #[test]
    fn incomplete_markdown_reports_soft_issues_not_blocking_min_gate() {
        let draft = "# Draft\n\n## 目标\nOnly one section\n";
        assert_eq!(
            validate_report_markdown(draft),
            Err(PlanReportValidationError::MissingSection("范围"))
        );
        assert_eq!(assert_report_ready_for_approval(draft), Ok(()));
        assert!(report_validation_issues(draft)
            .iter()
            .any(|i| matches!(i, PlanReportValidationError::MissingSection("范围"))));
    }

    #[test]
    fn min_gate_rejects_title_only() {
        assert_eq!(
            assert_report_ready_for_approval("# Only title\n"),
            Err(PlanReportValidationError::EmptyBody)
        );
    }

    #[test]
    fn extract_proposed_plan_is_line_oriented() {
        let text = "前言\n\n<proposed_plan>\n# 标题\n\n## 目标\n做点事\n</proposed_plan>\n\n后记";
        let extracted = extract_proposed_plan(text).expect("tagged plan");
        assert!(extracted.tagged);
        assert!(extracted.markdown.contains("# 标题"));
        assert!(!extracted.markdown.contains("前言"));
        assert_eq!(strip_proposed_plan_block(text).trim(), "前言\n\n后记");
    }

    #[test]
    fn extract_ignores_unclosed_tags() {
        assert!(extract_proposed_plan("<proposed_plan>\n# x\n").is_none());
    }

    #[test]
    fn freeform_user_style_plan_looks_like_report_and_normalizes() {
        let freeform = "好的，以下是正式计划报告：\n\n计划报告\n目标\n在 cpp/ 目录下创建项目\n\n范围\nCMake + GTest\n\n计划步骤\n1. 搭脚手架\n\n风险与假设\n- 网络可用\n\n验证方法\n运行测试\n";
        assert!(looks_like_plan_report(freeform));
        let normalized = normalize_report_markdown(freeform);
        assert!(normalized.lines().any(|l| l.trim_start().starts_with("# ")));
        assert!(normalized.lines().any(|l| l.trim() == "## 目标"));
        assert!(normalized.lines().any(|l| l.trim() == "## 范围"));
        assert_eq!(assert_report_ready_for_approval(&normalized), Ok(()));
        assert!(validate_report_markdown(&normalized).is_ok());
    }

    #[test]
    fn normalize_repairs_replacement_char_titles() {
        let broken = "# 计\u{FFFD}\u{FFFD}报告\n\n## 目标\n做点事\n\n## 范围\nx\n\n## 计划步骤\n1\n\n## 风险与假设\nr\n\n## 验证方法\nv\n";
        let normalized = normalize_report_markdown(broken);
        let title = normalized
            .lines()
            .find_map(|line| line.trim().strip_prefix("# "))
            .unwrap_or("");
        assert!(!title.contains('\u{FFFD}'));
        assert_eq!(title, "计划报告");
    }

    #[test]
    fn exploration_chatter_is_not_a_plan() {
        assert!(!looks_like_plan_report("我先读一下 src/main.rs 的结构。"));
        assert!(resolve_plan_report_body("我先读一下 src/main.rs 的结构。").is_none());
    }

    #[test]
    fn revision_hash_is_stable_and_content_sensitive() {
        assert_eq!(
            revision_hash(COMPLETE_REPORT),
            revision_hash(COMPLETE_REPORT)
        );
        assert_ne!(revision_hash(COMPLETE_REPORT), revision_hash("# changed"));
    }

    /// GUI used to `.trim()` plan bodies for display; that drops the trailing
    /// newline always added by `normalize_report_markdown` and breaks approve.
    #[test]
    fn revision_hash_survives_display_trim_when_renormalized() {
        let stored = normalize_report_markdown(COMPLETE_REPORT);
        let display_trimmed = stored.trim();
        assert_ne!(
            revision_hash(&stored),
            revision_hash(display_trimmed),
            "trim alone must change the hash (documents the bug)"
        );
        assert_eq!(
            revision_hash(&stored),
            revision_hash(&normalize_report_markdown(display_trimmed)),
            "desktop approve path re-normalizes before hashing"
        );
    }
}

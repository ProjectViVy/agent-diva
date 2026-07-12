use agent_diva_core::evolution::{
    EvidenceRef, EvidenceSource, EvolutionProposal, LaputaSectionName, ProposalState, ProposalType,
    RiskLevel,
};
use agent_diva_core::session::{
    SessionManager, SessionSearchHit, SessionSearchQuery, SessionSearchResponse,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_RENDER_LINES: usize = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotebookPeriod {
    Daily,
    Weekly,
    Monthly,
}

impl NotebookPeriod {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim() {
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            "monthly" => Ok(Self::Monthly),
            other => Err(format!("unsupported notebook period: {other}")),
        }
    }

    fn report_dir(self, workspace: &Path) -> PathBuf {
        match self {
            Self::Daily => workspace.join(".agent-diva/autodream/reports/daily"),
            Self::Weekly => workspace.join(".agent-diva/autodream/reports/weekly"),
            Self::Monthly => workspace.join("reports/monthly"),
        }
    }

    fn id_prefix(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotebookReportDto {
    pub id: String,
    pub period: String,
    pub date: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub generated_at: Option<String>,
    pub generated_by: Option<String>,
    pub schema_version: Option<String>,
    /// `llm_curated` or `deterministic_fallback` when present.
    pub generation_mode: Option<String>,
    /// `complete`, `partial`, or `fallback` when present.
    pub coverage_status: Option<String>,
    pub source_path: String,
    pub is_truncated: bool,
    pub original_line_count: usize,
    pub displayed_line_count: usize,
}

#[derive(Debug, Default, Deserialize)]
struct ReportFrontmatter {
    period: Option<String>,
    date: Option<String>,
    week: Option<String>,
    month: Option<String>,
    generated_at: Option<String>,
    generated_by: Option<String>,
    source: Option<String>,
    session_count: Option<u64>,
    token_used: Option<u64>,
    schema_version: Option<serde_yaml::Value>,
    generation_mode: Option<String>,
    coverage_status: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotebookProposalAction {
    Sop,
    Skill,
    Memory,
}

impl NotebookProposalAction {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim() {
            "sop" => Ok(Self::Sop),
            "skill" => Ok(Self::Skill),
            "memory" => Ok(Self::Memory),
            other => Err(format!("unsupported notebook proposal action: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotebookProposalPreviewDto {
    pub action: String,
    pub proposal_type: ProposalType,
    pub target_section: LaputaSectionName,
    pub extracted_summary: String,
    pub evidence_refs: Vec<EvidenceRef>,
    pub risk_level: RiskLevel,
    pub review_status: ProposalState,
    pub proposed_patch: String,
    pub needs_attention_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotebookSessionSearchRequest {
    pub query: String,
    pub max_files_scanned: Option<usize>,
    pub max_results: Option<usize>,
    pub max_snippet_chars: Option<usize>,
    pub max_total_bytes: Option<usize>,
}

pub fn load_notebook_reports(
    workspace: &Path,
    period: NotebookPeriod,
) -> Result<Vec<NotebookReportDto>, String> {
    let dir = period.report_dir(workspace);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut reports = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|error| {
        format!(
            "failed to read notebook report directory {}: {error}",
            dir.display()
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to read directory entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("md") {
            continue;
        }

        match parse_report_file(workspace, &path, period) {
            Ok(report) => reports.push(report),
            Err(error) => {
                tracing::warn!(
                    report_path = %path.display(),
                    report_period = %period.id_prefix(),
                    "skipping malformed notebook report: {error}"
                );
            }
        }
    }

    reports.sort_by(|left, right| {
        right
            .date
            .cmp(&left.date)
            .then_with(|| right.id.cmp(&left.id))
    });
    Ok(reports)
}

pub fn build_notebook_report_proposal(
    workspace: &Path,
    report_id: &str,
    action: NotebookProposalAction,
    created_by: &str,
    session_hits: Option<Vec<SessionSearchHit>>,
) -> Result<EvolutionProposal, String> {
    let report = find_notebook_report(workspace, report_id)?;
    let preview = build_notebook_proposal_preview_for_report(&report, action, session_hits)?;
    let now = Utc::now();
    let timestamp = now
        .timestamp_nanos_opt()
        .unwrap_or_else(|| now.timestamp_millis() * 1_000_000);
    let id = format!(
        "notebook-{}-{}-{}",
        preview.action,
        sanitize_id_component(&report.date),
        timestamp
    );
    Ok(EvolutionProposal {
        id,
        created_at: now,
        updated_at: now,
        created_by: created_by.to_string(),
        proposal_type: preview.proposal_type,
        target_section: preview.target_section,
        evidence_refs: preview.evidence_refs,
        proposed_patch: preview.proposed_patch,
        risk_level: preview.risk_level,
        state: preview.review_status,
        source_run_id: None,
    })
}

pub fn build_notebook_report_proposal_preview(
    workspace: &Path,
    report_id: &str,
    action: NotebookProposalAction,
    session_hits: Option<Vec<SessionSearchHit>>,
) -> Result<NotebookProposalPreviewDto, String> {
    let report = find_notebook_report(workspace, report_id)?;
    build_notebook_proposal_preview_for_report(&report, action, session_hits)
}

pub fn search_notebook_session_evidence(
    workspace: &Path,
    request: NotebookSessionSearchRequest,
) -> Result<SessionSearchResponse, String> {
    let mut query = SessionSearchQuery::new(request.query);
    if let Some(value) = request.max_files_scanned {
        query.max_files_scanned = value;
    }
    if let Some(value) = request.max_results {
        query.max_results = value;
    }
    if let Some(value) = request.max_snippet_chars {
        query.max_snippet_chars = value;
    }
    if let Some(value) = request.max_total_bytes {
        query.max_total_bytes = value;
    }
    SessionManager::new(workspace)
        .search(query)
        .map_err(|error| format!("failed to search session evidence: {error}"))
}

#[allow(dead_code)]
fn parse_report_file(
    workspace: &Path,
    path: &Path,
    period: NotebookPeriod,
) -> Result<NotebookReportDto, String> {
    let markdown = fs::read_to_string(path)
        .map_err(|error| format!("failed to read notebook report {}: {error}", path.display()))?;
    let (frontmatter, body) = split_frontmatter(&markdown)?;
    let title = extract_title(body).unwrap_or_else(|| {
        path.file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("Untitled report")
            .to_string()
    });
    let summary = extract_summary(body).unwrap_or_else(|| title.clone());
    let (content, original_line_count, displayed_line_count, is_truncated) =
        truncate_body(body, MAX_RENDER_LINES);

    let date = match period {
        NotebookPeriod::Daily => frontmatter
            .date
            .clone()
            .unwrap_or_else(|| fallback_file_stem(path)),
        NotebookPeriod::Weekly => frontmatter
            .week
            .clone()
            .or(frontmatter.date.clone())
            .unwrap_or_else(|| fallback_file_stem(path)),
        NotebookPeriod::Monthly => frontmatter
            .month
            .clone()
            .or(frontmatter.date.clone())
            .unwrap_or_else(|| fallback_file_stem(path)),
    };
    let schema_version = frontmatter
        .schema_version
        .as_ref()
        .map(schema_version_to_string);
    let source_path = path
        .strip_prefix(workspace)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    let _ = (
        frontmatter.source.as_deref(),
        frontmatter.session_count,
        frontmatter.token_used,
    );

    Ok(NotebookReportDto {
        id: format!("{}:{}", period.id_prefix(), date),
        period: frontmatter
            .period
            .clone()
            .unwrap_or_else(|| period.id_prefix().to_string()),
        date,
        title,
        summary,
        content,
        generated_at: frontmatter.generated_at.clone(),
        generated_by: frontmatter.generated_by.clone(),
        schema_version,
        generation_mode: frontmatter.generation_mode.clone(),
        coverage_status: frontmatter.coverage_status.clone(),
        source_path,
        is_truncated,
        original_line_count,
        displayed_line_count,
    })
}

fn find_notebook_report(workspace: &Path, report_id: &str) -> Result<NotebookReportDto, String> {
    let Some((period, _)) = report_id.split_once(':') else {
        return Err(format!("invalid notebook report id: {report_id}"));
    };
    let period = NotebookPeriod::parse(period)?;
    load_notebook_reports(workspace, period)?
        .into_iter()
        .find(|report| report.id == report_id)
        .ok_or_else(|| format!("notebook report not found: {report_id}"))
}

fn build_notebook_proposal_preview_for_report(
    report: &NotebookReportDto,
    action: NotebookProposalAction,
    session_hits: Option<Vec<SessionSearchHit>>,
) -> Result<NotebookProposalPreviewDto, String> {
    let extracted_summary = bounded_summary(report);
    let mut evidence_refs = vec![EvidenceRef {
        id: format!("evidence-{}", sanitize_id_component(&report.id)),
        source: EvidenceSource::Report,
        uri: format!("report://{}", report.source_path),
        excerpt: Some(extracted_summary.clone()),
        hash: None,
        created_at: Utc::now(),
    }];
    if let Some(session_hits) = session_hits {
        evidence_refs.extend(session_hits.into_iter().map(|hit| hit.to_evidence_ref()));
    }
    let (proposal_type, risk_level, review_status, needs_attention_reason) =
        classify_notebook_action(action, report);
    let target_section = proposal_type.target_section();
    let proposed_patch = render_notebook_proposed_patch(
        report,
        action,
        &proposal_type,
        &target_section,
        &extracted_summary,
        needs_attention_reason.as_deref(),
    )?;

    Ok(NotebookProposalPreviewDto {
        action: action_name(action).to_string(),
        proposal_type,
        target_section,
        extracted_summary,
        evidence_refs,
        risk_level,
        review_status,
        proposed_patch,
        needs_attention_reason,
    })
}

fn classify_notebook_action(
    action: NotebookProposalAction,
    report: &NotebookReportDto,
) -> (ProposalType, RiskLevel, ProposalState, Option<String>) {
    match action {
        NotebookProposalAction::Sop | NotebookProposalAction::Skill => (
            ProposalType::SopCreate,
            RiskLevel::Medium,
            ProposalState::PendingReview,
            None,
        ),
        NotebookProposalAction::Memory => classify_memory_report(report),
    }
}

fn classify_memory_report(
    report: &NotebookReportDto,
) -> (ProposalType, RiskLevel, ProposalState, Option<String>) {
    let content = report.content.to_lowercase();
    if content.contains("relationship") || content.contains("关系") {
        return (
            ProposalType::RelationshipUpdate,
            RiskLevel::High,
            ProposalState::PendingReview,
            None,
        );
    }
    if content.contains("identity") || content.contains("身份") {
        return (
            ProposalType::IdentityPatch,
            RiskLevel::High,
            ProposalState::PendingReview,
            None,
        );
    }
    if content.contains("preference") || content.contains("偏好") || content.contains("learned") {
        return (
            ProposalType::LearningNote,
            RiskLevel::Medium,
            ProposalState::PendingReview,
            None,
        );
    }
    if content.contains("memory") || content.contains("记忆") || content.contains("follow-up") {
        return (
            ProposalType::MemoryPatch,
            RiskLevel::Medium,
            ProposalState::PendingReview,
            None,
        );
    }

    (
        ProposalType::MemoryPatch,
        RiskLevel::High,
        ProposalState::NeedsAttention,
        Some("Report content does not clearly identify the durable memory target.".to_string()),
    )
}

fn render_notebook_proposed_patch(
    report: &NotebookReportDto,
    action: NotebookProposalAction,
    proposal_type: &ProposalType,
    target_section: &LaputaSectionName,
    extracted_summary: &str,
    needs_attention_reason: Option<&str>,
) -> Result<String, String> {
    let patch = serde_json::json!({
        "source": "notebook_report",
        "report_id": report.id,
        "report_title": report.title,
        "report_period": report.period,
        "report_date": report.date,
        "action": action_name(action),
        "sub_target": match action {
            NotebookProposalAction::Sop => "sop",
            NotebookProposalAction::Skill => "skill",
            NotebookProposalAction::Memory => "memory",
        },
        "proposal_type": proposal_type,
        "target_section": target_section,
        "summary": extracted_summary,
        "review_status": needs_attention_reason.map_or("pending_review", |_| "needs_attention"),
        "needs_attention_reason": needs_attention_reason,
        "review_notes": [
            "Created from Notebook report action.",
            "Authority files must change only after Laputa approval and apply."
        ],
    });
    serde_json::to_string_pretty(&patch)
        .map_err(|error| format!("failed to render notebook proposal patch: {error}"))
}

fn bounded_summary(report: &NotebookReportDto) -> String {
    let source = if report.summary.trim().is_empty() {
        report.content.trim()
    } else {
        report.summary.trim()
    };
    truncate_chars(source, 600)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut iter = value.chars();
    let truncated: String = iter.by_ref().take(max_chars).collect();
    if iter.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn action_name(action: NotebookProposalAction) -> &'static str {
    match action {
        NotebookProposalAction::Sop => "sop",
        NotebookProposalAction::Skill => "skill",
        NotebookProposalAction::Memory => "memory",
    }
}

fn sanitize_id_component(value: &str) -> String {
    let mut sanitized = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            sanitized.push(ch);
        } else {
            sanitized.push('-');
        }
    }
    sanitized.trim_matches('-').to_string()
}

fn split_frontmatter(markdown: &str) -> Result<(ReportFrontmatter, &str), String> {
    if !markdown.starts_with("---\n") {
        return Ok((ReportFrontmatter::default(), markdown));
    }

    let rest = &markdown[4..];
    let Some(end) = rest.find("\n---\n") else {
        return Err("invalid notebook report frontmatter fence".to_string());
    };
    let yaml = &rest[..end];
    let body = &rest[end + 5..];
    let frontmatter: ReportFrontmatter = serde_yaml::from_str(yaml)
        .map_err(|error| format!("invalid report frontmatter: {error}"))?;
    Ok((frontmatter, body))
}

fn extract_title(body: &str) -> Option<String> {
    body.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_summary(body: &str) -> Option<String> {
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .find(|line| !line.starts_with('#') && !line.starts_with("```"))
        .map(ToOwned::to_owned)
}

fn truncate_body(body: &str, max_lines: usize) -> (String, usize, usize, bool) {
    let lines: Vec<&str> = body.lines().collect();
    let original_line_count = lines.len();
    if original_line_count <= max_lines {
        return (
            body.to_string(),
            original_line_count,
            original_line_count,
            false,
        );
    }

    let truncated = lines[..max_lines].join(
        "
",
    );
    (truncated, original_line_count, max_lines, true)
}

fn fallback_file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn schema_version_to_string(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(value) => value.clone(),
        serde_yaml::Value::Number(value) => value.to_string(),
        other => serde_yaml::to_string(other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_notebook_report_proposal, build_notebook_report_proposal_preview,
        load_notebook_reports, search_notebook_session_evidence, NotebookPeriod,
        NotebookProposalAction, NotebookSessionSearchRequest, MAX_RENDER_LINES,
    };
    use agent_diva_core::evolution::{
        EvidenceSource, LaputaSectionName, ProposalState, ProposalType,
    };
    use agent_diva_core::session::SessionSearchHit;
    use std::fs;

    fn write_report(path: &std::path::Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, body).unwrap();
    }

    #[test]
    fn loads_daily_reports_from_autodream_contract_path() {
        let temp = tempfile::tempdir().unwrap();
        write_report(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/daily/2026-06-14.md"),
            "---
period: daily
date: 2026-06-14
generated_at: 2026-06-14T10:00:00Z
generated_by: agent-diva-autodream
schema_version: 1
---

# Daily Reflection

Summary paragraph.

## Details

- item
",
        );

        let reports = load_notebook_reports(temp.path(), NotebookPeriod::Daily).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].date, "2026-06-14");
        assert_eq!(reports[0].title, "Daily Reflection");
        assert_eq!(reports[0].summary, "Summary paragraph.");
        assert_eq!(
            reports[0].generated_by.as_deref(),
            Some("agent-diva-autodream")
        );
        assert_eq!(
            reports[0].source_path.replace('\\', "/"),
            ".agent-diva/autodream/reports/daily/2026-06-14.md"
        );
    }

    #[test]
    fn loads_weekly_reports_and_sorts_latest_first() {
        let temp = tempfile::tempdir().unwrap();
        write_report(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/weekly/2026-W23.md"),
            "---
period: weekly
week: 2026-W23
---

# Week 23

Older.
",
        );
        write_report(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/weekly/2026-W24.md"),
            "---
period: weekly
week: 2026-W24
---

# Week 24

Newer.
",
        );

        let reports = load_notebook_reports(temp.path(), NotebookPeriod::Weekly).unwrap();
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].date, "2026-W24");
        assert_eq!(reports[1].date, "2026-W23");
    }

    #[test]
    fn keeps_monthly_reports_isolated_under_report_owned_storage() {
        let temp = tempfile::tempdir().unwrap();
        write_report(
            &temp.path().join("reports/monthly/2026-06.md"),
            "---
period: monthly
month: 2026-06
---

# June Report

Monthly summary.
",
        );
        write_report(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/monthly/2026-06.md"),
            "---
period: monthly
month: 2026-06
---

# Wrong Path

Should not be read.
",
        );

        let reports = load_notebook_reports(temp.path(), NotebookPeriod::Monthly).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].title, "June Report");
        assert_eq!(
            reports[0].source_path.replace('\\', "/"),
            "reports/monthly/2026-06.md"
        );
    }

    #[test]
    fn returns_empty_when_report_directory_is_missing() {
        let temp = tempfile::tempdir().unwrap();
        let reports = load_notebook_reports(temp.path(), NotebookPeriod::Daily).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn skips_malformed_reports_and_keeps_valid_entries_visible() {
        let temp = tempfile::tempdir().unwrap();
        write_report(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/daily/2026-06-14.md"),
            "---
period: daily
date: 2026-06-14
---

# Valid Report

Summary paragraph.
",
        );
        write_report(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/daily/2026-06-15.md"),
            "---
period: daily
date: [broken
---

# Broken Report
",
        );

        let reports = load_notebook_reports(temp.path(), NotebookPeriod::Daily).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].id, "daily:2026-06-14");
    }

    #[test]
    fn truncates_large_markdown_before_returning_to_renderer() {
        let temp = tempfile::tempdir().unwrap();
        let mut content = String::from(
            "---
period: daily
date: 2026-06-14
---

# Huge Report

",
        );
        for index in 0..(MAX_RENDER_LINES + 10) {
            content.push_str(&format!(
                "line-{index}
"
            ));
        }
        write_report(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/daily/2026-06-14.md"),
            &content,
        );

        let reports = load_notebook_reports(temp.path(), NotebookPeriod::Daily).unwrap();
        assert_eq!(reports.len(), 1);
        assert!(reports[0].is_truncated);
        assert_eq!(reports[0].displayed_line_count, MAX_RENDER_LINES);
        assert_eq!(reports[0].original_line_count, MAX_RENDER_LINES + 13);
        assert!(!reports[0]
            .content
            .contains(&format!("line-{}", MAX_RENDER_LINES + 5)));
    }

    #[test]
    fn loads_monthly_report_with_generation_mode_metadata() {
        let temp = tempfile::tempdir().unwrap();
        write_report(
            &temp.path().join("reports/monthly/2026-06.md"),
            "---
period: monthly
month: 2026-06
generated_by: agent-diva-report-system
generation_mode: deterministic_fallback
coverage_status: fallback
schema_version: 1
---

# Monthly Report 2026-06

聚合 1 份日报（确定性降级）。
",
        );

        let reports = load_notebook_reports(temp.path(), NotebookPeriod::Monthly).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].date, "2026-06");
        assert_eq!(
            reports[0].generation_mode.as_deref(),
            Some("deterministic_fallback")
        );
        assert_eq!(reports[0].coverage_status.as_deref(), Some("fallback"));
        assert_eq!(
            reports[0].generated_by.as_deref(),
            Some("agent-diva-report-system")
        );
    }

    #[test]
    fn sop_report_action_creates_sop_proposal_targeting_identity() {
        let temp = tempfile::tempdir().unwrap();
        write_report(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/daily/2026-06-14.md"),
            "---
period: daily
date: 2026-06-14
---

# SOP Candidate

Standard operating procedure for reviewing reports.
",
        );

        let proposal = build_notebook_report_proposal(
            temp.path(),
            "daily:2026-06-14",
            NotebookProposalAction::Sop,
            "notebook",
            None,
        )
        .unwrap();

        assert_eq!(proposal.proposal_type, ProposalType::SopCreate);
        assert_eq!(proposal.target_section, LaputaSectionName::Identity);
        assert_eq!(proposal.state, ProposalState::PendingReview);
        assert!(proposal.proposed_patch.contains("\"sub_target\": \"sop\""));
        assert_eq!(proposal.evidence_refs[0].source, EvidenceSource::Report);
        assert_eq!(proposal.source_run_id, None);
    }

    #[test]
    fn skill_report_action_marks_skill_sub_target_without_direct_write() {
        let temp = tempfile::tempdir().unwrap();
        write_report(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/weekly/2026-W24.md"),
            "---
period: weekly
week: 2026-W24
---

# Skill Candidate

Reusable behavior should be captured as a skill.
",
        );

        let proposal = build_notebook_report_proposal(
            temp.path(),
            "weekly:2026-W24",
            NotebookProposalAction::Skill,
            "notebook",
            None,
        )
        .unwrap();

        assert_eq!(proposal.proposal_type, ProposalType::SopCreate);
        assert_eq!(proposal.target_section, LaputaSectionName::Identity);
        assert!(proposal
            .proposed_patch
            .contains("\"sub_target\": \"skill\""));
        assert!(!temp.path().join("SOUL.md").exists());
        assert!(!temp.path().join(".agents/skills/Skill Candidate").exists());
    }

    #[test]
    fn memory_report_action_routes_clear_memory_content() {
        let temp = tempfile::tempdir().unwrap();
        write_report(
            &temp.path().join("reports/monthly/2026-06.md"),
            "---
period: monthly
month: 2026-06
---

# June Report

Memory: the user prefers concise implementation reports.
",
        );

        let proposal = build_notebook_report_proposal(
            temp.path(),
            "monthly:2026-06",
            NotebookProposalAction::Memory,
            "notebook",
            None,
        )
        .unwrap();

        assert_eq!(proposal.proposal_type, ProposalType::MemoryPatch);
        assert_eq!(proposal.target_section, LaputaSectionName::MemoryMd);
        assert_eq!(proposal.state, ProposalState::PendingReview);
        assert!(!temp.path().join("memory/MEMORY.md").exists());
    }

    #[test]
    fn ambiguous_memory_report_preview_returns_needs_attention() {
        let temp = tempfile::tempdir().unwrap();
        write_report(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/daily/2026-06-15.md"),
            "---
period: daily
date: 2026-06-15
---

# Daily Report

Several unrelated tasks were discussed.
",
        );

        let preview = build_notebook_report_proposal_preview(
            temp.path(),
            "daily:2026-06-15",
            NotebookProposalAction::Memory,
            None,
        )
        .unwrap();

        assert_eq!(preview.proposal_type, ProposalType::MemoryPatch);
        assert_eq!(preview.review_status, ProposalState::NeedsAttention);
        assert!(preview.needs_attention_reason.is_some());
    }

    #[test]
    fn proposal_build_does_not_mutate_any_authority_paths_before_apply() {
        let temp = tempfile::tempdir().unwrap();
        write_report(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/daily/2026-06-16.md"),
            "---
period: daily
date: 2026-06-16
---

# Governance Report

Memory: persist the concise report preference.
",
        );

        let _preview = build_notebook_report_proposal_preview(
            temp.path(),
            "daily:2026-06-16",
            NotebookProposalAction::Memory,
            None,
        )
        .unwrap();
        let _proposal = build_notebook_report_proposal(
            temp.path(),
            "daily:2026-06-16",
            NotebookProposalAction::Memory,
            "notebook",
            None,
        )
        .unwrap();

        let forbidden = [
            temp.path().join("SOUL.md"),
            temp.path().join("IDENTITY.md"),
            temp.path().join("memory").join("MEMORY.md"),
            temp.path().join("memory").join("HISTORY.md"),
            temp.path()
                .join(".laputa")
                .join("sections")
                .join("identity.json"),
            temp.path()
                .join(".laputa")
                .join("sections")
                .join("memory_md.json"),
        ];

        for path in forbidden {
            assert!(
                !path.exists(),
                "Notebook proposal preparation must not write authority state before apply: {}",
                path.display()
            );
        }
    }

    #[test]
    fn search_session_evidence_returns_hits_and_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let sessions_dir = temp.path().join("sessions");
        fs::create_dir_all(&sessions_dir).unwrap();
        fs::write(
            sessions_dir.join("telegram_7.jsonl"),
            r#"{"_type":"metadata","created_at":"2026-06-16T00:00:00Z","updated_at":"2026-06-16T00:00:00Z","metadata":{}}
{"role":"user","content":"Launch discussion captured here.","timestamp":"2026-06-16T01:02:03Z"}"#,
        )
        .unwrap();
        fs::write(sessions_dir.join("bad.jsonl"), "{").unwrap();

        let result = search_notebook_session_evidence(
            temp.path(),
            NotebookSessionSearchRequest {
                query: "launch".to_string(),
                max_files_scanned: None,
                max_results: Some(5),
                max_snippet_chars: Some(120),
                max_total_bytes: Some(2048),
            },
        )
        .unwrap();

        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].source, EvidenceSource::Session);
        assert_eq!(result.diagnostics.len(), 1);
    }

    #[test]
    fn notebook_proposal_preview_can_attach_session_evidence_without_direct_write() {
        let temp = tempfile::tempdir().unwrap();
        write_report(
            &temp.path().join("reports/monthly/2026-06.md"),
            "---
period: monthly
month: 2026-06
---

# June Report

Memory: summarize the launch preference.
",
        );
        let session_hit = SessionSearchHit {
            session_id: "telegram:9".to_string(),
            timestamp: "2026-06-16T01:02:03Z".parse().unwrap(),
            snippet: "launch preference confirmed in session".to_string(),
            source_uri: "session://telegram%3A9?message_index=2".to_string(),
            hash: "abc123".to_string(),
            source: EvidenceSource::Session,
            snippet_truncated: false,
            message_index: 2,
        };

        let preview = build_notebook_report_proposal_preview(
            temp.path(),
            "monthly:2026-06",
            NotebookProposalAction::Memory,
            Some(vec![session_hit]),
        )
        .unwrap();

        assert_eq!(preview.evidence_refs.len(), 2);
        assert_eq!(preview.evidence_refs[1].source, EvidenceSource::Session);
        assert!(!temp.path().join("memory/MEMORY.md").exists());
    }
}

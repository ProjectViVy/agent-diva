use chrono::{Datelike, Local, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const MAX_RENDER_LINES: usize = 5_000;
const REPORT_SYSTEM_GENERATED_BY: &str = "agent-diva-report-system";
const REPORT_SYSTEM_SCHEMA_VERSION: u8 = 1;

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotebookGenerationResult {
    pub path: PathBuf,
    pub date_key: String,
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

        let report = parse_report_file(workspace, &path, period)?;
        reports.push(report);
    }

    reports.sort_by(|left, right| {
        right
            .date
            .cmp(&left.date)
            .then_with(|| right.id.cmp(&left.id))
    });
    Ok(reports)
}

pub fn generate_monthly_notebook_report(
    workspace: &Path,
) -> Result<NotebookGenerationResult, String> {
    let now = Local::now();
    generate_monthly_notebook_report_for(
        workspace,
        now.year(),
        now.month(),
        Utc::now().to_rfc3339(),
    )
}

fn generate_monthly_notebook_report_for(
    workspace: &Path,
    year: i32,
    month: u32,
    generated_at: String,
) -> Result<NotebookGenerationResult, String> {
    let month_key = format!("{year:04}-{month:02}");
    let path = workspace
        .join("reports/monthly")
        .join(format!("{month_key}.md"));
    let content = render_monthly_report_template(&month_key, &generated_at);
    atomic_write(&path, content.as_bytes())?;
    Ok(NotebookGenerationResult {
        path,
        date_key: month_key,
    })
}

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
        source_path,
        is_truncated,
        original_line_count,
        displayed_line_count,
    })
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

fn render_monthly_report_template(month: &str, generated_at: &str) -> String {
    let mut markdown = String::new();
    markdown.push_str(
        "---
",
    );
    markdown.push_str(
        "period: monthly
",
    );
    markdown.push_str(&format!(
        "month: {month}
"
    ));
    markdown.push_str(&format!(
        "generated_at: {generated_at}
"
    ));
    markdown.push_str(&format!(
        "generated_by: {REPORT_SYSTEM_GENERATED_BY}
"
    ));
    markdown.push_str(
        "source: manual
",
    );
    markdown.push_str(
        "session_count: 0
",
    );
    markdown.push_str(
        "token_used: 0
",
    );
    markdown.push_str(&format!(
        "schema_version: {REPORT_SYSTEM_SCHEMA_VERSION}
"
    ));
    markdown.push_str(
        "---

",
    );
    markdown.push_str(&format!(
        "# Monthly Report {month}

"
    ));
    markdown.push_str(
        "## Overview
",
    );
    markdown.push_str(&format!(
        "- **Coverage**: {month}-01 -> {month}-end
"
    ));
    markdown.push_str(
        "- **Session Count**: 0
",
    );
    markdown.push_str(
        "- **LLM Usage**: 0 tokens

",
    );
    markdown.push_str(
        "## Key Topics
",
    );
    markdown.push_str(
        "1. Manual generation placeholder pending report-system monthly summarization.

",
    );
    markdown.push_str(
        "## Key Decisions
",
    );
    markdown.push_str(
        "- Monthly report was manually regenerated from Notebook.

",
    );
    markdown.push_str(
        "## Completed Tasks
",
    );
    markdown.push_str(
        "- [x] Created Report-owned monthly report placeholder.

",
    );
    markdown.push_str(
        "## Follow-up Items
",
    );
    markdown.push_str(
        "- [ ] Replace placeholder content with report-system monthly summarization.

",
    );
    markdown.push_str(
        "## Knowledge Capture
",
    );
    markdown.push_str(
        "- Pending monthly synthesis implementation.

",
    );
    markdown.push_str(
        "## Improvement Suggestions
",
    );
    markdown.push_str(
        "- Implement report-system owned monthly summarization and cron scheduling.

",
    );
    markdown.push_str(
        "---
",
    );
    markdown.push_str(&format!(
        "*Generated by {REPORT_SYSTEM_GENERATED_BY} at {generated_at}*
"
    ));
    markdown.push_str(
        "*Source: manual | Schema: v1*
",
    );
    markdown
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create parent directory {}: {error}",
            parent.display()
        )
    })?;
    let temp = temp_path(path);
    {
        let mut file = fs::File::create(&temp)
            .map_err(|error| format!("failed to create temp report {}: {error}", temp.display()))?;
        file.write_all(bytes)
            .map_err(|error| format!("failed to write temp report {}: {error}", temp.display()))?;
        file.sync_all()
            .map_err(|error| format!("failed to sync temp report {}: {error}", temp.display()))?;
    }
    fs::rename(&temp, path)
        .map_err(|error| format!("failed to replace report {}: {error}", path.display()))?;
    sync_parent_dir(parent);
    Ok(())
}

fn temp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("notebook-report");
    path.with_file_name(format!(".{file_name}.tmp"))
}

fn sync_parent_dir(parent: &Path) {
    if let Ok(dir) = fs::File::open(parent) {
        let _ = dir.sync_all();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        generate_monthly_notebook_report, generate_monthly_notebook_report_for,
        load_notebook_reports, NotebookPeriod, MAX_RENDER_LINES, REPORT_SYSTEM_GENERATED_BY,
    };
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
            reports[0].source_path,
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
        assert_eq!(reports[0].source_path, "reports/monthly/2026-06.md");
    }

    #[test]
    fn returns_empty_when_report_directory_is_missing() {
        let temp = tempfile::tempdir().unwrap();
        let reports = load_notebook_reports(temp.path(), NotebookPeriod::Daily).unwrap();
        assert!(reports.is_empty());
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
        assert_eq!(reports[0].original_line_count, MAX_RENDER_LINES + 12);
        assert!(!reports[0]
            .content
            .contains(&format!("line-{}", MAX_RENDER_LINES + 5)));
    }

    #[test]
    fn generates_report_owned_monthly_report_with_v1_frontmatter() {
        let temp = tempfile::tempdir().unwrap();

        let result = generate_monthly_notebook_report_for(
            temp.path(),
            2026,
            6,
            "2026-06-15T00:00:00Z".to_string(),
        )
        .unwrap();
        assert!(result.path.exists());
        assert!(result.path.starts_with(temp.path().join("reports/monthly")));

        let markdown = fs::read_to_string(&result.path).unwrap();
        assert!(markdown.contains("period: monthly"));
        assert!(markdown.contains("month: 2026-06"));
        assert!(markdown.contains(&format!("generated_by: {REPORT_SYSTEM_GENERATED_BY}")));
        assert!(markdown.contains("source: manual"));
        assert!(markdown.contains("session_count: 0"));
        assert!(markdown.contains("token_used: 0"));
        assert!(markdown.contains("schema_version: 1"));
        assert!(markdown.contains("## Improvement Suggestions"));
    }

    #[test]
    fn monthly_generation_replaces_existing_report_atomically() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("reports/monthly/2099-12.md");
        write_report(&target, "old");

        let result = generate_monthly_notebook_report_for(
            temp.path(),
            2099,
            12,
            "2026-06-15T00:00:00Z".to_string(),
        )
        .unwrap();
        assert_eq!(result.path, target);
        assert!(!target.with_file_name(".2099-12.md.tmp").exists());

        let markdown = fs::read_to_string(target).unwrap();
        assert!(markdown.contains("month: 2099-12"));
        assert!(!markdown.contains(
            "
old"
        ));
    }

    #[test]
    fn public_monthly_generator_writes_current_month_file() {
        let temp = tempfile::tempdir().unwrap();
        let result = generate_monthly_notebook_report(temp.path()).unwrap();
        assert!(result.path.exists());
        assert_eq!(
            result.path,
            temp.path()
                .join("reports/monthly")
                .join(format!("{}.md", result.date_key))
        );
    }
}

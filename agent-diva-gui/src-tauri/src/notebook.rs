use agent_diva_core::session::{SessionManager, SessionSearchQuery, SessionSearchResponse};
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
            Self::Daily => workspace.join(".laputa/reports/daily"),
            Self::Weekly => workspace.join(".laputa/reports/weekly"),
            Self::Monthly => workspace.join(".laputa/reports/monthly"),
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
    use super::{load_notebook_reports, NotebookPeriod};

    #[test]
    fn notebook_period_parses_supported_values() {
        assert_eq!(
            NotebookPeriod::parse("daily").unwrap(),
            NotebookPeriod::Daily
        );
        assert!(NotebookPeriod::parse("yearly").is_err());
    }

    #[test]
    fn notebook_reports_load_empty_workspace() {
        let temp = tempfile::tempdir().unwrap();
        assert!(load_notebook_reports(temp.path(), NotebookPeriod::Daily)
            .unwrap()
            .is_empty());
    }
}

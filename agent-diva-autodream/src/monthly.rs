use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use agent_diva_core::reports::{
    collect_session_window_digest_for_dates, estimate_tokens, read_rhythm_report,
    RhythmReportDocument, SessionWindowDigest,
};
use chrono::{DateTime, Datelike, Days, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    atomic::{atomic_write, atomic_write_json},
    AutoDreamError, AutoDreamStorage, Result, RhythmReportWriteResult,
};

const GENERATED_BY: &str = "agent-diva-report-system";
const SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct MonthlyReportErrorMarker {
    pub month: String,
    pub generated_at: String,
    pub status: String,
    pub message: String,
    pub attempts: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct AutoDreamMonthlyReportGenerator {
    storage: AutoDreamStorage,
}

impl AutoDreamMonthlyReportGenerator {
    pub(crate) fn new(storage: AutoDreamStorage) -> Self {
        Self { storage }
    }

    pub(crate) fn generate_current_month(&self, attempt: u32) -> Result<RhythmReportWriteResult> {
        let now = Local::now();
        self.generate_for_month(now.year(), now.month(), Utc::now(), attempt)
    }

    pub(crate) fn generate_for_month(
        &self,
        year: i32,
        month: u32,
        generated_at: DateTime<Utc>,
        attempt: u32,
    ) -> Result<RhythmReportWriteResult> {
        let month_key = format!("{year:04}-{month:02}");
        let path = self.storage.paths().monthly_report_file(&month_key);
        match render_monthly_report_content(
            self.storage.paths().workspace_root(),
            year,
            month,
            &generated_at.to_rfc3339(),
        ) {
            Ok(markdown) => {
                atomic_write(&path, markdown.as_bytes())?;
                self.remove_error_marker(&month_key);
                Ok(RhythmReportWriteResult {
                    path,
                    bytes_written: markdown.len(),
                    evidence_ref_count: 0,
                })
            }
            Err(error) => {
                self.write_error_marker(&month_key, &generated_at.to_rfc3339(), &error, attempt)?;
                Err(AutoDreamError::InvalidState(error))
            }
        }
    }

    pub(crate) fn read_error_marker(
        &self,
        month_key: &str,
    ) -> Result<Option<MonthlyReportErrorMarker>> {
        let path = self.storage.paths().monthly_error_marker_file(month_key);
        match fs::read_to_string(&path) {
            Ok(raw) => Ok(Some(serde_json::from_str(&raw)?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(AutoDreamError::io(path, source)),
        }
    }

    fn write_error_marker(
        &self,
        month_key: &str,
        generated_at: &str,
        error: &str,
        attempts: u32,
    ) -> Result<()> {
        let marker = MonthlyReportErrorMarker {
            month: month_key.to_string(),
            generated_at: generated_at.to_string(),
            status: "failed".to_string(),
            message: error.to_string(),
            attempts,
        };
        atomic_write_json(
            &self.storage.paths().monthly_error_marker_file(month_key),
            &marker,
        )
    }

    fn remove_error_marker(&self, month_key: &str) {
        let _ = fs::remove_file(self.storage.paths().monthly_error_marker_file(month_key));
    }
}

fn render_monthly_report_content(
    workspace: &Path,
    year: i32,
    month: u32,
    generated_at: &str,
) -> std::result::Result<String, String> {
    let month_key = format!("{year:04}-{month:02}");
    let all_dates = dates_in_month(year, month)?;
    let mut daily_inputs = Vec::new();
    let mut missing_dates = BTreeSet::new();

    for date in &all_dates {
        let path = workspace
            .join(".agent-diva/autodream/reports/daily")
            .join(format!("{}.md", date.format("%Y-%m-%d")));
        match read_rhythm_report(&path) {
            Ok(document) => daily_inputs.push((*date, path, document)),
            Err(_) => {
                missing_dates.insert(*date);
            }
        }
    }

    let fallback_digest = collect_session_window_digest_for_dates(workspace, &missing_dates)
        .map_err(|error| format!("failed to collect monthly fallback sessions: {error}"))?;
    if daily_inputs.is_empty() && fallback_digest.items.is_empty() {
        return Err(format!(
            "no daily reports or fallback sessions found for monthly report {month_key}"
        ));
    }

    let mut session_count = fallback_digest.session_count;
    let mut token_used = fallback_digest.estimated_tokens;
    let mut daily_lines = Vec::new();

    for (date, _, document) in &daily_inputs {
        session_count += document.frontmatter.session_count.unwrap_or(0);
        token_used += document
            .frontmatter
            .token_used
            .unwrap_or_else(|| estimate_tokens(&document.body));
        daily_lines.push(format!(
            "- {}: {}",
            date.format("%Y-%m-%d"),
            document.summary
        ));
    }

    let summary = format!(
        "Aggregated {} daily reports with {} fallback sessions for {}.",
        daily_inputs.len(),
        fallback_digest.session_count,
        month_key
    );

    let mut markdown = String::new();
    markdown.push_str("---\n");
    markdown.push_str("period: monthly\n");
    markdown.push_str(&format!("month: {month_key}\n"));
    markdown.push_str(&format!("generated_at: {generated_at}\n"));
    markdown.push_str(&format!("generated_by: {GENERATED_BY}\n"));
    markdown.push_str("source: daily_aggregate\n");
    markdown.push_str(&format!("session_count: {session_count}\n"));
    markdown.push_str(&format!("token_used: {token_used}\n"));
    markdown.push_str(&format!(
        "fallback_used: {}\n",
        !fallback_digest.items.is_empty()
    ));
    markdown.push_str(&format!("daily_inputs_count: {}\n", daily_inputs.len()));
    markdown.push_str(&format!(
        "missing_daily_dates_count: {}\n",
        missing_dates.len()
    ));
    markdown.push_str(&format!("schema_version: {SCHEMA_VERSION}\n"));
    markdown.push_str("---\n\n");
    markdown.push_str(&format!("# Monthly Report {month_key}\n\n"));
    markdown.push_str(&format!("{summary}\n\n"));
    markdown.push_str("## Overview\n\n");
    markdown.push_str(&format!(
        "- **Coverage**: {}-01 -> {}-end\n",
        month_key, month_key
    ));
    markdown.push_str(&format!("- **Daily Inputs**: {}\n", daily_inputs.len()));
    markdown.push_str(&format!(
        "- **Recovered Session Gaps**: {}\n",
        fallback_digest.session_count
    ));
    markdown.push_str(&format!("- **Session Count**: {session_count}\n"));
    markdown.push_str(&format!("- **Estimated Token Usage**: {token_used}\n\n"));
    markdown.push_str("## Daily Highlights\n\n");
    if daily_lines.is_empty() {
        markdown.push_str("- No daily reports were available.\n\n");
    } else {
        markdown.push_str(&daily_lines.join("\n"));
        markdown.push_str("\n\n");
    }
    markdown.push_str("## Gap Recovery\n\n");
    if fallback_digest.items.is_empty() {
        markdown.push_str("- No session fallback was required.\n\n");
    } else {
        for item in &fallback_digest.items {
            markdown.push_str(&format!("- `{}`: {}\n", item.session_key, item.summary));
        }
        markdown.push('\n');
    }
    markdown.push_str("## Monthly Synthesis\n\n");
    markdown.push_str(&render_monthly_synthesis_paragraphs(
        &month_key,
        &daily_inputs,
        &fallback_digest,
    ));
    markdown.push_str("\n## Follow-up\n\n");
    if missing_dates.is_empty() {
        markdown.push_str("- Daily coverage was complete for this monthly synthesis.\n");
    } else {
        markdown.push_str(&format!(
            "- Missing daily dates were recovered from sessions: {}.\n",
            missing_dates
                .iter()
                .map(|date| date.format("%Y-%m-%d").to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    markdown.push_str(&format!(
        "\n---\n*Generated by {GENERATED_BY} at {generated_at}*\n"
    ));
    Ok(markdown)
}

fn render_monthly_synthesis_paragraphs(
    month_key: &str,
    daily_inputs: &[(NaiveDate, PathBuf, RhythmReportDocument)],
    fallback_digest: &SessionWindowDigest,
) -> String {
    let mut paragraphs = Vec::new();
    if !daily_inputs.is_empty() {
        let top = daily_inputs
            .iter()
            .take(5)
            .map(|(_, _, document)| document.summary.clone())
            .collect::<Vec<_>>()
            .join(" ");
        paragraphs.push(format!(
            "The report-system monthly synthesis for {month_key} was built primarily from {} daily summaries. {}",
            daily_inputs.len(),
            truncate_chars(&top, 600)
        ));
    }
    if !fallback_digest.items.is_empty() {
        let recovered = fallback_digest
            .items
            .iter()
            .map(|item| item.summary.clone())
            .collect::<Vec<_>>()
            .join(" ");
        paragraphs.push(format!(
            "Session fallback filled {} missing daily gaps: {}",
            fallback_digest.session_count,
            truncate_chars(&recovered, 600)
        ));
    }
    if paragraphs.is_empty() {
        paragraphs.push(format!(
            "No daily summaries or fallback sessions were available for {month_key}."
        ));
    }
    format!("{}\n", paragraphs.join("\n\n"))
}

fn dates_in_month(year: i32, month: u32) -> std::result::Result<Vec<NaiveDate>, String> {
    let start = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| format!("invalid month key: {year:04}-{month:02}"))?;
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let next = NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .ok_or_else(|| format!("invalid next month key: {next_year:04}-{next_month:02}"))?;
    let span_days = (next - start).num_days();
    let mut dates = Vec::with_capacity(span_days as usize);
    for offset in 0..span_days {
        if let Some(date) = start.checked_add_days(Days::new(offset as u64)) {
            dates.push(date);
        }
    }
    Ok(dates)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        truncated.push_str("...");
    }
    truncated
}

#[cfg(test)]
mod tests {
    use super::AutoDreamMonthlyReportGenerator;
    use crate::AutoDreamStorage;
    use chrono::{TimeZone, Utc};

    fn write_report(path: &std::path::Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn generates_monthly_report_with_daily_aggregate_frontmatter() {
        let temp = tempfile::tempdir().unwrap();
        let storage = AutoDreamStorage::open(temp.path()).unwrap();
        write_report(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/daily/2026-06-14.md"),
            "---
period: daily
date: 2026-06-14
generated_by: agent-diva-autodream
session_count: 2
token_used: 12
---

# Daily Reflection

Summarized the launch planning and follow-up actions.
",
        );

        let result = AutoDreamMonthlyReportGenerator::new(storage)
            .generate_for_month(
                2026,
                6,
                Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap(),
                1,
            )
            .unwrap();
        let markdown = std::fs::read_to_string(result.path).unwrap();
        assert!(markdown.contains("period: monthly"));
        assert!(markdown.contains("month: 2026-06"));
        assert!(markdown.contains("source: daily_aggregate"));
        assert!(markdown.contains("daily_inputs_count: 1"));
        assert!(markdown.contains("## Monthly Synthesis"));
    }

    #[test]
    fn monthly_generation_recovers_missing_daily_dates_from_sessions() {
        let temp = tempfile::tempdir().unwrap();
        let storage = AutoDreamStorage::open(temp.path()).unwrap();
        write_report(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/daily/2026-06-01.md"),
            "---
period: daily
date: 2026-06-01
session_count: 1
token_used: 7
---

# Daily 1

Kickoff summary.
",
        );
        let sessions_dir = temp.path().join("sessions");
        std::fs::create_dir_all(&sessions_dir).unwrap();
        std::fs::write(
            sessions_dir.join("telegram_8.jsonl"),
            r#"{"_type":"metadata","created_at":"2026-06-02T00:00:00Z","updated_at":"2026-06-02T00:00:00Z","metadata":{}}
{"role":"user","content":"Recovered second day context.","timestamp":"2026-06-02T01:02:03Z"}"#,
        )
        .unwrap();

        let result = AutoDreamMonthlyReportGenerator::new(storage)
            .generate_for_month(
                2026,
                6,
                Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap(),
                1,
            )
            .unwrap();
        let markdown = std::fs::read_to_string(result.path).unwrap();
        assert!(markdown.contains("fallback_used: true"));
        assert!(markdown.contains("Recovered second day context."));
    }

    #[test]
    fn monthly_generation_writes_error_marker_with_attempts() {
        let temp = tempfile::tempdir().unwrap();
        let storage = AutoDreamStorage::open(temp.path()).unwrap();
        let generator = AutoDreamMonthlyReportGenerator::new(storage.clone());

        let error = generator
            .generate_for_month(
                2026,
                6,
                Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap(),
                2,
            )
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("no daily reports or fallback sessions found"));

        let marker = generator.read_error_marker("2026-06").unwrap().unwrap();
        assert_eq!(marker.attempts, 2);
        assert_eq!(marker.month, "2026-06");
    }
}

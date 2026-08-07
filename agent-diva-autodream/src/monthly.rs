use std::{collections::BTreeSet, fs, sync::Arc};

use agent_diva_core::config::LlmCurationConfig;
use agent_diva_core::reports::{
    build_aggregate_fact_bundle, collect_session_window_digest_for_dates, daily_report_evidence,
    read_rhythm_report, DailyReportInput, FactBundleLimits, ReportNarrativeGenerator, ReportWindow,
};
use chrono::{DateTime, Datelike, Days, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    atomic::{atomic_write, atomic_write_json},
    curation::{content_from_rendered, render_monthly_markdown, resolve_report_body},
    AutoDreamError, AutoDreamStorage, Result, RhythmReportWriteResult,
};

const GENERATED_BY: &str = "agent-diva-report-system";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct MonthlyReportErrorMarker {
    pub month: String,
    pub generated_at: String,
    pub status: String,
    pub message: String,
    pub attempts: u32,
}

#[derive(Clone)]
pub(crate) struct AutoDreamMonthlyReportGenerator {
    storage: AutoDreamStorage,
    narrative_generator: Option<Arc<dyn ReportNarrativeGenerator>>,
    curation: LlmCurationConfig,
}

impl AutoDreamMonthlyReportGenerator {
    pub(crate) fn with_narrative(
        storage: AutoDreamStorage,
        narrative_generator: Option<Arc<dyn ReportNarrativeGenerator>>,
        curation: LlmCurationConfig,
    ) -> Self {
        Self {
            storage,
            narrative_generator,
            curation,
        }
    }

    pub(crate) async fn generate_current_month(
        &self,
        attempt: u32,
    ) -> Result<RhythmReportWriteResult> {
        let now = Local::now();
        self.generate_for_month(now.year(), now.month(), Utc::now(), attempt)
            .await
    }

    pub(crate) async fn generate_for_month(
        &self,
        year: i32,
        month: u32,
        generated_at: DateTime<Utc>,
        attempt: u32,
    ) -> Result<RhythmReportWriteResult> {
        let month_key = format!("{year:04}-{month:02}");
        let path = self.storage.paths().monthly_report_file(&month_key);
        match self
            .build_monthly_content(year, month, &generated_at.to_rfc3339())
            .await
        {
            Ok(markdown) => {
                atomic_write(&path, markdown.as_bytes())?;
                self.remove_error_marker(&month_key);
                let evidence_ref_count = markdown.matches("- id: `").count();
                Ok(RhythmReportWriteResult {
                    path,
                    bytes_written: markdown.len(),
                    evidence_ref_count,
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

    async fn build_monthly_content(
        &self,
        year: i32,
        month: u32,
        generated_at: &str,
    ) -> std::result::Result<String, String> {
        let month_key = format!("{year:04}-{month:02}");
        let all_dates = dates_in_month(year, month)?;
        let start = *all_dates
            .first()
            .ok_or_else(|| format!("invalid month key: {month_key}"))?;
        let end = *all_dates
            .last()
            .ok_or_else(|| format!("invalid month key: {month_key}"))?;

        let mut daily_inputs = Vec::new();
        let mut missing_dates = BTreeSet::new();

        for date in &all_dates {
            let path = self
                .storage
                .paths()
                .daily_reports_dir()
                .join(format!("{}.md", date.format("%Y-%m-%d")));
            match read_rhythm_report(&path) {
                Ok(document) => {
                    let path_display = path.to_string_lossy().to_string();
                    let evidence = daily_report_evidence(*date, &path_display, &document.summary);
                    daily_inputs.push(DailyReportInput {
                        date: *date,
                        path_display,
                        document,
                        evidence,
                    });
                }
                Err(_) => {
                    missing_dates.insert(*date);
                }
            }
        }

        let fallback_digest = collect_session_window_digest_for_dates(
            self.storage.paths().workspace_root(),
            &missing_dates,
        )
        .map_err(|error| format!("failed to collect monthly fallback sessions: {error}"))?;
        if daily_inputs.is_empty() && fallback_digest.items.is_empty() {
            return Err(format!(
                "no daily reports or fallback sessions found for monthly report {month_key}"
            ));
        }

        let window = ReportWindow::monthly(year, month, start, end);
        let language = self.curation.language.clone();
        let bundle = build_aggregate_fact_bundle(
            window,
            &daily_inputs,
            &fallback_digest,
            missing_dates.len(),
            language,
            FactBundleLimits::default(),
        );
        let rendered =
            resolve_report_body(&bundle, self.narrative_generator.as_ref(), &self.curation).await;
        let content = content_from_rendered(&bundle, rendered);
        Ok(render_monthly_markdown(
            &month_key,
            generated_at,
            GENERATED_BY,
            &content,
        ))
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

#[cfg(test)]
mod tests {
    use super::AutoDreamMonthlyReportGenerator;
    use crate::AutoDreamStorage;
    use chrono::{TimeZone, Utc};

    fn write_report(path: &std::path::Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[tokio::test]
    async fn generates_monthly_report_with_daily_aggregate_frontmatter() {
        let temp = tempfile::tempdir().unwrap();
        let storage = AutoDreamStorage::open(temp.path()).unwrap();
        write_report(
            &storage.paths().daily_report_file("2026-06-14"),
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

        let result = AutoDreamMonthlyReportGenerator::with_narrative(
            storage,
            None,
            agent_diva_core::config::LlmCurationConfig::default(),
        )
        .generate_for_month(
            2026,
            6,
            Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap(),
            1,
        )
        .await
        .unwrap();
        let markdown = std::fs::read_to_string(result.path).unwrap();
        assert!(markdown.contains("period: monthly"));
        assert!(markdown.contains("month: 2026-06"));
        assert!(markdown.contains("source: daily_aggregate"));
        assert!(markdown.contains("daily_inputs_count: 1"));
        assert!(markdown.contains("generation_mode: deterministic_fallback"));
        assert!(markdown.contains("## Evidence References"));
        assert!(markdown.contains("## 摘要"));
    }

    #[tokio::test]
    async fn monthly_generation_recovers_missing_daily_dates_from_sessions() {
        let temp = tempfile::tempdir().unwrap();
        let storage = AutoDreamStorage::open(temp.path()).unwrap();
        write_report(
            &storage.paths().daily_report_file("2026-06-01"),
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

        let result = AutoDreamMonthlyReportGenerator::with_narrative(
            storage,
            None,
            agent_diva_core::config::LlmCurationConfig::default(),
        )
        .generate_for_month(
            2026,
            6,
            Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap(),
            1,
        )
        .await
        .unwrap();
        let markdown = std::fs::read_to_string(result.path).unwrap();
        assert!(markdown.contains("fallback_used: true"));
        assert!(markdown.contains("Recovered second day context."));
    }

    #[tokio::test]
    async fn monthly_generation_writes_error_marker_with_attempts() {
        let temp = tempfile::tempdir().unwrap();
        let storage = AutoDreamStorage::open(temp.path()).unwrap();
        let generator = AutoDreamMonthlyReportGenerator::with_narrative(
            storage.clone(),
            None,
            agent_diva_core::config::LlmCurationConfig::default(),
        );

        let error = generator
            .generate_for_month(
                2026,
                6,
                Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap(),
                2,
            )
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("no daily reports or fallback sessions found"));

        let marker = generator.read_error_marker("2026-06").unwrap().unwrap();
        assert_eq!(marker.attempts, 2);
        assert_eq!(marker.month, "2026-06");
    }
}

use std::{collections::BTreeSet, path::Path, sync::Arc};

use agent_diva_core::config::LlmCurationConfig;
use agent_diva_core::reports::{
    build_aggregate_fact_bundle, build_daily_fact_bundle, collect_session_window_digest_for_dates,
    daily_report_evidence, read_rhythm_report, DailyReportInput, FactBundleLimits,
    ReportNarrativeGenerator, ReportWindow, SessionWindowDigest,
};
use chrono::{Datelike, Days, Local, NaiveDate, Utc, Weekday};

use crate::{
    curation::{content_from_rendered, resolve_report_body},
    AutoDreamError, AutoDreamReportWriter, AutoDreamStorage, Result, RhythmReportWriteResult,
};

pub struct AutoDreamRhythmReportGenerator {
    storage: AutoDreamStorage,
    narrative_generator: Option<Arc<dyn ReportNarrativeGenerator>>,
    curation: LlmCurationConfig,
}

impl AutoDreamRhythmReportGenerator {
    pub fn new(storage: AutoDreamStorage) -> Self {
        Self {
            storage,
            narrative_generator: None,
            curation: LlmCurationConfig::default(),
        }
    }

    pub fn with_narrative(
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

    pub async fn generate_daily_for_date(
        &self,
        date: NaiveDate,
    ) -> Result<RhythmReportWriteResult> {
        let dates = BTreeSet::from([date]);
        let digest = collect_digest(self.storage.paths().workspace_root(), &dates)?;
        if digest.items.is_empty() {
            return Err(AutoDreamError::InvalidState(format!(
                "no session activity found for daily report {}",
                date.format("%Y-%m-%d")
            )));
        }
        let language = self.curation.language.clone();
        let bundle = build_daily_fact_bundle(date, &digest, language, FactBundleLimits::default());
        let rendered =
            resolve_report_body(&bundle, self.narrative_generator.as_ref(), &self.curation).await;
        let content = content_from_rendered(&bundle, rendered);
        let writer = AutoDreamReportWriter::new(self.storage.clone());
        writer.write_daily_report(date.format("%Y-%m-%d").to_string(), Utc::now(), content)
    }

    pub async fn generate_weekly_for_week(
        &self,
        year: i32,
        week: u32,
    ) -> Result<RhythmReportWriteResult> {
        let week_start = NaiveDate::from_isoywd_opt(year, week, Weekday::Mon).ok_or_else(|| {
            AutoDreamError::InvalidState(format!(
                "weekly report key must be YYYY-Www: {year}-W{week:02}"
            ))
        })?;
        let mut all_dates = BTreeSet::new();
        for offset in 0..7 {
            if let Some(date) = week_start.checked_add_days(Days::new(offset)) {
                all_dates.insert(date);
            }
        }
        let week_end = *all_dates.iter().next_back().unwrap_or(&week_start);

        let mut daily_inputs = Vec::new();
        let mut missing_dates = BTreeSet::new();
        for date in &all_dates {
            let path = self
                .storage
                .paths()
                .daily_report_file(&date.format("%Y-%m-%d").to_string());
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

        let fallback_digest = if missing_dates.is_empty() {
            SessionWindowDigest::default()
        } else {
            collect_digest(self.storage.paths().workspace_root(), &missing_dates)?
        };

        if daily_inputs.is_empty() && fallback_digest.items.is_empty() {
            return Err(AutoDreamError::InvalidState(format!(
                "no daily reports or fallback sessions found for weekly report {year}-W{week:02}"
            )));
        }

        let window = ReportWindow::weekly(year, week, week_start, week_end);
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
        let writer = AutoDreamReportWriter::new(self.storage.clone());
        writer.write_weekly_report(format!("{year}-W{week:02}"), Utc::now(), content)
    }

    pub async fn generate_for_trigger(&self, trigger: &str) -> Result<RhythmReportWriteResult> {
        match trigger {
            "notebook-daily" => {
                self.generate_daily_for_date(Local::now().date_naive())
                    .await
            }
            "notebook-weekly" => {
                let now = Local::now().date_naive();
                self.generate_weekly_for_week(now.iso_week().year(), now.iso_week().week())
                    .await
            }
            other => Err(AutoDreamError::InvalidState(format!(
                "unsupported notebook report trigger: {other}"
            ))),
        }
    }
}

fn collect_digest(workspace: &Path, dates: &BTreeSet<NaiveDate>) -> Result<SessionWindowDigest> {
    collect_session_window_digest_for_dates(workspace, dates)
        .map_err(|error| AutoDreamError::InputCollection(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::AutoDreamRhythmReportGenerator;
    use crate::{AutoDreamReportWriter, AutoDreamStorage, RhythmReportContent};
    use agent_diva_core::{
        evolution::{EvidenceRef, EvidenceSource},
        session::{ChatMessage, Session, SessionManager},
    };
    use chrono::{NaiveDate, TimeZone, Utc};

    fn sample_evidence(id: &str) -> EvidenceRef {
        EvidenceRef {
            id: id.to_string(),
            source: EvidenceSource::AutoDreamRun,
            uri: format!("autodream://runs/run-123/evidence/{id}"),
            excerpt: Some("bounded evidence".to_string()),
            hash: None,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn generates_daily_report_from_sessions() {
        let temp = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(temp.path());
        let mut session = Session::new("telegram:1");
        session.add_full_message(ChatMessage {
            role: "user".to_string(),
            content: "Discussed the launch plan".to_string(),
            timestamp: Utc.with_ymd_and_hms(2026, 6, 18, 1, 0, 0).unwrap(),
            tool_call_id: None,
            tool_calls: None,
            name: None,
            reasoning_content: None,
            thinking_blocks: None,
            token_usage: None,
            metadata: Default::default(),
        });
        manager.save(&session).unwrap();

        let storage = AutoDreamStorage::open(temp.path()).unwrap();
        let generator = AutoDreamRhythmReportGenerator::new(storage);
        let result = generator
            .generate_daily_for_date(NaiveDate::from_ymd_opt(2026, 6, 18).unwrap())
            .await
            .unwrap();

        let markdown = std::fs::read_to_string(result.path).unwrap();
        assert!(markdown.contains("period: daily"));
        assert!(markdown.contains("session_count: 1"));
        assert!(markdown.contains("source: session_aggregate"));
        assert!(markdown.contains("generation_mode: deterministic_fallback"));
        assert!(!markdown.contains("## Session Highlights"));
        assert!(markdown.contains("## 摘要"));
    }

    #[tokio::test]
    async fn generates_weekly_report_from_daily_with_session_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let storage = AutoDreamStorage::open(temp.path()).unwrap();
        let writer = AutoDreamReportWriter::new(storage.clone());
        writer
            .write_daily_report(
                "2026-06-16",
                Utc.with_ymd_and_hms(2026, 6, 16, 10, 0, 0).unwrap(),
                RhythmReportContent {
                    title: "Daily Report 2026-06-16".to_string(),
                    summary: "Summarized one session.".to_string(),
                    sections: vec!["## Session Highlights\n\n- highlight".to_string()],
                    evidence_refs: vec![sample_evidence("daily-evidence")],
                    source: Some("session_aggregate".to_string()),
                    session_count: Some(1),
                    token_used: Some(20),
                    fallback_used: Some(false),
                    daily_inputs_count: Some(0),
                    missing_daily_dates_count: Some(0),
                    generation_mode: None,
                    narrative_schema_version: None,
                    prompt_version: None,
                    coverage_status: None,
                    fallback_reason: None,
                },
            )
            .unwrap();

        let manager = SessionManager::new(temp.path());
        let mut session = Session::new("telegram:2");
        session.add_full_message(ChatMessage {
            role: "assistant".to_string(),
            content: "Recovered gap session".to_string(),
            timestamp: Utc.with_ymd_and_hms(2026, 6, 18, 1, 0, 0).unwrap(),
            tool_call_id: None,
            tool_calls: None,
            name: None,
            reasoning_content: None,
            thinking_blocks: None,
            token_usage: None,
            metadata: Default::default(),
        });
        manager.save(&session).unwrap();

        let generator = AutoDreamRhythmReportGenerator::new(storage);
        let result = generator.generate_weekly_for_week(2026, 25).await.unwrap();
        let markdown = std::fs::read_to_string(result.path).unwrap();
        assert!(markdown.contains("period: weekly"));
        assert!(markdown.contains("daily_inputs_count: 1"));
        assert!(markdown.contains("missing_daily_dates_count: 6"));
        assert!(markdown.contains("fallback_used: true"));
        assert!(markdown.contains("generation_mode: deterministic_fallback"));
    }
}

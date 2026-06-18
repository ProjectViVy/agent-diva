use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use agent_diva_core::reports::{
    collect_session_window_digest_for_dates, estimate_tokens, read_rhythm_report,
    SessionWindowDigest,
};
use chrono::{Datelike, Days, Local, NaiveDate, Utc, Weekday};

use crate::{
    AutoDreamError, AutoDreamReportWriter, AutoDreamStorage, Result, RhythmReportContent,
    RhythmReportWriteResult,
};

const DAILY_GENERATED_SOURCE: &str = "session_aggregate";
const WEEKLY_GENERATED_SOURCE: &str = "daily_aggregate";

pub struct AutoDreamRhythmReportGenerator {
    storage: AutoDreamStorage,
}

impl AutoDreamRhythmReportGenerator {
    pub fn new(storage: AutoDreamStorage) -> Self {
        Self { storage }
    }

    pub fn generate_daily_for_date(&self, date: NaiveDate) -> Result<RhythmReportWriteResult> {
        let dates = BTreeSet::from([date]);
        let digest = collect_digest(self.storage.paths().workspace_root(), &dates)?;
        if digest.items.is_empty() {
            return Err(AutoDreamError::InvalidState(format!(
                "no session activity found for daily report {}",
                date.format("%Y-%m-%d")
            )));
        }
        let writer = AutoDreamReportWriter::new(self.storage.clone());
        writer.write_daily_report(
            date.format("%Y-%m-%d").to_string(),
            Utc::now(),
            build_daily_content(date, &digest),
        )
    }

    pub fn generate_weekly_for_week(
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

        let mut daily_docs = Vec::new();
        let mut missing_dates = BTreeSet::new();
        for date in &all_dates {
            let path = self
                .storage
                .paths()
                .daily_report_file(&date.format("%Y-%m-%d").to_string());
            match read_rhythm_report(&path) {
                Ok(document) => daily_docs.push((path, *date, document)),
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

        if daily_docs.is_empty() && fallback_digest.items.is_empty() {
            return Err(AutoDreamError::InvalidState(format!(
                "no daily reports or fallback sessions found for weekly report {year}-W{week:02}"
            )));
        }

        let content = build_weekly_content(
            year,
            week,
            daily_docs,
            &fallback_digest,
            missing_dates.len(),
        );
        let writer = AutoDreamReportWriter::new(self.storage.clone());
        writer.write_weekly_report(format!("{year}-W{week:02}"), Utc::now(), content)
    }

    pub fn generate_for_trigger(&self, trigger: &str) -> Result<RhythmReportWriteResult> {
        match trigger {
            "notebook-daily" => self.generate_daily_for_date(Local::now().date_naive()),
            "notebook-weekly" => {
                let now = Local::now().date_naive();
                self.generate_weekly_for_week(now.iso_week().year(), now.iso_week().week())
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

fn build_daily_content(date: NaiveDate, digest: &SessionWindowDigest) -> RhythmReportContent {
    let overview = format!(
        "## Overview\n\n- Date: {}\n- Sessions Covered: {}\n- Messages Summarized: {}\n- Estimated Token Usage: {}\n",
        date.format("%Y-%m-%d"),
        digest.session_count,
        digest.message_count,
        digest.estimated_tokens
    );
    let highlights = digest
        .items
        .iter()
        .map(|item| format!("- `{}`: {}", item.session_key, item.summary))
        .collect::<Vec<_>>()
        .join("\n");
    let session_details = format!("## Session Highlights\n\n{}\n", highlights);
    RhythmReportContent {
        title: format!("Daily Report {}", date.format("%Y-%m-%d")),
        summary: format!(
            "Summarized {} sessions and {} messages for {}.",
            digest.session_count,
            digest.message_count,
            date.format("%Y-%m-%d")
        ),
        sections: vec![overview, session_details],
        evidence_refs: digest
            .items
            .iter()
            .map(|item| item.evidence.clone())
            .collect(),
        source: Some(DAILY_GENERATED_SOURCE.to_string()),
        session_count: Some(digest.session_count),
        token_used: Some(digest.estimated_tokens),
        fallback_used: Some(false),
        daily_inputs_count: Some(0),
        missing_daily_dates_count: Some(0),
    }
}

fn build_weekly_content(
    year: i32,
    week: u32,
    daily_docs: Vec<(
        PathBuf,
        NaiveDate,
        agent_diva_core::reports::RhythmReportDocument,
    )>,
    fallback_digest: &SessionWindowDigest,
    missing_count: usize,
) -> RhythmReportContent {
    let mut session_count: u64 = fallback_digest.session_count;
    let mut token_used: u64 = fallback_digest.estimated_tokens;
    let mut daily_summaries = Vec::new();
    let mut evidence_refs = Vec::new();

    for (path, date, document) in daily_docs {
        session_count += document.frontmatter.session_count.unwrap_or(0);
        token_used += document
            .frontmatter
            .token_used
            .unwrap_or_else(|| estimate_tokens(&document.body));
        daily_summaries.push(format!(
            "- {}: {}",
            date.format("%Y-%m-%d"),
            document.summary
        ));
        evidence_refs.push(agent_diva_core::evolution::EvidenceRef {
            id: format!("daily-{}", date.format("%Y-%m-%d")),
            source: agent_diva_core::evolution::EvidenceSource::Report,
            uri: format!("report://{}", path.to_string_lossy()),
            excerpt: Some(document.summary),
            hash: None,
            created_at: Utc::now(),
        });
    }

    evidence_refs.extend(
        fallback_digest
            .items
            .iter()
            .map(|item| item.evidence.clone()),
    );

    let overview = format!(
        "## Overview\n\n- ISO Week: {year}-W{week:02}\n- Daily Inputs Used: {}\n- Missing Daily Dates: {}\n- Fallback Sessions: {}\n- Estimated Token Usage: {}\n",
        daily_summaries.len(),
        missing_count,
        fallback_digest.session_count,
        token_used
    );
    let daily_section = if daily_summaries.is_empty() {
        "## Daily Inputs\n\n- No daily inputs available.\n".to_string()
    } else {
        format!("## Daily Inputs\n\n{}\n", daily_summaries.join("\n"))
    };
    let fallback_section = if fallback_digest.items.is_empty() {
        "## Gap Recovery\n\n- No session fallback required.\n".to_string()
    } else {
        format!(
            "## Gap Recovery\n\n{}\n",
            fallback_digest
                .items
                .iter()
                .map(|item| format!("- `{}`: {}", item.session_key, item.summary))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    RhythmReportContent {
        title: format!("Weekly Report {year}-W{week:02}"),
        summary: format!(
            "Aggregated {} daily reports with {} fallback sessions for {year}-W{week:02}.",
            daily_summaries.len(),
            fallback_digest.session_count
        ),
        sections: vec![overview, daily_section, fallback_section],
        evidence_refs,
        source: Some(WEEKLY_GENERATED_SOURCE.to_string()),
        session_count: Some(session_count),
        token_used: Some(token_used),
        fallback_used: Some(!fallback_digest.items.is_empty()),
        daily_inputs_count: Some(daily_summaries.len() as u64),
        missing_daily_dates_count: Some(missing_count as u64),
    }
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

    #[test]
    fn generates_daily_report_from_sessions() {
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
        });
        manager.save(&session).unwrap();

        let storage = AutoDreamStorage::open(temp.path()).unwrap();
        let generator = AutoDreamRhythmReportGenerator::new(storage);
        let result = generator
            .generate_daily_for_date(NaiveDate::from_ymd_opt(2026, 6, 18).unwrap())
            .unwrap();

        let markdown = std::fs::read_to_string(result.path).unwrap();
        assert!(markdown.contains("period: daily"));
        assert!(markdown.contains("session_count: 1"));
        assert!(markdown.contains("source: session_aggregate"));
    }

    #[test]
    fn generates_weekly_report_from_daily_with_session_fallback() {
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
        });
        manager.save(&session).unwrap();

        let generator = AutoDreamRhythmReportGenerator::new(storage);
        let result = generator.generate_weekly_for_week(2026, 25).unwrap();
        let markdown = std::fs::read_to_string(result.path).unwrap();
        assert!(markdown.contains("period: weekly"));
        assert!(markdown.contains("daily_inputs_count: 1"));
        assert!(markdown.contains("missing_daily_dates_count: 6"));
        assert!(markdown.contains("fallback_used: true"));
    }
}

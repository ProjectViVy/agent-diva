use std::path::PathBuf;

use agent_diva_core::evolution::EvidenceRef;
use chrono::{DateTime, NaiveDate, Utc};

use crate::{atomic::atomic_write, AutoDreamError, AutoDreamStorage, Result};

const GENERATED_BY: &str = "agent-diva-autodream";
const SCHEMA_VERSION: u8 = 1;
const MAX_EVIDENCE_REFS: usize = 50;
const MAX_EXCERPT_CHARS: usize = 240;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RhythmReportPeriod {
    Daily,
    Weekly,
}

impl RhythmReportPeriod {
    fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
        }
    }
}

impl std::str::FromStr for RhythmReportPeriod {
    type Err = AutoDreamError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            other => Err(AutoDreamError::InvalidState(format!(
                "unsupported rhythm report period: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RhythmReportContent {
    pub title: String,
    pub summary: String,
    pub sections: Vec<String>,
    pub evidence_refs: Vec<EvidenceRef>,
    pub source: Option<String>,
    pub session_count: Option<u64>,
    pub token_used: Option<u64>,
    pub fallback_used: Option<bool>,
    pub daily_inputs_count: Option<u64>,
    pub missing_daily_dates_count: Option<u64>,
    pub generation_mode: Option<String>,
    pub narrative_schema_version: Option<u32>,
    pub prompt_version: Option<String>,
    pub coverage_status: Option<String>,
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RhythmReportWriteRequest {
    pub period: RhythmReportPeriod,
    pub date_or_week: String,
    pub generated_at: DateTime<Utc>,
    pub content: RhythmReportContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RhythmReportWriteResult {
    pub path: PathBuf,
    pub bytes_written: usize,
    pub evidence_ref_count: usize,
}

#[derive(Debug, Clone)]
pub struct AutoDreamReportWriter {
    storage: AutoDreamStorage,
}

impl AutoDreamReportWriter {
    pub fn new(storage: AutoDreamStorage) -> Self {
        Self { storage }
    }

    pub fn write_report(
        &self,
        request: RhythmReportWriteRequest,
    ) -> Result<RhythmReportWriteResult> {
        validate_report_request(&request)?;
        let path = match request.period {
            RhythmReportPeriod::Daily => self
                .storage
                .paths()
                .daily_report_file(&request.date_or_week),
            RhythmReportPeriod::Weekly => self
                .storage
                .paths()
                .weekly_report_file(&request.date_or_week),
        };
        let markdown = render_report_markdown(&request);
        atomic_write(&path, markdown.as_bytes())?;
        Ok(RhythmReportWriteResult {
            path,
            bytes_written: markdown.len(),
            evidence_ref_count: request.content.evidence_refs.len().min(MAX_EVIDENCE_REFS),
        })
    }

    pub fn write_daily_report(
        &self,
        date: impl Into<String>,
        generated_at: DateTime<Utc>,
        content: RhythmReportContent,
    ) -> Result<RhythmReportWriteResult> {
        self.write_report(RhythmReportWriteRequest {
            period: RhythmReportPeriod::Daily,
            date_or_week: date.into(),
            generated_at,
            content,
        })
    }

    pub fn write_weekly_report(
        &self,
        week: impl Into<String>,
        generated_at: DateTime<Utc>,
        content: RhythmReportContent,
    ) -> Result<RhythmReportWriteResult> {
        self.write_report(RhythmReportWriteRequest {
            period: RhythmReportPeriod::Weekly,
            date_or_week: week.into(),
            generated_at,
            content,
        })
    }
}

fn validate_report_request(request: &RhythmReportWriteRequest) -> Result<()> {
    match request.period {
        RhythmReportPeriod::Daily => validate_daily_key(&request.date_or_week)?,
        RhythmReportPeriod::Weekly => validate_weekly_key(&request.date_or_week)?,
    }
    if request.content.evidence_refs.is_empty() {
        return Err(AutoDreamError::InvalidState(
            "rhythm reports require at least one evidence ref".to_string(),
        ));
    }
    if request.content.title.trim().is_empty() {
        return Err(AutoDreamError::InvalidState(
            "rhythm report title cannot be empty".to_string(),
        ));
    }
    Ok(())
}

fn validate_daily_key(key: &str) -> Result<()> {
    if key.contains('/') || key.contains('\\') || key.contains("..") {
        return Err(AutoDreamError::InvalidState(
            "daily report date cannot contain path traversal".to_string(),
        ));
    }
    NaiveDate::parse_from_str(key, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| {
            AutoDreamError::InvalidState("daily report date must be YYYY-MM-DD".to_string())
        })
}

fn validate_weekly_key(key: &str) -> Result<()> {
    if key.contains('/') || key.contains('\\') || key.contains("..") {
        return Err(AutoDreamError::InvalidState(
            "weekly report key cannot contain path traversal".to_string(),
        ));
    }
    let Some((year, week)) = key.split_once("-W") else {
        return Err(AutoDreamError::InvalidState(
            "weekly report key must be YYYY-Www".to_string(),
        ));
    };
    if year.len() != 4 || week.len() != 2 {
        return Err(AutoDreamError::InvalidState(
            "weekly report key must be YYYY-Www".to_string(),
        ));
    }
    let year = year
        .parse::<i32>()
        .map_err(|_| AutoDreamError::InvalidState("weekly report year is invalid".to_string()))?;
    let week = week
        .parse::<u32>()
        .map_err(|_| AutoDreamError::InvalidState("weekly report week is invalid".to_string()))?;
    if !(1..=53).contains(&week) || year < 1970 {
        return Err(AutoDreamError::InvalidState(
            "weekly report week is outside supported range".to_string(),
        ));
    }
    Ok(())
}

fn render_report_markdown(request: &RhythmReportWriteRequest) -> String {
    let mut markdown = String::new();
    markdown.push_str("---\n");
    markdown.push_str(&format!("period: {}\n", request.period.as_str()));
    match request.period {
        RhythmReportPeriod::Daily => {
            markdown.push_str(&format!("date: {}\n", request.date_or_week));
        }
        RhythmReportPeriod::Weekly => {
            markdown.push_str(&format!("week: {}\n", request.date_or_week));
        }
    }
    markdown.push_str(&format!(
        "generated_at: {}\n",
        request.generated_at.to_rfc3339()
    ));
    markdown.push_str(&format!("generated_by: {GENERATED_BY}\n"));
    if let Some(source) = request.content.source.as_deref() {
        markdown.push_str(&format!("source: {source}\n"));
    }
    if let Some(session_count) = request.content.session_count {
        markdown.push_str(&format!("session_count: {session_count}\n"));
    }
    if let Some(token_used) = request.content.token_used {
        markdown.push_str(&format!("token_used: {token_used}\n"));
    }
    if let Some(fallback_used) = request.content.fallback_used {
        markdown.push_str(&format!("fallback_used: {fallback_used}\n"));
    }
    if let Some(daily_inputs_count) = request.content.daily_inputs_count {
        markdown.push_str(&format!("daily_inputs_count: {daily_inputs_count}\n"));
    }
    if let Some(missing_daily_dates_count) = request.content.missing_daily_dates_count {
        markdown.push_str(&format!(
            "missing_daily_dates_count: {missing_daily_dates_count}\n"
        ));
    }
    if let Some(generation_mode) = request.content.generation_mode.as_deref() {
        markdown.push_str(&format!("generation_mode: {generation_mode}\n"));
    }
    if let Some(narrative_schema_version) = request.content.narrative_schema_version {
        markdown.push_str(&format!(
            "narrative_schema_version: {narrative_schema_version}\n"
        ));
    }
    if let Some(prompt_version) = request.content.prompt_version.as_deref() {
        markdown.push_str(&format!("prompt_version: {prompt_version}\n"));
    }
    if let Some(coverage_status) = request.content.coverage_status.as_deref() {
        markdown.push_str(&format!("coverage_status: {coverage_status}\n"));
    }
    if let Some(fallback_reason) = request.content.fallback_reason.as_deref() {
        markdown.push_str(&format!("fallback_reason: {fallback_reason}\n"));
    }
    markdown.push_str(&format!("schema_version: {SCHEMA_VERSION}\n"));
    markdown.push_str("---\n\n");
    markdown.push_str(&format!("# {}\n\n", request.content.title.trim()));
    markdown.push_str(request.content.summary.trim());
    markdown.push_str("\n\n");
    for section in request
        .content
        .sections
        .iter()
        .filter(|section| !section.trim().is_empty())
    {
        markdown.push_str(section.trim());
        markdown.push_str("\n\n");
    }
    markdown.push_str("## Evidence References\n\n");
    for evidence in request.content.evidence_refs.iter().take(MAX_EVIDENCE_REFS) {
        markdown.push_str(&format!("- id: `{}`\n", evidence.id));
        markdown.push_str(&format!("  source: `{:?}`\n", evidence.source));
        markdown.push_str(&format!("  uri: `{}`\n", evidence.uri));
        if let Some(hash) = &evidence.hash {
            markdown.push_str(&format!("  hash: `{hash}`\n"));
        }
        if let Some(excerpt) = &evidence.excerpt {
            markdown.push_str(&format!("  excerpt: {}\n", bounded_excerpt(excerpt)));
        }
    }
    markdown
}

fn bounded_excerpt(excerpt: &str) -> String {
    let trimmed = excerpt.trim();
    let mut bounded: String = trimmed.chars().take(MAX_EXCERPT_CHARS).collect();
    if trimmed.chars().count() > MAX_EXCERPT_CHARS {
        bounded.push_str("...");
    }
    bounded.replace('\n', " ")
}

use std::collections::{BTreeSet, HashSet};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::evolution::EvidenceRef;

use super::{
    digest::{estimate_tokens, sanitize_id_component, truncate_chars, SessionWindowDigest},
    frontmatter::RhythmReportDocument,
    period::ReportWindow,
};

const DEFAULT_MAX_FACTS: usize = 40;
const DEFAULT_MAX_FACT_CHARS: usize = 320;
const DEFAULT_MAX_INPUT_CHARS: usize = 12_000;

/// Kind of atomic report fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportFactKind {
    SessionActivity,
    DailySummary,
    GapSession,
    CoverageNote,
}

/// One auditable fact sent to the narrative generator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportFact {
    pub id: String,
    pub kind: ReportFactKind,
    pub source_label: String,
    pub occurred_at: Option<DateTime<Utc>>,
    pub content: String,
    pub evidence_id: String,
    pub priority_score: u32,
}

/// Source coverage statistics retained for data-口径 sections.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ReportSourceCoverage {
    pub session_count: u64,
    pub message_count: u64,
    pub estimated_tokens: u64,
    pub daily_inputs_count: u64,
    pub missing_daily_dates_count: u64,
    pub fallback_session_count: u64,
    pub truncated_fact_count: u64,
    pub filtered_noise_count: u64,
}

/// Bounded, deduplicated fact package for LLM curation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportFactBundle {
    pub window: ReportWindow,
    pub facts: Vec<ReportFact>,
    pub evidence_refs: Vec<EvidenceRef>,
    pub coverage: ReportSourceCoverage,
    pub language: String,
}

impl ReportFactBundle {
    pub fn evidence_ids(&self) -> HashSet<String> {
        self.evidence_refs
            .iter()
            .map(|evidence| evidence.id.clone())
            .collect()
    }

    pub fn estimated_input_chars(&self) -> usize {
        self.facts
            .iter()
            .map(|fact| fact.content.chars().count() + fact.source_label.chars().count() + 16)
            .sum()
    }
}

/// Options controlling fact package size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactBundleLimits {
    pub max_facts: usize,
    pub max_fact_chars: usize,
    pub max_input_chars: usize,
}

impl Default for FactBundleLimits {
    fn default() -> Self {
        Self {
            max_facts: DEFAULT_MAX_FACTS,
            max_fact_chars: DEFAULT_MAX_FACT_CHARS,
            max_input_chars: DEFAULT_MAX_INPUT_CHARS,
        }
    }
}

/// Build a daily fact bundle from a session window digest.
pub fn build_daily_fact_bundle(
    date: NaiveDate,
    digest: &SessionWindowDigest,
    language: impl Into<String>,
    limits: FactBundleLimits,
) -> ReportFactBundle {
    let mut filtered_noise = 0u64;
    let mut seen_content = HashSet::new();
    let mut facts = Vec::new();
    let mut evidence_refs = Vec::new();

    for item in &digest.items {
        if is_low_signal_summary(&item.summary) {
            filtered_noise += 1;
            // Keep coverage note but still retain evidence for audit.
            evidence_refs.push(item.evidence.clone());
            continue;
        }
        let normalized = normalize_for_dedupe(&item.summary);
        if !seen_content.insert(normalized) {
            filtered_noise += 1;
            evidence_refs.push(item.evidence.clone());
            continue;
        }

        let content = truncate_chars(&item.summary, limits.max_fact_chars);
        let priority = priority_for_content(&content);
        facts.push(ReportFact {
            id: format!("fact-{}", item.evidence.id),
            kind: ReportFactKind::SessionActivity,
            source_label: "session".to_string(),
            occurred_at: Some(item.first_timestamp),
            content,
            evidence_id: item.evidence.id.clone(),
            priority_score: priority,
        });
        evidence_refs.push(item.evidence.clone());
    }

    // If everything was filtered as noise, keep short facts so the model can say
    // "no substantive work observed" with valid evidence ids.
    if facts.is_empty() && !digest.items.is_empty() {
        for item in digest.items.iter().take(limits.max_facts.min(5)) {
            facts.push(ReportFact {
                id: format!("fact-{}", item.evidence.id),
                kind: ReportFactKind::SessionActivity,
                source_label: "session".to_string(),
                occurred_at: Some(item.first_timestamp),
                content: truncate_chars(&item.summary, limits.max_fact_chars),
                evidence_id: item.evidence.id.clone(),
                priority_score: 1,
            });
            if !evidence_refs.iter().any(|e| e.id == item.evidence.id) {
                evidence_refs.push(item.evidence.clone());
            }
        }
    }

    let (facts, truncated) = apply_limits(facts, limits);
    let coverage = ReportSourceCoverage {
        session_count: digest.session_count,
        message_count: digest.message_count as u64,
        estimated_tokens: digest.estimated_tokens,
        daily_inputs_count: 0,
        missing_daily_dates_count: 0,
        fallback_session_count: 0,
        truncated_fact_count: truncated,
        filtered_noise_count: filtered_noise,
    };

    ReportFactBundle {
        window: ReportWindow::daily(date),
        facts,
        evidence_refs,
        coverage,
        language: language.into(),
    }
}

/// Lower-period report input for weekly/monthly aggregation.
#[derive(Debug, Clone)]
pub struct DailyReportInput {
    pub date: NaiveDate,
    pub path_display: String,
    pub document: RhythmReportDocument,
    pub evidence: EvidenceRef,
}

/// Build weekly or monthly fact bundle from daily reports + gap sessions.
pub fn build_aggregate_fact_bundle(
    window: ReportWindow,
    daily_inputs: &[DailyReportInput],
    fallback_digest: &SessionWindowDigest,
    missing_daily_dates: usize,
    language: impl Into<String>,
    limits: FactBundleLimits,
) -> ReportFactBundle {
    let mut facts = Vec::new();
    let mut evidence_refs = Vec::new();
    let mut session_count = fallback_digest.session_count;
    let mut estimated_tokens = fallback_digest.estimated_tokens;
    let mut seen = HashSet::new();

    for daily in daily_inputs {
        session_count += daily.document.frontmatter.session_count.unwrap_or(0);
        estimated_tokens += daily
            .document
            .frontmatter
            .token_used
            .unwrap_or_else(|| estimate_tokens(&daily.document.body));
        evidence_refs.push(daily.evidence.clone());

        let summary = daily.document.summary.trim();
        if summary.is_empty() {
            continue;
        }
        let normalized = normalize_for_dedupe(summary);
        if !seen.insert(normalized) {
            continue;
        }
        facts.push(ReportFact {
            id: format!("fact-{}", daily.evidence.id),
            kind: ReportFactKind::DailySummary,
            source_label: daily.date.format("%Y-%m-%d").to_string(),
            occurred_at: None,
            content: truncate_chars(summary, limits.max_fact_chars),
            evidence_id: daily.evidence.id.clone(),
            priority_score: priority_for_content(summary).saturating_add(2),
        });
    }

    for item in &fallback_digest.items {
        if is_low_signal_summary(&item.summary) {
            evidence_refs.push(item.evidence.clone());
            continue;
        }
        let normalized = normalize_for_dedupe(&item.summary);
        if !seen.insert(normalized) {
            evidence_refs.push(item.evidence.clone());
            continue;
        }
        evidence_refs.push(item.evidence.clone());
        facts.push(ReportFact {
            id: format!("fact-{}", item.evidence.id),
            kind: ReportFactKind::GapSession,
            source_label: "gap_session".to_string(),
            occurred_at: Some(item.first_timestamp),
            content: truncate_chars(&item.summary, limits.max_fact_chars),
            evidence_id: item.evidence.id.clone(),
            priority_score: priority_for_content(&item.summary),
        });
    }

    if facts.is_empty() && !fallback_digest.items.is_empty() {
        for item in fallback_digest.items.iter().take(5) {
            facts.push(ReportFact {
                id: format!("fact-{}", item.evidence.id),
                kind: ReportFactKind::GapSession,
                source_label: "gap_session".to_string(),
                occurred_at: Some(item.first_timestamp),
                content: truncate_chars(&item.summary, limits.max_fact_chars),
                evidence_id: item.evidence.id.clone(),
                priority_score: 1,
            });
            if !evidence_refs.iter().any(|e| e.id == item.evidence.id) {
                evidence_refs.push(item.evidence.clone());
            }
        }
    }

    let (facts, truncated) = apply_limits(facts, limits);
    let coverage = ReportSourceCoverage {
        session_count,
        message_count: fallback_digest.message_count as u64,
        estimated_tokens,
        daily_inputs_count: daily_inputs.len() as u64,
        missing_daily_dates_count: missing_daily_dates as u64,
        fallback_session_count: fallback_digest.session_count,
        truncated_fact_count: truncated,
        filtered_noise_count: 0,
    };

    ReportFactBundle {
        window,
        facts,
        evidence_refs,
        coverage,
        language: language.into(),
    }
}

/// Helper to build a daily-report evidence ref for aggregate inputs.
pub fn daily_report_evidence(date: NaiveDate, path_display: &str, summary: &str) -> EvidenceRef {
    EvidenceRef {
        id: format!("daily-{}", date.format("%Y-%m-%d")),
        source: crate::evolution::EvidenceSource::Report,
        uri: format!("report://{path_display}"),
        excerpt: Some(truncate_chars(summary, 240)),
        hash: None,
        created_at: Utc::now(),
    }
}

fn apply_limits(mut facts: Vec<ReportFact>, limits: FactBundleLimits) -> (Vec<ReportFact>, u64) {
    facts.sort_by(|left, right| {
        right
            .priority_score
            .cmp(&left.priority_score)
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut truncated = 0u64;
    if facts.len() > limits.max_facts {
        truncated += (facts.len() - limits.max_facts) as u64;
        facts.truncate(limits.max_facts);
    }

    // Soft char budget: drop lowest-priority remaining facts first.
    let mut total_chars = facts
        .iter()
        .map(|fact| fact.content.chars().count())
        .sum::<usize>();
    while total_chars > limits.max_input_chars && facts.len() > 1 {
        if let Some(removed) = facts.pop() {
            total_chars = total_chars.saturating_sub(removed.content.chars().count());
            truncated += 1;
        } else {
            break;
        }
    }

    // Stable chronological-ish order for the model: by occurred_at then id.
    facts.sort_by(|left, right| {
        left.occurred_at
            .cmp(&right.occurred_at)
            .then_with(|| left.id.cmp(&right.id))
    });

    (facts, truncated)
}

fn normalize_for_dedupe(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_low_signal_summary(summary: &str) -> bool {
    let trimmed = summary.trim();
    if trimmed.is_empty() {
        return true;
    }
    let lower = trimmed.to_lowercase();
    let greetings = [
        "你好",
        "您好",
        "hello",
        "hi ",
        "hi!",
        "hey",
        "欢迎",
        "good morning",
        "good afternoon",
        "good evening",
        "有什么可以帮",
        "how can i help",
        "what can i help",
    ];
    if greetings.iter().any(|g| lower.contains(g)) && trimmed.chars().count() < 80 {
        return true;
    }
    let tool_catalog = [
        "available tools",
        "tool catalog",
        "工具列表",
        "可用工具",
        "functions:",
        "you can use the following tools",
    ];
    if tool_catalog.iter().any(|g| lower.contains(g)) {
        return true;
    }
    false
}

fn priority_for_content(content: &str) -> u32 {
    let lower = content.to_lowercase();
    let mut score = 2u32;
    for keyword in [
        "决定",
        "decision",
        "阻塞",
        "blocker",
        "风险",
        "risk",
        "完成",
        "done",
        "fix",
        "实现",
        "implement",
        "计划",
        "plan",
        "下周",
        "next",
        "todo",
        "bug",
        "发布",
        "release",
    ] {
        if lower.contains(keyword) {
            score = score.saturating_add(2);
        }
    }
    let len = content.chars().count() as u32;
    score.saturating_add(len.min(40) / 10)
}

/// Dates covered by an ISO week starting Monday.
pub fn dates_in_iso_week(start_monday: NaiveDate) -> BTreeSet<NaiveDate> {
    use chrono::Days;
    let mut dates = BTreeSet::new();
    for offset in 0..7 {
        if let Some(date) = start_monday.checked_add_days(Days::new(offset)) {
            dates.insert(date);
        }
    }
    dates
}

/// Convenience re-export helper for id sanitization used by callers.
pub fn sanitize_report_id_component(value: &str) -> String {
    sanitize_id_component(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evolution::{EvidenceRef, EvidenceSource};
    use chrono::TimeZone;

    fn session_item(key: &str, summary: &str) -> crate::reports::SessionDigestItem {
        let evidence = EvidenceRef {
            id: format!("session-{}", sanitize_id_component(key)),
            source: EvidenceSource::Session,
            uri: format!("session://{key}"),
            excerpt: Some(summary.to_string()),
            hash: None,
            created_at: Utc.with_ymd_and_hms(2026, 7, 12, 1, 0, 0).unwrap(),
        };
        crate::reports::SessionDigestItem {
            session_key: key.to_string(),
            first_timestamp: evidence.created_at,
            summary: summary.to_string(),
            message_count: 2,
            estimated_tokens: 20,
            evidence,
        }
    }

    #[test]
    fn filters_greeting_noise_but_keeps_evidence() {
        let digest = SessionWindowDigest {
            items: vec![
                session_item("a", "你好，有什么可以帮你的吗？"),
                session_item("b", "完成了报告系统 LLM 归纳改造与 evidence 校验。"),
            ],
            session_count: 2,
            estimated_tokens: 40,
            message_count: 4,
        };
        let bundle = build_daily_fact_bundle(
            NaiveDate::from_ymd_opt(2026, 7, 12).unwrap(),
            &digest,
            "zh-CN",
            FactBundleLimits::default(),
        );
        assert_eq!(bundle.facts.len(), 1);
        assert!(bundle.facts[0].content.contains("LLM"));
        assert_eq!(bundle.evidence_refs.len(), 2);
        assert!(bundle.coverage.filtered_noise_count >= 1);
    }

    #[test]
    fn keeps_low_signal_only_day_with_valid_evidence() {
        let digest = SessionWindowDigest {
            items: vec![
                session_item("a", "你好"),
                session_item("b", "Hello! How can I help?"),
            ],
            session_count: 2,
            estimated_tokens: 10,
            message_count: 2,
        };
        let bundle = build_daily_fact_bundle(
            NaiveDate::from_ymd_opt(2026, 7, 12).unwrap(),
            &digest,
            "zh-CN",
            FactBundleLimits::default(),
        );
        assert!(!bundle.facts.is_empty());
        assert!(!bundle.evidence_refs.is_empty());
        for fact in &bundle.facts {
            assert!(bundle.evidence_ids().contains(&fact.evidence_id));
        }
    }
}

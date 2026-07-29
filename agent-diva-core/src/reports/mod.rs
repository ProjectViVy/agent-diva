//! Rhythm / notebook report contracts: digests, fact bundles, curated narratives.

mod digest;
mod fact_bundle;
mod frontmatter;
mod generator;
mod narrative;
mod period;
mod render;
mod validate;

pub use digest::{
    collect_session_window_digest_for_dates, estimate_tokens, parse_rhythm_report,
    read_rhythm_report, SessionDigestItem, SessionWindowDigest,
};
pub use fact_bundle::{
    build_aggregate_fact_bundle, build_daily_fact_bundle, daily_report_evidence, dates_in_iso_week,
    sanitize_report_id_component, DailyReportInput, FactBundleLimits, ReportFact, ReportFactBundle,
    ReportFactKind, ReportSourceCoverage,
};
pub use frontmatter::{RhythmReportDocument, RhythmReportFrontmatter};
pub use generator::{ReportNarrativeError, ReportNarrativeGenerator, ReportNarrativeOptions};
pub use narrative::{
    CoverageStatus, CuratedReportNarrative, GenerationMode, NarrativeItem,
    ReportGenerationMetadata, NARRATIVE_SCHEMA_VERSION, PROMPT_VERSION,
};
pub use period::{ReportPeriod, ReportWindow};
pub use render::{render_curated_body, render_deterministic_fallback, RenderedReportBody};
pub use validate::{validate_curated_narrative, NarrativeValidationError};

#[cfg(test)]
mod tests {
    use super::{collect_session_window_digest_for_dates, parse_rhythm_report};
    use crate::session::{ChatMessage, Session, SessionManager};
    use chrono::{NaiveDate, TimeZone, Utc};
    use std::collections::BTreeSet;

    #[test]
    fn parses_rhythm_report_document() {
        let document = parse_rhythm_report(
            "---
period: daily
date: 2026-06-18
generated_by: agent-diva-autodream
session_count: 2
token_used: 42
generation_mode: llm_curated
---

# Daily Report

Summary line.

## Details

- item
",
        )
        .unwrap();

        assert_eq!(document.frontmatter.period.as_deref(), Some("daily"));
        assert_eq!(document.frontmatter.session_count, Some(2));
        assert_eq!(
            document.frontmatter.generation_mode.as_deref(),
            Some("llm_curated")
        );
        assert_eq!(document.title, "Daily Report");
        assert_eq!(document.summary, "Summary line.");
    }

    #[test]
    fn collects_session_digest_for_selected_dates() {
        let temp = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(temp.path());
        let mut session = Session::new("telegram:1");
        session.add_full_message(ChatMessage {
            role: "user".to_string(),
            content: "daily digest message".to_string(),
            timestamp: Utc.with_ymd_and_hms(2026, 6, 18, 1, 0, 0).unwrap(),
            tool_call_id: None,
            tool_calls: None,
            name: None,
            reasoning_content: None,
            thinking_blocks: None,
            metadata: None,
            token_usage: None,
        });
        manager.save(&session).unwrap();

        let mut dates = BTreeSet::new();
        dates.insert(NaiveDate::from_ymd_opt(2026, 6, 18).unwrap());
        let digest = collect_session_window_digest_for_dates(temp.path(), &dates).unwrap();
        assert_eq!(digest.session_count, 1);
        assert_eq!(digest.message_count, 1);
        assert_eq!(digest.items[0].session_key, "telegram:1");
    }
}

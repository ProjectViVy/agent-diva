use std::fs;

use agent_diva_autodream::{
    AutoDreamReportWriter, AutoDreamStorage, RhythmReportContent, RhythmReportPeriod,
    RhythmReportWriteRequest,
};
use agent_diva_core::evolution::{EvidenceRef, EvidenceSource};
use chrono::{TimeZone, Utc};
use std::str::FromStr;

fn sample_time() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 6, 14, 10, 0, 0).unwrap()
}

fn sample_evidence(id: &str) -> EvidenceRef {
    EvidenceRef {
        id: id.to_string(),
        source: EvidenceSource::AutoDreamRun,
        uri: format!("autodream://runs/run-123/evidence/{id}"),
        excerpt: Some("bounded evidence suitable for later proposal creation".to_string()),
        hash: Some("sha256:abc".to_string()),
        created_at: sample_time(),
    }
}

fn sample_content() -> RhythmReportContent {
    RhythmReportContent {
        title: "Daily Reflection".to_string(),
        summary: "The run summarized recent evidence without applying authority writes."
            .to_string(),
        sections: vec!["## Signals\n\n- User prefers durable governance boundaries.".to_string()],
        evidence_refs: vec![sample_evidence("evidence-1")],
        source: Some("session_aggregate".to_string()),
        session_count: Some(1),
        token_used: Some(32),
        fallback_used: Some(false),
        daily_inputs_count: Some(0),
        missing_daily_dates_count: Some(0),
    }
}

#[test]
fn writes_daily_report_to_contract_path_with_frontmatter_and_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let writer = AutoDreamReportWriter::new(storage);

    let result = writer
        .write_daily_report("2026-06-14", sample_time(), sample_content())
        .unwrap();

    let expected = temp
        .path()
        .join(".agent-diva/autodream/reports/daily/2026-06-14.md");
    assert_eq!(result.path, expected);
    assert_eq!(result.evidence_ref_count, 1);
    let markdown = fs::read_to_string(expected).unwrap();
    assert!(markdown.contains("period: daily"));
    assert!(markdown.contains("date: 2026-06-14"));
    assert!(markdown.contains("generated_at: 2026-06-14T10:00:00+00:00"));
    assert!(markdown.contains("generated_by: agent-diva-autodream"));
    assert!(markdown.contains("source: session_aggregate"));
    assert!(markdown.contains("session_count: 1"));
    assert!(markdown.contains("token_used: 32"));
    assert!(markdown.contains("schema_version: 1"));
    assert!(markdown.contains("## Evidence References"));
    assert!(markdown.contains("autodream://runs/run-123/evidence/evidence-1"));
}

#[test]
fn writes_weekly_report_to_contract_path_with_week_frontmatter() {
    let temp = tempfile::tempdir().unwrap();
    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let writer = AutoDreamReportWriter::new(storage);

    let result = writer
        .write_weekly_report("2026-W24", sample_time(), sample_content())
        .unwrap();

    let expected = temp
        .path()
        .join(".agent-diva/autodream/reports/weekly/2026-W24.md");
    assert_eq!(result.path, expected);
    let markdown = fs::read_to_string(expected).unwrap();
    assert!(markdown.contains("period: weekly"));
    assert!(markdown.contains("week: 2026-W24"));
    assert!(!markdown.contains("date: 2026-W24"));
}

#[test]
fn replaces_existing_report_atomically_without_temp_file_leftover() {
    let temp = tempfile::tempdir().unwrap();
    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let writer = AutoDreamReportWriter::new(storage);
    let path = temp
        .path()
        .join(".agent-diva/autodream/reports/daily/2026-06-14.md");

    writer
        .write_daily_report("2026-06-14", sample_time(), sample_content())
        .unwrap();
    let mut replacement = sample_content();
    replacement.summary = "Replacement summary".to_string();
    writer
        .write_daily_report("2026-06-14", sample_time(), replacement)
        .unwrap();

    let markdown = fs::read_to_string(&path).unwrap();
    assert!(markdown.contains("Replacement summary"));
    assert!(!markdown.contains("The run summarized recent evidence"));
    assert!(!path.with_file_name(".2026-06-14.md.tmp").exists());
}

#[test]
fn rejects_monthly_period_and_path_traversal_keys() {
    let temp = tempfile::tempdir().unwrap();
    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let writer = AutoDreamReportWriter::new(storage);

    let monthly_period = RhythmReportPeriod::from_str("monthly");
    assert!(monthly_period
        .unwrap_err()
        .to_string()
        .contains("unsupported rhythm report period: monthly"));

    let monthly = writer.write_report(RhythmReportWriteRequest {
        period: RhythmReportPeriod::Weekly,
        date_or_week: "2026-06".to_string(),
        generated_at: sample_time(),
        content: sample_content(),
    });
    assert!(monthly
        .unwrap_err()
        .to_string()
        .contains("weekly report key must be YYYY-Www"));

    let traversal = writer.write_daily_report("../2026-06-14", sample_time(), sample_content());
    assert!(traversal
        .unwrap_err()
        .to_string()
        .contains("path traversal"));
    assert!(!temp.path().join("2026-06-14.md").exists());
}

#[test]
fn report_system_can_read_markdown_without_autodream_types() {
    let temp = tempfile::tempdir().unwrap();
    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let writer = AutoDreamReportWriter::new(storage);
    writer
        .write_daily_report("2026-06-14", sample_time(), sample_content())
        .unwrap();

    let report_system_path = temp
        .path()
        .join(".agent-diva/autodream/reports/daily/2026-06-14.md");
    let read_by_consumer = fs::read_to_string(report_system_path).unwrap();
    assert!(read_by_consumer.starts_with("---\nperiod: daily"));
    assert!(read_by_consumer.contains("# Daily Reflection"));
    assert!(!temp.path().join(".agent-diva/reports/monthly").exists());
}

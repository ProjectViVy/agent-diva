//! E2E tracer — writes structured trace files for each scenario run.
//!
//! Each scenario run produces a single JSON trace file containing the
//! scenario name, pass/fail status, duration, assertion results, and
//! event metadata. These traces are the raw material for [`crate::report`]
//! aggregation and flaky detection.

use crate::assertions::AssertionResult;
use crate::collector::CollectedEvents;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// A single trace entry for one scenario run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    /// Name of the scenario (derived from file stem or YAML).
    pub scenario: String,
    /// RFC 3339 timestamp when the trace was written.
    pub timestamp: String,
    /// Wall-clock duration of the scenario run in milliseconds.
    pub duration_ms: u64,
    /// Whether all assertions passed.
    pub passed: bool,
    /// Individual assertion results.
    pub assertions: Vec<AssertionResult>,
    /// Error messages collected during the run.
    pub errors: Vec<String>,
    /// Number of events in the full event timeline.
    pub event_count: usize,
    /// Number of tool calls recorded.
    pub tool_call_count: usize,
}

/// Writes trace JSON files for each scenario run.
///
/// Traces are stored under `trace_dir` with filenames like
/// `{scenario_name}_{timestamp}.json`.
pub struct E2ETracer {
    trace_dir: PathBuf,
}

impl E2ETracer {
    /// Create a new tracer that writes traces into `trace_dir`.
    ///
    /// The directory is created if it does not exist.
    pub fn new(trace_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&trace_dir).ok();
        Self { trace_dir }
    }

    /// Write a trace JSON file for a completed scenario run.
    ///
    /// Returns the path to the written file on success.
    pub fn write_trace(
        &self,
        scenario_name: &str,
        passed: bool,
        assertions: &[AssertionResult],
        events: &CollectedEvents,
        duration: Duration,
    ) -> std::io::Result<PathBuf> {
        use chrono::Utc;

        let entry = TraceEntry {
            scenario: scenario_name.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            duration_ms: duration.as_millis() as u64,
            passed,
            assertions: assertions.to_vec(),
            errors: events.errors.clone(),
            event_count: events.timeline.len(),
            tool_call_count: events.tool_calls.len(),
        };

        let filename = format!(
            "{}_{}_{}.json",
            scenario_name.replace(' ', "_"),
            Utc::now().format("%Y%m%d_%H%M%S"),
            &uuid::Uuid::new_v4().to_string()[..8]
        );
        let path = self.trace_dir.join(filename);
        let json = serde_json::to_string_pretty(&entry)?;
        std::fs::write(&path, json)?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assertions::AssertionResult;

    /// Helper: build a minimal CollectedEvents for testing.
    fn make_events(error_count: usize, tool_call_count: usize) -> CollectedEvents {
        CollectedEvents {
            errors: (0..error_count).map(|i| format!("error_{i}")).collect(),
            tool_calls: (0..tool_call_count)
                .map(|i| crate::collector::ToolCallRecord {
                    tool_name: format!("tool_{i}"),
                    input: None,
                    result: None,
                    is_error: None,
                })
                .collect(),
            timeline: vec![], // populated via events
            ..Default::default()
        }
    }

    #[test]
    fn test_trace_file_creation() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let tracer = E2ETracer::new(dir.path().to_path_buf());

        let events = make_events(0, 2);
        let assertions = vec![AssertionResult {
            passed: true,
            description: "check output".into(),
            detail: "Found expected text".into(),
        }];

        let path = tracer
            .write_trace(
                "test_scenario",
                true,
                &assertions,
                &events,
                Duration::from_secs(1),
            )
            .expect("write_trace should succeed");

        assert!(path.exists(), "trace file should exist");
        assert_eq!(
            path.extension().unwrap(),
            "json",
            "trace should be a .json file"
        );

        // Read back and validate content
        let content = std::fs::read_to_string(&path).expect("read trace file");
        let parsed: TraceEntry = serde_json::from_str(&content).expect("parse trace JSON");

        assert_eq!(parsed.scenario, "test_scenario");
        assert!(parsed.passed);
        assert_eq!(parsed.duration_ms, 1000);
        assert_eq!(parsed.assertions.len(), 1);
        assert!(parsed.assertions[0].passed);
        assert_eq!(parsed.event_count, 0); // no timeline events
        assert_eq!(parsed.tool_call_count, 2);
        assert!(
            parsed.timestamp.contains('T'),
            "timestamp should be RFC 3339"
        );
    }

    #[test]
    fn test_trace_file_creation_failed_scenario() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let tracer = E2ETracer::new(dir.path().to_path_buf());

        let events = make_events(1, 0);
        let assertions = vec![AssertionResult {
            passed: false,
            description: "check no errors".into(),
            detail: "Found error: something went wrong".into(),
        }];

        let path = tracer
            .write_trace(
                "failing_test",
                false,
                &assertions,
                &events,
                Duration::from_millis(500),
            )
            .expect("write_trace should succeed for failed scenarios too");

        let content = std::fs::read_to_string(&path).expect("read trace file");
        let parsed: TraceEntry = serde_json::from_str(&content).expect("parse trace JSON");

        assert_eq!(parsed.scenario, "failing_test");
        assert!(!parsed.passed);
        assert_eq!(parsed.duration_ms, 500);
        assert_eq!(parsed.assertions.len(), 1);
        assert!(!parsed.assertions[0].passed);
        assert_eq!(parsed.errors.len(), 1);
        assert_eq!(parsed.errors[0], "error_0");
        assert_eq!(parsed.tool_call_count, 0);
    }

    #[test]
    fn test_trace_creates_directory_if_not_exists() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let nested = dir.path().join("nested").join("traces");
        assert!(!nested.exists(), "nested dir should not exist yet");

        let tracer = E2ETracer::new(nested.clone());
        let events = make_events(0, 0);
        let path = tracer
            .write_trace("dir_creation", true, &[], &events, Duration::ZERO)
            .expect("write_trace should create directory");

        assert!(
            path.exists(),
            "trace file should exist in newly created dir"
        );
        assert!(nested.is_dir(), "nested dir should have been created");
    }

    #[test]
    fn test_trace_content_format() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let tracer = E2ETracer::new(dir.path().to_path_buf());

        let events = make_events(0, 0);
        let assertions = vec![];
        let path = tracer
            .write_trace(
                "format_check",
                true,
                &assertions,
                &events,
                Duration::from_millis(123),
            )
            .expect("write_trace should succeed");

        let content = std::fs::read_to_string(&path).expect("read trace file");

        // Must be valid, pretty-printed JSON with expected keys
        let value: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
        assert!(value.get("scenario").is_some(), "missing 'scenario'");
        assert!(value.get("timestamp").is_some(), "missing 'timestamp'");
        assert!(value.get("duration_ms").is_some(), "missing 'duration_ms'");
        assert!(value.get("passed").is_some(), "missing 'passed'");
        assert!(value.get("assertions").is_some(), "missing 'assertions'");
        assert!(value.get("errors").is_some(), "missing 'errors'");
        assert!(value.get("event_count").is_some(), "missing 'event_count'");
        assert!(
            value.get("tool_call_count").is_some(),
            "missing 'tool_call_count'"
        );

        // Verify types
        assert_eq!(value["scenario"].as_str(), Some("format_check"));
        assert_eq!(value["passed"].as_bool(), Some(true));
        assert_eq!(value["duration_ms"].as_u64(), Some(123));
    }
}

//! E2E report generation and flaky scenario detection.
//!
//! Aggregates trace files written by [`crate::tracer::E2ETracer`] into a
//! structured [`E2EReport`] with pass/fail counts, cost estimates, and
//! automatic flaky-scenario identification (scenarios with ≥3 runs and
//! <80% pass rate).

use crate::tracer::TraceEntry;
use std::collections::HashMap;
use std::path::Path;

/// Aggregated E2E test report built from all trace files in a trace directory.
#[derive(Debug, Clone, serde::Serialize)]
pub struct E2EReport {
    /// Total number of scenario traces found.
    pub total_scenarios: usize,
    /// Number of passed scenario runs.
    pub passed: usize,
    /// Number of failed scenario runs.
    pub failed: usize,
    /// Number of skipped scenarios (always 0 — skipped scenarios don't produce traces).
    pub skipped: usize,
    /// Rough cost estimate in USD (0.002 per scenario run).
    pub total_cost_estimate_usd: f64,
    /// Names of scenarios flagged as flaky (≥3 runs with <80% pass rate).
    pub flaky_scenarios: Vec<String>,
    /// All individual scenario traces.
    pub scenarios: Vec<TraceEntry>,
}

/// Generate an [`E2EReport`] by reading all JSON trace files in `trace_dir`.
///
/// Scans `trace_dir` for `*.json` files, deserializes each as a [`TraceEntry`],
/// aggregates pass/fail counts, and runs flaky detection across scenarios with
/// 3 or more recorded runs.
///
/// # Errors
///
/// Returns an I/O error if the directory cannot be read. Missing or malformed
/// trace files are silently skipped.
pub fn generate_report(trace_dir: &Path) -> std::io::Result<E2EReport> {
    let mut scenarios = Vec::new();
    let mut flaky_counts: HashMap<String, Vec<bool>> = HashMap::new();

    if trace_dir.exists() {
        for entry in std::fs::read_dir(trace_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(trace) = serde_json::from_str::<TraceEntry>(&content) {
                        let name = trace.scenario.clone();
                        flaky_counts.entry(name).or_default().push(trace.passed);
                        scenarios.push(trace);
                    }
                }
            }
        }
    }

    let passed = scenarios.iter().filter(|s| s.passed).count();
    let failed = scenarios.len() - passed;

    let flaky_scenarios: Vec<String> = flaky_counts
        .iter()
        .filter(|(_, results)| {
            let total = results.len();
            if total >= 3 {
                let pass_count = results.iter().filter(|&&r| r).count();
                let pass_rate = pass_count as f64 / total as f64;
                pass_rate < 0.8
            } else {
                false
            }
        })
        .map(|(name, _)| name.clone())
        .collect();

    Ok(E2EReport {
        total_scenarios: scenarios.len(),
        passed,
        failed,
        skipped: 0,
        total_cost_estimate_usd: scenarios.len() as f64 * 0.002,
        flaky_scenarios,
        scenarios,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracer::E2ETracer;
    use std::time::Duration;

    /// Helper: write a trace to a directory via E2ETracer.
    fn write_trace(dir: &Path, scenario: &str, passed: bool, duration_ms: u64) {
        let tracer = E2ETracer::new(dir.to_path_buf());
        let events = crate::collector::CollectedEvents::default();
        tracer
            .write_trace(
                scenario,
                passed,
                &[],
                &events,
                Duration::from_millis(duration_ms),
            )
            .expect("write test trace");
    }

    #[test]
    fn test_generate_report_empty_directory() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let empty_dir = dir.path().join("empty");
        // Don't create it — report should handle non-existent dir gracefully
        let report = generate_report(&empty_dir).expect("report from non-existent dir");
        assert_eq!(report.total_scenarios, 0);
        assert_eq!(report.passed, 0);
        assert_eq!(report.failed, 0);
        assert!(report.flaky_scenarios.is_empty());
        assert!(report.scenarios.is_empty());
    }

    #[test]
    fn test_generate_report_empty_existing_directory() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let report = generate_report(dir.path()).expect("report from empty dir");
        assert_eq!(report.total_scenarios, 0);
        assert_eq!(report.passed, 0);
        assert_eq!(report.failed, 0);
        assert_eq!(report.total_cost_estimate_usd, 0.0);
    }

    #[test]
    fn test_generate_report_with_passing_scenarios() {
        let dir = tempfile::TempDir::new().expect("create temp dir");

        write_trace(dir.path(), "test_a", true, 100);
        write_trace(dir.path(), "test_b", true, 200);

        let report = generate_report(dir.path()).expect("report");
        assert_eq!(report.total_scenarios, 2);
        assert_eq!(report.passed, 2);
        assert_eq!(report.failed, 0);
        assert_eq!(report.total_cost_estimate_usd, 0.004);
        assert!(report.flaky_scenarios.is_empty());
    }

    #[test]
    fn test_generate_report_with_mixed_results() {
        let dir = tempfile::TempDir::new().expect("create temp dir");

        write_trace(dir.path(), "passing_test", true, 100);
        write_trace(dir.path(), "failing_test", false, 50);

        let report = generate_report(dir.path()).expect("report");
        assert_eq!(report.total_scenarios, 2);
        assert_eq!(report.passed, 1);
        assert_eq!(report.failed, 1);
        assert_eq!(report.total_cost_estimate_usd, 0.004);
    }

    #[test]
    fn test_flaky_detection_requires_three_runs() {
        let dir = tempfile::TempDir::new().expect("create temp dir");

        // Only 2 runs — not enough data to be considered flaky
        write_trace(dir.path(), "almost_flaky", true, 10);
        write_trace(dir.path(), "almost_flaky", false, 10);

        let report = generate_report(dir.path()).expect("report");
        assert!(
            report.flaky_scenarios.is_empty(),
            "2 runs should not be enough for flaky detection: {:?}",
            report.flaky_scenarios
        );
    }

    #[test]
    fn test_flaky_detection_triggers() {
        let dir = tempfile::TempDir::new().expect("create temp dir");

        // 5 runs, 3 pass (60%) — below 80% threshold, should be flaky
        for _ in 0..3 {
            write_trace(dir.path(), "flaky_boy", true, 10);
        }
        for _ in 0..2 {
            write_trace(dir.path(), "flaky_boy", false, 10);
        }

        let report = generate_report(dir.path()).expect("report");
        assert!(
            report.flaky_scenarios.contains(&"flaky_boy".to_string()),
            "flaky_boy should be flagged as flaky"
        );
    }

    #[test]
    fn test_flaky_detection_above_threshold() {
        let dir = tempfile::TempDir::new().expect("create temp dir");

        // 5 runs, 5 pass (100%) — should not be flaky
        for _ in 0..5 {
            write_trace(dir.path(), "stable_test", true, 10);
        }

        let report = generate_report(dir.path()).expect("report");
        assert!(
            !report.flaky_scenarios.contains(&"stable_test".to_string()),
            "stable_test should NOT be flagged as flaky"
        );
    }

    #[test]
    fn test_report_serialization() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        write_trace(dir.path(), "ser_test", true, 100);

        let report = generate_report(dir.path()).expect("report");
        let json = serde_json::to_string_pretty(&report).expect("serialize report");

        // Must contain key fields
        assert!(json.contains(r#""total_scenarios""#));
        assert!(json.contains(r#""flaky_scenarios""#));
        assert!(json.contains(r#""scenarios""#));
        assert!(json.contains(r#""ser_test""#));
    }

    #[test]
    fn test_report_skips_non_json_files() {
        let dir = tempfile::TempDir::new().expect("create temp dir");

        // Write a valid trace
        write_trace(dir.path(), "valid", true, 10);

        // Write a non-JSON file alongside it
        std::fs::write(dir.path().join("readme.txt"), b"not a trace").expect("write text file");

        let report = generate_report(dir.path()).expect("report");
        assert_eq!(
            report.total_scenarios, 1,
            "non-JSON files should be ignored"
        );
    }

    #[test]
    fn test_report_handles_corrupted_json_gracefully() {
        let dir = tempfile::TempDir::new().expect("create temp dir");

        // Write a valid trace
        write_trace(dir.path(), "good", true, 10);

        // Write a corrupted JSON file
        std::fs::write(dir.path().join("corrupted.json"), b"this is not valid json")
            .expect("write corrupted file");

        let report = generate_report(dir.path()).expect("report");
        assert_eq!(
            report.total_scenarios, 1,
            "corrupted JSON should be silently skipped"
        );
        assert_eq!(report.scenarios[0].scenario, "good");
    }

    #[test]
    fn test_flaky_multiple_scenarios() {
        let dir = tempfile::TempDir::new().expect("create temp dir");

        // Scenario A: 5 runs, 2 pass (40%) → flaky
        for _ in 0..2 {
            write_trace(dir.path(), "scenario_a", true, 10);
        }
        for _ in 0..3 {
            write_trace(dir.path(), "scenario_a", false, 10);
        }

        // Scenario B: 3 runs, 3 pass (100%) → not flaky
        for _ in 0..3 {
            write_trace(dir.path(), "scenario_b", true, 10);
        }

        // Scenario C: 5 runs, 4 pass (80%) → not flaky (≥80%)
        for _ in 0..4 {
            write_trace(dir.path(), "scenario_c", true, 10);
        }
        write_trace(dir.path(), "scenario_c", false, 10);

        let report = generate_report(dir.path()).expect("report");
        assert_eq!(report.total_scenarios, 13);

        assert!(
            report.flaky_scenarios.contains(&"scenario_a".to_string()),
            "scenario_a should be flaky (40%)"
        );
        assert!(
            !report.flaky_scenarios.contains(&"scenario_b".to_string()),
            "scenario_b should NOT be flaky (100%)"
        );
        assert!(
            !report.flaky_scenarios.contains(&"scenario_c".to_string()),
            "scenario_c should NOT be flaky (80% is threshold boundary)"
        );
    }
}

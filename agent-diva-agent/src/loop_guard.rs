use serde_json::Value;
use std::time::{Duration, Instant};

pub(crate) const DEFAULT_AGENT_LOOP_TIMEOUT: Duration = Duration::from_secs(300);
pub(crate) const DEFAULT_SUBAGENT_LOOP_TIMEOUT: Duration = Duration::from_secs(120);
pub(crate) const DEFAULT_REPEATED_FAILURE_THRESHOLD: usize = 3;
pub(crate) const DEFAULT_SUBAGENT_MAX_ITERATIONS: usize = 15;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LoopStopReason {
    MaxIterationsExceeded {
        max_iterations: usize,
    },
    WallClockTimeout {
        elapsed: Duration,
        timeout: Duration,
    },
    RepeatedFailedToolCall {
        tool_name: String,
        consecutive_failures: usize,
        threshold: usize,
    },
}

impl LoopStopReason {
    pub(crate) fn user_message(&self) -> String {
        match self {
            Self::MaxIterationsExceeded { max_iterations } => format!(
                "Stopped after reaching the maximum tool iterations ({max_iterations}). Try a more focused request or a different approach."
            ),
            Self::WallClockTimeout { timeout, .. } => format!(
                "Stopped after exceeding the loop time budget ({} seconds). Try a smaller task or a different approach.",
                timeout.as_secs()
            ),
            Self::RepeatedFailedToolCall {
                tool_name,
                consecutive_failures,
                ..
            } => format!(
                "Stopped after {consecutive_failures} repeated failures from tool '{tool_name}'. Try a different approach."
            ),
        }
    }
}

pub(crate) struct LoopGuard {
    max_iterations: usize,
    timeout: Duration,
    repeated_failure_threshold: usize,
    started_at: Instant,
    last_failed_fingerprint: Option<String>,
    consecutive_identical_failures: usize,
}

impl LoopGuard {
    pub(crate) fn new(
        max_iterations: usize,
        timeout: Duration,
        repeated_failure_threshold: usize,
    ) -> Self {
        Self {
            max_iterations,
            timeout,
            repeated_failure_threshold: repeated_failure_threshold.max(1),
            started_at: Instant::now(),
            last_failed_fingerprint: None,
            consecutive_identical_failures: 0,
        }
    }

    pub(crate) fn begin_iteration(
        &self,
        completed_iterations: usize,
    ) -> Result<usize, LoopStopReason> {
        self.check_elapsed()?;
        if completed_iterations >= self.max_iterations {
            return Err(LoopStopReason::MaxIterationsExceeded {
                max_iterations: self.max_iterations,
            });
        }
        Ok(completed_iterations + 1)
    }

    pub(crate) fn check_elapsed(&self) -> Result<(), LoopStopReason> {
        let elapsed = self.started_at.elapsed();
        if elapsed > self.timeout {
            return Err(LoopStopReason::WallClockTimeout {
                elapsed,
                timeout: self.timeout,
            });
        }
        Ok(())
    }

    pub(crate) fn record_tool_result(
        &mut self,
        tool_name: &str,
        arguments: &Value,
        result: &str,
    ) -> Option<LoopStopReason> {
        let fingerprint = fingerprint_tool_call(tool_name, arguments);
        if is_tool_error_result(result) {
            if self.last_failed_fingerprint.as_deref() == Some(fingerprint.as_str()) {
                self.consecutive_identical_failures += 1;
            } else {
                self.last_failed_fingerprint = Some(fingerprint);
                self.consecutive_identical_failures = 1;
            }

            if self.consecutive_identical_failures >= self.repeated_failure_threshold {
                return Some(LoopStopReason::RepeatedFailedToolCall {
                    tool_name: tool_name.to_string(),
                    consecutive_failures: self.consecutive_identical_failures,
                    threshold: self.repeated_failure_threshold,
                });
            }
        } else {
            self.last_failed_fingerprint = None;
            self.consecutive_identical_failures = 0;
        }

        None
    }
}

pub(crate) fn is_tool_error_result(result: &str) -> bool {
    result.starts_with("Error")
}

pub(crate) fn fingerprint_tool_call(tool_name: &str, arguments: &Value) -> String {
    let normalized = normalize_json(arguments);
    let normalized_json = serde_json::to_string(&normalized).unwrap_or_default();
    format!("{tool_name}:{normalized_json}")
}

fn normalize_json(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(normalize_json).collect()),
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            let mut normalized = serde_json::Map::with_capacity(entries.len());
            for (key, value) in entries {
                normalized.insert(key.clone(), normalize_json(value));
            }
            Value::Object(normalized)
        }
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_fingerprint_tool_call_ignores_object_key_order() {
        let first = json!({"b": 2, "a": 1, "nested": {"y": 2, "x": 1}});
        let second = json!({"nested": {"x": 1, "y": 2}, "a": 1, "b": 2});

        assert_eq!(
            fingerprint_tool_call("shell", &first),
            fingerprint_tool_call("shell", &second)
        );
    }

    #[test]
    fn test_fingerprint_tool_call_changes_when_arguments_change() {
        let first = json!({"command": "dir"});
        let second = json!({"command": "git status"});

        assert_ne!(
            fingerprint_tool_call("shell", &first),
            fingerprint_tool_call("shell", &second)
        );
    }

    #[test]
    fn test_loop_guard_trips_on_repeated_identical_failures() {
        let mut guard = LoopGuard::new(5, Duration::from_secs(30), 3);
        let args = json!({"command": "dir"});

        assert!(guard
            .record_tool_result("shell", &args, "Error: first failure")
            .is_none());
        assert!(guard
            .record_tool_result("shell", &args, "Error: second failure")
            .is_none());

        let reason = guard
            .record_tool_result("shell", &args, "Error: third failure")
            .expect("third identical failure should stop");

        assert_eq!(
            reason,
            LoopStopReason::RepeatedFailedToolCall {
                tool_name: "shell".to_string(),
                consecutive_failures: 3,
                threshold: 3,
            }
        );
    }

    #[test]
    fn test_loop_guard_resets_failure_streak_after_success() {
        let mut guard = LoopGuard::new(5, Duration::from_secs(30), 2);
        let args = json!({"command": "dir"});

        assert!(guard
            .record_tool_result("shell", &args, "Error: first failure")
            .is_none());
        assert!(guard.record_tool_result("shell", &args, "ok").is_none());
        assert!(guard
            .record_tool_result("shell", &args, "Error: second first failure")
            .is_none());
    }

    #[test]
    fn test_loop_guard_times_out_on_elapsed_budget() {
        let guard = LoopGuard::new(5, Duration::from_millis(1), 2);
        std::thread::sleep(Duration::from_millis(5));

        let reason = guard.check_elapsed().expect_err("guard should time out");
        assert!(matches!(reason, LoopStopReason::WallClockTimeout { .. }));
    }
}

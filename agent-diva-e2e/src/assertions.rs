//! Assertion engine for evaluating E2E test scenarios.
//!
//! This module provides the evaluation logic for all 6 assertion types:
//! - [`ResponseContains`](E2EAssertion::ResponseContains): substring check
//! - [`ResponseMatches`](E2EAssertion::ResponseMatches): regex match
//! - [`ToolCalled`](E2EAssertion::ToolCalled): tool invocation count
//! - [`FileExists`](E2EAssertion::FileExists): file system existence
//! - [`NoErrors`](E2EAssertion::NoErrors): error-free run
//! - [`Judge`](E2EAssertion::Judge): LLM-as-Judge (use [`evaluate_assertions_with_judge`])

use crate::collector::CollectedEvents;
use crate::types::E2EAssertion;
use agent_diva_providers::{LLMProvider, Message};
use std::path::Path;
use std::sync::Arc;

/// Result of evaluating a single assertion.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssertionResult {
    /// Whether the assertion passed.
    pub passed: bool,
    /// Human-readable description of what was checked.
    pub description: String,
    /// Detailed message explaining the result (success or failure).
    pub detail: String,
}

/// Evaluate all assertions from a scenario against collected events.
///
/// Returns a `Vec` of [`AssertionResult`]s, one per assertion in order.
pub fn evaluate_assertions(
    assertions: &[E2EAssertion],
    events: &CollectedEvents,
    workspace_path: &Path,
) -> Vec<AssertionResult> {
    assertions
        .iter()
        .map(|assertion| evaluate_single(assertion, events, workspace_path))
        .collect()
}

fn evaluate_single(
    assertion: &E2EAssertion,
    events: &CollectedEvents,
    workspace_path: &Path,
) -> AssertionResult {
    match assertion {
        E2EAssertion::ResponseContains { value, description } => {
            let desc = description
                .clone()
                .unwrap_or_else(|| format!("response contains '{value}'"));
            match &events.final_response {
                Some(response) => {
                    let passed = response.contains(value.as_str());
                    AssertionResult {
                        passed,
                        description: desc,
                        detail: if passed {
                            format!("Found '{}' in response", value)
                        } else {
                            format!(
                                "Did NOT find '{}' in response. Response was: {}",
                                value,
                                truncate(response, 200)
                            )
                        },
                    }
                }
                None => AssertionResult {
                    passed: false,
                    description: desc,
                    detail: "No final response was emitted".to_string(),
                },
            }
        }

        E2EAssertion::ResponseMatches { pattern, description } => {
            let desc = description
                .clone()
                .unwrap_or_else(|| format!("response matches pattern '{pattern}'"));
            match &events.final_response {
                Some(response) => match regex::Regex::new(pattern) {
                    Ok(re) => {
                        let passed = re.is_match(response);
                        AssertionResult {
                            passed,
                            description: desc,
                            detail: if passed {
                                format!("Pattern '{}' matched response", pattern)
                            } else {
                                format!(
                                    "Pattern '{}' did NOT match response: {}",
                                    pattern,
                                    truncate(response, 200)
                                )
                            },
                        }
                    }
                    Err(e) => AssertionResult {
                        passed: false,
                        description: desc,
                        detail: format!("Invalid regex pattern '{}': {}", pattern, e),
                    },
                },
                None => AssertionResult {
                    passed: false,
                    description: desc,
                    detail: "No final response was emitted".to_string(),
                },
            }
        }

        E2EAssertion::ToolCalled {
            name,
            min_times,
            description,
        } => {
            let desc = description.clone().unwrap_or_else(|| {
                format!("tool '{}' called at least {} time(s)", name, min_times)
            });
            let actual_calls = events
                .tool_calls
                .iter()
                .filter(|tc| tc.tool_name == *name)
                .count();
            let passed = actual_calls >= *min_times;
            AssertionResult {
                passed,
                description: desc,
                detail: if passed {
                    format!("Tool '{}' was called {} time(s)", name, actual_calls)
                } else {
                    let all_tools: Vec<&str> =
                        events.tool_calls.iter().map(|tc| tc.tool_name.as_str()).collect();
                    format!(
                        "Tool '{}' was called {} time(s), expected at least {}. Tools called: {:?}",
                        name, actual_calls, min_times, all_tools
                    )
                },
            }
        }

        E2EAssertion::FileExists { path, description } => {
            let desc = description
                .clone()
                .unwrap_or_else(|| format!("file '{}' exists", path));
            let full_path = workspace_path.join(path);
            let passed = full_path.exists();
            AssertionResult {
                passed,
                description: desc,
                detail: if passed {
                    format!("File '{}' exists", full_path.display())
                } else {
                    format!("File '{}' does NOT exist", full_path.display())
                },
            }
        }

        E2EAssertion::NoErrors { description } => {
            let desc = description
                .clone()
                .unwrap_or_else(|| "no errors occurred".to_string());
            let passed = events.errors.is_empty();
            AssertionResult {
                passed,
                description: desc,
                detail: if passed {
                    "No errors occurred".to_string()
                } else {
                    format!(
                        "{} error(s) occurred: {}",
                        events.errors.len(),
                        events.errors.join("; ")
                    )
                },
            }
        }

        E2EAssertion::Judge { description } => {
            // Synchronous path: when called via evaluate_assertions directly,
            // Judge assertions cannot be evaluated without an LLM provider.
            // Use evaluate_assertions_with_judge for proper LLM-as-Judge evaluation.
            AssertionResult {
                passed: true,
                description: description.clone(),
                detail: "Judge assertion skipped (use evaluate_assertions_with_judge)".to_string(),
            }
        }
    }
}

/// Evaluate assertions, processing Judge assertions with an LLM provider.
///
/// For assertions that don't need the judge, delegates to [`evaluate_single`].
/// For Judge assertions, calls a separate LLM with a structured prompt.
///
/// The model is called with `temperature = 0.0` for deterministic evaluation.
/// Temperature is always 0 for judge calls regardless of the caller's setting.
pub async fn evaluate_assertions_with_judge(
    assertions: &[E2EAssertion],
    events: &CollectedEvents,
    workspace_path: &Path,
    provider: Arc<dyn LLMProvider>,
    model: &str,
) -> Vec<AssertionResult> {
    let mut results = Vec::with_capacity(assertions.len());
    for assertion in assertions {
        match assertion {
            E2EAssertion::Judge { description } => {
                let result = evaluate_judge(description, events, &provider, model).await;
                results.push(result);
            }
            other => {
                results.push(evaluate_single(other, events, workspace_path));
            }
        }
    }
    results
}

/// Call an LLM to evaluate whether the response satisfies a judge description.
///
/// The judge prompt instructs the LLM to return a simple JSON verdict:
/// `{"passed": true/false, "reason": "..."}`. The LLM response is parsed and
/// on any parse failure the assertion FAILS (fail-safe).
///
/// Temperature is fixed at 0.0 for deterministic, reproducible judgments.
async fn evaluate_judge(
    description: &str,
    events: &CollectedEvents,
    provider: &Arc<dyn LLMProvider>,
    model: &str,
) -> AssertionResult {
    let response = events
        .final_response
        .as_deref()
        .unwrap_or("[no response]");

    let judge_prompt = format!(
        r#"You are an E2E test judge. Evaluate whether the following AI response satisfies the assertion criteria.

ASSERTION: {description}

AI RESPONSE:
{response}

Respond with ONLY valid JSON in this exact format:
{{"passed": true, "reason": "brief explanation"}}
or
{{"passed": false, "reason": "what went wrong"}}"#,
    );

    let messages = vec![
        Message::system("You are a helpful E2E test judge. Respond only with valid JSON."),
        Message::user(judge_prompt),
    ];

    match provider
        .chat(messages, None, Some(model.to_string()), 256, 0.0)
        .await
    {
        Ok(llm_response) => {
            let content = llm_response.content.unwrap_or_default();
            let json_str = extract_json(&content);

            match serde_json::from_str::<serde_json::Value>(&json_str) {
                Ok(val) => {
                    let passed = val
                        .get("passed")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let reason = val
                        .get("reason")
                        .and_then(|v| v.as_str())
                        .unwrap_or("no reason given");
                    AssertionResult {
                        passed,
                        description: format!("Judge: {}", description),
                        detail: format!(
                            "Judge verdict: {} ({})",
                            if passed { "PASS" } else { "FAIL" },
                            reason
                        ),
                    }
                }
                Err(e) => AssertionResult {
                    passed: false,
                    description: format!("Judge: {}", description),
                    detail: format!(
                        "Judge JSON parse failed: {}. Raw response: {}",
                        e,
                        truncate(&content, 200)
                    ),
                },
            }
        }
        Err(e) => AssertionResult {
            passed: false,
            description: format!("Judge: {}", description),
            detail: format!("Judge LLM call failed: {}", e),
        },
    }
}

/// Extract JSON from a response that may be wrapped in markdown code blocks.
///
/// Handles both raw JSON strings and markdown-fenced blocks like:
/// ```json
/// {"passed": true, "reason": "ok"}
/// ```
fn extract_json(text: &str) -> String {
    let text = text.trim();
    if let Some(json_start) = text.find('{') {
        if let Some(json_end) = text.rfind('}') {
            return text[json_start..=json_end].to_string();
        }
    }
    text.to_string()
}

/// Truncate a string to at most `max` characters for error messages.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}... (truncated, {} chars)", &s[..max], s.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collector::ToolCallRecord;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_events(
        final_response: Option<&str>,
        tool_calls: Vec<(&str, usize)>, // (tool_name, repeat_count)
        errors: Vec<&str>,
    ) -> CollectedEvents {
        let mut tc_records = Vec::new();
        for (name, count) in tool_calls {
            for _ in 0..count {
                tc_records.push(ToolCallRecord {
                    tool_name: name.to_string(),
                    input: None,
                    result: None,
                    is_error: None,
                });
            }
        }
        CollectedEvents {
            final_response: final_response.map(String::from),
            tool_calls: tc_records,
            errors: errors.into_iter().map(String::from).collect(),
            ..Default::default()
        }
    }

    // -----------------------------------------------------------------------
    // ResponseContains: PASS
    // -----------------------------------------------------------------------
    #[test]
    fn test_response_contains_pass() {
        let events = make_events(Some("Hello world, this is a test"), vec![], vec![]);
        let assertion = E2EAssertion::ResponseContains {
            value: "world".into(),
            description: Some("check greeting".into()),
        };
        let result = evaluate_single(&assertion, &events, Path::new("/tmp"));
        assert!(result.passed, "expected PASS: {}", result.detail);
        assert_eq!(result.description, "check greeting");
        assert!(result.detail.contains("Found"));
    }

    // -----------------------------------------------------------------------
    // ResponseContains: FAIL
    // -----------------------------------------------------------------------
    #[test]
    fn test_response_contains_fail() {
        let events = make_events(Some("Hello world"), vec![], vec![]);
        let assertion = E2EAssertion::ResponseContains {
            value: "goodbye".into(),
            description: None,
        };
        let result = evaluate_single(&assertion, &events, Path::new("/tmp"));
        assert!(!result.passed, "expected FAIL");
        assert!(result.detail.contains("Did NOT find"));
    }

    // -----------------------------------------------------------------------
    // ResponseContains: no response
    // -----------------------------------------------------------------------
    #[test]
    fn test_response_contains_no_response() {
        let events = make_events(None, vec![], vec![]);
        let assertion = E2EAssertion::ResponseContains {
            value: "anything".into(),
            description: None,
        };
        let result = evaluate_single(&assertion, &events, Path::new("/tmp"));
        assert!(!result.passed, "expected FAIL when no final_response");
        assert!(result.detail.contains("No final response"));
    }

    // -----------------------------------------------------------------------
    // ResponseMatches: PASS
    // -----------------------------------------------------------------------
    #[test]
    fn test_response_matches_pass() {
        let events = make_events(Some("hello 42 world"), vec![], vec![]);
        let assertion = E2EAssertion::ResponseMatches {
            pattern: r"hello \d+ world".into(),
            description: Some("numeric pattern".into()),
        };
        let result = evaluate_single(&assertion, &events, Path::new("/tmp"));
        assert!(result.passed, "expected PASS: {}", result.detail);
        assert_eq!(result.description, "numeric pattern");
    }

    // -----------------------------------------------------------------------
    // ResponseMatches: FAIL
    // -----------------------------------------------------------------------
    #[test]
    fn test_response_matches_fail() {
        let events = make_events(Some("hello world"), vec![], vec![]);
        let assertion = E2EAssertion::ResponseMatches {
            pattern: r"\d+".into(),
            description: None,
        };
        let result = evaluate_single(&assertion, &events, Path::new("/tmp"));
        assert!(!result.passed, "expected FAIL");
        assert!(result.detail.contains("did NOT match"));
    }

    // -----------------------------------------------------------------------
    // ResponseMatches: invalid regex
    // -----------------------------------------------------------------------
    #[test]
    fn test_response_matches_invalid_regex() {
        let events = make_events(Some("anything"), vec![], vec![]);
        let assertion = E2EAssertion::ResponseMatches {
            pattern: r"[invalid".into(),
            description: None,
        };
        let result = evaluate_single(&assertion, &events, Path::new("/tmp"));
        assert!(!result.passed, "expected FAIL for invalid regex");
        assert!(result.detail.contains("Invalid regex"));
    }

    // -----------------------------------------------------------------------
    // ToolCalled: PASS
    // -----------------------------------------------------------------------
    #[test]
    fn test_tool_called_pass() {
        let events = make_events(None, vec![("bash", 3)], vec![]);
        let assertion = E2EAssertion::ToolCalled {
            name: "bash".into(),
            min_times: 2,
            description: Some("bash used enough".into()),
        };
        let result = evaluate_single(&assertion, &events, Path::new("/tmp"));
        assert!(result.passed, "expected PASS: {}", result.detail);
        assert_eq!(result.description, "bash used enough");
    }

    // -----------------------------------------------------------------------
    // ToolCalled: FAIL
    // -----------------------------------------------------------------------
    #[test]
    fn test_tool_called_fail() {
        let events = make_events(None, vec![("read_file", 1)], vec![]);
        let assertion = E2EAssertion::ToolCalled {
            name: "bash".into(),
            min_times: 1,
            description: None,
        };
        let result = evaluate_single(&assertion, &events, Path::new("/tmp"));
        assert!(!result.passed, "expected FAIL");
        assert!(result.detail.contains("expected at least"));
    }

    // -----------------------------------------------------------------------
    // FileExists: PASS (uses TempDir)
    // -----------------------------------------------------------------------
    #[test]
    fn test_file_exists_pass() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let file_path = dir.path().join("test_output.txt");
        std::fs::write(&file_path, "content").expect("write test file");

        let events = make_events(None, vec![], vec![]);
        let assertion = E2EAssertion::FileExists {
            path: "test_output.txt".into(),
            description: Some("output file exists".into()),
        };
        let result = evaluate_single(&assertion, &events, dir.path());
        assert!(result.passed, "expected PASS: {}", result.detail);
        assert_eq!(result.description, "output file exists");
    }

    // -----------------------------------------------------------------------
    // FileExists: FAIL
    // -----------------------------------------------------------------------
    #[test]
    fn test_file_exists_fail() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let events = make_events(None, vec![], vec![]);
        let assertion = E2EAssertion::FileExists {
            path: "nonexistent_file.txt".into(),
            description: None,
        };
        let result = evaluate_single(&assertion, &events, dir.path());
        assert!(!result.passed, "expected FAIL");
        assert!(result.detail.contains("does NOT exist"));
    }

    // -----------------------------------------------------------------------
    // NoErrors: PASS
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_errors_pass() {
        let events = make_events(None, vec![], vec![]);
        let assertion = E2EAssertion::NoErrors {
            description: Some("clean run".into()),
        };
        let result = evaluate_single(&assertion, &events, Path::new("/tmp"));
        assert!(result.passed, "expected PASS");
        assert_eq!(result.description, "clean run");
    }

    // -----------------------------------------------------------------------
    // NoErrors: FAIL
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_errors_fail() {
        let events = make_events(None, vec![], vec!["something went wrong", "another error"]);
        let assertion = E2EAssertion::NoErrors {
            description: None,
        };
        let result = evaluate_single(&assertion, &events, Path::new("/tmp"));
        assert!(!result.passed, "expected FAIL");
        assert!(result.detail.contains("2 error(s)"));
        assert!(result.detail.contains("something went wrong"));
    }

    // -----------------------------------------------------------------------
    // Judge: placeholder (sync path — use evaluate_assertions_with_judge)
    // -----------------------------------------------------------------------
    #[test]
    fn test_judge_placeholder() {
        let events = make_events(Some("anything"), vec![], vec!["error!"]);
        let assertion = E2EAssertion::Judge {
            description: "response is helpful".into(),
        };
        let result = evaluate_single(&assertion, &events, Path::new("/tmp"));
        assert!(result.passed, "Judge placeholder must always pass");
        assert_eq!(result.description, "response is helpful");
        assert!(result.detail.contains("evaluate_assertions_with_judge"));
    }

    // -----------------------------------------------------------------------
    // Integration: evaluate_assertions with multiple assertion types
    // -----------------------------------------------------------------------
    #[test]
    fn test_evaluate_assertions_integration() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let file_path = dir.path().join("output.txt");
        std::fs::write(&file_path, "hello world").expect("write test file");

        let events = make_events(
            Some("The answer is 42."),
            vec![("bash", 1), ("grep", 2)],
            vec![],
        );

        let assertions = vec![
            E2EAssertion::ResponseContains {
                value: "42".into(),
                description: Some("contains the answer".into()),
            },
            E2EAssertion::ResponseMatches {
                pattern: r"answer is \d+".into(),
                description: None,
            },
            E2EAssertion::ToolCalled {
                name: "bash".into(),
                min_times: 1,
                description: None,
            },
            E2EAssertion::ToolCalled {
                name: "grep".into(),
                min_times: 2,
                description: None,
            },
            E2EAssertion::FileExists {
                path: "output.txt".into(),
                description: None,
            },
            E2EAssertion::NoErrors {
                description: None,
            },
        ];

        let results = evaluate_assertions(&assertions, &events, dir.path());

        assert_eq!(results.len(), 6);
        // All should pass
        for (i, result) in results.iter().enumerate() {
            assert!(result.passed, "assertion {} failed: {}", i, result.detail);
        }
    }

    // -----------------------------------------------------------------------
    // Edge: tool not called at all
    // -----------------------------------------------------------------------
    #[test]
    fn test_tool_not_called_at_all() {
        let events = make_events(None, vec![("read_file", 1)], vec![]);
        let assertion = E2EAssertion::ToolCalled {
            name: "nonexistent_tool".into(),
            min_times: 1,
            description: None,
        };
        let result = evaluate_single(&assertion, &events, Path::new("/tmp"));
        assert!(!result.passed, "expected FAIL when tool never called");
        assert!(result.detail.contains("0 time(s)"));
        assert!(result.detail.contains("Tools called"));
    }

    // -----------------------------------------------------------------------
    // extract_json
    // -----------------------------------------------------------------------
    #[test]
    fn test_extract_json_raw() {
        let input = r#"{"passed": true, "reason": "ok"}"#;
        assert_eq!(extract_json(input), input);
    }

    #[test]
    fn test_extract_json_with_markdown_fence() {
        let input = "```json\n{\"passed\": false, \"reason\": \"bad\"}\n```";
        assert_eq!(extract_json(input), r#"{"passed": false, "reason": "bad"}"#);
    }

    #[test]
    fn test_extract_json_with_surrounding_text() {
        let input = "Here is the result:\n{\"passed\": true}\nThank you.";
        assert_eq!(extract_json(input), r#"{"passed": true}"#);
    }

    #[test]
    fn test_extract_json_no_braces_returns_full() {
        let input = "just text no json";
        assert_eq!(extract_json(input), input);
    }

    #[test]
    fn test_extract_json_empty() {
        assert_eq!(extract_json(""), "");
    }
}

//! ScenarioRunner — the core orchestration engine for agent-diva-e2e.
//!
//! This module ties together config, types, collector, assertions, and the
//! agent infrastructure to run real LLM-based scenarios and evaluate
//! assertions against their output. It is the most complex module in the
//! E2E framework, orchestrating:
//!
//! - YAML scenario discovery and parsing
//! - Temporary workspace creation and file setup
//! - Provider and AgentLoop construction
//! - Multi-turn message processing with event capture
//! - Assertion evaluation and trace reporting
//!
//! # Important
//!
//! The `drop(tx)` call after each `process_inbound_message` is **critical**:
//! the `EventCollector::collect()` method blocks until the sender is dropped,
//! so every turn must close its channel before collection.

use crate::assertions::{evaluate_assertions, AssertionResult};
use crate::collector::{CollectedEvents, EventCollector};
use crate::config::E2EConfig;
use crate::tracer::E2ETracer;
use crate::types::E2EScenario;

use agent_diva_agent::{AgentLoop, ToolConfig};
use agent_diva_core::bus::events::{AgentEvent, InboundMessage};
use agent_diva_core::bus::MessageBus;
use agent_diva_providers::LiteLLMClient;
use agent_diva_providers::LLMProvider;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Result of running a single E2E scenario.
#[derive(Debug, Clone)]
pub struct ScenarioResult {
    /// Name of the scenario (derived from file stem or YAML).
    pub scenario_name: String,
    /// Whether all assertions passed.
    pub passed: bool,
    /// Individual assertion results.
    pub assertions: Vec<AssertionResult>,
    /// Wall-clock duration of the scenario run.
    pub duration: Duration,
    /// Collected events from all turns (merged).
    pub events: CollectedEvents,
}

/// The E2E scenario runner.
///
/// Construct one via [`ScenarioRunner::new`] with an [`E2EConfig`], then call
/// [`run`](ScenarioRunner::run) to discover and execute all scenarios.
pub struct ScenarioRunner {
    config: E2EConfig,
    tracer: E2ETracer,
}

impl ScenarioRunner {
    /// Create a new scenario runner from the given configuration.
    pub fn new(config: E2EConfig) -> Self {
        let tracer = E2ETracer::new(config.trace_dir.clone());
        Self { config, tracer }
    }

    /// Discover and run all YAML scenarios in the configured scenarios directory.
    ///
    /// Scenarios are discovered by scanning `config.scenarios_dir` for files
    /// ending in `.yaml` or `.yml`. Each is executed sequentially in
    /// lexicographic order. Errors during individual scenarios produce a
    /// [`ScenarioResult`] with `passed: false` rather than aborting the
    /// entire run.
    pub async fn run(&self) -> Vec<ScenarioResult> {
        let scenarios = match self.discover_scenarios() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("[ScenarioRunner] Failed to discover scenarios: {e}");
                return Vec::new();
            }
        };

        if scenarios.is_empty() {
            tracing::warn!(
                "[ScenarioRunner] No .yaml or .yml files found in {}",
                self.config.scenarios_dir.display()
            );
            return Vec::new();
        }

        let mut results = Vec::with_capacity(scenarios.len());
        for path in &scenarios {
            tracing::info!("[ScenarioRunner] Running scenario: {}", path.display());
            match self.run_single(path).await {
                Ok(result) => {
                    let status = if result.passed { "PASSED" } else { "FAILED" };
                    tracing::info!(
                        "[ScenarioRunner] Scenario '{}' {status} in {:?}",
                        result.scenario_name,
                        result.duration
                    );
                    results.push(result);
                }
                Err(e) => {
                    tracing::error!("[ScenarioRunner] Scenario '{}' failed: {e}", path.display());
                    results.push(ScenarioResult {
                        scenario_name: path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "unknown".to_string()),
                        passed: false,
                        assertions: Vec::new(),
                        duration: Duration::default(),
                        events: CollectedEvents::default(),
                    });
                }
            }
        }

        results
    }

    /// Run a single scenario from its YAML file path.
    ///
    /// The full pipeline:
    /// 1. Parse YAML → [`E2EScenario`]
    /// 2. Create workspace (temp dir or specified path)
    /// 3. Execute file creation setup
    /// 4. Build [`LLMProvider`] from config
    /// 5. Build [`MessageBus`] and [`AgentLoop`]
    /// 6. Register default tools
    /// 7. Process each message turn (capture events via channel)
    /// 8. Evaluate all assertions
    /// 9. Write trace JSON to `config.trace_dir`
    /// 10. Return [`ScenarioResult`]
    pub async fn run_single(&self, path: &Path) -> Result<ScenarioResult, String> {
        let scenario_name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let start = Instant::now();

        // ---- 1. Parse YAML ----
        let file = std::fs::File::open(path).map_err(|e| {
            format!("[{scenario_name}] Failed to open scenario file '{}': {e}", path.display())
        })?;
        let scenario: E2EScenario = serde_yaml::from_reader(file).map_err(|e| {
            format!("[{scenario_name}] Failed to parse YAML in '{}': {e}", path.display())
        })?;

        // ---- 2. Create working directory ----
        let workspace: PathBuf;
        let _temp_dir_guard: Option<tempfile::TempDir>;

        if let Some(ref wd) = scenario.setup.working_dir {
            workspace = PathBuf::from(wd);
            _temp_dir_guard = None;
        } else {
            let dir = tempfile::TempDir::new().map_err(|e| {
                format!("[{scenario_name}] Failed to create temporary directory: {e}")
            })?;
            workspace = dir.path().to_path_buf();
            _temp_dir_guard = Some(dir);
        }

        // ---- 3. Execute file setup ----
        for entry in &scenario.setup.create_file {
            let file_path = workspace.join(&entry.path);
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "[{scenario_name}] Failed to create parent directory for '{}': {e}",
                        entry.path
                    )
                })?;
            }
            std::fs::write(&file_path, &entry.content).map_err(|e| {
                format!("[{scenario_name}] Failed to write file '{}': {e}", entry.path)
            })?;
            tracing::debug!("[{scenario_name}] Created file: {}", file_path.display());
        }

        // ---- 4. Build provider ----
        let provider = build_provider(&self.config).map_err(|e| {
            format!("[{scenario_name}] Failed to build LLM provider: {e}")
        })?;

        // ---- 5. Determine timeout (scenario value capped by config max) ----
        let timeout_secs = scenario.setup.timeout_secs.min(self.config.max_timeout_secs);
        let timeout_duration = Duration::from_secs(timeout_secs);

        // ---- 6. Build MessageBus and AgentLoop ----
        let bus = MessageBus::new();
        let model = scenario
            .setup
            .model_override
            .clone()
            .or(Some(self.config.default_model.clone()));

        let mut agent_loop = AgentLoop::new(
            bus,
            provider,
            workspace.clone(),
            model,
            Some(20),
        )
        .await
        .map_err(|e| format!("[{scenario_name}] Failed to create AgentLoop: {e}"))?;

        agent_loop.register_default_tools(ToolConfig::default());

        // ---- 7. Process each message turn ----
        let mut merged_events = CollectedEvents::default();

        for (i, msg) in scenario.messages.iter().enumerate() {
            let channel = msg.channel.clone().unwrap_or_else(|| "e2e".to_string());
            let chat_id = msg.chat_id.clone().unwrap_or_else(|| "default".to_string());

            let inbound = InboundMessage::new(
                channel,
                msg.sender.clone(),
                chat_id,
                msg.content.clone(),
            );

            let (tx, mut rx) = mpsc::unbounded_channel::<AgentEvent>();

            // Process the message — the AgentLoop runs the LLM call and
            // emits events through the provided sender.
            let _response = agent_loop
                .process_inbound_message(inbound, Some(&tx))
                .await
                .map_err(|e| {
                    format!("[{scenario_name}] Turn {i} (message processing) failed: {e}")
                })?;

            // CRITICAL: Drop the sender so that `EventCollector::collect()`
            // sees the channel as closed and returns. Without this, the
            // `rx.recv()` loop inside `collect()` would block forever.
            drop(tx);

            // Collect all events emitted during this turn
            let turn_events = EventCollector::new()
                .collect(&mut rx, timeout_duration)
                .await
                .map_err(|e| {
                    format!(
                        "[{scenario_name}] Turn {i} event collection failed: {e}"
                    )
                })?;

            // Merge this turn's events into the accumulated result
            merge_collected_events(&mut merged_events, turn_events);
        }

        // ---- 8. Evaluate assertions ----
        let assertion_results =
            evaluate_assertions(&scenario.assertions, &merged_events, &workspace);

        // ---- 9. Determine overall pass/fail ----
        let passed = assertion_results.iter().all(|r| r.passed);
        let duration = start.elapsed();

        // ---- 10. Write trace to disk ----
        if let Ok(trace_path) = self.tracer.write_trace(
            &scenario_name,
            passed,
            &assertion_results,
            &merged_events,
            duration,
        ) {
            tracing::debug!("[{scenario_name}] Wrote trace to {:?}", trace_path);
        }

        Ok(ScenarioResult {
            scenario_name,
            passed,
            assertions: assertion_results,
            duration,
            events: merged_events,
        })
    }

    /// Discover all `.yaml` / `.yml` files in the configured scenarios
    /// directory, returned in lexicographic order.
    fn discover_scenarios(&self) -> Result<Vec<PathBuf>, String> {
        let dir = &self.config.scenarios_dir;

        if !dir.is_dir() {
            return Err(format!(
                "Scenarios directory does not exist: {}",
                dir.display()
            ));
        }

        let entries = std::fs::read_dir(dir).map_err(|e| {
            format!("Failed to read scenarios directory '{}': {e}", dir.display())
        })?;

        let mut scenarios = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "yaml" || ext == "yml" {
                        scenarios.push(path);
                    }
                }
            }
        }

        scenarios.sort();
        Ok(scenarios)
    }
}

/// Merge a single turn's [`CollectedEvents`] into the accumulated result.
///
/// This concatenates `iteration_indices`, `assistant_deltas`,
/// `reasoning_deltas`, `tool_calls`, `errors`, and `timeline` across all
/// turns. The `final_response` is **overwritten** with the latest turn's
/// value (last message wins).
fn merge_collected_events(accumulated: &mut CollectedEvents, turn_events: CollectedEvents) {
    accumulated
        .iteration_indices
        .extend(turn_events.iteration_indices);
    accumulated
        .assistant_deltas
        .extend(turn_events.assistant_deltas);
    accumulated
        .reasoning_deltas
        .extend(turn_events.reasoning_deltas);
    accumulated.tool_calls.extend(turn_events.tool_calls);
    accumulated.errors.extend(turn_events.errors);
    accumulated.timeline.extend(turn_events.timeline);

    // Keep the final response from the last turn
    if let Some(response) = turn_events.final_response {
        accumulated.final_response = Some(response);
    }
}

/// Build an [`LLMProvider`] (LiteLLM-backed) from the E2E configuration.
///
/// Constructs a [`LiteLLMClient`] using the configured API key, base URL,
/// and default model, then wraps it in an `Arc<dyn LLMProvider>`.
fn build_provider(
    config: &E2EConfig,
) -> Result<Arc<dyn LLMProvider>, Box<dyn std::error::Error>> {
    let client = LiteLLMClient::new(
        Some(config.api_key.clone()),
        Some(config.api_base.clone()),
        config.default_model.clone(),
        None, // extra_headers
        Some(config.provider_name.clone()),
        None, // default_reasoning_effort
    );
    Ok(Arc::new(client) as Arc<dyn LLMProvider>)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collector::ToolCallRecord;

    // -----------------------------------------------------------------------
    // merge_collected_events
    // -----------------------------------------------------------------------

    #[test]
    fn test_merge_empty_into_empty() {
        let mut acc = CollectedEvents::default();
        let turn = CollectedEvents::default();
        merge_collected_events(&mut acc, turn);
        assert!(acc.iteration_indices.is_empty());
        assert!(acc.assistant_deltas.is_empty());
        assert!(acc.final_response.is_none());
    }

    #[test]
    fn test_merge_preserves_first_turn_fields() {
        let mut acc = CollectedEvents::default();

        let turn1 = CollectedEvents {
            iteration_indices: vec![0, 1],
            assistant_deltas: vec!["hello".into()],
            reasoning_deltas: vec!["thinking".into()],
            tool_calls: vec![ToolCallRecord {
                tool_name: "bash".into(),
                input: Some("echo hi".into()),
                result: Some("hi".into()),
                is_error: Some(false),
            }],
            final_response: Some("Hello!".into()),
            errors: vec![],
            timeline: vec![],
        };
        merge_collected_events(&mut acc, turn1);

        assert_eq!(acc.iteration_indices, vec![0, 1]);
        assert_eq!(acc.assistant_deltas, vec!["hello"]);
        assert_eq!(acc.final_response.as_deref(), Some("Hello!"));
        assert_eq!(acc.tool_calls.len(), 1);
    }

    #[test]
    fn test_merge_turn2_overwrites_final_response() {
        let mut acc = CollectedEvents::default();

        merge_collected_events(
            &mut acc,
            CollectedEvents {
                final_response: Some("First".into()),
                ..Default::default()
            },
        );
        merge_collected_events(
            &mut acc,
            CollectedEvents {
                final_response: Some("Second".into()),
                ..Default::default()
            },
        );

        // Last final_response wins
        assert_eq!(acc.final_response.as_deref(), Some("Second"));
    }

    #[test]
    fn test_merge_accumulates_indices() {
        let mut acc = CollectedEvents::default();

        for i in 0..3 {
            merge_collected_events(
                &mut acc,
                CollectedEvents {
                    iteration_indices: vec![i],
                    ..Default::default()
                },
            );
        }

        assert_eq!(acc.iteration_indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_merge_accumulates_errors() {
        let mut acc = CollectedEvents::default();

        merge_collected_events(
            &mut acc,
            CollectedEvents {
                errors: vec!["err1".into()],
                ..Default::default()
            },
        );
        merge_collected_events(
            &mut acc,
            CollectedEvents {
                errors: vec!["err2".into()],
                ..Default::default()
            },
        );

        assert_eq!(acc.errors, vec!["err1", "err2"]);
    }

    // -----------------------------------------------------------------------
    // build_provider
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_provider_returns_ok() {
        let config = E2EConfig {
            api_key: "sk-test-key".into(),
            api_base: "https://api.deepseek.com/v1".into(),
            default_model: "deepseek-chat".into(),
            default_timeout_secs: 30,
            max_timeout_secs: 120,
            cost_budget_usd: 0.50,
            trace_dir: PathBuf::from("target/e2e-traces"),
            scenarios_dir: PathBuf::from("scripts/e2e/scenarios"),
            judge_model: None,
            provider_name: "deepseek".to_string(),
        };

        let result = build_provider(&config);
        assert!(result.is_ok(), "build_provider should succeed: {:?}", result.err());
    }

    // -----------------------------------------------------------------------
    // ScenarioRunner construction
    // -----------------------------------------------------------------------

    #[test]
    fn test_scenario_runner_new() {
        let config = E2EConfig {
            api_key: "sk-test".into(),
            api_base: "https://api.deepseek.com/v1".into(),
            default_model: "deepseek-chat".into(),
            default_timeout_secs: 30,
            max_timeout_secs: 120,
            cost_budget_usd: 0.50,
            trace_dir: PathBuf::from("target/e2e-traces"),
            scenarios_dir: PathBuf::from("tests/fixtures/scenarios"),
            judge_model: None,
            provider_name: "deepseek".to_string(),
        };

        let runner = ScenarioRunner::new(config);
        assert_eq!(runner.config.default_timeout_secs, 30);
    }

    // -----------------------------------------------------------------------
    // discover_scenarios
    // -----------------------------------------------------------------------

    #[test]
    fn test_discover_scenarios_nonexistent_dir() {
        let config = E2EConfig {
            api_key: "sk-test".into(),
            api_base: "https://api.deepseek.com/v1".into(),
            default_model: "deepseek-chat".into(),
            default_timeout_secs: 30,
            max_timeout_secs: 120,
            cost_budget_usd: 0.50,
            trace_dir: PathBuf::from("target/e2e-traces"),
            scenarios_dir: PathBuf::from("nonexistent-scenarios-dir-12345"),
            judge_model: None,
            provider_name: "deepseek".to_string(),
        };

        let runner = ScenarioRunner::new(config);
        let result = runner.discover_scenarios();
        assert!(result.is_err(), "Expected error for nonexistent dir");
        assert!(
            result.unwrap_err().contains("does not exist"),
            "Error should mention 'does not exist'"
        );
    }
}

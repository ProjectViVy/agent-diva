use std::path::Path;
use std::time::Duration;
use agent_diva_e2e::config::E2EConfig;
use agent_diva_e2e::runner::ScenarioRunner;

/// E2E test: verify Agent does not crash when a tool encounters an error.
///
/// This test asks the Agent to read a non-existent file. If the Agent
/// handles the tool error gracefully (returns a polite error message
/// instead of crashing), the scenario passes.
///
/// When DEEPSEEK_API_KEY is not set, the test skips gracefully (no failure).
#[tokio::test]
async fn test_error_handling_scenario() {
    if !E2EConfig::is_api_key_available() {
        eprintln!("SKIP: error_handling_test - DEEPSEEK_API_KEY not set");
        return;
    }

    let config = E2EConfig::from_env().expect("E2EConfig should be constructable with API key");
    let runner = ScenarioRunner::new(config);

    // CARGO_MANIFEST_DIR points to agent-diva-e2e/; the scenario yamls live
    // at the workspace root under scripts/e2e/scenarios/.
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let scenario_path = crate_dir
        .parent()
        .expect("agent-diva-e2e is a workspace member")
        .join("scripts")
        .join("e2e")
        .join("scenarios")
        .join("error_handling.yaml");

    assert!(
        scenario_path.exists(),
        "error_handling.yaml should exist at {:?}",
        scenario_path
    );

    let result = tokio::time::timeout(Duration::from_secs(60), runner.run_single(&scenario_path))
        .await;

    match result {
        Ok(Ok(scenario_result)) => {
            assert!(
                scenario_result.passed,
                "error_handling should pass. Assertions: {:?}",
                scenario_result
                    .assertions
                    .iter()
                    .map(|a| format!("{}: passed={}", a.description, a.passed))
                    .collect::<Vec<_>>()
            );
            // Print results for debugging
            println!(
                "✓ error_handling PASSED in {:.2}s",
                scenario_result.duration.as_secs_f64()
            );
            for a in &scenario_result.assertions {
                println!(
                    "  {}: {} - {}",
                    a.description,
                    if a.passed { "✓" } else { "✗" },
                    a.detail
                );
            }
        }
        Ok(Err(e)) => {
            if cfg!(feature = "ci") {
                panic!("error_handling scenario failed with error: {}", e);
            } else {
                eprintln!("WARN: error_handling scenario could not run: {}", e);
            }
        }
        Err(_elapsed) => {
            if cfg!(feature = "ci") {
                panic!("error_handling scenario timed out after 60s");
            } else {
                eprintln!("WARN: error_handling scenario timed out after 60s (not failing in non-CI mode)");
            }
        }
    }
}

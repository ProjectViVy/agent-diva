use agent_diva_e2e::config::E2EConfig;
use agent_diva_e2e::runner::ScenarioRunner;
use std::path::Path;
use std::time::Duration;

/// E2E test: verify Agent can generate a long-form response.
///
/// This test asks the Agent to list numbers from 1 to 20. The response
/// must contain numbers 10-20, verifying that the Agent can produce
/// structured output without truncation or errors.
///
/// When DEEPSEEK_API_KEY is not set, the test skips gracefully (no failure).
#[tokio::test]
async fn test_long_response_scenario() {
    if !E2EConfig::is_api_key_available() {
        eprintln!("SKIP: long_response_test - DEEPSEEK_API_KEY not set");
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
        .join("long_response.yaml");

    assert!(
        scenario_path.exists(),
        "long_response.yaml should exist at {:?}",
        scenario_path
    );

    // Long response scenarios may need more time for the full output
    let result =
        tokio::time::timeout(Duration::from_secs(75), runner.run_single(&scenario_path)).await;

    match result {
        Ok(Ok(scenario_result)) => {
            assert!(
                scenario_result.passed,
                "long_response should pass. Assertions: {:?}",
                scenario_result
                    .assertions
                    .iter()
                    .map(|a| format!("{}: passed={}", a.description, a.passed))
                    .collect::<Vec<_>>()
            );
            // Print results for debugging
            println!(
                "✓ long_response PASSED in {:.2}s",
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
            panic!("long_response scenario failed with error: {}", e);
        }
        Err(_elapsed) => {
            panic!("long_response scenario timed out after 75s");
        }
    }
}

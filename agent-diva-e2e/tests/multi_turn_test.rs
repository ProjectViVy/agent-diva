use agent_diva_e2e::config::E2EConfig;
use agent_diva_e2e::runner::ScenarioRunner;
use std::path::Path;
use std::time::Duration;

/// E2E test: verify Agent's multi-turn conversation context retention.
///
/// This test sends two messages:
/// 1. "我叫小明" (I'm Xiao Ming)
/// 2. "我叫什么名字？" (What's my name?)
///
/// The Agent must remember the name from the first turn and correctly
/// repeat it in the second turn.
///
/// When DEEPSEEK_API_KEY is not set, the test skips gracefully (no failure).
#[tokio::test]
async fn test_multi_turn_scenario() {
    if !E2EConfig::is_api_key_available() {
        eprintln!("SKIP: multi_turn_test - DEEPSEEK_API_KEY not set");
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
        .join("multi_turn.yaml");

    assert!(
        scenario_path.exists(),
        "multi_turn.yaml should exist at {:?}",
        scenario_path
    );

    // Multi-turn scenarios may take longer due to two message turns
    let result =
        tokio::time::timeout(Duration::from_secs(90), runner.run_single(&scenario_path)).await;

    match result {
        Ok(Ok(scenario_result)) => {
            assert!(
                scenario_result.passed,
                "multi_turn should pass. Assertions: {:?}",
                scenario_result
                    .assertions
                    .iter()
                    .map(|a| format!("{}: passed={}", a.description, a.passed))
                    .collect::<Vec<_>>()
            );
            // Print results for debugging
            println!(
                "✓ multi_turn PASSED in {:.2}s",
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
            panic!("multi_turn scenario failed with error: {}", e);
        }
        Err(_elapsed) => {
            panic!("multi_turn scenario timed out after 90s");
        }
    }
}

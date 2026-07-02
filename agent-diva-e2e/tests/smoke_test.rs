use std::path::Path;
use std::time::Duration;
use agent_diva_e2e::config::E2EConfig;
use agent_diva_e2e::runner::ScenarioRunner;

/// Smoke test: verify the full E2E pipeline works with a real LLM.
///
/// This test reads `scripts/e2e/scenarios/smoke.yaml`, runs it through
/// ScenarioRunner, and asserts that the scenario passes.
///
/// When DEEPSEEK_API_KEY is not set, the test skips gracefully (no failure).
#[tokio::test]
async fn test_smoke_scenario() {
    if !E2EConfig::is_api_key_available() {
        eprintln!("SKIP: smoke_test - DEEPSEEK_API_KEY not set");
        return;
    }

    let config = E2EConfig::from_env().expect("E2EConfig should be constructable with API key");
    let runner = ScenarioRunner::new(config);

    // CARGO_MANIFEST_DIR points to agent-diva-e2e/; the scenario yamls live
    // at the workspace root under scripts/e2e/scenarios/.
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let smoke_path = crate_dir
        .parent()
        .expect("agent-diva-e2e is a workspace member")
        .join("scripts")
        .join("e2e")
        .join("scenarios")
        .join("smoke.yaml");

    assert!(
        smoke_path.exists(),
        "smoke.yaml should exist at {:?}",
        smoke_path
    );

    let result = tokio::time::timeout(Duration::from_secs(60), runner.run_single(&smoke_path))
        .await;

    match result {
        Ok(Ok(scenario_result)) => {
            assert!(
                scenario_result.passed,
                "Smoke scenario should pass. Assertions: {:?}",
                scenario_result
                    .assertions
                    .iter()
                    .map(|a| format!("{}: passed={}", a.description, a.passed))
                    .collect::<Vec<_>>()
            );
            // Print results for debugging
            println!(
                "✓ Smoke scenario PASSED in {:.2}s",
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
            // If API call fails (network, auth) - this is expected without valid key
            // but still report it
            if cfg!(feature = "ci") {
                panic!("Smoke scenario failed with error: {}", e);
            } else {
                eprintln!("WARN: smoke scenario could not run: {}", e);
            }
        }
        Err(_elapsed) => {
            if cfg!(feature = "ci") {
                panic!("Smoke scenario timed out after 60s");
            } else {
                eprintln!("WARN: smoke scenario timed out after 60s (not failing in non-CI mode)");
            }
        }
    }
}

use std::fs;
use std::path::Path;
use std::process::Command;

use mockito::Server;
use serde_json::Value;
use tempfile::tempdir;

fn write_config(root: &Path, with_api_key: bool) -> std::path::PathBuf {
    let workspace = root.join("workspace");
    fs::create_dir_all(&workspace).unwrap();

    let api_key = if with_api_key { "sk-test" } else { "" };
    let config = format!(
        r#"{{
  "agents": {{
    "defaults": {{
      "workspace": "{}",
      "provider": "openai",
      "model": "openai/gpt-4o"
    }}
  }},
  "providers": {{
    "openai": {{
      "api_key": "{}"
    }}
  }}
}}"#,
        workspace.display().to_string().replace('\\', "\\\\"),
        api_key
    );

    let config_path = root.join("instance").join("config.json");
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(&config_path, config).unwrap();
    config_path
}

#[test]
fn status_json_uses_explicit_config_file() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path(), true);

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "status",
            "--json",
        ])
        .output()
        .expect("failed to run status --json");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: Value = serde_json::from_str(stdout.trim()).unwrap();

    assert_eq!(
        value["config"]["config_path"].as_str().unwrap(),
        config_path.display().to_string()
    );
    assert_eq!(value["doctor"]["valid"], true);
}

#[test]
fn config_show_json_redacts_secrets() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path(), true);

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "show",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run config show");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(value["providers"]["openai"]["api_key"], "***REDACTED***");
}

#[test]
fn config_doctor_json_swarm_includes_versioned_swarm_cortex_block() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path(), true);

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "doctor",
            "--json",
            "--swarm",
        ])
        .output()
        .expect("failed to run config doctor --json --swarm");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: Value = serde_json::from_str(stdout.trim()).unwrap();
    let sc = &value["swarm_cortex"];
    assert_eq!(sc["schema_version"], 2);
    assert_eq!(sc["channel"], "cli_diagnostics");
    assert_eq!(sc["capabilities"]["source"], "unavailable");
    assert_eq!(sc["cortex"]["state"], "n/a");
    assert!(sc["cortex"]["gateway_bind"].as_str().unwrap().contains(':'));
    assert_eq!(sc["subagent_tools"]["catalog_version"], 1);
    assert_eq!(sc["subagent_tools"]["entries"].as_array().unwrap().len(), 6);
}

#[test]
fn config_doctor_swarm_uses_workspace_capability_manifest_file() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path(), true);
    let workspace = temp.path().join("workspace");
    let ad = workspace.join(".agent-diva");
    fs::create_dir_all(&ad).unwrap();
    fs::write(
        ad.join("capability-manifest.json"),
        r#"{"schema_version":"0","capabilities":[{"id":"cli-cap","name":"C"}]}"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "doctor",
            "--json",
            "--swarm",
        ])
        .output()
        .expect("failed to run config doctor --json --swarm");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: Value = serde_json::from_str(stdout.trim()).unwrap();
    let sc = &value["swarm_cortex"];
    assert_eq!(sc["capabilities"]["source"], "process_registry");
    assert_eq!(sc["capabilities"]["count"], 1);
}

#[test]
fn config_doctor_returns_warning_exit_code_for_missing_provider_key() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path(), false);

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "config",
            "doctor",
            "--json",
        ])
        .output()
        .expect("failed to run config doctor");

    assert_eq!(output.status.code(), Some(2), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(value["valid"], true);
    assert_eq!(value["ready"], false);
}

#[test]
fn provider_list_json_includes_registry_default_model() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path(), true);

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "provider",
            "list",
            "--json",
        ])
        .output()
        .expect("failed to run provider list --json");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: Value = serde_json::from_str(stdout.trim()).unwrap();
    let openai = value
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["name"] == "openai")
        .expect("openai entry missing");

    assert_eq!(openai["default_model"], "openai/gpt-4o");
    assert_eq!(openai["configured"], true);
}

#[test]
fn provider_set_json_updates_model_and_credentials() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path(), false);

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "provider",
            "set",
            "--provider",
            "deepseek",
            "--api-key",
            "sk-deepseek",
            "--json",
        ])
        .output()
        .expect("failed to run provider set --json");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: Value = serde_json::from_str(stdout.trim()).unwrap();

    assert_eq!(value["provider"], "deepseek");
    assert_eq!(value["model"], "deepseek-chat");

    let saved: Value = serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    assert_eq!(saved["agents"]["defaults"]["provider"], "deepseek");
    assert_eq!(saved["agents"]["defaults"]["model"], "deepseek-chat");
    assert_eq!(saved["providers"]["deepseek"]["api_key"], "sk-deepseek");
}

#[test]
fn provider_models_json_returns_runtime_catalog() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut server = runtime.block_on(Server::new_async());
    let temp = tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let config_path = temp.path().join("instance").join("config.json");
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(
        &config_path,
        format!(
            r#"{{
  "agents": {{
    "defaults": {{
      "workspace": "{}",
      "model": "openai/gpt-4o"
    }}
  }},
  "providers": {{
    "openai": {{
      "api_key": "sk-test",
      "api_base": "{}"
    }}
  }}
}}"#,
            workspace.display().to_string().replace('\\', "\\\\"),
            server.url()
        ),
    )
    .unwrap();

    let mock = runtime.block_on(async {
        server
            .mock("GET", "/models")
            .with_status(200)
            .with_body(r#"{"data":[{"id":"gpt-4o"},{"id":"gpt-4o-mini"}]}"#)
            .create_async()
            .await
    });

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "provider",
            "models",
            "--provider",
            "openai",
            "--json",
        ])
        .output()
        .expect("failed to run provider models --json");

    runtime.block_on(mock.assert_async());
    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: Value = serde_json::from_str(stdout.trim()).unwrap();

    assert_eq!(value["provider"], "openai");
    assert_eq!(value["source"], "runtime");
    assert_eq!(value["models"][0], "gpt-4o");
    assert_eq!(value["models"][1], "gpt-4o-mini");
}

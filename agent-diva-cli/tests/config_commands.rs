use std::fs;
use std::path::Path;
use std::process::Command;

use agent_diva_core::auth::{OAuthProfileState, ProviderAuthService, ProviderTokenSet};
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
fn provider_status_json_reports_qwen_login_oauth_state() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path(), false);
    let auth = ProviderAuthService::new(config_path.parent().unwrap());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        auth.store_oauth_profile(
            "qwen-login",
            "default",
            OAuthProfileState {
                token_set: ProviderTokenSet {
                    access_token: "oauth-bearer".into(),
                    refresh_token: Some("refresh".into()),
                    id_token: None,
                    expires_at: None,
                    token_type: Some("Bearer".into()),
                    scope: Some("openid".into()),
                },
                account_id: Some("acct-qwen".into()),
                metadata: std::collections::BTreeMap::from([(
                    "api_base".to_string(),
                    "https://oauth.example/v1".to_string(),
                )]),
            },
            true,
        )
        .await
        .unwrap();
    });

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "provider",
            "status",
            "qwen-login",
            "--json",
        ])
        .output()
        .expect("failed to run provider status qwen-login --json");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: Value = serde_json::from_str(stdout.trim()).unwrap();
    let provider = &value["providers"][0];
    assert_eq!(provider["provider"], "qwen-login");
    assert_eq!(provider["auth_mode"], "oauth");
    assert_eq!(provider["authenticated"], true);
    assert_eq!(provider["active_profile"], "default");
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

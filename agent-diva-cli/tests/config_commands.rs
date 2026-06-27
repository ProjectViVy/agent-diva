use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use mockito::Server;
use serde_json::Value;
use tempfile::tempdir;

static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn test_lock() -> &'static Mutex<()> {
    TEST_LOCK.get_or_init(|| Mutex::new(()))
}

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

fn write_harness_config(root: &Path) -> std::path::PathBuf {
    let workspace = root.join("workspace");
    fs::create_dir_all(&workspace).unwrap();

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
      "api_key": "sk-test"
    }}
  }},
  "security": {{
    "level": "strict",
    "workspace_only": true,
    "max_actions_per_hour": 42
  }},
  "presence": {{
    "active_timeout_s": 11,
    "distracted_timeout_s": 22,
    "gone_timeout_s": 33,
    "distracted_heartbeat_multiplier": 3.5
  }},
  "heartbeat": {{
    "enabled": true,
    "interval_s": 99
  }},
  "audit": {{
    "enabled": true,
    "emit_presence_changed": false
  }},
  "pii": {{
    "enabled": true,
    "redact_email": false
  }},
  "injection": {{
    "enabled": true,
    "detect_tool_abuse": false
  }}
}}"#,
        workspace.display().to_string().replace('\\', "\\\\"),
    );

    let config_path = root.join("instance").join("config.json");
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(&config_path, config).unwrap();
    config_path
}

fn write_invalid_harness_config(root: &Path) -> std::path::PathBuf {
    let workspace = root.join("workspace");
    fs::create_dir_all(&workspace).unwrap();

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
      "api_key": "sk-test"
    }}
  }},
  "security": {{
    "max_actions_per_hour": 0
  }},
  "presence": {{
    "active_timeout_s": 0,
    "distracted_timeout_s": 22,
    "gone_timeout_s": 33,
    "distracted_heartbeat_multiplier": 3.5
  }},
  "heartbeat": {{
    "enabled": true,
    "interval_s": 0
  }}
}}"#,
        workspace.display().to_string().replace('\\', "\\\\"),
    );

    let config_path = root.join("instance").join("config.json");
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(&config_path, config).unwrap();
    config_path
}

#[test]
fn status_json_uses_explicit_config_file() {
    let _guard = test_lock().lock().unwrap();
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
    let _guard = test_lock().lock().unwrap();
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
fn config_show_json_includes_harness_domain_sections() {
    let _guard = test_lock().lock().unwrap();
    let temp = tempdir().unwrap();
    let config_path = write_harness_config(temp.path());

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

    assert_eq!(value["security"]["max_actions_per_hour"], 42);
    assert_eq!(value["presence"]["active_timeout_s"], 11);
    assert_eq!(value["heartbeat"]["interval_s"], 99);
    assert_eq!(value["audit"]["emit_presence_changed"], false);
    assert_eq!(value["pii"]["redact_email"], false);
    assert_eq!(value["injection"]["detect_tool_abuse"], false);
}

#[test]
fn status_json_rejects_invalid_harness_config() {
    let _guard = test_lock().lock().unwrap();
    let temp = tempdir().unwrap();
    let config_path = write_invalid_harness_config(temp.path());

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "status",
            "--json",
        ])
        .output()
        .expect("failed to run status --json");

    assert!(!output.status.success(), "{:?}", output);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("security.max_actions_per_hour"));
    assert!(stderr.contains("presence.active_timeout_s"));
    assert!(stderr.contains("heartbeat.interval_s"));
}

#[test]
fn config_doctor_returns_warning_exit_code_for_missing_provider_key() {
    let _guard = test_lock().lock().unwrap();
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
    let _guard = test_lock().lock().unwrap();
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
    let _guard = test_lock().lock().unwrap();
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
fn provider_models_json_returns_provider_catalog() {
    let _guard = test_lock().lock().unwrap();
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

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: Value = serde_json::from_str(stdout.trim()).unwrap();

    assert_eq!(value["provider"], "openai");
    assert!(
        matches!(
            value["source"].as_str(),
            Some("runtime") | Some("static_fallback")
        ),
        "unexpected catalog source: {}",
        value["source"]
    );
    let models = value["models"]
        .as_array()
        .expect("models should be an array");
    assert!(models.iter().any(|model| model == "gpt-4o"));
    assert!(models.iter().any(|model| model == "gpt-4o-mini"));
    drop(mock);
}

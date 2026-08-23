//! Wave B 表征测试：`CliRuntime::effective_workspace()` 当前行为快照。
//!
//! 这些测试钉住 Phase 0 的现状，为 Wave C1 引入 `WorkspaceContext` 统一解析
//! 提供回归基线。行为变化必须由 Wave C1 有意识地更新这些断言。

use std::fs;
use std::path::{Path, PathBuf};

use agent_diva_cli::cli_runtime::CliRuntime;

fn write_config(root: &Path, workspace_value: Option<&str>) -> PathBuf {
    let workspace_line = match workspace_value {
        Some(value) => format!("\"workspace\": \"{}\",\n", value.replace('\\', "\\\\")),
        None => String::new(),
    };

    let config = format!(
        r#"{{
  "agents": {{
    "defaults": {{
      {workspace_line}"provider": "openai",
      "model": "openai/gpt-4o"
    }}
  }},
  "providers": {{
    "openai": {{
      "api_key": "sk-test"
    }}
  }}
}}"#
    );

    let config_path = root.join("instance").join("config.json");
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(&config_path, config).unwrap();
    config_path
}

fn runtime_with(
    config_path: PathBuf,
    workspace_override: Option<PathBuf>,
) -> (CliRuntime, agent_diva_core::config::Config) {
    let runtime = CliRuntime::from_paths(Some(config_path), None, workspace_override);
    let config = runtime.load_config().expect("config should load");
    (runtime, config)
}

#[test]
fn default_workspace_is_legacy_home_default() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = write_config(temp.path(), None);
    let (runtime, config) = runtime_with(config_path, None);

    let workspace = runtime.effective_workspace(&config);
    let home = dirs::home_dir().expect("home dir");
    assert_eq!(
        workspace,
        home.join(".agent-diva").join("workspace"),
        "未指定 --workspace 且 config 未保存 workspace 时，当前默认仍是 ~/.agent-diva/workspace"
    );
}

#[test]
fn explicit_override_wins_without_normalization() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = write_config(temp.path(), None);
    let override_path = PathBuf::from("some/relative/dir");
    let (runtime, config) = runtime_with(config_path, Some(override_path.clone()));

    let workspace = runtime.effective_workspace(&config);
    assert_eq!(
        workspace, override_path,
        "--workspace 覆盖优先，且当前不做绝对化/canonical 化（Wave C1 将统一处理）"
    );
}

#[test]
fn configured_workspace_with_tilde_is_expanded() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = write_config(temp.path(), Some("~/custom-ws"));
    let (runtime, config) = runtime_with(config_path, None);

    let workspace = runtime.effective_workspace(&config);
    let home = dirs::home_dir().expect("home dir");
    assert_eq!(workspace, home.join("custom-ws"));
}

#[test]
fn configured_absolute_workspace_is_used_as_is() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("project-root");
    fs::create_dir_all(&target).unwrap();
    let config_path = write_config(temp.path(), Some(target.to_str().expect("utf8 path")));
    let (runtime, config) = runtime_with(config_path, None);

    let workspace = runtime.effective_workspace(&config);
    assert_eq!(workspace, target);
}

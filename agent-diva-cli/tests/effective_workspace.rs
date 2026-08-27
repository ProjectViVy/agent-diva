//! Wave B + C1 表征测试：`CliRuntime::effective_workspace()` 合同。
//!
//! Wave B 钉住 Phase 0 现状；Wave C1 将默认语义切换到进程 CWD 并绝对化/
//! canonicalize 所有覆盖路径。本文件已更新以反映 Wave C1 合同。

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
fn default_workspace_is_process_cwd_not_legacy_home() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = write_config(temp.path(), None);
    let (runtime, config) = runtime_with(config_path, None);

    let workspace = runtime.effective_workspace(&config);
    let cwd = std::env::current_dir().unwrap();
    assert!(
        workspace.is_absolute(),
        "未指定 --workspace 且 config 为旧默认时，解析结果必须绝对化：{workspace:?}"
    );
    let expected = cwd.canonicalize().unwrap_or(cwd);
    assert_eq!(
        workspace, expected,
        "LegacyDefault 应解析到进程 CWD，而非 ~/.agent-diva/workspace"
    );
}

#[test]
fn explicit_override_wins_and_is_absolutized() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("override-ws");
    fs::create_dir_all(&target).unwrap();
    let config_path = write_config(temp.path(), None);
    let (runtime, config) = runtime_with(config_path, Some(target.clone()));

    let workspace = runtime.effective_workspace(&config);
    assert!(workspace.is_absolute());
    assert_eq!(
        workspace,
        target.canonicalize().unwrap_or(target),
        "--workspace 覆盖优先且立即 canonicalize"
    );
}

#[test]
fn relative_cli_override_is_absolutized() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = write_config(temp.path(), None);
    let (runtime, config) = runtime_with(config_path, Some(PathBuf::from(".")));

    let workspace = runtime.effective_workspace(&config);
    assert!(workspace.is_absolute());
}

#[test]
fn configured_workspace_with_tilde_is_expanded() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = write_config(temp.path(), Some("~/custom-ws"));
    let (runtime, config) = runtime_with(config_path, None);

    let workspace = runtime.effective_workspace(&config);
    let home = dirs::home_dir().expect("home dir");
    let expected = home.join("custom-ws");
    assert!(
        workspace == expected || workspace == expected.canonicalize().unwrap(),
        "~/custom-ws 应展开到 {expected:?}，实际是 {workspace:?}"
    );
}

#[test]
fn configured_absolute_workspace_is_canonicalized() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("project-root");
    fs::create_dir_all(&target).unwrap();
    let config_path = write_config(temp.path(), Some(target.to_str().expect("utf8 path")));
    let (runtime, config) = runtime_with(config_path, None);

    let workspace = runtime.effective_workspace(&config);
    assert_eq!(workspace, target.canonicalize().unwrap());
}

#[test]
fn workspace_context_reports_source() {
    use agent_diva_core::workspace::WorkspaceSource;
    let temp = tempfile::tempdir().unwrap();
    let config_path = write_config(temp.path(), None);
    let (runtime, config) = runtime_with(config_path, None);

    let ctx = runtime.workspace_context(&config);
    assert_eq!(
        ctx.source,
        WorkspaceSource::LegacyDefault,
        "旧默认值应被标记为 LegacyDefault 以便 doctor 提示"
    );
}

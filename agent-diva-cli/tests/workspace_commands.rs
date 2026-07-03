use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

fn write_config(root: &Path) -> std::path::PathBuf {
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
  }}
}}"#,
        workspace.display().to_string().replace('\\', "\\\\")
    );

    let config_path = root.join("instance").join("config.json");
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(&config_path, config).unwrap();
    config_path
}

#[test]
fn workspace_create_and_list() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path());
    let config_dir = config_path.parent().unwrap();

    // Create workspace
    let create_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config-dir",
            config_dir.to_str().unwrap(),
            "workspace",
            "create",
            "test-ws",
        ])
        .output()
        .expect("failed to run workspace create");

    assert!(create_output.status.success(), "{:?}", create_output);
    let create_stdout = String::from_utf8(create_output.stdout).unwrap();
    assert!(
        create_stdout.contains("Created workspace"),
        "stdout: {}",
        create_stdout
    );

    // List workspaces
    let list_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config-dir",
            config_dir.to_str().unwrap(),
            "workspace",
            "list",
        ])
        .output()
        .expect("failed to run workspace list");

    assert!(list_output.status.success(), "{:?}", list_output);
    let list_stdout = String::from_utf8(list_output.stdout).unwrap();
    assert!(
        list_stdout.contains("test-ws"),
        "list should contain test-ws: {}",
        list_stdout
    );
}

#[test]
fn workspace_switch_updates_config() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path());
    let config_dir = config_path.parent().unwrap();

    // Create workspace first
    let create_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config-dir",
            config_dir.to_str().unwrap(),
            "workspace",
            "create",
            "switch-ws",
        ])
        .output()
        .expect("failed to run workspace create");
    assert!(create_output.status.success(), "{:?}", create_output);

    // Switch workspace
    let switch_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config-dir",
            config_dir.to_str().unwrap(),
            "workspace",
            "switch",
            "switch-ws",
        ])
        .output()
        .expect("failed to run workspace switch");

    assert!(switch_output.status.success(), "{:?}", switch_output);
    let switch_stdout = String::from_utf8(switch_output.stdout).unwrap();
    assert!(
        switch_stdout.contains("Switched to workspace"),
        "stdout: {}",
        switch_stdout
    );
    assert!(
        switch_stdout.contains("Restart gateway for changes to take effect"),
        "stdout: {}",
        switch_stdout
    );

    // Verify config was updated
    let config_content = fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("switch-ws"),
        "config should contain switch-ws"
    );
}

#[test]
fn workspace_delete_removes_workspace() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path());
    let config_dir = config_path.parent().unwrap();

    // Create workspace
    let create_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config-dir",
            config_dir.to_str().unwrap(),
            "workspace",
            "create",
            "delete-ws",
        ])
        .output()
        .expect("failed to run workspace create");
    assert!(create_output.status.success(), "{:?}", create_output);

    // Delete workspace with --force to skip interactive confirmation
    let delete_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config-dir",
            config_dir.to_str().unwrap(),
            "workspace",
            "delete",
            "delete-ws",
            "--force",
        ])
        .output()
        .expect("failed to run workspace delete");

    assert!(delete_output.status.success(), "{:?}", delete_output);
    let delete_stdout = String::from_utf8(delete_output.stdout).unwrap();
    assert!(
        delete_stdout.contains("Deleted workspace"),
        "stdout: {}",
        delete_stdout
    );

    // Verify workspace is gone
    let list_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config-dir",
            config_dir.to_str().unwrap(),
            "workspace",
            "list",
        ])
        .output()
        .expect("failed to run workspace list");

    assert!(list_output.status.success(), "{:?}", list_output);
    let list_stdout = String::from_utf8(list_output.stdout).unwrap();
    assert!(
        !list_stdout.contains("delete-ws"),
        "list should not contain delete-ws: {}",
        list_stdout
    );
}

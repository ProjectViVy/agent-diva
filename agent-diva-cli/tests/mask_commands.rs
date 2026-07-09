use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn setup_workspace(root: &Path) -> PathBuf {
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

    let config_dir = root.join("config");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.json");
    fs::write(&config_path, config).unwrap();

    let refresh_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config-dir",
            config_dir.to_str().unwrap(),
            "config",
            "refresh",
        ])
        .output()
        .expect("failed to run config refresh");
    assert!(
        refresh_output.status.success(),
        "config refresh failed: {:?}",
        refresh_output
    );

    config_dir
}

#[test]
fn mask_list_shows_default_and_sample_masks() {
    let temp = tempdir().unwrap();
    let config_dir = setup_workspace(temp.path());

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args(["--config-dir", config_dir.to_str().unwrap(), "mask", "list"])
        .output()
        .expect("failed to run mask list");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("我就是我"),
        "list should contain default mask: {}",
        stdout
    );
    assert!(
        stdout.contains("代码专家"),
        "list should contain coder sample mask: {}",
        stdout
    );
    assert!(
        stdout.contains("研究专家"),
        "list should contain researcher sample mask: {}",
        stdout
    );
}

#[test]
fn mask_switch_writes_active_mask() {
    let temp = tempdir().unwrap();
    let config_dir = setup_workspace(temp.path());
    let workspace = temp.path().join("workspace");
    let active_mask_file = workspace.join("masks").join(".active-mask");

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config-dir",
            config_dir.to_str().unwrap(),
            "mask",
            "switch",
            "--name",
            "coder",
        ])
        .output()
        .expect("failed to run mask switch");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Switched to mask"), "stdout: {}", stdout);
    assert!(active_mask_file.exists(), ".active-mask file should exist");
    let active = fs::read_to_string(&active_mask_file).unwrap();
    assert!(
        active.contains("coder"),
        ".active-mask should contain 'coder': {}",
        active
    );
}

#[test]
fn mask_show_prints_active_mask() {
    let temp = tempdir().unwrap();
    let config_dir = setup_workspace(temp.path());

    let switch_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config-dir",
            config_dir.to_str().unwrap(),
            "mask",
            "switch",
            "--name",
            "researcher",
        ])
        .output()
        .expect("failed to run mask switch");
    assert!(switch_output.status.success(), "{:?}", switch_output);

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args(["--config-dir", config_dir.to_str().unwrap(), "mask", "show"])
        .output()
        .expect("failed to run mask show");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("researcher"),
        "show should contain active mask name: {}",
        stdout
    );
    assert!(
        stdout.contains("---"),
        "show should print frontmatter delimiters: {}",
        stdout
    );
}

#[test]
fn mask_create_from_file() {
    let temp = tempdir().unwrap();
    let config_dir = setup_workspace(temp.path());
    let workspace = temp.path().join("workspace");

    let mask_file = temp.path().join("new-mask.md");
    fs::write(
        &mask_file,
        "---\nname: \"Test Mask\"\nicon: \"🧪\"\ndescription: \"A test mask\"\n---\n\nThis is a test mask body.\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config-dir",
            config_dir.to_str().unwrap(),
            "mask",
            "create",
            "--file",
            mask_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run mask create");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Created mask"), "stdout: {}", stdout);
    assert!(
        stdout.contains("Test Mask"),
        "stdout should contain mask name: {}",
        stdout
    );

    let list_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args(["--config-dir", config_dir.to_str().unwrap(), "mask", "list"])
        .output()
        .expect("failed to run mask list");
    assert!(list_output.status.success(), "{:?}", list_output);
    let list_stdout = String::from_utf8(list_output.stdout).unwrap();
    assert!(
        list_stdout.contains("Test Mask"),
        "list should include created mask: {}",
        list_stdout
    );

    let created_file = workspace.join("masks").join("test-mask.md");
    assert!(created_file.exists(), "created mask file should exist");
    let content = fs::read_to_string(&created_file).unwrap();
    assert!(
        content.contains("Test Mask"),
        "created file should contain mask name: {}",
        content
    );
}

#[test]
fn mask_delete_resets_active_mask() {
    let temp = tempdir().unwrap();
    let config_dir = setup_workspace(temp.path());
    let workspace = temp.path().join("workspace");
    let masks_dir = workspace.join("masks");
    let active_mask_file = masks_dir.join(".active-mask");
    let coder_file = masks_dir.join("coder.md");

    let switch_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config-dir",
            config_dir.to_str().unwrap(),
            "mask",
            "switch",
            "--name",
            "coder",
        ])
        .output()
        .expect("failed to run mask switch");
    assert!(switch_output.status.success(), "{:?}", switch_output);
    assert!(
        active_mask_file.exists(),
        ".active-mask should exist after switch"
    );

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config-dir",
            config_dir.to_str().unwrap(),
            "mask",
            "delete",
            "--name",
            "coder",
        ])
        .output()
        .expect("failed to run mask delete");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Deleted mask"), "stdout: {}", stdout);
    assert!(!coder_file.exists(), "coder mask file should be removed");
    assert!(
        !active_mask_file.exists(),
        ".active-mask should be removed after deleting active mask"
    );
}

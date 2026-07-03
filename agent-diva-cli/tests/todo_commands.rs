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
fn todo_add_creates_todo() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path());

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "todo",
            "add",
            "test todo item",
        ])
        .output()
        .expect("failed to run todo add");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Created todo"), "stdout: {}", stdout);
    assert!(stdout.contains("test todo item"), "stdout: {}", stdout);
}

#[test]
fn todo_list_shows_added_todo() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path());

    // Add a todo first
    let add_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "todo",
            "add",
            "list me",
        ])
        .output()
        .expect("failed to run todo add");
    assert!(add_output.status.success(), "{:?}", add_output);
    let add_stdout = String::from_utf8(add_output.stdout).unwrap();

    // Extract the todo ID from output like "Created todo <id>: <title>"
    // Skip the ASCII art lines and find the line with "Created todo"
    let id_line = add_stdout
        .lines()
        .find(|l| l.contains("Created todo"))
        .expect("expected 'Created todo' line");
    let words: Vec<&str> = id_line.split_whitespace().collect();
    let id = words
        .get(2)
        .expect("expected todo id in output")
        .trim_end_matches(':');

    // List todos
    let list_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args(["--config", config_path.to_str().unwrap(), "todo", "list"])
        .output()
        .expect("failed to run todo list");

    assert!(list_output.status.success(), "{:?}", list_output);
    let list_stdout = String::from_utf8(list_output.stdout).unwrap();
    assert!(
        list_stdout.contains(id),
        "list should contain todo id: {}",
        list_stdout
    );
    assert!(
        list_stdout.contains("list me"),
        "list should contain title: {}",
        list_stdout
    );
}

#[test]
fn todo_update_status_succeeds() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path());

    // Add a todo
    let add_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "todo",
            "add",
            "update me",
        ])
        .output()
        .expect("failed to run todo add");
    assert!(add_output.status.success(), "{:?}", add_output);
    let add_stdout = String::from_utf8(add_output.stdout).unwrap();

    // Extract the todo ID from output like "Created todo <id>: <title>"
    // Skip the ASCII art lines and find the line with "Created todo"
    let id_line = add_stdout
        .lines()
        .find(|l| l.contains("Created todo"))
        .expect("expected 'Created todo' line");
    let words: Vec<&str> = id_line.split_whitespace().collect();
    let id = words
        .get(2)
        .expect("expected todo id in output")
        .trim_end_matches(':');

    // Update status to done
    let update_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "todo",
            "update",
            id,
            "--status",
            "done",
        ])
        .output()
        .expect("failed to run todo update");

    assert!(update_output.status.success(), "{:?}", update_output);
    let update_stdout = String::from_utf8(update_output.stdout).unwrap();
    assert!(
        update_stdout.contains("Updated todo") || update_stdout.contains("completed"),
        "stdout: {}",
        update_stdout
    );

    // Verify via list
    let list_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args(["--config", config_path.to_str().unwrap(), "todo", "list"])
        .output()
        .expect("failed to run todo list");

    assert!(list_output.status.success(), "{:?}", list_output);
    let list_stdout = String::from_utf8(list_output.stdout).unwrap();
    assert!(
        list_stdout.contains("completed"),
        "list should show completed: {}",
        list_stdout
    );
}

#[test]
fn todo_archive_and_purge_smoke() {
    let temp = tempdir().unwrap();
    let config_path = write_config(temp.path());

    // Add a todo
    let add_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "todo",
            "add",
            "archive me",
        ])
        .output()
        .expect("failed to run todo add");
    assert!(add_output.status.success(), "{:?}", add_output);
    let add_stdout = String::from_utf8(add_output.stdout).unwrap();

    // Extract the todo ID
    let id_line = add_stdout
        .lines()
        .find(|l| l.contains("Created todo"))
        .expect("expected 'Created todo' line");
    let words: Vec<&str> = id_line.split_whitespace().collect();
    let id = words
        .get(2)
        .expect("expected todo id in output")
        .trim_end_matches(':');

    // Update status to done
    let update_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "todo",
            "update",
            id,
            "--status",
            "done",
        ])
        .output()
        .expect("failed to run todo update");
    assert!(update_output.status.success(), "{:?}", update_output);

    // Archive with 0 days (should archive the completed todo)
    let archive_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "todo",
            "archive",
            "--days",
            "0",
        ])
        .output()
        .expect("failed to run todo archive");

    assert!(archive_output.status.success(), "{:?}", archive_output);
    let archive_stdout = String::from_utf8(archive_output.stdout).unwrap();
    assert!(
        archive_stdout.contains("Archived"),
        "archive stdout: {}",
        archive_stdout
    );

    // List should be empty now
    let list_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args(["--config", config_path.to_str().unwrap(), "todo", "list"])
        .output()
        .expect("failed to run todo list");
    assert!(list_output.status.success(), "{:?}", list_output);
    let list_stdout = String::from_utf8(list_output.stdout).unwrap();
    assert!(
        list_stdout.contains("No todos found"),
        "list should be empty: {}",
        list_stdout
    );

    // Purge with 0 months (should purge the archive)
    let purge_output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "todo",
            "purge",
            "--months",
            "0",
        ])
        .output()
        .expect("failed to run todo purge");

    assert!(purge_output.status.success(), "{:?}", purge_output);
    let purge_stdout = String::from_utf8(purge_output.stdout).unwrap();
    assert!(
        purge_stdout.contains("Purged"),
        "purge stdout: {}",
        purge_stdout
    );
}

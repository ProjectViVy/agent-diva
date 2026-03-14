use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use tempfile::tempdir;

fn spawn_mock_openai_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().expect("local addr");

    thread::spawn(move || {
        for stream in listener.incoming().take(2) {
            let mut stream = match stream {
                Ok(stream) => stream,
                Err(_) => continue,
            };

            let mut buffer = Vec::new();
            let mut header_end = None;
            loop {
                let mut chunk = [0u8; 1024];
                let read = stream.read(&mut chunk).expect("read request");
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
                if header_end.is_none() {
                    header_end = buffer.windows(4).position(|item| item == b"\r\n\r\n");
                }
                if let Some(end) = header_end {
                    let header_text = String::from_utf8_lossy(&buffer[..end + 4]);
                    let content_length = header_text
                        .lines()
                        .find_map(|line| {
                            let lower = line.to_ascii_lowercase();
                            lower
                                .strip_prefix("content-length:")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    let total_needed = end + 4 + content_length;
                    if buffer.len() >= total_needed {
                        break;
                    }
                }
            }

            let request = String::from_utf8_lossy(&buffer);
            let is_stream = request.contains("\"stream\":true");
            let (content_type, body) = if is_stream {
                (
                    "text/event-stream",
                    concat!(
                        "data: {\"choices\":[{\"delta\":{\"content\":\"mock streamed reply\"},\"finish_reason\":null}]}\n\n",
                        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1,\"total_tokens\":2}}\n\n",
                        "data: [DONE]\n\n"
                    )
                    .to_string(),
                )
            } else {
                (
                    "application/json",
                    "{\"choices\":[{\"message\":{\"content\":\"mock direct reply\",\"tool_calls\":[],\"reasoning_content\":null},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1,\"total_tokens\":2}}".to_string(),
                )
            };

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        }
    });

    format!("http://{}", addr)
}

fn write_config(root: &Path, api_base: &str) -> PathBuf {
    let workspace = root.join("config-workspace");
    fs::create_dir_all(&workspace).unwrap();

    let config = format!(
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
        api_base
    );

    let config_path = root.join("instance").join("config.json");
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(&config_path, config).unwrap();
    config_path
}

#[test]
fn agent_message_smoke_supports_config_and_workspace_override() {
    let temp = tempdir().unwrap();
    let api_base = spawn_mock_openai_server();
    let config_path = write_config(temp.path(), &api_base);
    let workspace_override = temp.path().join("explicit-workspace");

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "--workspace",
            workspace_override.to_str().unwrap(),
            "agent",
            "--message",
            "hello",
        ])
        .output()
        .expect("failed to run agent smoke");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mock "), "{stdout}");
    assert!(workspace_override.exists());
    assert!(workspace_override.join("skills").exists());
}

#[test]
fn agent_logs_and_session_smoke_succeeds() {
    let temp = tempdir().unwrap();
    let api_base = spawn_mock_openai_server();
    let config_path = write_config(temp.path(), &api_base);

    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "agent",
            "--message",
            "hello with logs",
            "--session",
            "cli:test-logs",
            "--no-markdown",
            "--logs",
        ])
        .output()
        .expect("failed to run agent logs smoke");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mock streamed reply"), "{stdout}");
}

#[test]
fn chat_help_smoke_lists_light_chat_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_agent-diva"))
        .args(["chat", "--help"])
        .output()
        .expect("failed to run chat --help");

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--model"), "{stdout}");
    assert!(stdout.contains("--session"), "{stdout}");
    assert!(stdout.contains("--markdown"), "{stdout}");
    assert!(stdout.contains("--no-markdown"), "{stdout}");
    assert!(stdout.contains("--logs"), "{stdout}");
    assert!(stdout.contains("--no-logs"), "{stdout}");
}

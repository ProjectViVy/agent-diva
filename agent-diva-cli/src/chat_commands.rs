use crate::cli_runtime::{
    build_provider, ensure_workspace_templates, session_channel_and_chat_id, CliRuntime,
};
use crate::client::ApiClient;
use agent_diva_agent::{
    agent_loop::SoulGovernanceSettings,
    context::SoulContextSettings,
    runtime_control::RuntimeControlCommand,
    tool_config::network::{
        NetworkToolConfig, WebFetchRuntimeConfig, WebRuntimeConfig, WebSearchRuntimeConfig,
    },
    AgentEvent, AgentLoop, ToolConfig,
};
use agent_diva_core::bus::MessageBus;
use agent_diva_core::config::Config;
use agent_diva_core::cron::CronService;
use agent_diva_files::{FileConfig, FileManager};
use anyhow::Result;
use console::style;
use dialoguer::Input;
use std::sync::Arc;
use tokio::sync::mpsc;

fn render_assistant_response(response: &str, _markdown: bool, show_header: bool) {
    if show_header {
        println!("\n{}", style("Response:").bold());
    }
    println!("{}", response);
}

pub fn build_network_tool_config(config: &Config) -> NetworkToolConfig {
    let api_key = config.tools.web.search.api_key.trim().to_string();
    NetworkToolConfig {
        web: WebRuntimeConfig {
            search: WebSearchRuntimeConfig {
                provider: config.tools.web.search.provider.clone(),
                enabled: config.tools.web.search.enabled,
                api_key: if api_key.is_empty() {
                    None
                } else {
                    Some(api_key)
                },
                max_results: config.tools.web.search.max_results,
            },
            fetch: WebFetchRuntimeConfig {
                enabled: config.tools.web.fetch.enabled,
            },
        },
    }
}

async fn build_local_cli_agent(
    runtime: &CliRuntime,
    model: Option<String>,
    with_runtime_control: bool,
) -> Result<(
    Config,
    String,
    AgentLoop,
    Option<mpsc::UnboundedSender<RuntimeControlCommand>>,
)> {
    let config = runtime.load_config()?;
    let selected_model = model.unwrap_or_else(|| config.agents.defaults.model.clone());
    let workspace = runtime.effective_workspace(&config);
    let _ = ensure_workspace_templates(&workspace)?;

    let bus = MessageBus::new();
    let provider = Arc::new(build_provider(&config, &selected_model)?);
    let tool_config = ToolConfig {
        network: build_network_tool_config(&config),
        exec_timeout: config.tools.exec.timeout,
        restrict_to_workspace: config.tools.restrict_to_workspace,
        mcp_servers: config.tools.active_mcp_servers(),
        cron_service: Some(Arc::new(CronService::new(runtime.cron_store_path(), None))),
        soul_context: SoulContextSettings {
            enabled: config.agents.soul.enabled,
            max_chars: config.agents.soul.max_chars,
            bootstrap_once: config.agents.soul.bootstrap_once,
        },
        notify_on_soul_change: config.agents.soul.notify_on_change,
        soul_governance: SoulGovernanceSettings {
            frequent_change_window_secs: config.agents.soul.frequent_change_window_secs,
            frequent_change_threshold: config.agents.soul.frequent_change_threshold,
            boundary_confirmation_hint: config.agents.soul.boundary_confirmation_hint,
        },
    };

    let (runtime_control_tx, runtime_control_rx) = if with_runtime_control {
        let (tx, rx) = mpsc::unbounded_channel();
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };

    // Initialize shared FileManager for attachment handling
    let storage_path = dirs::data_local_dir()
        .map(|p| p.join("agent-diva").join("files"))
        .unwrap_or_else(|| std::path::PathBuf::from(".agent-diva/files"));
    let file_config = FileConfig::with_path(&storage_path);
    let file_manager = Arc::new(FileManager::new(file_config).await?);

    let agent = AgentLoop::with_tools(
        bus,
        provider,
        workspace,
        Some(selected_model.clone()),
        Some(config.agents.defaults.max_tool_iterations as usize),
        tool_config,
        runtime_control_rx,
        file_manager,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to create agent loop: {}", e))?;

    Ok((config, selected_model, agent, runtime_control_tx))
}

async fn run_local_agent_turn(
    agent: &mut AgentLoop,
    message: &str,
    session_key: &str,
    markdown: bool,
    logs: bool,
    show_response_header: bool,
) -> Result<()> {
    let (channel, chat_id) = session_channel_and_chat_id(session_key);
    if !logs {
        let response = agent
            .process_direct(message, session_key, channel, chat_id)
            .await
            .map_err(|err| anyhow::anyhow!("Failed to process message: {}", err))?;
        render_assistant_response(&response, markdown, show_response_header);
        return Ok(());
    }

    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AgentEvent>();
    let mut response_fut = std::pin::pin!(agent.process_direct_stream(
        message.to_string(),
        session_key.to_string(),
        channel.to_string(),
        chat_id.to_string(),
        event_tx,
    ));
    let mut completed = false;
    let mut final_response = String::new();

    loop {
        tokio::select! {
            result = &mut response_fut, if !completed => {
                completed = true;
                match result {
                    Ok(response) if final_response.is_empty() => final_response = response,
                    Ok(_) => {}
                    Err(err) => anyhow::bail!("Failed to process message: {}", err),
                }
            }
            event = event_rx.recv() => {
                match event {
                    Some(AgentEvent::AssistantDelta { text }) => {
                        print!("{}", text);
                        final_response.push_str(&text);
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                    }
                    Some(AgentEvent::ReasoningDelta { text }) => {
                        print!("{}", style(text).dim());
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                    }
                    Some(AgentEvent::ToolCallStarted { name, args_preview, .. }) => {
                        println!("\n{}", style(format!("[tool:start] {} {}", name, args_preview)).yellow());
                    }
                    Some(AgentEvent::ToolCallFinished { name, result, is_error, .. }) => {
                        let prefix = if is_error { "[tool:error]" } else { "[tool:done]" };
                        println!("\n{}", style(format!("{} {} {}", prefix, name, result)).yellow());
                    }
                    Some(AgentEvent::FinalResponse { content }) => {
                        if final_response.is_empty() {
                            final_response = content;
                        }
                        println!();
                    }
                    Some(AgentEvent::Error { message }) => anyhow::bail!("Failed to process message: {}", message),
                    Some(_) => {}
                    None if completed => break,
                    None => {}
                }
            }
        }
    }

    if !final_response.is_empty() {
        render_assistant_response(&final_response, markdown, show_response_header);
    }

    Ok(())
}

pub async fn run_agent(
    runtime: &CliRuntime,
    message: &str,
    model: Option<String>,
    session: Option<String>,
    markdown: bool,
    logs: bool,
) -> Result<()> {
    let (_config, _selected_model, mut agent, _runtime_control_tx) =
        build_local_cli_agent(runtime, model, false).await?;

    let session_key = session.unwrap_or_else(|| "cli:direct".to_string());

    if logs {
        println!("{}", style("Processing...").cyan());
    }
    run_local_agent_turn(&mut agent, message, &session_key, markdown, logs, true).await
}

async fn run_remote_agent_turn(
    client: &ApiClient,
    message: &str,
    session_key: &str,
    markdown: bool,
    logs: bool,
    show_response_header: bool,
) -> Result<()> {
    let (channel, chat_id) = session_channel_and_chat_id(session_key);
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AgentEvent>();
    let mut chat_fut = std::pin::pin!(client.chat_with_target(
        message.to_string(),
        Some(channel),
        Some(chat_id),
        event_tx,
    ));
    let mut completed = false;
    let mut final_response = String::new();

    loop {
        tokio::select! {
            result = &mut chat_fut, if !completed => {
                completed = true;
                result?;
            }
            event = event_rx.recv() => {
                match event {
                    Some(AgentEvent::AssistantDelta { text }) => {
                        if logs {
                            print!("{}", text);
                            use std::io::Write;
                            let _ = std::io::stdout().flush();
                        }
                        final_response.push_str(&text);
                    }
                    Some(AgentEvent::ReasoningDelta { text }) => {
                        if logs {
                            print!("{}", style(text).dim());
                            use std::io::Write;
                            let _ = std::io::stdout().flush();
                        }
                    }
                    Some(AgentEvent::ToolCallStarted { name, args_preview, .. }) if logs => {
                        println!("\n{}", style(format!("[tool:start] {} {}", name, args_preview)).yellow());
                    }
                    Some(AgentEvent::ToolCallFinished { name, result, is_error, .. }) if logs => {
                        let prefix = if is_error { "[tool:error]" } else { "[tool:done]" };
                        println!("\n{}", style(format!("{} {} {}", prefix, name, result)).yellow());
                    }
                    Some(AgentEvent::FinalResponse { content }) => {
                        if final_response.is_empty() {
                            final_response = content;
                        }
                        if logs {
                            println!();
                        }
                    }
                    Some(AgentEvent::Error { message }) => anyhow::bail!("Remote error: {}", message),
                    Some(_) => {}
                    None if completed => break,
                    None => {}
                }
            }
        }
    }

    if !final_response.is_empty() {
        render_assistant_response(&final_response, markdown, show_response_header);
    }

    Ok(())
}

pub async fn run_agent_remote(
    message: &str,
    session: Option<String>,
    markdown: bool,
    logs: bool,
    api_url: Option<String>,
) -> Result<()> {
    let client = ApiClient::new(api_url);
    let session_key = session.unwrap_or_else(|| "cli:direct:remote".to_string());
    if logs {
        println!("{}", style("Processing (remote)...").cyan());
    }
    run_remote_agent_turn(&client, message, &session_key, markdown, logs, true).await
}

pub async fn run_chat(
    runtime: &CliRuntime,
    model: Option<String>,
    session: Option<String>,
    markdown: bool,
    logs: bool,
) -> Result<()> {
    let (_config, selected_model, mut agent, runtime_control_tx) =
        build_local_cli_agent(runtime, model, true).await?;
    let mut current_session = session.unwrap_or_else(|| "cli:chat".to_string());

    println!("{}", style("Agent Diva Chat").bold().cyan());
    println!("  model: {}", selected_model);
    println!("  session: {}", current_session);
    println!("  commands: /quit /clear /new /stop");

    loop {
        let input: String = Input::new()
            .with_prompt("You")
            .allow_empty(false)
            .interact_text()?;
        let command = input.trim();
        if command.is_empty() {
            continue;
        }

        match command {
            "/quit" => break,
            "/clear" => {
                print!("\x1B[2J\x1B[H");
                continue;
            }
            "/new" => {
                current_session =
                    format!("cli:chat:{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
                println!("session -> {}", current_session);
                continue;
            }
            "/stop" => {
                if let Some(tx) = &runtime_control_tx {
                    let _ = tx.send(RuntimeControlCommand::StopSession {
                        session_key: current_session.clone(),
                    });
                    println!("{}", style("stop requested").yellow());
                }
                continue;
            }
            _ => {}
        }

        run_local_agent_turn(&mut agent, command, &current_session, markdown, logs, true).await?;
    }

    Ok(())
}

pub async fn run_chat_remote(
    _model: Option<String>,
    session: Option<String>,
    markdown: bool,
    logs: bool,
    api_url: Option<String>,
) -> Result<()> {
    let client = ApiClient::new(api_url);
    let mut current_session = session.unwrap_or_else(|| "cli:chat:remote".to_string());

    println!("{}", style("Agent Diva Chat (remote)").bold().cyan());
    println!("  session: {}", current_session);
    println!("  commands: /quit /clear /new /stop");

    loop {
        let input: String = Input::new()
            .with_prompt("You")
            .allow_empty(false)
            .interact_text()?;
        let command = input.trim();
        if command.is_empty() {
            continue;
        }

        match command {
            "/quit" => break,
            "/clear" => {
                print!("\x1B[2J\x1B[H");
                continue;
            }
            "/new" => {
                current_session = format!(
                    "cli:chat:remote:{}",
                    chrono::Local::now().format("%Y%m%d%H%M%S")
                );
                println!("session -> {}", current_session);
                continue;
            }
            "/stop" => {
                let (channel, chat_id) = session_channel_and_chat_id(&current_session);
                let stopped = client.stop(Some(channel), Some(chat_id)).await?;
                println!(
                    "{}",
                    if stopped {
                        style("stop requested").yellow()
                    } else {
                        style("no running task for session").dim()
                    }
                );
                continue;
            }
            _ => {}
        }

        run_remote_agent_turn(&client, command, &current_session, markdown, logs, true).await?;
    }

    Ok(())
}

use crate::approval_commands::{
    print_human, ApprovalCliResult, APPROVAL_QUEUE_UNAVAILABLE, APPROVAL_REQUIRED_NONINTERACTIVE,
};
use crate::cli_runtime::{build_provider, session_channel_and_chat_id, CliRuntime};
use crate::client::ApiClient;
use agent_diva_agent::{
    mask::{MaskFile, MaskRegistry},
    runtime_control::RuntimeControlCommand,
    tool_config::network::{
        NetworkToolConfig, WebFetchRuntimeConfig, WebRuntimeConfig, WebSearchRuntimeConfig,
    },
    tool_config::PlanningConfig,
    AgentEvent, AgentLoop, BuiltInToolsConfig, ToolConfig,
};
use agent_diva_core::ask_user::AskUserCoordinator;
use agent_diva_core::bus::MessageBus;
use agent_diva_core::config::Config;
use agent_diva_core::cron::CronService;
use agent_diva_core::governance::{ApprovalCoordinator, SqliteGovernanceLedger};
use agent_diva_core::reasoning::ThinkingMode;
use agent_diva_files::{FileConfig, FileManager};
use anyhow::Result;
use console::style;
use dialoguer::{Input, Select};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch};

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

pub fn build_builtin_tools_config(config: &Config) -> BuiltInToolsConfig {
    BuiltInToolsConfig {
        filesystem: config.tools.builtin.filesystem,
        shell: config.tools.builtin.shell,
        web_search: config.tools.builtin.web_search,
        web_fetch: config.tools.builtin.web_fetch,
        spawn: config.tools.builtin.spawn,
        cron: config.tools.builtin.cron,
        mcp: config.tools.builtin.mcp,
        attachment: config.tools.builtin.attachment,
        enqueue_background_task: config.tools.builtin.enqueue_background_task,
        update_plan: config.tools.builtin.update_plan,
        ask_user: config.tools.builtin.ask_user,
        memory: config.tools.builtin.memory,
        working_memory: config.tools.builtin.working_memory,
        tool_discovery: config.tools.builtin.tool_discovery,
    }
}

async fn build_local_cli_agent(
    runtime: &CliRuntime,
    model: Option<String>,
    with_runtime_control: bool,
    command_approvals: Option<agent_diva_sandbox::CommandApprovalCoordinator>,
) -> Result<(
    Config,
    String,
    AgentLoop,
    Option<mpsc::UnboundedSender<RuntimeControlCommand>>,
    AskUserCoordinator,
)> {
    let config = runtime.load_config()?;
    let selected_model = model.unwrap_or_else(|| config.agents.defaults.model.clone());
    let workspace = runtime.effective_workspace(&config);

    let bus = MessageBus::new();
    let provider = build_provider(&config, &selected_model)?;
    let planning = Some(PlanningConfig::open_workspace(&workspace).await?);
    let ask_user = AskUserCoordinator::default();
    let tool_config = ToolConfig {
        config_dir: Some(runtime.config_dir().to_path_buf()),
        builtin: build_builtin_tools_config(&config),
        network: build_network_tool_config(&config),
        planning,
        exec_timeout: config.tools.exec.timeout,
        global_timeout_secs: 120,
        command_approvals,
        approval_policy: agent_diva_sandbox::AskForApproval::default(),
        ask_user: Some(ask_user.clone()),
        restrict_to_workspace: config.tools.restrict_to_workspace,
        mcp_servers: config.tools.active_mcp_servers(),
        cron_service: Some(Arc::new(CronService::new(runtime.cron_store_path(), None))),
        run_store: None,
        budget: config.tools.budget.clone().into(),
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

    let mut agent = AgentLoop::with_tools(
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
    agent
        .configure_session_admission(config.agents.defaults.session_admission)
        .map_err(|error| anyhow::anyhow!("Invalid session admission config: {error}"))?;

    Ok((config, selected_model, agent, runtime_control_tx, ask_user))
}

/// Background task that answers pending `ask_user` questions from the terminal.
///
/// The turn itself blocks inside the tool call, so this task polls the
/// coordinator in parallel and resolves questions as the user answers.
/// Non-interactive runtimes (no TTY) cancel every pending question so a
/// headless turn cannot hang for the full coordinator timeout.
pub struct AskUserAnswerer {
    handle: tokio::task::JoinHandle<()>,
    stop_tx: watch::Sender<bool>,
}

impl AskUserAnswerer {
    pub fn spawn(coordinator: AskUserCoordinator, interactive: bool) -> Self {
        let (stop_tx, mut stop_rx) = watch::channel(false);
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop_rx.changed() => break,
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {
                        let questions = coordinator.pending().await;
                        for question in questions {
                            if !interactive {
                                let _ = coordinator.cancel(&question.question_id).await;
                                continue;
                            }
                            Self::prompt_and_answer(&coordinator, &question).await;
                        }
                    }
                }
            }
        });
        Self { handle, stop_tx }
    }

    async fn prompt_and_answer(
        coordinator: &AskUserCoordinator,
        question: &agent_diva_core::ask_user::AskUserQuestion,
    ) {
        println!(
            "\n{} {}",
            style("[ask_user]").bold().cyan(),
            style(&question.question).bold()
        );
        if let Some(context) = &question.context {
            println!("{}", context);
        }

        if !question.choices.is_empty() {
            let mut items: Vec<String> = question.choices.clone();
            if question.allow_other {
                items.push("Other...".to_string());
            }
            let labels: Vec<&str> = items.iter().map(String::as_str).collect();
            let selection = Select::new()
                .with_prompt("Your answer")
                .items(&labels)
                .default(0)
                .interact();
            match selection {
                Ok(index) if index < question.choices.len() => {
                    let _ = coordinator
                        .answer(&question.question_id, Some(index), None)
                        .await;
                }
                Ok(_) => {
                    if let Ok(text) = Input::<String>::new().with_prompt("Other").interact_text() {
                        let _ = coordinator
                            .answer(&question.question_id, None, Some(text))
                            .await;
                    }
                }
                Err(_) => {
                    let _ = coordinator.cancel(&question.question_id).await;
                }
            }
        } else if question.allow_other {
            match Input::<String>::new()
                .with_prompt("Your answer")
                .interact_text()
            {
                Ok(text) => {
                    let _ = coordinator
                        .answer(&question.question_id, None, Some(text))
                        .await;
                }
                Err(_) => {
                    let _ = coordinator.cancel(&question.question_id).await;
                }
            }
        } else {
            let _ = coordinator.cancel(&question.question_id).await;
            println!(
                "{}",
                style("[ask_user] no choices provided; question cancelled").yellow()
            );
        }
    }

    /// Stop the polling loop, wait for it to exit, then cancel leftovers so a
    /// stale question cannot block the next turn for the full timeout.
    pub async fn finish(self) {
        let _ = self.stop_tx.send(true);
        let _ = self.handle.await;
    }
}

/// Cancel any question still pending after a turn (e.g. answered from another
/// surface or abandoned by an interrupted prompt).
pub async fn cancel_stale_ask_user_questions(coordinator: &AskUserCoordinator) {
    for question in coordinator.pending().await {
        let _ = coordinator.cancel(&question.question_id).await;
    }
}

async fn run_local_agent_turn(
    agent: &mut AgentLoop,
    message: &str,
    session_key: &str,
    markdown: bool,
    logs: bool,
    show_response_header: bool,
    ask_user: &AskUserCoordinator,
) -> Result<()> {
    let interactive = std::io::stdin().is_terminal();
    let answerer = AskUserAnswerer::spawn(ask_user.clone(), interactive);
    let result = run_local_agent_turn_inner(
        agent,
        message,
        session_key,
        markdown,
        logs,
        show_response_header,
    )
    .await;
    answerer.finish().await;
    cancel_stale_ask_user_questions(ask_user).await;
    result
}

async fn run_local_agent_turn_inner(
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

pub struct AgentRunOptions {
    pub model: Option<String>,
    pub session: Option<String>,
    pub markdown: bool,
    pub logs: bool,
    pub queue: bool,
    pub json: bool,
}

pub async fn run_agent(
    runtime: &CliRuntime,
    message: &str,
    options: AgentRunOptions,
) -> Result<()> {
    if options.queue {
        emit_headless_error(APPROVAL_QUEUE_UNAVAILABLE, options.json)?;
        anyhow::bail!(APPROVAL_QUEUE_UNAVAILABLE);
    }
    let config = runtime.load_config()?;
    let workspace = runtime.effective_workspace(&config);
    let governance_dir = workspace.join(".laputa");
    std::fs::create_dir_all(&governance_dir)?;
    let governance_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(governance_dir.join("governance.db"))
                .create_if_missing(true),
        )
        .await?;
    let governance = ApprovalCoordinator::new(Arc::new(
        SqliteGovernanceLedger::new(governance_pool).await?,
    ));
    let rules = Arc::new(agent_diva_sandbox::CommandRuleStore::open(
        runtime.config_dir().join("execpolicy.toml"),
    )?);
    let coordinator = agent_diva_sandbox::CommandApprovalCoordinator::default()
        .with_command_rules(rules)
        .governed(
            governance,
            agent_diva_core::workspace_identity::canonical_workspace_id(&workspace),
        );
    let mut requests = coordinator.subscribe();
    let saw_approval = Arc::new(AtomicBool::new(false));
    let saw_approval_task = saw_approval.clone();
    let resolver = coordinator.clone();
    let rejection_task = tokio::spawn(async move {
        while let Ok(request) = requests.recv().await {
            saw_approval_task.store(true, Ordering::SeqCst);
            let _ = resolver
                .resolve(
                    &request.approval_id,
                    agent_diva_sandbox::ApprovalDecision::Reject,
                )
                .await;
        }
    });
    let (_config, _selected_model, mut agent, _runtime_control_tx, ask_user) =
        build_local_cli_agent(runtime, options.model, false, Some(coordinator)).await?;

    let session_key = options.session.unwrap_or_else(|| "cli:direct".to_string());

    if options.logs {
        println!("{}", style("Processing...").cyan());
    }
    let result = run_local_agent_turn(
        &mut agent,
        message,
        &session_key,
        options.markdown,
        options.logs,
        true,
        &ask_user,
    )
    .await;
    rejection_task.abort();
    result?;
    if saw_approval.load(Ordering::SeqCst) {
        emit_headless_error(APPROVAL_REQUIRED_NONINTERACTIVE, options.json)?;
        anyhow::bail!(APPROVAL_REQUIRED_NONINTERACTIVE);
    }
    Ok(())
}

fn emit_headless_error(reason_code: &str, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string(&ApprovalCliResult {
                ok: false,
                reason_code: Some(reason_code.to_string()),
                approval: None,
            })?
        );
    } else {
        eprintln!("{reason_code}");
    }
    Ok(())
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
                    Some(AgentEvent::ReasoningDelta { text }) if logs => {
                        print!("{}", style(text).dim());
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
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

pub async fn run_agent_remote_governed(
    message: &str,
    session: Option<String>,
    markdown: bool,
    logs: bool,
    api_url: Option<String>,
    queue: bool,
    json: bool,
) -> Result<()> {
    let client = ApiClient::new(api_url);
    let session_key = session.unwrap_or_else(|| "cli:direct:remote".to_string());
    let baseline_page = match client
        .list_approvals(Some("pending"), Some(&session_key))
        .await
    {
        Ok(page) => page,
        Err(_) => {
            let reason = if queue {
                APPROVAL_QUEUE_UNAVAILABLE
            } else {
                APPROVAL_REQUIRED_NONINTERACTIVE
            };
            emit_headless_error(reason, json)?;
            anyhow::bail!(reason);
        }
    };
    let baseline = baseline_page
        .approvals
        .into_iter()
        .map(|approval| approval.request_id)
        .collect::<std::collections::HashSet<_>>();
    let (channel, chat_id) = session_channel_and_chat_id(&session_key);
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AgentEvent>();
    let mut chat = std::pin::pin!(client.chat_with_target(
        message.to_string(),
        Some(channel),
        Some(chat_id),
        event_tx,
    ));
    let mut poll = tokio::time::interval(std::time::Duration::from_millis(100));
    let mut final_response = String::new();
    loop {
        tokio::select! {
            result = &mut chat => {
                result?;
                if !final_response.is_empty() {
                    render_assistant_response(&final_response, markdown, true);
                }
                return Ok(());
            }
            _ = poll.tick() => {
                let pending = match client
                    .list_approvals(Some("pending"), Some(&session_key))
                    .await
                {
                    Ok(page) => page,
                    Err(_) => {
                        let reason = if queue {
                            APPROVAL_QUEUE_UNAVAILABLE
                        } else {
                            APPROVAL_REQUIRED_NONINTERACTIVE
                        };
                        emit_headless_error(reason, json)?;
                        anyhow::bail!(reason);
                    }
                };
                if let Some(approval) = pending.approvals.into_iter()
                    .find(|approval| !baseline.contains(&approval.request_id))
                {
                    if queue {
                        if approval.domain == "command" {
                            let key = format!("cli-queue-cancel-{}-v{}", approval.request_id, approval.version);
                            let _ = client.cancel_approval(&approval.request_id, approval.version, &key).await;
                            emit_headless_error(APPROVAL_QUEUE_UNAVAILABLE, json)?;
                            anyhow::bail!(APPROVAL_QUEUE_UNAVAILABLE);
                        }
                        if json {
                            println!("{}", serde_json::to_string(&ApprovalCliResult {
                                ok: false,
                                reason_code: Some(APPROVAL_REQUIRED_NONINTERACTIVE.to_string()),
                                approval: Some(approval),
                            })?);
                        } else {
                            println!("Approval queued; execution has not succeeded.");
                            print_human(&approval);
                            println!("status: agent-diva approvals list --session {}", session_key);
                        }
                        anyhow::bail!(APPROVAL_REQUIRED_NONINTERACTIVE);
                    }
                    let key = format!("cli-headless-cancel-{}-v{}", approval.request_id, approval.version);
                    let _ = client.cancel_approval(&approval.request_id, approval.version, &key).await;
                    emit_headless_error(APPROVAL_REQUIRED_NONINTERACTIVE, json)?;
                    anyhow::bail!(APPROVAL_REQUIRED_NONINTERACTIVE);
                }
            }
            event = event_rx.recv() => {
                match event {
                    Some(AgentEvent::AssistantDelta { text }) => {
                        if logs { print!("{text}"); }
                        final_response.push_str(&text);
                    }
                    Some(AgentEvent::FinalResponse { content }) if final_response.is_empty() => {
                        final_response = content;
                    }
                    Some(AgentEvent::Error { message }) => anyhow::bail!(message),
                    Some(_) => {}
                    None => {}
                }
            }
        }
    }
}

pub async fn run_chat(
    runtime: &CliRuntime,
    model: Option<String>,
    session: Option<String>,
    markdown: bool,
    logs: bool,
) -> Result<()> {
    let (config, selected_model, mut agent, runtime_control_tx, ask_user) =
        build_local_cli_agent(runtime, model, true, None).await?;
    let mut current_session = session.unwrap_or_else(|| "cli:chat".to_string());

    // Initialize mask registry from workspace/masks/
    let workspace = runtime.effective_workspace(&config);
    let masks_dir = workspace.join("masks");
    let mut mask_registry = MaskRegistry::new(&masks_dir);

    println!("{}", style("Agent Diva Chat").bold().cyan());
    println!("  model: {}", selected_model);
    println!("  session: {}", current_session);
    println!("  title: (untitled)");
    println!("  commands: /quit /clear /new /stop /mask /thinking auto|on|off /compact");

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
                println!("title -> (untitled)");
                continue;
            }
            "/stop" => {
                if let Some(tx) = &runtime_control_tx {
                    let (reply_tx, _reply_rx) = tokio::sync::oneshot::channel();
                    let _ = tx.send(RuntimeControlCommand::StopSession {
                        session_key: current_session.clone(),
                        request_id: None,
                        reply_tx,
                    });
                    println!("{}", style("stop requested").yellow());
                }
                continue;
            }
            cmd if cmd.starts_with("/thinking ") => {
                let mode_str = cmd.trim_start_matches("/thinking ").trim();
                let mode = match mode_str {
                    "auto" => ThinkingMode::Auto,
                    "on" => ThinkingMode::On,
                    "off" => ThinkingMode::Off,
                    _ => {
                        println!("{}", style("usage: /thinking auto|on|off").yellow());
                        continue;
                    }
                };
                if let Some(tx) = &runtime_control_tx {
                    let _ = tx.send(RuntimeControlCommand::SetThinking { mode });
                    println!("{}", style(format!("thinking mode -> {:?}", mode)).green());
                }
                continue;
            }
            "/compact" => {
                if let Some(tx) = &runtime_control_tx {
                    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                    let _ = tx.send(RuntimeControlCommand::CompactSession {
                        session_key: current_session.clone(),
                        reply_tx,
                    });
                    println!("{}", style("compacting...").cyan());
                    match reply_rx.await {
                        Ok(Ok(msg)) => println!("{}", style(msg).green()),
                        Ok(Err(e)) => println!("{}", style(format!("compact error: {}", e)).red()),
                        Err(_) => println!("{}", style("compact: no response from agent").red()),
                    }
                } else {
                    println!(
                        "{}",
                        style("/compact requires local agent (runtime control unavailable)")
                            .yellow()
                    );
                }
                continue;
            }
            cmd if cmd.starts_with("/mask") => {
                handle_mask_command(cmd, &mut mask_registry);
                continue;
            }
            _ => {}
        }

        run_local_agent_turn(
            &mut agent,
            command,
            &current_session,
            markdown,
            logs,
            true,
            &ask_user,
        )
        .await?;
    }

    Ok(())
}

/// Handle `/mask` subcommands: list, wear <name>, off, status, reload.
fn handle_mask_command(cmd: &str, registry: &mut MaskRegistry) {
    let parts: Vec<&str> = cmd.splitn(3, ' ').collect();
    let sub = parts.get(1).copied().unwrap_or("status");

    match sub {
        "list" => {
            let masks = registry.list();
            println!("{}", style("🎭 可用面具:").bold());
            for m in &masks {
                let icon = m.frontmatter.icon.as_deref().unwrap_or("🎭");
                let desc = m.frontmatter.description.as_deref().unwrap_or("（无描述）");
                println!("  {} {} — {}", icon, m.frontmatter.name, desc);
            }
        }
        "wear" => {
            let name = match parts.get(2) {
                Some(n) => n.trim(),
                None => {
                    println!("{}", style("用法: /mask wear <name>").yellow());
                    return;
                }
            };
            // Suggest context compression before switching.
            println!(
                "{}",
                style("💡 建议切换前执行 /compress 以压缩上下文").dim()
            );
            match registry.switch_to(name) {
                Ok(mask) => {
                    let icon = mask.frontmatter.icon.as_deref().unwrap_or("🎭");
                    println!(
                        "{}",
                        style(format!("🎭 已切换为「{}」{} 模式", name, icon)).green()
                    );
                }
                Err(e) => {
                    println!("{}", style(format!("❌ {}", e)).red());
                    let available: Vec<&str> = registry
                        .list()
                        .iter()
                        .map(|m| m.frontmatter.name.as_str())
                        .collect();
                    println!("  可用: {}", available.join(", "));
                }
            }
        }
        "off" => {
            registry.switch_off();
            println!("{}", style("🎭 已摘下面具，恢复默认模式").green());
        }
        "status" => match registry.current_mask_name() {
            Some(name) => {
                let mask = registry.current_mask().unwrap();
                let icon = mask.frontmatter.icon.as_deref().unwrap_or("🎭");
                println!("🎭 当前面具: {} {}", icon, name);
            }
            None => {
                println!(
                    "🎭 当前面具: {} (默认)",
                    style(MaskFile::DEFAULT_NAME).dim()
                );
            }
        },
        "reload" => {
            registry.reload();
            let count = registry.list().len();
            println!(
                "{}",
                style(format!("🎭 已重新加载面具文件 ({} 个面具)", count)).green()
            );
        }
        other => {
            println!(
                "{}",
                style(format!(
                    "未知子命令: {}。可用: list | wear <name> | off | status | reload",
                    other
                ))
                .yellow()
            );
        }
    }
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
    println!("  commands: /quit /clear /new /stop /thinking auto|on|off /compact");

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
            "/compact" => {
                println!(
                    "{}",
                    style("/compact is not supported in remote mode — use local chat instead")
                        .yellow()
                );
                continue;
            }
            _ => {}
        }

        run_remote_agent_turn(&client, command, &current_session, markdown, logs, true).await?;
    }

    Ok(())
}

#[cfg(test)]
mod ask_user_answerer_tests {
    use super::*;
    use agent_diva_core::ask_user::AskUserStatus;

    async fn wait_pending(coordinator: &AskUserCoordinator) -> String {
        for _ in 0..100 {
            if let Some(question) = coordinator.pending().await.into_iter().next() {
                return question.question_id;
            }
            tokio::task::yield_now().await;
        }
        panic!("no pending question registered");
    }

    #[tokio::test]
    async fn headless_answerer_cancels_pending_questions() {
        let coordinator = AskUserCoordinator::default();
        let answerer = AskUserAnswerer::spawn(coordinator.clone(), false);
        let ask = tokio::spawn({
            let coordinator = coordinator.clone();
            async move { coordinator.request("Proceed?", vec![], false, None).await }
        });
        let question_id = wait_pending(&coordinator).await;

        // The headless answerer cancels the question without a TTY (100ms poll).
        for _ in 0..50 {
            if coordinator.pending().await.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(coordinator.pending().await.is_empty());

        let response = ask.await.unwrap().unwrap();
        assert_eq!(response.status, AskUserStatus::Cancelled);
        assert_eq!(response.question_id, question_id);

        answerer.finish().await;
    }

    #[tokio::test]
    async fn finish_cancels_stale_questions() {
        let coordinator = AskUserCoordinator::default();
        let answerer = AskUserAnswerer::spawn(coordinator.clone(), false);
        let ask = tokio::spawn({
            let coordinator = coordinator.clone();
            async move { coordinator.request("Stale?", vec![], false, None).await }
        });
        let _question_id = wait_pending(&coordinator).await;

        // Stop the answerer and cancel leftovers directly.
        answerer.finish().await;
        cancel_stale_ask_user_questions(&coordinator).await;
        assert!(coordinator.pending().await.is_empty());

        let response = ask.await.unwrap().unwrap();
        assert_eq!(response.status, AskUserStatus::Cancelled);
    }
}

#[cfg(test)]
mod approval_mode_tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn pending(domain: &str) -> serde_json::Value {
        serde_json::json!({
            "request_id": format!("{domain}-queued"),
            "version": 1,
            "domain": domain,
            "capability": match domain {
                "command" => "command_execute",
                "plan" => "plan_execute",
                _ => "memory_apply"
            },
            "resource": {
                "workspace_id": "workspace",
                "session_id": "cli:queued",
                "kind": domain,
                "resource_id": format!("{domain}-resource"),
                "boundary": null
            },
            "risk": "high",
            "status": "pending",
            "expires_at": "2026-08-03T12:05:00Z",
            "evidence": [{"kind":"test","reference":"fixture"}],
            "actions": ["allow", "deny", "cancel"],
            "presentation": {"title": format!("{domain} approval")},
            "reason_code": null
        })
    }

    async fn mount_queued_turn(server: &MockServer, approval: serde_json::Value) {
        let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let response_reads = reads.clone();
        Mock::given(method("GET"))
            .and(path("/api/approvals"))
            .respond_with(move |_request: &wiremock::Request| {
                let approvals = if response_reads.fetch_add(1, Ordering::SeqCst) == 0 {
                    Vec::new()
                } else {
                    vec![approval.clone()]
                };
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "approvals": approvals, "next_cursor": null
                }))
            })
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/runtime/turns"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_delay(std::time::Duration::from_secs(2))
                    .set_body_string("event: final\ndata: done\n\n"),
            )
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn explicit_queue_returns_plan_pending_without_waiting_for_completion() {
        let server = MockServer::start().await;
        mount_queued_turn(&server, pending("plan")).await;
        let result = run_agent_remote_governed(
            "prepare plan",
            Some("cli:queued".into()),
            false,
            false,
            Some(format!("{}/api", server.uri())),
            true,
            false,
        )
        .await;
        assert!(result
            .unwrap_err()
            .to_string()
            .contains(APPROVAL_REQUIRED_NONINTERACTIVE));
    }

    #[tokio::test]
    async fn default_headless_cancels_high_risk_plan_pending() {
        let server = MockServer::start().await;
        let approval = pending("plan");
        mount_queued_turn(&server, approval.clone()).await;
        Mock::given(method("POST"))
            .and(path("/api/approvals/plan-queued/cancel"))
            .respond_with(ResponseTemplate::new(200).set_body_json({
                let mut value = approval;
                value["status"] = "revoked".into();
                value
            }))
            .expect(1)
            .mount(&server)
            .await;
        let error = run_agent_remote_governed(
            "apply memory",
            Some("cli:queued".into()),
            false,
            false,
            Some(format!("{}/api", server.uri())),
            false,
            false,
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains(APPROVAL_REQUIRED_NONINTERACTIVE));
    }

    #[tokio::test]
    async fn explicit_queue_rejects_command_because_raw_payload_is_not_durable() {
        let server = MockServer::start().await;
        let approval = pending("command");
        mount_queued_turn(&server, approval.clone()).await;
        Mock::given(method("POST"))
            .and(path("/api/approvals/command-queued/cancel"))
            .respond_with(ResponseTemplate::new(200).set_body_json({
                let mut value = approval;
                value["status"] = "revoked".into();
                value
            }))
            .expect(1)
            .mount(&server)
            .await;
        let error = run_agent_remote_governed(
            "run shell",
            Some("cli:queued".into()),
            false,
            false,
            Some(format!("{}/api", server.uri())),
            true,
            false,
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains(APPROVAL_QUEUE_UNAVAILABLE));
    }
}

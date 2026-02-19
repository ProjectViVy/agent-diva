//! CLI entry point for agent-diva

use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use dialoguer::{Confirm, Input, Select};
use agent_diva_agent::{AgentEvent, AgentLoop, ToolConfig};
use agent_diva_channels::ChannelManager;
use agent_diva_core::bus::{InboundMessage, MessageBus};
use agent_diva_core::config::{Config, ConfigLoader, ProviderConfig, ProvidersConfig};
use agent_diva_core::cron::{CronSchedule, CronService};
use agent_diva_providers::{LiteLLMClient, ProviderRegistry};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "agent-diva")]
#[command(about = "A lightweight personal AI assistant framework")]
#[command(version = "0.2.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration directory
    #[arg(short, long, global = true)]
    config_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize agent-diva configuration
    Onboard,
    /// Run the agent gateway
    Gateway,
    /// Send a message to the agent
    Agent {
        /// Message to send
        #[arg(short, long)]
        message: Option<String>,
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
        /// Session key for conversation continuity
        #[arg(short, long)]
        session: Option<String>,
    },
    /// Launch interactive TUI chat
    Tui {
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
        /// Session key for conversation continuity
        #[arg(short, long)]
        session: Option<String>,
    },
    /// Show status information
    Status,
    /// Manage channels
    Channels {
        #[command(subcommand)]
        command: ChannelCommands,
    },
    /// Manage cron jobs
    Cron {
        #[command(subcommand)]
        command: CronCommands,
    },
}

#[derive(Subcommand)]
enum ChannelCommands {
    /// Login to a channel
    Login { channel: String },
    /// Show channel status
    Status,
}

#[derive(Subcommand)]
enum CronCommands {
    /// Add a cron job
    Add {
        /// Job name
        #[arg(short, long)]
        name: String,
        /// Message for agent
        #[arg(short, long)]
        message: String,
        /// Run every N seconds
        #[arg(short, long)]
        every: Option<i64>,
        /// Cron expression (e.g. '0 9 * * *')
        #[arg(short, long)]
        cron_expr: Option<String>,
        /// ISO 8601 timestamp (e.g. '2023-10-27T10:00:00Z') for one-time task
        #[arg(long)]
        at: Option<String>,
        /// Timezone for cron expression (e.g. 'America/New_York')
        #[arg(long)]
        timezone: Option<String>,
        /// Deliver response to channel
        #[arg(short, long)]
        deliver: bool,
        /// Recipient for delivery
        #[arg(short = 't', long)]
        to: Option<String>,
        /// Channel for delivery (e.g. 'telegram', 'whatsapp')
        #[arg(long)]
        channel: Option<String>,
    },
    /// List cron jobs
    List {
        /// Include disabled jobs
        #[arg(short, long)]
        all: bool,
    },
    /// Remove a cron job
    Remove {
        /// Job ID to remove
        job_id: String,
    },
    /// Enable or disable a cron job
    Enable {
        /// Job ID
        job_id: String,
        /// Enable or disable
        #[arg(short, long)]
        enabled: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Create config loader
    let config_loader = if let Some(dir) = cli.config_dir {
        ConfigLoader::with_dir(dir)
    } else {
        ConfigLoader::new()
    };

    match cli.command {
        Commands::Onboard => {
            info!("Running onboard command");
            run_onboard(&config_loader).await?;
        }
        Commands::Gateway => {
            info!("Starting gateway");
            run_gateway(&config_loader).await?;
        }
        Commands::Agent {
            message,
            model,
            session,
        } => {
            if let Some(msg) = message {
                info!("Processing message: {}", msg);
                run_agent(&config_loader, &msg, model, session).await?;
            } else {
                warn!("No message provided");
                println!("Use --message to provide a message");
                println!("Example: agent-diva agent --message 'Hello, world!'");
            }
        }
        Commands::Tui { model, session } => {
            info!("Starting TUI");
            run_tui(&config_loader, model, session).await?;
        }
        Commands::Status => {
            info!("Showing status");
            run_status(&config_loader).await?;
        }
        Commands::Channels { command } => match command {
            ChannelCommands::Login { channel } => {
                info!("Logging in to channel: {}", channel);
                run_channel_login(&config_loader, channel).await?;
            }
            ChannelCommands::Status => {
                info!("Showing channel status");
                run_channel_status(&config_loader).await?;
            }
        },
        Commands::Cron { command } => match command {
            CronCommands::Add {
                name,
                message,
                every,
                cron_expr,
                at,
                timezone,
                deliver,
                to,
                channel,
            } => {
                info!("Adding cron job");
                run_cron_add(
                    &config_loader,
                    name,
                    message,
                    every,
                    cron_expr,
                    at,
                    timezone,
                    deliver,
                    to,
                    channel,
                )
                .await?;
            }
            CronCommands::List { all } => {
                info!("Listing cron jobs");
                run_cron_list(&config_loader, all).await?;
            }
            CronCommands::Remove { job_id } => {
                info!("Removing cron job: {}", job_id);
                run_cron_remove(&config_loader, job_id).await?;
            }
            CronCommands::Enable { job_id, enabled } => {
                info!("Enabling/disabling cron job: {}", job_id);
                run_cron_enable(&config_loader, job_id, enabled).await?;
            }
        },
    }

    Ok(())
}

/// Expand tilde in path
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn provider_config_by_name<'a>(
    providers: &'a ProvidersConfig,
    name: &str,
) -> Option<&'a ProviderConfig> {
    match name {
        "anthropic" => Some(&providers.anthropic),
        "openai" => Some(&providers.openai),
        "openrouter" => Some(&providers.openrouter),
        "deepseek" => Some(&providers.deepseek),
        "groq" => Some(&providers.groq),
        "zhipu" => Some(&providers.zhipu),
        "dashscope" => Some(&providers.dashscope),
        "vllm" => Some(&providers.vllm),
        "gemini" => Some(&providers.gemini),
        "moonshot" => Some(&providers.moonshot),
        "minimax" => Some(&providers.minimax),
        "aihubmix" => Some(&providers.aihubmix),
        "custom" => Some(&providers.custom),
        _ => None,
    }
}

fn build_provider(config: &Config, model: &str) -> Result<LiteLLMClient> {
    let registry = ProviderRegistry::new();
    let name_from_prefix = model
        .split('/')
        .next()
        .and_then(|prefix| registry.find_by_name(prefix))
        .map(|spec| spec.name.clone());
    let spec = name_from_prefix
        .as_deref()
        .and_then(|name| registry.find_by_name(name))
        .or_else(|| registry.find_by_model(model))
        .ok_or_else(|| anyhow::anyhow!("No provider found for model: {}", model))?;
    let provider_config = provider_config_by_name(&config.providers, &spec.name);

    let api_key = provider_config
        .map(|cfg| cfg.api_key.clone())
        .filter(|key| !key.is_empty());
    let api_base = provider_config
        .and_then(|cfg| cfg.api_base.clone())
        .and_then(|base| {
            if base.trim().is_empty() {
                None
            } else {
                Some(base)
            }
        });
    let extra_headers = provider_config
        .and_then(|cfg| cfg.extra_headers.clone())
        .filter(|headers| !headers.is_empty());

    Ok(LiteLLMClient::new(
        api_key,
        api_base,
        model.to_string(),
        extra_headers,
        Some(spec.name.clone()),
    ))
}

/// Run the onboard wizard
async fn run_onboard(loader: &ConfigLoader) -> Result<()> {
    println!("{}", style("Welcome to Agent Diva!").bold().cyan());
    println!("Let's set up your configuration.\n");

    let config_path = loader.config_dir().join("config.json");
    if config_path.exists() {
        let overwrite = Confirm::new()
            .with_prompt("Configuration already exists. Overwrite?")
            .default(false)
            .interact()?;
        if !overwrite {
            println!("Onboard cancelled.");
            return Ok(());
        }
    }

    // Select provider
    let providers = vec![
        "anthropic",
        "openai",
        "openrouter",
        "deepseek",
        "groq",
        "zhipu",
        "dashscope",
        "gemini",
        "moonshot",
        "minimax",
        "aihubmix",
    ];
    let provider_idx = Select::new()
        .with_prompt("Select your LLM provider")
        .items(&providers)
        .default(0)
        .interact()?;
    let provider_name = providers[provider_idx];

    // API Key
    let api_key: String = Input::new()
        .with_prompt(format!("Enter your {} API key", provider_name))
        .interact_text()?;

    // Model
    let default_model = match provider_name {
        "anthropic" => "anthropic/claude-sonnet-4-5",
        "openai" => "openai/gpt-4o",
        "openrouter" => "openrouter/anthropic/claude-sonnet-4",
        "deepseek" => "deepseek/deepseek-chat",
        "groq" => "groq/llama-3.3-70b-versatile",
        "zhipu" => "zhipu/glm-4-flash",
        "dashscope" => "dashscope/qwen-max",
        "gemini" => "gemini/gemini-2.0-flash",
        "moonshot" => "moonshot/moonshot-v1-8k",
        "minimax" => "minimax/MiniMax-M2.1",
        "aihubmix" => "aihubmix/claude-sonnet-4",
        _ => "anthropic/claude-sonnet-4-5",
    };
    let model: String = Input::new()
        .with_prompt("Enter the model to use")
        .default(default_model.to_string())
        .interact_text()?;

    // Workspace
    let workspace: String = Input::new()
        .with_prompt("Enter workspace directory")
        .default("~/.agent-diva/workspace".to_string())
        .interact_text()?;

    // Create config
    let mut config = Config::default();
    config.agents.defaults.model = model;
    config.agents.defaults.workspace = workspace;

    // Set provider API key
    match provider_name {
        "anthropic" => config.providers.anthropic.api_key = api_key,
        "openai" => config.providers.openai.api_key = api_key,
        "openrouter" => config.providers.openrouter.api_key = api_key,
        "deepseek" => config.providers.deepseek.api_key = api_key,
        "groq" => config.providers.groq.api_key = api_key,
        "zhipu" => config.providers.zhipu.api_key = api_key,
        "dashscope" => config.providers.dashscope.api_key = api_key,
        "gemini" => config.providers.gemini.api_key = api_key,
        "moonshot" => config.providers.moonshot.api_key = api_key,
        "minimax" => config.providers.minimax.api_key = api_key,
        "aihubmix" => config.providers.aihubmix.api_key = api_key,
        _ => {}
    }

    // Save config
    loader.save(&config)?;

    // Create workspace directory
    let workspace_path = expand_tilde(&config.agents.defaults.workspace);
    std::fs::create_dir_all(&workspace_path)?;

    println!(
        "\n{}",
        style("Configuration saved successfully!").green().bold()
    );
    println!("Config location: {}", config_path.display());
    println!("\nYou can now run:");
    println!("  {} - Start the gateway", style("agent-diva gateway").cyan());
    println!(
        "  {} - Send a message",
        style("agent-diva agent --message 'Hello!'").cyan()
    );

    Ok(())
}

/// Run the agent gateway
async fn run_gateway(loader: &ConfigLoader) -> Result<()> {
    let config = loader.load()?;

    // Check if API key is configured
    let registry = ProviderRegistry::new();
    let provider_config = registry.find_by_model(&config.agents.defaults.model);

    if provider_config.is_none() {
        anyhow::bail!(
            "No provider found for model: {}",
            config.agents.defaults.model
        );
    }

    let workspace = expand_tilde(&config.agents.defaults.workspace);
    std::fs::create_dir_all(&workspace)?;

    println!("{}", style("Starting Agent Diva Gateway...").bold().cyan());
    println!("Model: {}", config.agents.defaults.model);
    println!("Workspace: {}", workspace.display());

    // Create message bus
    let bus = MessageBus::new();

    // Create provider
    let provider = Arc::new(build_provider(&config, &config.agents.defaults.model)?);

    // Extract config values before moving
    let model = config.agents.defaults.model.clone();
    let max_iterations = config.agents.defaults.max_tool_iterations as usize;
    let api_key = config.tools.web.search.api_key.clone();
    let exec_timeout = config.tools.exec.timeout;
    let restrict_to_workspace = config.tools.restrict_to_workspace;

    // Create agent loop with tools
    let tool_config = ToolConfig {
        brave_api_key: if api_key.is_empty() {
            None
        } else {
            Some(api_key)
        },
        exec_timeout,
        restrict_to_workspace,
        mcp_servers: config.tools.mcp_servers.clone(),
    };

    let agent = AgentLoop::with_tools(
        bus.clone(),
        provider,
        workspace,
        Some(model),
        Some(max_iterations),
        tool_config,
    );

    // Create and initialize channel manager
    let mut channel_manager = ChannelManager::new(config.clone());

    // Bridge channel inbound queue -> message bus inbound queue
    let (inbound_tx, mut inbound_rx) = mpsc::channel::<InboundMessage>(1024);
    channel_manager.set_inbound_sender(inbound_tx);
    let bus_for_inbound_bridge = bus.clone();
    let inbound_bridge_handle = tokio::spawn(async move {
        while let Some(msg) = inbound_rx.recv().await {
            if let Err(e) = bus_for_inbound_bridge.publish_inbound(msg) {
                error!("Failed to publish inbound message to bus: {}", e);
            }
        }
    });

    println!(
        "\n{}",
        style("Gateway is running. Press Ctrl+C to stop.").green()
    );

    // Initialize channels
    if let Err(e) = channel_manager.initialize().await {
        error!("Failed to initialize channels: {}", e);
    }

    // Subscribe outbound messages to each enabled channel.
    // Note: only channels initialized in manager will actually send.
    let configured_channels = [
        ("telegram", config.channels.telegram.enabled),
        ("discord", config.channels.discord.enabled),
        ("whatsapp", config.channels.whatsapp.enabled),
        ("feishu", config.channels.feishu.enabled),
        ("dingtalk", config.channels.dingtalk.enabled),
        ("email", config.channels.email.enabled),
        ("slack", config.channels.slack.enabled),
        ("qq", config.channels.qq.enabled),
    ];

    let channel_manager = Arc::new(channel_manager);
    for (channel_name, enabled) in configured_channels {
        if !enabled {
            continue;
        }
        let manager = channel_manager.clone();
        let channel_name = channel_name.to_string();
        let channel_key = channel_name.clone();
        bus.subscribe_outbound(channel_name, move |msg| {
            let manager = manager.clone();
            let channel_key = channel_key.clone();
            async move {
                if let Err(e) = manager.send(&channel_key, msg).await {
                    error!("Failed to send outbound message to {}: {}", channel_key, e);
                }
            }
        })
        .await;
    }

    // Start outbound dispatcher loop
    let bus_for_outbound_dispatch = bus.clone();
    let outbound_dispatch_handle = tokio::spawn(async move {
        bus_for_outbound_dispatch.dispatch_outbound_loop().await;
    });

    // Start channel manager
    let channel_manager_for_task = channel_manager.clone();
    let _channel_handle = tokio::spawn(async move {
        if let Err(e) = channel_manager_for_task.start_all().await {
            error!("Channel manager error: {}", e);
        }
    });

    // Run agent loop
    // Note: we need to use Option to take ownership and avoid moving agent twice
    let agent = Some(agent);
    let _agent_handle = tokio::spawn(async move {
        if let Some(mut agent) = agent {
            if let Err(e) = agent.run().await {
                error!("Agent loop error: {}", e);
            }
        }
    });

    // Wait for shutdown signal
    // Note: we only wait for ctrl_c here, the spawned tasks run independently
    tokio::signal::ctrl_c().await?;
    println!("\n{}", style("Shutting down...").yellow());

    // Stop the bus to signal all components to shut down
    bus.stop().await;
    if let Err(e) = channel_manager.stop_all().await {
        error!("Failed to stop channels: {}", e);
    }

    inbound_bridge_handle.abort();
    let _ = inbound_bridge_handle.await;
    outbound_dispatch_handle.abort();
    let _ = outbound_dispatch_handle.await;

    println!("{}", style("Gateway stopped.").green());
    Ok(())
}

/// Run agent in direct mode
async fn run_agent(
    loader: &ConfigLoader,
    message: &str,
    model: Option<String>,
    session: Option<String>,
) -> Result<()> {
    let config = loader.load()?;
    let selected_model = model
        .clone()
        .unwrap_or_else(|| config.agents.defaults.model.clone());

    let workspace = expand_tilde(&config.agents.defaults.workspace);
    std::fs::create_dir_all(&workspace)?;

    let bus = MessageBus::new();
    let provider = Arc::new(build_provider(&config, &selected_model)?);

    let tool_config = ToolConfig {
        brave_api_key: Some(config.tools.web.search.api_key.clone()).filter(|s| !s.is_empty()),
        exec_timeout: config.tools.exec.timeout,
        restrict_to_workspace: config.tools.restrict_to_workspace,
        mcp_servers: config.tools.mcp_servers.clone(),
    };

    let mut agent = AgentLoop::with_tools(
        bus,
        provider,
        workspace,
        Some(selected_model),
        Some(config.agents.defaults.max_tool_iterations as usize),
        tool_config,
    );

    let session_key = session.unwrap_or_else(|| "cli:direct".to_string());
    let chat_id = session_key.split(':').nth(1).unwrap_or("direct");

    println!("{}", style("Processing...").cyan());

    match agent
        .process_direct(message, &session_key, "cli", chat_id)
        .await
    {
        Ok(response) => {
            println!("\n{}", style("Response:").bold());
            println!("{}", response);
        }
        Err(e) => {
            error!("Error processing message: {}", e);
            anyhow::bail!("Failed to process message: {}", e);
        }
    }

    Ok(())
}

#[derive(Clone)]
enum TimelineKind {
    User,
    Assistant,
    Tool,
    System,
    Error,
}

#[derive(Clone)]
struct TimelineItem {
    kind: TimelineKind,
    text: String,
}

struct TuiApp {
    input: String,
    timeline: Vec<TimelineItem>,
    pending: bool,
    should_quit: bool,
    scroll: u16,
    assistant_line: Option<usize>,
    session_key: String,
    model: String,
}

impl TuiApp {
    fn new(session_key: String, model: String) -> Self {
        Self {
            input: String::new(),
            timeline: vec![TimelineItem {
                kind: TimelineKind::System,
                text: "Welcome to Agent Diva TUI. Enter to send, Shift+Enter for newline. /new /clear /quit".to_string(),
            }],
            pending: false,
            should_quit: false,
            scroll: 0,
            assistant_line: None,
            session_key,
            model,
        }
    }

    fn add_line(&mut self, kind: TimelineKind, text: impl Into<String>) {
        self.timeline.push(TimelineItem {
            kind,
            text: text.into(),
        });
        self.scroll = self.scroll.saturating_add(1);
    }

    fn apply_agent_event(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::IterationStarted {
                index,
                max_iterations,
            } => {
                self.assistant_line = None;
                self.add_line(
                    TimelineKind::System,
                    format!("iteration {}/{}", index, max_iterations),
                );
            }
            AgentEvent::AssistantDelta { text } => {
                if text.is_empty() {
                    return;
                }
                if let Some(idx) = self.assistant_line {
                    if let Some(item) = self.timeline.get_mut(idx) {
                        item.text.push_str(&text);
                    }
                } else {
                    self.timeline.push(TimelineItem {
                        kind: TimelineKind::Assistant,
                        text,
                    });
                    self.assistant_line = Some(self.timeline.len() - 1);
                }
            }
            AgentEvent::ToolCallStarted {
                name,
                args_preview,
                call_id,
            } => {
                self.assistant_line = None;
                self.add_line(
                    TimelineKind::Tool,
                    format!("[tool:start] {} [{}] {}", name, call_id, args_preview),
                );
            }
            AgentEvent::ToolCallFinished {
                name,
                result,
                is_error,
                call_id,
            } => {
                self.assistant_line = None;
                let prefix = if is_error {
                    "[tool:error]"
                } else {
                    "[tool:done]"
                };
                self.add_line(
                    if is_error {
                        TimelineKind::Error
                    } else {
                        TimelineKind::Tool
                    },
                    format!("{} {} [{}] {}", prefix, name, call_id, result),
                );
            }
            AgentEvent::FinalResponse { .. } => {
                self.pending = false;
                self.assistant_line = None;
            }
            AgentEvent::Error { message } => {
                self.pending = false;
                self.assistant_line = None;
                self.add_line(TimelineKind::Error, format!("error: {}", message));
            }
        }
    }
}

async fn run_tui(
    loader: &ConfigLoader,
    model: Option<String>,
    session: Option<String>,
) -> Result<()> {
    let config = loader.load()?;
    let selected_model = model.unwrap_or_else(|| config.agents.defaults.model.clone());
    let workspace = expand_tilde(&config.agents.defaults.workspace);
    std::fs::create_dir_all(&workspace)?;

    let bus = MessageBus::new();
    let provider = Arc::new(build_provider(&config, &selected_model)?);

    let tool_config = ToolConfig {
        brave_api_key: Some(config.tools.web.search.api_key.clone()).filter(|s| !s.is_empty()),
        exec_timeout: config.tools.exec.timeout,
        restrict_to_workspace: config.tools.restrict_to_workspace,
        mcp_servers: config.tools.mcp_servers.clone(),
    };

    let mut agent = AgentLoop::with_tools(
        bus,
        provider,
        workspace,
        Some(selected_model.clone()),
        Some(config.agents.defaults.max_tool_iterations as usize),
        tool_config,
    );

    let current_session = session.unwrap_or_else(|| "cli:tui".to_string());
    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<(String, String)>();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AgentEvent>();

    tokio::spawn(async move {
        while let Some((prompt, session_key)) = request_rx.recv().await {
            let chat_id = session_key.split(':').nth(2).unwrap_or("tui").to_string();
            let _ = agent
                .process_direct_stream(prompt, session_key, "cli", chat_id, event_tx.clone())
                .await;
        }
    });

    let mut stdout = io::stdout();
    enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new(current_session, selected_model);
    loop {
        while let Ok(evt) = event_rx.try_recv() {
            app.apply_agent_event(evt);
        }

        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(5),
                ])
                .split(frame.area());

            let status = if app.pending { "processing" } else { "idle" };
            let status_line = format!(
                "model: {} | session: {} | status: {}",
                app.model, app.session_key, status
            );
            frame.render_widget(
                Paragraph::new(status_line)
                    .block(Block::default().borders(Borders::ALL).title("agent-diva tui")),
                chunks[0],
            );

            let mut lines = Vec::new();
            for item in &app.timeline {
                let (label, color) = match item.kind {
                    TimelineKind::User => ("user", Color::Cyan),
                    TimelineKind::Assistant => ("assistant", Color::Green),
                    TimelineKind::Tool => ("tool", Color::Yellow),
                    TimelineKind::System => ("system", Color::Blue),
                    TimelineKind::Error => ("error", Color::Red),
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("[{}] ", label), Style::default().fg(color)),
                    Span::raw(item.text.clone()),
                ]));
            }

            let timeline = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title("timeline"))
                .wrap(Wrap { trim: false })
                .scroll((app.scroll, 0));
            frame.render_widget(timeline, chunks[1]);

            frame.render_widget(
                Paragraph::new(app.input.clone())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("input (Enter send, Shift+Enter newline)"),
                    )
                    .wrap(Wrap { trim: false }),
                chunks[2],
            );
            frame.set_cursor_position((chunks[2].x + 1 + app.input.len() as u16, chunks[2].y + 1));
        })?;

        if event::poll(std::time::Duration::from_millis(60))? {
            if let CEvent::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.should_quit = true;
                    }
                    KeyCode::Esc => app.should_quit = true,
                    KeyCode::PageUp | KeyCode::Up => {
                        app.scroll = app.scroll.saturating_sub(1);
                    }
                    KeyCode::PageDown | KeyCode::Down => {
                        app.scroll = app.scroll.saturating_add(1);
                    }
                    KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        app.input.push('\n');
                    }
                    KeyCode::Enter => {
                        let content = app.input.trim().to_string();
                        app.input.clear();
                        if content.is_empty() {
                            continue;
                        }
                        if content == "/quit" {
                            app.should_quit = true;
                        } else if content == "/clear" {
                            app.timeline.clear();
                            app.assistant_line = None;
                        } else if content == "/new" {
                            app.session_key =
                                format!("cli:tui:{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
                            app.assistant_line = None;
                            app.add_line(
                                TimelineKind::System,
                                format!("new session: {}", app.session_key),
                            );
                        } else {
                            app.add_line(TimelineKind::User, content.clone());
                            app.pending = true;
                            app.assistant_line = None;
                            let _ = request_tx.send((content, app.session_key.clone()));
                        }
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    KeyCode::Char(ch) => {
                        app.input.push(ch);
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Show system status
async fn run_status(loader: &ConfigLoader) -> Result<()> {
    let config = loader.load()?;

    println!("{}", style("Agent Diva Status").bold().cyan());
    println!("Version: 0.2.0 (Rust)\n");

    // Config info
    println!("{}", style("Configuration:").bold());
    println!("  Config directory: {}", loader.config_dir().display());
    println!("  Workspace: {}", config.agents.defaults.workspace);
    println!("  Default model: {}", config.agents.defaults.model);
    println!();

    // Provider info
    println!("{}", style("Providers:").bold());
    let _registry = ProviderRegistry::new();
    for (name, spec) in [
        ("anthropic", &config.providers.anthropic),
        ("openai", &config.providers.openai),
        ("openrouter", &config.providers.openrouter),
        ("deepseek", &config.providers.deepseek),
        ("groq", &config.providers.groq),
        ("minimax", &config.providers.minimax),
    ] {
        let status = if spec.api_key.is_empty() {
            style("not configured").red()
        } else {
            style("configured").green()
        };
        println!("  {}: {}", name, status);
    }
    println!();

    // Channel info
    println!("{}", style("Channels:").bold());
    let channels = vec![
        ("Telegram", config.channels.telegram.enabled),
        ("Discord", config.channels.discord.enabled),
        ("WhatsApp", config.channels.whatsapp.enabled),
        ("Feishu", config.channels.feishu.enabled),
        ("DingTalk", config.channels.dingtalk.enabled),
        ("Email", config.channels.email.enabled),
        ("Slack", config.channels.slack.enabled),
        ("QQ", config.channels.qq.enabled),
    ];
    for (name, enabled) in channels {
        let status = if enabled {
            style("enabled").green()
        } else {
            style("disabled").dim()
        };
        println!("  {}: {}", name, status);
    }

    Ok(())
}

/// Show channel status
async fn run_channel_status(loader: &ConfigLoader) -> Result<()> {
    let config = loader.load()?;

    println!("{}", style("Channel Status").bold().cyan());
    println!();

    let channels = vec![
        (
            "Telegram",
            config.channels.telegram.enabled,
            config.channels.telegram.token.is_empty(),
        ),
        (
            "Discord",
            config.channels.discord.enabled,
            config.channels.discord.token.is_empty(),
        ),
        ("WhatsApp", config.channels.whatsapp.enabled, false),
        (
            "Feishu",
            config.channels.feishu.enabled,
            config.channels.feishu.app_id.is_empty(),
        ),
        (
            "DingTalk",
            config.channels.dingtalk.enabled,
            config.channels.dingtalk.client_id.is_empty(),
        ),
        (
            "Email",
            config.channels.email.enabled,
            config.channels.email.imap_username.is_empty(),
        ),
        (
            "Slack",
            config.channels.slack.enabled,
            config.channels.slack.bot_token.is_empty(),
        ),
        (
            "QQ",
            config.channels.qq.enabled,
            config.channels.qq.app_id.is_empty(),
        ),
    ];

    for (name, enabled, missing_creds) in channels {
        if enabled {
            if missing_creds {
                println!(
                    "  {}: {} (missing credentials)",
                    name,
                    style("enabled").yellow()
                );
            } else {
                println!("  {}: {}", name, style("enabled").green());
            }
        } else {
            println!("  {}: {}", name, style("disabled").dim());
        }
    }

    Ok(())
}

fn run_process(command: &str, args: &[&str], cwd: &Path, envs: &[(&str, String)]) -> Result<()> {
    let mut process = Command::new(command);
    process
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    for (key, value) in envs {
        process.env(key, value);
    }

    let status = process
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to start '{} {}': {}", command, args.join(" "), e))?;
    if !status.success() {
        anyhow::bail!(
            "Command failed: '{} {}' (exit: {})",
            command,
            args.join(" "),
            status
        );
    }
    Ok(())
}

fn copy_bridge_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if name == "node_modules" || name == "dist" {
            continue;
        }

        let target = dst.join(&file_name);
        if path.is_dir() {
            copy_bridge_dir(&path, &target)?;
        } else if path.is_file() {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

fn find_bridge_source_dir() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join("../bridge"),
        manifest_dir.join("../../bridge"),
        std::env::current_dir().ok()?.join("bridge"),
    ];

    candidates
        .into_iter()
        .find(|path| path.join("package.json").exists())
}

fn setup_bridge(loader: &ConfigLoader) -> Result<PathBuf> {
    let bridge_dir = loader.config_dir().join("bridge");
    if bridge_dir.join("dist").join("index.js").exists() {
        return Ok(bridge_dir);
    }

    run_process("npm", &["--version"], Path::new("."), &[]).map_err(|_| {
        anyhow::anyhow!("npm not found. Please install Node.js >= 20 and ensure npm is in PATH.")
    })?;

    let source = find_bridge_source_dir().ok_or_else(|| {
        anyhow::anyhow!("Bridge source not found. Expected a local 'bridge' directory.")
    })?;

    println!("{}", style("Setting up WhatsApp bridge...").cyan());
    if bridge_dir.exists() {
        fs::remove_dir_all(&bridge_dir)?;
    }
    copy_bridge_dir(&source, &bridge_dir)?;

    println!("Installing bridge dependencies...");
    run_process("npm", &["install"], &bridge_dir, &[])?;
    println!("Building bridge...");
    run_process("npm", &["run", "build"], &bridge_dir, &[])?;
    println!("{}", style("Bridge is ready.").green());
    Ok(bridge_dir)
}

fn extract_bridge_port(bridge_url: &str) -> Option<String> {
    let without_scheme = bridge_url.split("://").nth(1).unwrap_or(bridge_url);
    let host_port = without_scheme.split('/').next().unwrap_or(without_scheme);
    host_port.rsplit(':').next().and_then(|s| {
        if s.chars().all(|c| c.is_ascii_digit()) {
            Some(s.to_string())
        } else {
            None
        }
    })
}

async fn run_channel_login(loader: &ConfigLoader, channel: String) -> Result<()> {
    if channel.to_lowercase() != "whatsapp" {
        println!("Channel '{}' login flow is not implemented yet.", channel);
        return Ok(());
    }

    let config = loader.load()?;
    let bridge_dir = setup_bridge(loader)?;
    let auth_dir = loader.config_dir().join("whatsapp-auth");
    let media_dir = loader.config_dir().join("whatsapp-media");
    fs::create_dir_all(&auth_dir)?;
    fs::create_dir_all(&media_dir)?;

    println!("{}", style("Starting WhatsApp bridge...").cyan());
    println!("Scan the QR code shown below with WhatsApp Linked Devices.\n");

    let mut envs = vec![
        ("AUTH_DIR", auth_dir.to_string_lossy().to_string()),
        ("MEDIA_DIR", media_dir.to_string_lossy().to_string()),
    ];
    if let Some(port) = extract_bridge_port(&config.channels.whatsapp.bridge_url) {
        envs.push(("BRIDGE_PORT", port));
    }

    run_process("npm", &["start"], &bridge_dir, &envs)?;
    Ok(())
}

/// Add a cron job
#[allow(clippy::too_many_arguments)]
async fn run_cron_add(
    loader: &ConfigLoader,
    name: String,
    message: String,
    every: Option<i64>,
    cron_expr: Option<String>,
    at: Option<String>,
    timezone: Option<String>,
    deliver: bool,
    to: Option<String>,
    channel: Option<String>,
) -> Result<()> {
    // Determine schedule type
    let schedule = if let Some(seconds) = every {
        CronSchedule::every(seconds * 1000)
    } else if let Some(expr) = cron_expr {
        CronSchedule::cron(expr, timezone)
    } else if let Some(iso_time) = at {
        let dt = chrono::DateTime::parse_from_rfc3339(&iso_time)
            .map_err(|e| anyhow::anyhow!("Invalid ISO time: {}", e))?;
        CronSchedule::at(dt.timestamp_millis())
    } else {
        anyhow::bail!("Must specify --every, --cron-expr, or --at");
    };

    // Get data directory
    let data_dir = loader.config_dir().join("data");
    std::fs::create_dir_all(&data_dir)?;
    let store_path = data_dir.join("cron").join("jobs.json");
    std::fs::create_dir_all(store_path.parent().unwrap())?;

    let service = Arc::new(CronService::new(store_path, None));
    service.start().await;

    let job = service
        .add_job(name.clone(), schedule, message, deliver, channel, to, false)
        .await;

    service.stop().await;

    println!(
        "{} Added job '{}' ({})",
        style("✓").green().bold(),
        job.name,
        job.id
    );

    Ok(())
}

/// List cron jobs
async fn run_cron_list(loader: &ConfigLoader, include_all: bool) -> Result<()> {
    let data_dir = loader.config_dir().join("data");
    let store_path = data_dir.join("cron").join("jobs.json");

    if !store_path.exists() {
        println!("No scheduled jobs.");
        return Ok(());
    }

    let service = Arc::new(CronService::new(store_path, None));
    service.start().await;

    let jobs = service.list_jobs(include_all).await;

    service.stop().await;

    if jobs.is_empty() {
        println!("No scheduled jobs.");
        return Ok(());
    }

    println!("{}", style("Scheduled Jobs").bold().cyan());
    println!();

    for job in jobs {
        // Format schedule
        let schedule_str = match &job.schedule {
            CronSchedule::Every { every_ms } => {
                format!("every {}s", every_ms / 1000)
            }
            CronSchedule::Cron { expr, tz } => {
                if let Some(tz) = tz {
                    format!("{} ({})", expr, tz)
                } else {
                    expr.clone()
                }
            }
            CronSchedule::At { at_ms } => {
                use chrono::TimeZone;
                let secs = at_ms / 1000;
                let nsecs = ((at_ms % 1000) * 1_000_000) as u32;
                let dt = chrono::Utc
                    .timestamp_opt(secs, nsecs)
                    .single()
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "(invalid)".to_string());
                format!("at {}", dt)
            }
        };

        // Format next run
        let next_run_str = if let Some(next_ms) = job.state.next_run_at_ms {
            use chrono::TimeZone;
            let secs = next_ms / 1000;
            let nsecs = ((next_ms % 1000) * 1_000_000) as u32;
            chrono::Utc
                .timestamp_opt(secs, nsecs)
                .single()
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "(invalid)".to_string())
        } else {
            "-".to_string()
        };

        let status = if job.enabled {
            style("enabled").green()
        } else {
            style("disabled").dim()
        };

        println!("  {} ({})", style(&job.name).bold(), job.id);
        println!("    Schedule: {}", schedule_str);
        println!("    Status: {}", status);
        println!("    Next run: {}", next_run_str);
        println!();
    }

    Ok(())
}

/// Remove a cron job
async fn run_cron_remove(loader: &ConfigLoader, job_id: String) -> Result<()> {
    let data_dir = loader.config_dir().join("data");
    let store_path = data_dir.join("cron").join("jobs.json");

    if !store_path.exists() {
        println!("No scheduled jobs.");
        return Ok(());
    }

    let service = Arc::new(CronService::new(store_path, None));
    service.start().await;

    let removed = service.remove_job(&job_id).await;

    service.stop().await;

    if removed {
        println!("{} Removed job {}", style("✓").green().bold(), job_id);
    } else {
        println!("{} Job {} not found", style("✗").red(), job_id);
    }

    Ok(())
}

/// Enable or disable a cron job
async fn run_cron_enable(loader: &ConfigLoader, job_id: String, enabled: bool) -> Result<()> {
    let data_dir = loader.config_dir().join("data");
    let store_path = data_dir.join("cron").join("jobs.json");

    if !store_path.exists() {
        println!("No scheduled jobs.");
        return Ok(());
    }

    let service = Arc::new(CronService::new(store_path, None));
    service.start().await;

    let result = service.enable_job(&job_id, enabled).await;

    service.stop().await;

    if result.is_some() {
        let action = if enabled { "Enabled" } else { "Disabled" };
        println!("{} {} job {}", style("✓").green().bold(), action, job_id);
    } else {
        println!("{} Job {} not found", style("✗").red(), job_id);
    }

    Ok(())
}

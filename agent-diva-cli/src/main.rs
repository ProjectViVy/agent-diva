//! CLI entry point for agent-diva

use agent_diva_agent::{
    agent_loop::SoulGovernanceSettings, context::SoulContextSettings,
    runtime_control::RuntimeControlCommand, AgentEvent, AgentLoop, ToolConfig,
};
use agent_diva_cli::chat_commands::{
    build_network_tool_config, run_agent, run_agent_remote, run_chat, run_chat_remote,
};
use agent_diva_cli::cli_runtime::{
    available_provider_names, build_provider, channel_statuses, collect_status_report,
    current_provider_name, default_model_from_registry, doctor_report, ensure_workspace_templates,
    fetch_provider_models, print_json, redacted_config_value, set_provider_credentials, CliRuntime,
    SwarmCortexDoctorV1,
};
use agent_diva_cli::provider_commands::{
    run_provider_list, run_provider_login, run_provider_models, run_provider_set,
    run_provider_status,
};
use agent_diva_core::bus::MessageBus;
use agent_diva_core::config::validate::validate_config;
use agent_diva_core::config::Config;
use agent_diva_core::cron::{CronSchedule, CronService};
use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use console::style;
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use dialoguer::{Input, Select};
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

mod service;
use agent_diva_cli::client::ApiClient;
use service::{run_service_command, ServiceCommands};

use agent_diva_manager::{run_local_gateway, GatewayRuntimeConfig, DEFAULT_GATEWAY_PORT};
use agent_diva_tools::wtf;

#[derive(Parser)]
#[command(name = "agent-diva")]
#[command(about = "A lightweight personal AI assistant framework")]
#[command(version = "0.4.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration file path
    #[arg(long, global = true, conflicts_with = "config_dir")]
    config: Option<PathBuf>,

    /// Configuration directory
    #[arg(short, long, global = true)]
    config_dir: Option<PathBuf>,

    /// Override workspace for this command without modifying config.json
    #[arg(short = 'w', long, global = true)]
    workspace: Option<PathBuf>,

    /// Connect to remote agent-diva-manager
    #[arg(long, global = true)]
    remote: bool,

    /// Remote API URL (default: http://localhost:3000/api)
    #[arg(long, global = true)]
    api_url: Option<String>,
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Commands {
    /// Initialize agent-diva configuration
    Onboard(OnboardArgs),
    /// Run and manage the agent gateway
    Gateway {
        #[command(subcommand)]
        command: Option<GatewayCommands>,
    },
    /// Send a message to the agent
    Agent {
        /// Message to send
        #[arg(long)]
        message: Option<String>,
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
        /// Session key for conversation continuity
        #[arg(short, long)]
        session: Option<String>,
        /// Render assistant output as markdown-friendly text
        #[arg(long = "markdown", action = clap::ArgAction::SetTrue, overrides_with = "no_markdown")]
        markdown: bool,
        /// Disable markdown-friendly rendering
        #[arg(long = "no-markdown", action = clap::ArgAction::SetTrue)]
        no_markdown: bool,
        /// Show streaming reasoning/tool logs while the agent runs
        #[arg(long = "logs", action = clap::ArgAction::SetTrue, overrides_with = "no_logs")]
        logs: bool,
        /// Disable streaming reasoning/tool logs
        #[arg(long = "no-logs", action = clap::ArgAction::SetTrue)]
        no_logs: bool,
    },
    /// Start lightweight interactive chat
    Chat {
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
        /// Session key for conversation continuity
        #[arg(short, long)]
        session: Option<String>,
        /// Render assistant output as markdown-friendly text
        #[arg(long = "markdown", action = clap::ArgAction::SetTrue, overrides_with = "no_markdown")]
        markdown: bool,
        /// Disable markdown-friendly rendering
        #[arg(long = "no-markdown", action = clap::ArgAction::SetTrue)]
        no_markdown: bool,
        /// Show streaming reasoning/tool logs while the agent runs
        #[arg(long = "logs", action = clap::ArgAction::SetTrue, overrides_with = "no_logs")]
        logs: bool,
        /// Disable streaming reasoning/tool logs
        #[arg(long = "no-logs", action = clap::ArgAction::SetTrue)]
        no_logs: bool,
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
    Status(StatusArgs),
    /// Manage channels
    Channels {
        #[command(subcommand)]
        command: ChannelCommands,
    },
    /// Manage providers
    Provider {
        #[command(subcommand)]
        command: ProviderCommands,
    },
    /// Manage configuration and instance paths
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Manage the Windows service companion
    Service {
        #[command(subcommand)]
        command: ServiceCommands,
    },
    /// Manage cron jobs
    Cron {
        #[command(subcommand)]
        command: CronCommands,
    },
}

fn command_writes_logs_to_terminal(command: &Commands) -> bool {
    !matches!(command, Commands::Tui { .. } | Commands::Agent { .. })
}

fn command_shows_startup_branding(command: &Commands) -> bool {
    !matches!(command, Commands::Agent { .. })
}

#[derive(Subcommand, Clone, Copy)]
#[command(rename_all = "kebab-case")]
enum GatewayCommands {
    /// Run the gateway in foreground mode
    Run,
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum ChannelCommands {
    /// Login to a channel
    Login { channel: String },
    /// Show channel status
    Status(StatusArgs),
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum ProviderCommands {
    /// List manageable providers
    List(StatusArgs),
    /// Show provider readiness and active model/provider
    Status(StatusArgs),
    /// Update the default provider/model and credentials
    Set {
        /// Provider name to activate
        #[arg(long)]
        provider: String,
        /// Model to store in config; defaults to registry default_model when present
        #[arg(long)]
        model: Option<String>,
        /// API key to store for the selected provider
        #[arg(long)]
        api_key: Option<String>,
        /// Optional API base URL for the selected provider
        #[arg(long)]
        api_base: Option<String>,
        /// Output structured JSON
        #[arg(long)]
        json: bool,
    },
    /// Fetch the provider's available models from the runtime API
    Models {
        /// Provider name to query
        #[arg(long)]
        provider: String,
        /// Output structured JSON
        #[arg(long)]
        json: bool,
        /// Fall back to bundled static models if runtime discovery fails or is unsupported
        #[arg(long)]
        static_fallback: bool,
    },
    /// Authenticate with a provider
    Login {
        /// Provider name
        provider: String,
        /// Output structured JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
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
    /// Manually run a cron job
    Run {
        /// Job ID
        job_id: String,
        /// Run even if job is disabled
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Args, Clone, Default)]
struct StatusArgs {
    /// Output structured JSON
    #[arg(long)]
    json: bool,
    /// Include swarm / cortex / capability diagnostics (developer-facing; CLI or status JSON only — not user chat transcripts, NFR-R2).
    #[arg(long, visible_alias = "cortex")]
    swarm: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ConfigOutputFormat {
    Json,
    Pretty,
}

#[derive(Args, Clone, Default)]
struct OnboardArgs {
    /// Provider name to configure
    #[arg(long)]
    provider: Option<String>,
    /// Model to store in config
    #[arg(long)]
    model: Option<String>,
    /// API key to store for the selected provider
    #[arg(long)]
    api_key: Option<String>,
    /// Optional API base URL for the selected provider
    #[arg(long)]
    api_base: Option<String>,
    /// Workspace path to store in config
    #[arg(long)]
    workspace: Option<PathBuf>,
    /// Overwrite config with defaults before applying user input
    #[arg(long)]
    force: bool,
    /// Refresh config by preserving existing values and filling missing defaults
    #[arg(long)]
    refresh: bool,
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum ConfigCommands {
    /// Print resolved config and runtime paths
    Path(StatusArgs),
    /// Initialize config non-interactively using onboard semantics
    Init(OnboardArgs),
    /// Refresh config.json and workspace templates without overwriting user values
    Refresh,
    /// Validate config schema and semantic rules
    Validate(StatusArgs),
    /// Run validation plus runtime readiness checks
    Doctor(StatusArgs),
    /// Print the current effective config with secrets redacted
    Show {
        #[arg(long, value_enum, default_value_t = ConfigOutputFormat::Pretty)]
        format: ConfigOutputFormat,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let structured_output = is_structured_output(&cli.command);
    let enable_terminal_logs = command_writes_logs_to_terminal(&cli.command);
    let show_startup_branding = command_shows_startup_branding(&cli.command);
    if !structured_output && show_startup_branding {
        wtf::print_ascii_agent_diva_logo();
    }

    let runtime = CliRuntime::from_paths(
        cli.config.clone(),
        cli.config_dir.clone(),
        cli.workspace.clone(),
    );

    // Load config for logging
    let config = runtime.loader().load().unwrap_or_default();

    // Initialize tracing
    let _guard = agent_diva_core::logging::init_logging_with_terminal_output(
        &config.logging,
        enable_terminal_logs,
    );

    match cli.command {
        Commands::Onboard(args) => {
            if !structured_output {
                info!("Running onboard command");
            }
            run_onboard(&runtime, args).await?;
        }
        Commands::Gateway { command } => match command.unwrap_or(GatewayCommands::Run) {
            GatewayCommands::Run => {
                if !structured_output {
                    info!("Starting gateway");
                }
                run_gateway(&runtime).await?;
            }
        },
        Commands::Agent {
            message,
            model,
            session,
            markdown,
            no_markdown,
            logs,
            no_logs,
        } => {
            let markdown = markdown || !no_markdown;
            let logs = logs && !no_logs;
            if let Some(msg) = message {
                if !structured_output {
                    info!("Processing message: {}", msg);
                }
                if cli.remote {
                    run_agent_remote(&msg, session, markdown, logs, cli.api_url).await?;
                } else {
                    run_agent(&runtime, &msg, model, session, markdown, logs).await?;
                }
            } else {
                warn!("No message provided");
                println!("Use --message to provide a message");
                println!("Example: agent-diva agent --message 'Hello, world!'");
            }
        }
        Commands::Chat {
            model,
            session,
            markdown,
            no_markdown,
            logs,
            no_logs,
        } => {
            let markdown = markdown || !no_markdown;
            let logs = logs && !no_logs;
            if cli.remote {
                run_chat_remote(model, session, markdown, logs, cli.api_url).await?;
            } else {
                run_chat(&runtime, model, session, markdown, logs).await?;
            }
        }
        Commands::Tui { model, session } => {
            if !structured_output {
                info!("Starting TUI");
            }
            if cli.remote {
                run_tui_remote(cli.api_url, session).await?;
            } else {
                run_tui(&runtime, model, session).await?;
            }
        }
        Commands::Status(args) => {
            if !structured_output {
                info!("Showing status");
            }
            run_status(&runtime, args.json, args.swarm).await?;
        }
        Commands::Channels { command } => match command {
            ChannelCommands::Login { channel } => {
                if !structured_output {
                    info!("Logging in to channel: {}", channel);
                }
                run_channel_login(&runtime, channel).await?;
            }
            ChannelCommands::Status(args) => {
                if !structured_output {
                    info!("Showing channel status");
                }
                run_channel_status(&runtime, args.json).await?;
            }
        },
        Commands::Provider { command } => match command {
            ProviderCommands::List(args) => run_provider_list(&runtime, args.json).await?,
            ProviderCommands::Status(args) => run_provider_status(&runtime, args.json).await?,
            ProviderCommands::Set {
                provider,
                model,
                api_key,
                api_base,
                json,
            } => {
                run_provider_set(&runtime, provider, model, api_key, api_base, json).await?;
            }
            ProviderCommands::Models {
                provider,
                json,
                static_fallback,
            } => {
                run_provider_models(&runtime, provider, json, static_fallback).await?;
            }
            ProviderCommands::Login { provider, json } => {
                run_provider_login(provider, json).await?;
            }
        },
        Commands::Config { command } => match command {
            ConfigCommands::Path(args) => run_config_path(&runtime, args.json).await?,
            ConfigCommands::Init(args) => run_onboard(&runtime, args).await?,
            ConfigCommands::Refresh => run_config_refresh(&runtime).await?,
            ConfigCommands::Validate(args) => run_config_validate(&runtime, args.json).await?,
            ConfigCommands::Doctor(args) => {
                run_config_doctor(&runtime, args.json, args.swarm).await?
            }
            ConfigCommands::Show { format } => run_config_show(&runtime, format).await?,
        },
        Commands::Service { command } => {
            if !structured_output {
                info!("Processing service command");
            }
            let service_config_dir = runtime.loader().config_dir().to_path_buf();
            run_service_command(Some(&service_config_dir), command).await?;
        }
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
                if !structured_output {
                    info!("Adding cron job");
                }
                run_cron_add(
                    &runtime, name, message, every, cron_expr, at, timezone, deliver, to, channel,
                )
                .await?;
            }
            CronCommands::List { all } => {
                if !structured_output {
                    info!("Listing cron jobs");
                }
                run_cron_list(&runtime, all).await?;
            }
            CronCommands::Remove { job_id } => {
                if !structured_output {
                    info!("Removing cron job: {}", job_id);
                }
                run_cron_remove(&runtime, job_id).await?;
            }
            CronCommands::Enable { job_id, enabled } => {
                if !structured_output {
                    info!("Enabling/disabling cron job: {}", job_id);
                }
                run_cron_enable(&runtime, job_id, enabled).await?;
            }
            CronCommands::Run { job_id, force } => {
                if !structured_output {
                    info!("Running cron job: {}", job_id);
                }
                run_cron_run(&runtime, job_id, force).await?;
            }
        },
    }

    Ok(())
}

/// Run the onboard wizard
async fn run_onboard(runtime: &CliRuntime, args: OnboardArgs) -> Result<()> {
    println!("{}", style("Welcome to Agent Diva!").bold().cyan());
    println!("Let's set up your configuration.\n");

    let config_path = runtime.config_path().to_path_buf();
    let config_exists = config_path.exists();

    let mut config = if args.force {
        Config::default()
    } else if config_exists {
        runtime.load_config()?
    } else {
        Config::default()
    };

    if config_exists
        && !args.force
        && !args.refresh
        && args.provider.is_none()
        && args.model.is_none()
    {
        let action_idx = Select::new()
            .with_prompt("Configuration already exists. Choose action")
            .items(&[
                "Refresh existing config",
                "Overwrite with new setup",
                "Cancel",
            ])
            .default(0)
            .interact()?;
        match action_idx {
            0 => {}
            1 => config = Config::default(),
            _ => {
                println!("Onboard cancelled.");
                return Ok(());
            }
        }
    }

    let provider_names = available_provider_names();
    let provider_name = if let Some(provider) = args.provider.clone() {
        provider
    } else {
        let default_index = provider_names
            .iter()
            .position(|name| name == "deepseek")
            .unwrap_or(0);
        let provider_idx = Select::new()
            .with_prompt("Select your LLM provider")
            .items(&provider_names)
            .default(default_index)
            .interact()?;
        provider_names[provider_idx].clone()
    };

    if !provider_names.iter().any(|name| name == &provider_name) {
        anyhow::bail!(
            "Unknown provider '{}'. Supported: {}",
            provider_name,
            provider_names.join(", ")
        );
    }

    let api_key = match args.api_key {
        Some(value) => Some(value),
        None if args.refresh => None,
        None => {
            let value: String = Input::new()
                .with_prompt(format!(
                    "Enter your {} API key (leave empty to keep current)",
                    provider_name
                ))
                .allow_empty(true)
                .interact_text()?;
            if value.trim().is_empty() {
                None
            } else {
                Some(value)
            }
        }
    };

    let api_base = match args.api_base {
        Some(value) => Some(value),
        None if args.refresh => None,
        None => {
            let value: String = Input::new()
                .with_prompt("Optional API base URL (leave empty for default)")
                .allow_empty(true)
                .interact_text()?;
            if value.trim().is_empty() {
                None
            } else {
                Some(value)
            }
        }
    };

    let current_provider_model =
        if current_provider_name(&config).as_deref() == Some(&provider_name) {
            Some(config.agents.defaults.model.clone())
        } else {
            None
        };
    let preferred_model = current_provider_model
        .clone()
        .or_else(|| default_model_from_registry(&provider_name))
        .unwrap_or_else(|| config.agents.defaults.model.clone());
    let model = if let Some(model) = args.model {
        model
    } else {
        let mut discovery_config = config.clone();
        set_provider_credentials(
            &mut discovery_config,
            &provider_name,
            api_key.clone(),
            api_base.clone(),
        );
        let discovered_models = fetch_provider_models(&discovery_config, &provider_name, true)
            .await
            .map(|catalog| catalog.models)
            .unwrap_or_default();

        if discovered_models.is_empty() {
            Input::new()
                .with_prompt("Enter the model to use")
                .default(preferred_model)
                .interact_text()?
        } else {
            let manual_label = "<Enter model manually>";
            let mut options = discovered_models;
            options.push(manual_label.to_string());
            let default_index = options
                .iter()
                .position(|item| item == &preferred_model)
                .or_else(|| {
                    default_model_from_registry(&provider_name)
                        .and_then(|model| options.iter().position(|item| item == &model))
                })
                .unwrap_or(0);
            let selected_index = Select::new()
                .with_prompt("Select the model to use")
                .items(&options)
                .default(default_index)
                .interact()?;
            if options[selected_index] == manual_label {
                Input::new()
                    .with_prompt("Enter the model to use")
                    .default(preferred_model)
                    .interact_text()?
            } else {
                options[selected_index].clone()
            }
        }
    };

    let workspace = if let Some(workspace) = args.workspace {
        workspace
    } else {
        PathBuf::from(
            Input::new()
                .with_prompt("Enter workspace directory")
                .default(config.agents.defaults.workspace.clone())
                .interact_text()?,
        )
    };

    config.agents.defaults.provider = Some(provider_name.clone());
    config.agents.defaults.model = model;
    config.agents.defaults.workspace = workspace.display().to_string();
    set_provider_credentials(&mut config, &provider_name, api_key, api_base);

    runtime.loader().save(&config)?;

    let workspace_path = runtime.effective_workspace(&config);
    let added_templates = ensure_workspace_templates(&workspace_path)?;

    println!(
        "\n{}",
        style("Configuration saved successfully!").green().bold()
    );
    println!("Config location: {}", config_path.display());
    println!("Runtime root: {}", runtime.runtime_dir().display());
    println!("Workspace: {}", workspace_path.display());
    if !added_templates.is_empty() {
        println!("Templates added: {}", added_templates.join(", "));
    }
    println!("\nYou can now run:");
    println!(
        "  {} - Start the gateway",
        style("agent-diva gateway run").cyan()
    );
    println!(
        "  {} - Validate configuration",
        style("agent-diva config doctor").cyan()
    );
    println!(
        "  {} - Send a message",
        style("agent-diva agent --message 'Hello!'").cyan()
    );
    println!(
        "  Skills: {}",
        style("~/.agent-diva/workspace/skills/<skill-name>/SKILL.md").cyan()
    );
    println!("  First chats will guide soul identity initialization.");
    println!("  The agent will ask for its name, style, and collaboration boundaries.");

    Ok(())
}

/// Run the agent gateway
fn build_gateway_runtime_config(
    runtime: &CliRuntime,
    config: Config,
    workspace: PathBuf,
) -> GatewayRuntimeConfig {
    GatewayRuntimeConfig {
        config,
        loader: runtime.loader().clone(),
        workspace,
        cron_store: runtime.cron_store_path(),
        port: DEFAULT_GATEWAY_PORT,
    }
}

async fn run_gateway(runtime: &CliRuntime) -> Result<()> {
    // Remote CLI flows continue to use HTTP APIs and do not cross this boundary.
    let config = runtime.load_config()?;

    if current_provider_name(&config).is_none() {
        anyhow::bail!(
            "No provider found for model: {}",
            config.agents.defaults.model
        );
    }

    let workspace = runtime.effective_workspace(&config);
    let _ = ensure_workspace_templates(&workspace)?;

    println!("{}", style("Starting Agent Diva Gateway...").bold().cyan());
    println!("Model: {}", config.agents.defaults.model);
    println!("Workspace: {}", workspace.display());
    println!(
        "\n{}",
        style("Gateway is running. Press Ctrl+C to stop.").green()
    );
    println!(
        "{}",
        style(format!(
            "API Server running on http://localhost:{}",
            DEFAULT_GATEWAY_PORT
        ))
        .green()
    );

    let result = run_local_gateway(build_gateway_runtime_config(runtime, config, workspace)).await;

    println!("{}", style("Gateway stopped.").green());
    result
}

#[derive(Clone)]
enum TimelineKind {
    User,
    Assistant,
    Tool,
    System,
    Error,
    Thinking,
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

fn chat_id_from_tui_session(session_key: &str) -> String {
    session_key.split(':').nth(2).unwrap_or("tui").to_string()
}

impl TuiApp {
    fn new(session_key: String, model: String) -> Self {
        Self {
            input: String::new(),
            timeline: vec![TimelineItem {
                kind: TimelineKind::System,
                text: "Welcome to Agent Diva TUI. Enter to send, Shift+Enter for newline. /new /clear /stop /quit".to_string(),
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
            AgentEvent::ReasoningDelta { text } => {
                if text.is_empty() {
                    return;
                }
                self.assistant_line = None;
                if let Some(item) = self.timeline.last_mut() {
                    if matches!(item.kind, TimelineKind::Thinking) {
                        item.text.push_str(&text);
                        return;
                    }
                }
                self.add_line(TimelineKind::Thinking, text);
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
            AgentEvent::RunTelemetry(snap) => {
                self.assistant_line = None;
                self.add_line(
                    TimelineKind::System,
                    format!(
                        "[run_telemetry] mainLoop={} preludeLlm={} phases={} convergence={:?} overBudget={:?}",
                        snap.internal_step_count,
                        snap.prelude_llm_calls,
                        snap.phase_count,
                        snap.full_swarm_convergence_rounds,
                        snap.over_suggested_budget
                    ),
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
            _ => {}
        }
    }
}

async fn run_tui(
    runtime: &CliRuntime,
    model: Option<String>,
    session: Option<String>,
) -> Result<()> {
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

    let (runtime_control_tx, runtime_control_rx) = mpsc::unbounded_channel();
    let mut agent = AgentLoop::with_tools(
        bus,
        provider,
        workspace,
        Some(selected_model.clone()),
        Some(config.agents.defaults.max_tool_iterations as usize),
        tool_config,
        Some(runtime_control_rx),
    );

    let current_session = session.unwrap_or_else(|| "cli:tui".to_string());
    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<(String, String)>();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AgentEvent>();

    tokio::spawn(async move {
        while let Some((prompt, session_key)) = request_rx.recv().await {
            let chat_id = chat_id_from_tui_session(&session_key);
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
                Paragraph::new(status_line).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("agent-diva tui"),
                ),
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
                    TimelineKind::Thinking => ("thinking", Color::DarkGray),
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
                        } else if content == "/stop" {
                            let chat_id = chat_id_from_tui_session(&app.session_key);
                            let session_key = format!("cli:{}", chat_id);
                            let _ = runtime_control_tx
                                .send(RuntimeControlCommand::StopSession { session_key });
                            app.add_line(TimelineKind::System, "stop requested");
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

fn print_swarm_cortex_doctor_human(block: &SwarmCortexDoctorV1) {
    println!();
    println!(
        "{}",
        style("Swarm / cortex diagnostics (developer — not user transcript)").bold()
    );
    println!(
        "  Capabilities: source={} count={}",
        block.capabilities.source, block.capabilities.count
    );
    if !block.capabilities.ids_preview.is_empty() {
        println!(
            "  Ids (preview): {}",
            block.capabilities.ids_preview.join(", ")
        );
    }
    if !block.capabilities.note.is_empty() {
        println!("  Note: {}", block.capabilities.note);
    }
    println!(
        "  Subagent tools (Layer A catalog, v{}): {} entries",
        block.subagent_tools.catalog_version,
        block.subagent_tools.entries.len()
    );
    for e in &block.subagent_tools.entries {
        let gate = if e.availability_note.is_empty() {
            String::new()
        } else {
            format!(" [{}]", e.availability_note)
        };
        println!(
            "    - {} → {} (risk={}){}",
            e.capability_id, e.tool_name, e.risk_tier, gate
        );
    }
    println!("  Cortex state: {}", block.cortex.state);
    println!("  Gateway bind (config): {}", block.cortex.gateway_bind);
    println!("  Cortex note: {}", block.cortex.note);
}

/// Show system status
async fn run_status(runtime: &CliRuntime, json: bool, swarm: bool) -> Result<()> {
    let report = collect_status_report(runtime, swarm).await?;

    if json {
        return print_json(&report);
    }

    let config = runtime.load_config()?;
    let doctor = doctor_report(runtime, &config, swarm);

    println!("{}", style("Agent Diva Status").bold().cyan());
    println!("Version: 0.4.0 (Rust)\n");
    println!("{}", style("Paths:").bold());
    println!("  Config: {}", report.config.config_path);
    println!("  Runtime root: {}", report.config.runtime_dir);
    println!("  Workspace: {}", report.config.workspace);
    println!("  Cron store: {}", report.config.cron_store);
    println!();

    println!("{}", style("Agent:").bold());
    println!("  Default model: {}", report.default_model);
    println!(
        "  Default provider: {}",
        report
            .default_provider
            .clone()
            .unwrap_or_else(|| "unresolved".to_string())
    );
    println!(
        "  Logging: {} / {}",
        report.logging.level, report.logging.format
    );
    println!();

    println!("{}", style("Providers:").bold());
    for provider in &report.providers {
        let marker = if provider.ready {
            style("configured").green()
        } else {
            style("not configured").red()
        };
        let active = if provider.current { " [default]" } else { "" };
        println!("  {}: {}{}", provider.name, marker, active);
    }
    println!();

    println!("{}", style("Channels:").bold());
    for channel in &report.channels {
        let status = if !channel.enabled {
            style("disabled").dim()
        } else if channel.ready {
            style("enabled (ready)").green()
        } else {
            style("enabled (missing credentials)").yellow()
        };
        println!("  {}: {}", channel.name, status);
    }
    println!();

    println!("{}", style("Health:").bold());
    println!(
        "  Config valid: {}",
        if doctor.valid { "yes" } else { "no" }
    );
    println!(
        "  Runtime ready: {}",
        if doctor.ready { "yes" } else { "no" }
    );
    println!("  Cron jobs: {}", report.cron_jobs);
    println!(
        "  MCP servers: {} configured / {} disabled",
        report.mcp_servers.configured, report.mcp_servers.disabled
    );
    if !doctor.warnings.is_empty() {
        println!("\n{}", style("Warnings:").bold().yellow());
        for warning in doctor.warnings {
            println!("  - {}", warning);
        }
    }

    if swarm {
        if let Some(ref block) = doctor.swarm_cortex {
            print_swarm_cortex_doctor_human(block);
        }
    }

    Ok(())
}

fn is_structured_output(command: &Commands) -> bool {
    match command {
        Commands::Status(args) => args.json,
        Commands::Channels {
            command: ChannelCommands::Status(args),
        } => args.json,
        Commands::Provider { command } => match command {
            ProviderCommands::List(args) | ProviderCommands::Status(args) => args.json,
            ProviderCommands::Set { json, .. }
            | ProviderCommands::Models { json, .. }
            | ProviderCommands::Login { json, .. } => *json,
        },
        Commands::Config { command } => match command {
            ConfigCommands::Path(args)
            | ConfigCommands::Validate(args)
            | ConfigCommands::Doctor(args) => args.json,
            ConfigCommands::Show { format } => matches!(format, ConfigOutputFormat::Json),
            _ => false,
        },
        _ => false,
    }
}

/// Show channel status
async fn run_channel_status(runtime: &CliRuntime, json: bool) -> Result<()> {
    let config = runtime.load_config()?;
    let channels = channel_statuses(&config);

    if json {
        return print_json(&channels);
    }

    println!("{}", style("Channel Status").bold().cyan());
    println!();

    for channel in channels {
        if !channel.enabled {
            println!("  {}: {}", channel.name, style("disabled").dim());
            continue;
        }

        if channel.ready {
            println!("  {}: {}", channel.name, style("enabled (ready)").green());
        } else {
            println!(
                "  {}: {} [{}]",
                channel.name,
                style("enabled (missing credentials)").yellow(),
                channel.missing_fields.join(", ")
            );
        }
    }

    Ok(())
}

async fn run_config_path(runtime: &CliRuntime, json: bool) -> Result<()> {
    let config = runtime.load_config().unwrap_or_default();
    let report = runtime.path_report(&config);

    if json {
        return print_json(&report);
    }

    println!("{}", style("Config Paths").bold().cyan());
    println!("  Config: {}", report.config_path);
    println!("  Config dir: {}", report.config_dir);
    println!("  Runtime root: {}", report.runtime_dir);
    println!("  Workspace: {}", report.workspace);
    println!("  Cron store: {}", report.cron_store);
    println!("  Bridge dir: {}", report.bridge_dir);
    println!("  WhatsApp auth: {}", report.whatsapp_auth_dir);
    println!("  WhatsApp media: {}", report.whatsapp_media_dir);
    Ok(())
}

async fn run_config_refresh(runtime: &CliRuntime) -> Result<()> {
    let config = runtime.load_config()?;
    runtime.loader().save(&config)?;
    let workspace = runtime.effective_workspace(&config);
    let added = ensure_workspace_templates(&workspace)?;

    println!("{}", style("Configuration refreshed.").green().bold());
    println!("Config: {}", runtime.config_path().display());
    println!("Workspace: {}", workspace.display());
    if added.is_empty() {
        println!("Templates: no new files added");
    } else {
        println!("Templates added: {}", added.join(", "));
    }
    Ok(())
}

async fn run_config_validate(runtime: &CliRuntime, json: bool) -> Result<()> {
    let config = runtime.load_config()?;
    let result = validate_config(&config);
    let payload = match result {
        Ok(_) => serde_json::json!({
            "valid": true,
            "errors": [],
        }),
        Err(err) => serde_json::json!({
            "valid": false,
            "errors": [err.to_string()],
        }),
    };

    if json {
        print_json(&payload)?;
    } else if payload["valid"] == true {
        println!("{}", style("Config is valid.").green().bold());
    } else {
        println!("{}", style("Config is invalid.").red().bold());
        for error in payload["errors"].as_array().into_iter().flatten() {
            println!("  - {}", error.as_str().unwrap_or_default());
        }
    }

    if payload["valid"] != true {
        std::process::exit(1);
    }
    Ok(())
}

async fn run_config_doctor(runtime: &CliRuntime, json: bool, swarm: bool) -> Result<()> {
    let config = runtime.load_config()?;
    let report = doctor_report(runtime, &config, swarm);

    if json {
        print_json(&report)?;
    } else {
        println!("{}", style("Config Doctor").bold().cyan());
        println!("  Valid: {}", if report.valid { "yes" } else { "no" });
        println!("  Ready: {}", if report.ready { "yes" } else { "no" });
        if let Some(provider) = &report.provider {
            println!("  Provider: {}", provider);
        }
        if !report.errors.is_empty() {
            println!("\n{}", style("Errors:").bold().red());
            for error in &report.errors {
                println!("  - {}", error);
            }
        }
        if !report.warnings.is_empty() {
            println!("\n{}", style("Warnings:").bold().yellow());
            for warning in &report.warnings {
                println!("  - {}", warning);
            }
        }
        if let Some(ref block) = report.swarm_cortex {
            print_swarm_cortex_doctor_human(block);
        }
    }

    if !report.valid {
        std::process::exit(1);
    }
    if !report.ready {
        std::process::exit(2);
    }
    Ok(())
}

async fn run_config_show(runtime: &CliRuntime, format: ConfigOutputFormat) -> Result<()> {
    let config = runtime.load_config()?;
    let value = redacted_config_value(&config)?;
    match format {
        ConfigOutputFormat::Json => println!("{}", serde_json::to_string(&value)?),
        ConfigOutputFormat::Pretty => println!("{}", serde_json::to_string_pretty(&value)?),
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

fn setup_bridge(runtime: &CliRuntime) -> Result<PathBuf> {
    let bridge_dir = runtime.bridge_dir();
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

async fn run_channel_login(runtime: &CliRuntime, channel: String) -> Result<()> {
    if channel.to_lowercase() != "whatsapp" {
        println!("Channel '{}' login flow is not implemented yet.", channel);
        return Ok(());
    }

    let config = runtime.load_config()?;
    let bridge_dir = setup_bridge(runtime)?;
    let auth_dir = runtime.whatsapp_auth_dir();
    let media_dir = runtime.whatsapp_media_dir();
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
    runtime: &CliRuntime,
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
    let data_dir = runtime.config_dir().join("data");
    std::fs::create_dir_all(&data_dir)?;
    let store_path = runtime.cron_store_path();
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
async fn run_cron_list(runtime: &CliRuntime, include_all: bool) -> Result<()> {
    let store_path = runtime.cron_store_path();

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
async fn run_cron_remove(runtime: &CliRuntime, job_id: String) -> Result<()> {
    let store_path = runtime.cron_store_path();

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
async fn run_cron_enable(runtime: &CliRuntime, job_id: String, enabled: bool) -> Result<()> {
    let store_path = runtime.cron_store_path();

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

/// Manually run a cron job
async fn run_cron_run(runtime: &CliRuntime, job_id: String, force: bool) -> Result<()> {
    let store_path = runtime.cron_store_path();

    if !store_path.exists() {
        println!("No scheduled jobs.");
        return Ok(());
    }

    let service = Arc::new(CronService::new(store_path, None));
    service.start().await;

    let result = service.run_job(&job_id, force).await;

    service.stop().await;

    if result {
        println!("{} Ran job {}", style("[OK]").green().bold(), job_id);
    } else {
        println!(
            "{} Job {} not found or disabled",
            style("[ERR]").red(),
            job_id
        );
    }

    Ok(())
}

async fn run_tui_remote(api_url: Option<String>, session: Option<String>) -> Result<()> {
    let current_session = session.unwrap_or_else(|| "cli:tui:remote".to_string());
    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<(String, String)>();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AgentEvent>();

    let client = ApiClient::new(api_url);
    let client = Arc::new(client);
    let client_for_sender = client.clone();

    tokio::spawn(async move {
        while let Some((prompt, session_key)) = request_rx.recv().await {
            let client = client_for_sender.clone();
            let event_tx = event_tx.clone();
            tokio::spawn(async move {
                let chat_id = chat_id_from_tui_session(&session_key);
                if let Err(e) = client
                    .chat_with_target(prompt, Some("cli"), Some(&chat_id), event_tx)
                    .await
                {
                    error!("Remote chat error: {}", e);
                }
            });
        }
    });

    let mut stdout = io::stdout();
    enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new(current_session, "remote".to_string());

    // TUI Loop (duplicated from run_tui for now)
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
                Paragraph::new(status_line).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("agent-diva tui (remote)"),
                ),
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
                    TimelineKind::Thinking => ("thinking", Color::Magenta),
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
                        } else if content == "/stop" {
                            let chat_id = chat_id_from_tui_session(&app.session_key);
                            match client.stop(Some("cli"), Some(&chat_id)).await {
                                Ok(_) => app.add_line(TimelineKind::System, "stop requested"),
                                Err(e) => {
                                    app.add_line(TimelineKind::Error, format!("stop failed: {}", e))
                                }
                            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tui_disables_terminal_logging() {
        assert!(!command_writes_logs_to_terminal(&Commands::Tui {
            model: None,
            session: None,
        }));
    }

    #[test]
    fn chat_keeps_terminal_logging() {
        assert!(command_writes_logs_to_terminal(&Commands::Chat {
            model: None,
            session: None,
            markdown: false,
            no_markdown: false,
            logs: false,
            no_logs: false,
        }));
    }

    #[test]
    fn agent_disables_terminal_logging_and_startup_branding() {
        let command = Commands::Agent {
            message: Some("hello".to_string()),
            model: None,
            session: None,
            markdown: false,
            no_markdown: false,
            logs: false,
            no_logs: false,
        };

        assert!(!command_writes_logs_to_terminal(&command));
        assert!(!command_shows_startup_branding(&command));
    }

    #[test]
    fn provider_model_resolution_prefers_explicit_model() {
        let config = Config::default();
        let resolved = agent_diva_cli::cli_runtime::resolve_provider_model_with_default(
            &config,
            "openai",
            Some("openai/gpt-4o"),
            Some("openai/gpt-5".to_string()),
        )
        .unwrap();

        assert_eq!(resolved, "openai/gpt-5");
    }

    #[test]
    fn provider_model_resolution_falls_back_to_current_model_when_same_provider() {
        let mut config = Config::default();
        config.agents.defaults.model = "openai/gpt-5".to_string();

        let resolved = agent_diva_cli::cli_runtime::resolve_provider_model_with_default(
            &config, "openai", None, None,
        )
        .unwrap();

        assert_eq!(resolved, "openai/gpt-5");
    }

    #[test]
    fn provider_model_resolution_errors_when_no_default_and_provider_changes() {
        let mut config = Config::default();
        config.agents.defaults.model = "anthropic/claude-sonnet-4-5".to_string();

        let err = agent_diva_cli::cli_runtime::resolve_provider_model_with_default(
            &config, "openai", None, None,
        )
        .unwrap_err();

        assert!(err.to_string().contains("pass --model explicitly"));
    }

    #[test]
    fn explicit_provider_is_used_for_unknown_model() {
        let mut config = Config::default();
        config.agents.defaults.provider = Some("minimax".to_string());
        config.agents.defaults.model = "MiniMax-M2.2".to_string();

        let provider = agent_diva_cli::cli_runtime::current_provider_name(&config);

        assert_eq!(provider.as_deref(), Some("minimax"));
    }
}

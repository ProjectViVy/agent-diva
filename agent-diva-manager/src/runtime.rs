mod bootstrap;
mod shutdown;
mod task_runtime;

use crate::state::ManagerCommand;
use agent_diva_agent::{
    agent_loop::SoulGovernanceSettings, context::SoulContextSettings,
    runtime_control::RuntimeControlCommand, tool_config::network::NetworkToolConfig,
    tool_config::network::WebFetchRuntimeConfig, tool_config::network::WebRuntimeConfig,
    tool_config::network::WebSearchRuntimeConfig, AgentLoop, ToolConfig,
};
use agent_diva_channels::ChannelManager;
use agent_diva_core::auth::ProviderAuthService;
use agent_diva_core::bus::{InboundMessage, MessageBus};
use agent_diva_core::config::{Config, ConfigLoader};
use agent_diva_core::cron::service::JobCallback;
use agent_diva_core::cron::CronService;
use agent_diva_providers::{
    backends::openai_codex::OpenAiCodexProvider, DynamicProvider, LLMProvider, LiteLLMClient,
    ProviderAccess, ProviderCatalogService, ProviderRegistry, RuntimeBackend,
};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tracing::error;

pub const DEFAULT_GATEWAY_PORT: u16 = 3000;

#[derive(Clone)]
pub struct GatewayRuntimeConfig {
    pub config: Config,
    pub loader: ConfigLoader,
    pub workspace: PathBuf,
    pub cron_store: PathBuf,
    pub port: u16,
}

struct GatewayBootstrap {
    config: Config,
    loader: ConfigLoader,
    port: u16,
    bus: MessageBus,
    cron_service: Arc<CronService>,
    dynamic_provider: Arc<DynamicProvider>,
    runtime_control_tx: mpsc::UnboundedSender<RuntimeControlCommand>,
    provider_api_key: Option<String>,
    provider_api_base: Option<String>,
    agent: AgentLoop,
}

struct ChannelBootstrap {
    channel_manager: Arc<ChannelManager>,
    inbound_bridge_handle: JoinHandle<()>,
}

struct GatewayTasks {
    bus: MessageBus,
    cron_service: Arc<CronService>,
    channel_manager: Arc<ChannelManager>,
    server_shutdown_tx: broadcast::Sender<()>,
    inbound_bridge_handle: JoinHandle<()>,
    outbound_dispatch_handle: JoinHandle<()>,
    channel_handle: JoinHandle<()>,
    agent_handle: JoinHandle<()>,
    manager_handle: JoinHandle<Result<()>>,
    server_handle: JoinHandle<()>,
    _api_tx_keepalive: mpsc::Sender<ManagerCommand>,
}

fn provider_registry() -> ProviderRegistry {
    ProviderRegistry::new()
}

fn infer_provider_name_from_model(model: &str) -> Option<String> {
    let registry = provider_registry();
    model
        .split('/')
        .next()
        .and_then(|prefix| registry.find_by_name(prefix))
        .or_else(|| registry.find_by_model(model))
        .map(|spec| spec.name.clone())
}

fn current_provider_name(config: &Config) -> Option<String> {
    let preferred_provider = config
        .agents
        .defaults
        .provider
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    preferred_provider.or_else(|| infer_provider_name_from_model(&config.agents.defaults.model))
}

fn resolve_provider_name_for_model(
    config: &Config,
    model: &str,
    preferred_provider: Option<&str>,
) -> Option<String> {
    let inferred_provider = infer_provider_name_from_model(model);
    if let Some(provider_name) = preferred_provider
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let registry = provider_registry();
        if registry.find_by_name(provider_name).is_some() {
            return Some(provider_name.to_string());
        }
        if inferred_provider.as_deref() == Some(provider_name) {
            return inferred_provider;
        }
    }

    inferred_provider.or_else(|| {
        (model == config.agents.defaults.model)
            .then(|| current_provider_name(config))
            .flatten()
    })
}

fn build_provider(
    config: &Config,
    config_dir: &std::path::Path,
    model: &str,
) -> Result<Arc<dyn LLMProvider>> {
    let catalog = ProviderCatalogService::new();
    let provider_name = resolve_provider_name_for_model(
        config,
        model,
        (model == config.agents.defaults.model)
            .then_some(config.agents.defaults.provider.as_deref())
            .flatten(),
    )
    .ok_or_else(|| anyhow::anyhow!("No provider found for model: {}", model))?;
    let access = catalog
        .get_provider_access(config, &provider_name)
        .unwrap_or_else(|| ProviderAccess::from_config(None));
    let extra_headers = (!access.extra_headers.is_empty()).then(|| {
        access
            .extra_headers
            .into_iter()
            .collect::<std::collections::HashMap<String, String>>()
    });

    let provider_spec = provider_registry()
        .find_by_name(&provider_name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No provider spec found for provider: {}", provider_name))?;
    let provider: Arc<dyn LLMProvider> = match provider_spec.runtime_backend {
        RuntimeBackend::OpenaiCodex => Arc::new(OpenAiCodexProvider::new(
            ProviderAuthService::new(config_dir),
            model.to_string(),
            None,
        )),
        RuntimeBackend::OpenaiCompatible => Arc::new(LiteLLMClient::new(
            access.api_key,
            access.api_base,
            model.to_string(),
            extra_headers,
            Some(provider_name),
            config.agents.defaults.reasoning_effort.clone(),
        )),
    };
    Ok(provider)
}

fn build_network_tool_config(config: &Config) -> NetworkToolConfig {
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

pub async fn run_local_gateway(runtime: GatewayRuntimeConfig) -> Result<()> {
    let bootstrap = bootstrap::bootstrap_runtime(runtime).await?;
    let channel_bootstrap =
        bootstrap::bootstrap_channel_runtime(&bootstrap.config, bootstrap.bus.clone()).await;
    let mut tasks = task_runtime::start_runtime_tasks(bootstrap, channel_bootstrap).await;
    let manager_handle_completed = shutdown::wait_for_shutdown(&mut tasks).await;
    shutdown::shutdown_runtime(tasks, manager_handle_completed).await;
    Ok(())
}

async fn start_cron_service(cron_store: PathBuf, bus: MessageBus) -> Arc<CronService> {
    let cron_service = Arc::new(CronService::new(cron_store, Some(build_cron_callback(bus))));
    cron_service.start().await;
    cron_service
}

fn build_cron_callback(bus: MessageBus) -> JobCallback {
    Arc::new(
        move |job: agent_diva_core::cron::CronJob,
              cancel_token|
              -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send>> {
            let bus = bus.clone();
            Box::pin(async move {
                if cancel_token.is_cancelled() {
                    return Some("Error: cancelled".to_string());
                }
                let deliver = job.payload.deliver;
                if !deliver {
                    return Some("skipped (deliver=false)".to_string());
                }

                let target_channel = job
                    .payload
                    .channel
                    .clone()
                    .unwrap_or_else(|| "cli".to_string());
                let target_chat_id = job
                    .payload
                    .to
                    .clone()
                    .unwrap_or_else(|| "direct".to_string());
                let (conversation_channel, conversation_chat_id) = if target_channel == "gui" {
                    let chat_id = if target_chat_id.starts_with("cron:") {
                        target_chat_id
                    } else {
                        format!("cron:{}", target_chat_id)
                    };
                    ("api".to_string(), chat_id)
                } else {
                    (target_channel.clone(), target_chat_id)
                };

                let inbound = InboundMessage::new(
                    conversation_channel,
                    "cron",
                    conversation_chat_id,
                    job.payload.message,
                )
                .with_metadata("cron_job_id", job.id.clone())
                .with_metadata("cron_trigger", "scheduled")
                .with_metadata("cron_delivery_channel", target_channel);

                if let Err(e) = bus.publish_inbound(inbound) {
                    error!("Failed to publish cron inbound job {}: {}", job.id, e);
                    return Some(format!(
                        "failed to publish cron inbound job {}: {}",
                        job.id, e
                    ));
                }

                Some("triggered agent turn".to_string())
            })
        },
    )
}

fn build_agent_loop(
    config: &Config,
    bus: MessageBus,
    dynamic_provider: Arc<DynamicProvider>,
    workspace: PathBuf,
    runtime_control_rx: mpsc::UnboundedReceiver<RuntimeControlCommand>,
    cron_service: Arc<CronService>,
) -> AgentLoop {
    let agent_provider: Arc<dyn LLMProvider> = dynamic_provider;
    let tool_config = ToolConfig {
        network: build_network_tool_config(config),
        exec_timeout: config.tools.exec.timeout,
        restrict_to_workspace: config.tools.restrict_to_workspace,
        mcp_servers: config.tools.active_mcp_servers(),
        cron_service: Some(cron_service),
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

    AgentLoop::with_tools(
        bus,
        agent_provider,
        workspace,
        Some(config.agents.defaults.model.clone()),
        Some(config.agents.defaults.max_tool_iterations as usize),
        tool_config,
        Some(runtime_control_rx),
    )
}

fn resolve_provider_credentials(config: &Config) -> Result<(Option<String>, Option<String>)> {
    let provider_name = current_provider_name(config)
        .ok_or_else(|| anyhow::anyhow!("No provider found for model"))?;
    let catalog = ProviderCatalogService::new();
    let access = catalog
        .get_provider_access(config, &provider_name)
        .unwrap_or_else(|| ProviderAccess::from_config(None));
    let resolved_api_base = access.api_base.clone().or_else(|| {
        catalog
            .get_provider_view(config, &provider_name)
            .and_then(|view| view.api_base)
    });
    Ok((access.api_key, resolved_api_base))
}

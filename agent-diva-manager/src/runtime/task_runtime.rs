use super::*;
use crate::{run_server, AppState, Manager};

pub(super) async fn start_runtime_tasks(
    bootstrap: GatewayBootstrap,
    channel_bootstrap: ChannelBootstrap,
) -> GatewayTasks {
    let GatewayBootstrap {
        config,
        loader,
        port,
        bus,
        cron_service,
        dynamic_provider,
        runtime_control_tx,
        provider_api_key,
        provider_api_base,
        agent,
    } = bootstrap;
    let ChannelBootstrap {
        channel_manager,
        inbound_bridge_handle,
    } = channel_bootstrap;

    subscribe_configured_outbound_channels(&bus, &channel_manager, &config).await;

    let (api_tx, api_rx) = mpsc::channel(100);
    let manager = Manager::new(
        api_rx,
        bus.clone(),
        dynamic_provider,
        loader,
        config.agents.defaults.provider.clone(),
        config.agents.defaults.model.clone(),
        provider_api_key,
        provider_api_base,
        Some(channel_manager.clone()),
        Some(runtime_control_tx),
        Arc::clone(&cron_service),
    );
    let api_tx_keepalive = api_tx.clone();

    let outbound_dispatch_handle = spawn_outbound_dispatch(bus.clone());
    let channel_handle = spawn_channel_runtime(channel_manager.clone());
    let agent_handle = spawn_agent_runtime(agent);
    let manager_handle = spawn_manager_runtime(manager);
    let (server_shutdown_tx, server_handle) = spawn_server_runtime(
        port,
        AppState {
            api_tx,
            bus: bus.clone(),
        },
    );

    GatewayTasks {
        bus,
        cron_service,
        channel_manager,
        server_shutdown_tx,
        inbound_bridge_handle,
        outbound_dispatch_handle,
        channel_handle,
        agent_handle,
        manager_handle,
        server_handle,
        _api_tx_keepalive: api_tx_keepalive,
    }
}

async fn subscribe_configured_outbound_channels(
    bus: &MessageBus,
    channel_manager: &Arc<ChannelManager>,
    config: &Config,
) {
    for channel_name in configured_channels(config) {
        let manager = channel_manager.clone();
        let channel_key = channel_name.clone();
        bus.subscribe_outbound(channel_name, move |msg| {
            let manager = manager.clone();
            let channel_key = channel_key.clone();
            async move {
                if let Err(e) = manager.send(&channel_key, msg).await {
                    tracing::error!("Failed to send outbound message to {}: {}", channel_key, e);
                }
            }
        })
        .await;
    }
}

fn configured_channels(config: &Config) -> Vec<String> {
    [
        ("telegram", config.channels.telegram.enabled),
        ("discord", config.channels.discord.enabled),
        ("whatsapp", config.channels.whatsapp.enabled),
        ("feishu", config.channels.feishu.enabled),
        ("dingtalk", config.channels.dingtalk.enabled),
        ("email", config.channels.email.enabled),
        ("slack", config.channels.slack.enabled),
        ("qq", config.channels.qq.enabled),
        ("matrix", config.channels.matrix.enabled),
    ]
    .into_iter()
    .filter_map(|(channel_name, enabled)| enabled.then_some(channel_name.to_string()))
    .collect()
}

fn spawn_outbound_dispatch(bus: MessageBus) -> JoinHandle<()> {
    tokio::spawn(async move {
        bus.dispatch_outbound_loop().await;
    })
}

fn spawn_channel_runtime(channel_manager: Arc<ChannelManager>) -> JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = channel_manager.start_all().await {
            tracing::error!("Channel manager error: {}", e);
        }
    })
}

fn spawn_agent_runtime(agent: AgentLoop) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut agent = agent;
        if let Err(e) = agent.run().await {
            tracing::error!("Agent loop error: {}", e);
        }
    })
}

fn spawn_manager_runtime(manager: Manager) -> JoinHandle<Result<()>> {
    tokio::spawn(async move {
        if let Err(e) = manager.run().await {
            if e.to_string().contains("RESTART_REQUIRED") {
                return Err(e);
            }
            tracing::error!("Manager loop error: {}", e);
        }
        Ok(())
    })
}

fn spawn_server_runtime(port: u16, state: AppState) -> (broadcast::Sender<()>, JoinHandle<()>) {
    let (server_shutdown_tx, server_shutdown_rx) = broadcast::channel(1);
    let server_handle = tokio::spawn(async move {
        if let Err(e) = run_server(state, port, server_shutdown_rx).await {
            tracing::error!("API Server error: {}", e);
        }
    });
    (server_shutdown_tx, server_handle)
}

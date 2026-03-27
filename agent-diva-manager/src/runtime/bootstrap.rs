use super::*;

pub(super) async fn bootstrap_runtime(runtime: GatewayRuntimeConfig) -> Result<GatewayBootstrap> {
    let GatewayRuntimeConfig {
        config,
        loader,
        workspace,
        cron_store,
        port,
    } = runtime;

    let bus = MessageBus::new();
    let cron_service = start_cron_service(cron_store, bus.clone()).await;
    let dynamic_provider = Arc::new(DynamicProvider::new(
        build_provider(&config, loader.config_dir(), &config.agents.defaults.model).await?,
    ));
    let (runtime_control_tx, runtime_control_rx) = mpsc::unbounded_channel();
    let agent = build_agent_loop(
        &config,
        bus.clone(),
        dynamic_provider.clone(),
        workspace.clone(),
        runtime_control_rx,
        Arc::clone(&cron_service),
    );
    let (provider_api_key, provider_api_base) = resolve_provider_credentials(&config)?;

    Ok(GatewayBootstrap {
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
    })
}

pub(super) async fn bootstrap_channel_runtime(
    config: &Config,
    bus: MessageBus,
) -> ChannelBootstrap {
    let mut channel_manager = ChannelManager::new(config.clone());
    let (inbound_tx, mut inbound_rx) = mpsc::channel::<InboundMessage>(1024);
    channel_manager.set_inbound_sender(inbound_tx);
    let inbound_bridge_handle = tokio::spawn(async move {
        while let Some(msg) = inbound_rx.recv().await {
            if let Err(e) = bus.publish_inbound(msg) {
                tracing::error!("Failed to publish inbound message to bus: {}", e);
            }
        }
    });

    if let Err(e) = channel_manager.initialize().await {
        tracing::error!("Failed to initialize channels: {}", e);
        tracing::warn!("Continuing gateway startup without fully initialized channels");
    }

    ChannelBootstrap {
        channel_manager: Arc::new(channel_manager),
        inbound_bridge_handle,
    }
}

use super::*;

pub(super) async fn bootstrap_runtime(runtime: GatewayRuntimeConfig) -> Result<GatewayBootstrap> {
    let GatewayRuntimeConfig {
        config,
        loader,
        workspace,
        cron_store,
        port,
        debug_run,
    } = runtime;

    let bus = MessageBus::new();
    let debug_logger = match debug_run {
        Some(run) => Some(DebugEventLogger::new(run)?),
        None => None,
    };
    let cron_service = start_cron_service(cron_store, bus.clone(), debug_logger.clone()).await;
    let dynamic_provider = Arc::new(DynamicProvider::new(Arc::new(build_provider(
        &config,
        &config.agents.defaults.model,
    )?)));

    // Initialize shared FileManager for attachment handling
    let storage_path = default_data_dir_or_fallback();
    let file_config = FileConfig::with_path(&storage_path);
    let file_manager = Arc::new(FileManager::new(file_config).await?);

    let (runtime_control_tx, runtime_control_rx) = mpsc::unbounded_channel();
    let agent = build_agent_loop(
        &config,
        bus.clone(),
        dynamic_provider.clone(),
        workspace.clone(),
        runtime_control_rx,
        Arc::clone(&cron_service),
        Arc::clone(&file_manager),
        debug_logger.clone(),
    )
    .await?;
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
        file_manager,
        debug_logger,
    })
}

pub(super) async fn bootstrap_channel_runtime(
    config: &Config,
    bus: MessageBus,
    debug_logger: Option<Arc<DebugEventLogger>>,
) -> ChannelBootstrap {
    let mut channel_manager = ChannelManager::new(config.clone());
    let (inbound_tx, mut inbound_rx) = mpsc::channel::<InboundMessage>(1024);
    channel_manager.set_inbound_sender(inbound_tx);
    let bridge_debug_logger = debug_logger.clone();
    let inbound_bridge_handle = tokio::spawn(async move {
        while let Some(mut msg) = inbound_rx.recv().await {
            let trace_id = msg
                .metadata
                .get("trace_id")
                .and_then(|value| value.as_str())
                .map(TraceId::from)
                .unwrap_or_else(TraceId::new);
            msg.metadata.insert(
                "trace_id".to_string(),
                serde_json::Value::String(trace_id.as_str().to_string()),
            );
            if let Some(logger) = &bridge_debug_logger {
                let session_id = msg.session_key();
                let _ = logger.write_event(DebugEvent::new(
                    Some(trace_id.as_str().to_string()),
                    Some(session_id.clone()),
                    "gateway",
                    "channel_inbound",
                    serde_json::json!({
                        "channel": msg.channel,
                        "chat_id": msg.chat_id,
                        "sender_id": msg.sender_id,
                    }),
                ));
                let _ = logger.write_raw(DebugEvent::new(
                    Some(trace_id.as_str().to_string()),
                    Some(session_id),
                    "gateway",
                    "channel_inbound_raw",
                    serde_json::to_value(&msg).unwrap_or_else(
                        |error| serde_json::json!({"serialization_error": error.to_string()}),
                    ),
                ));
            }
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
        debug_logger,
    }
}

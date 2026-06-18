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
    let cron_service = start_cron_service(cron_store, bus.clone(), workspace.clone()).await;
    ensure_notebook_monthly_cron_job(&cron_service).await?;
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
        workspace,
        runtime_control_tx,
        provider_api_key,
        provider_api_base,
        agent,
        file_manager,
    })
}

async fn ensure_notebook_monthly_cron_job(cron_service: &CronService) -> Result<()> {
    let schedule = agent_diva_core::cron::CronSchedule::cron("5 0 * * *".to_string(), None);
    let payload = agent_diva_core::cron::CronPayload {
        kind: NOTEBOOK_MONTHLY_CRON_KIND.to_string(),
        message: String::new(),
        deliver: false,
        channel: None,
        to: None,
    };
    if let Some(existing) = cron_service
        .list_job_views(true)
        .await
        .into_iter()
        .find(|job| job.job.payload.kind == NOTEBOOK_MONTHLY_CRON_KIND)
    {
        let needs_update = existing.job.name != "Notebook Monthly Productionize"
            || existing.job.payload.kind != NOTEBOOK_MONTHLY_CRON_KIND
            || existing.job.payload.deliver
            || existing.job.payload.channel.is_some()
            || existing.job.payload.to.is_some()
            || !matches!(
                existing.job.schedule,
                agent_diva_core::cron::CronSchedule::Cron { ref expr, .. } if expr == "5 0 * * *"
            );
        if needs_update {
            cron_service
                .update_job(
                    &existing.job.id,
                    agent_diva_core::cron::UpdateCronJobRequest {
                        name: "Notebook Monthly Productionize".to_string(),
                        schedule,
                        payload,
                        delete_after_run: false,
                        enabled: true,
                    },
                )
                .await
                .map_err(anyhow::Error::msg)?;
        }
    } else {
        cron_service
            .create_job(agent_diva_core::cron::CreateCronJobRequest {
                name: "Notebook Monthly Productionize".to_string(),
                schedule,
                payload,
                delete_after_run: false,
                enabled: true,
            })
            .await
            .map_err(anyhow::Error::msg)?;
    }
    Ok(())
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

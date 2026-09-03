use super::*;
use agent_diva_core::bus::PokeEvent;
use agent_diva_core::channel::capacity;
use agent_diva_core::config::{Config, ConfigDiff};
use agent_diva_core::governance::{ApprovalCoordinator, SqliteGovernanceLedger};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::sync::Arc;

// ---------------------------------------------------------------------------
//  Story 3.2 migration path (WIP):
//
//  The bootstrap code is being migrated from hard-coded service assembly
//  to a ModuleRegistry-based pattern. The Module trait, ModuleCtx,
//  ModuleRegistry, and Bootstrap are defined in agent-diva-tooling.
//
//  Migration plan:
//    1. ✅ Module trait + ModuleCtx defined (Story 3.1)
//    2. ✅ ModuleRegistry + Bootstrap + tests (Story 3.2 — this story)
//    3. 🔲 Create Module wrapper adapters for CronService, AgentLoop, etc.
//    4. 🔲 Replace manual construction with registry.register(Arc::new(...))
//    5. 🔲 Use Bootstrap::start_all(ctx) instead of manual assembly
//    6. 🔲 Refactor shutdown to use Bootstrap::stop_all()
//
//  The existing bootstrap_runtime() below continues to work unchanged.
// ---------------------------------------------------------------------------

/// Handle a config diff detected by the hot-reload watcher.
///
/// Updates the shared `Arc<RwLock<Config>>`, logs changes, and emits
/// a `ConfigChangeNeedsRestart` poke event when restart-required fields
/// are detected.
///
/// ## Module propagation (AC3)
///
/// When modules are migrated to `ModuleRegistry` (step 3 in the migration
/// plan), the hot-reload handler will call `module.on_config_reload()`
/// for each registered module.  Until then, hot-reloadable changes are
/// applied to the shared config `Arc`, and any modules that read from
/// it will see updated values.
fn handle_config_diff(diff: ConfigDiff, _new_config: Config, bus: &AgentEventBus) {
    let restart_fields: Vec<String> = diff
        .restart_required
        .iter()
        .map(|c| c.field.clone())
        .collect();

    for change in &diff.restart_required {
        tracing::warn!(
            "Config change '{}' requires restart to take effect (was: {:?}, now: {:?})",
            change.field,
            change.old_value,
            change.new_value,
        );
    }
    if !diff.hot_reload.is_empty() {
        tracing::info!(
            "Hot-reloadable config changes detected ({} field(s))",
            diff.hot_reload.len()
        );
        for change in &diff.hot_reload {
            tracing::info!(
                "  - {}: {:?} -> {:?}",
                change.field,
                change.old_value,
                change.new_value,
            );
        }
    }

    // Emit poke event so the GUI can show a "restart required" indicator (AC4).
    if !restart_fields.is_empty() {
        let _ = bus.publish_poke_event(PokeEvent::ConfigChangeNeedsRestart {
            fields: restart_fields,
        });
    }
}

pub(super) async fn bootstrap_runtime(runtime: GatewayRuntimeConfig) -> Result<GatewayBootstrap> {
    let GatewayRuntimeConfig {
        config,
        loader,
        workspace,
        cron_store,
        port,
    } = runtime;
    let workspace_root = workspace.root.clone();

    let bus = AgentEventBus::new();
    let (fabric_handle, fabric_consumer) = FabricKernel::new().into_parts();
    let bus_for_hotreload = bus.clone();
    let run_store_root = supervised_store_root_from_cron_store(&cron_store);
    let run_store = Arc::new(RunStore::new(&run_store_root).await?);
    let command_rules = Arc::new(agent_diva_sandbox::CommandRuleStore::open(
        loader.config_dir().join("execpolicy.toml"),
    )?);
    let governance_dir = workspace_root.join(".laputa");
    std::fs::create_dir_all(&governance_dir)?;
    let governance_pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(governance_dir.join("governance.db"))
                .create_if_missing(true),
        )
        .await?;
    let governance = ApprovalCoordinator::new(Arc::new(
        SqliteGovernanceLedger::new(governance_pool).await?,
    ));
    let command_approvals = CommandApprovalCoordinator::default()
        .with_command_rules(command_rules)
        .governed(
            governance.clone(),
            agent_diva_core::workspace_identity::canonical_workspace_id(&workspace_root),
        );
    let recovered = command_approvals.recover_incomplete().await?;
    if recovered > 0 {
        tracing::warn!(
            recovered,
            "revoked incomplete command approvals during gateway restart"
        );
    }

    // Start config hot-reload background task.
    // The handle is intentionally dropped — the tokio runtime will clean up
    // the task on shutdown.
    let _hot_reload_handle = loader.start_hot_reload(move |diff, new_config| {
        handle_config_diff(diff, new_config, &bus_for_hotreload);
    });

    let cron_service =
        start_cron_service(cron_store, workspace_root.clone(), fabric_handle.clone()).await;
    ensure_notebook_monthly_cron_job(&cron_service).await?;
    let dynamic_provider = Arc::new(DynamicProvider::new(build_provider(
        &config,
        &config.agents.defaults.model,
    )?));

    // Initialize shared FileManager for attachment handling
    let storage_path = default_data_dir_or_fallback();
    let file_config = FileConfig::with_path(&storage_path);
    let file_manager = Arc::new(FileManager::new(file_config).await?);

    let (runtime_control_tx, runtime_control_rx) = mpsc::channel(capacity::CONTROL);
    let (egress_tx, egress_rx) = mpsc::channel(capacity::ADAPTER_EGRESS);
    let ask_user = agent_diva_core::ask_user::AskUserCoordinator::default();
    let memory_home = agent_diva_laputa::MemoryHome::with_l1_budget(
        loader.config_dir(),
        config.memory.l1_index_lines,
    );
    let persisted_sessions = agent_diva_core::session::SessionManager::new(&workspace_root)
        .list_sessions()
        .into_iter()
        .map(|session| session.key)
        .collect::<Vec<_>>();
    let active_session_ids = persisted_sessions
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if let Err(error) = memory_home.run_startup_gc(&active_session_ids).await {
        tracing::warn!(%error, "session checkpoint startup GC unavailable");
    }
    if let Err(error) = memory_home.warmup().await {
        tracing::warn!(%error, "BML startup index unavailable; ACTMEM remains available");
    }
    let mut agent = build_agent_loop(
        &config,
        bus.clone(),
        dynamic_provider.clone(),
        workspace_root,
        loader.config_dir().to_path_buf(),
        memory_home.clone(),
        runtime_control_rx,
        Arc::clone(&cron_service),
        Arc::clone(&file_manager),
        Arc::clone(&run_store),
        command_approvals.clone(),
        ask_user.clone(),
        governance.clone(),
    )
    .await?;
    agent.set_egress_sender(egress_tx);
    agent.set_fabric_handle(fabric_handle.clone()).await;
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
        run_store,
        command_approvals,
        ask_user,
        governance,
        memory_home,
        fabric_handle,
        fabric_consumer,
        egress_rx,
    })
}

fn supervised_store_root_from_cron_store(cron_store: &std::path::Path) -> PathBuf {
    cron_store
        .parent()
        .and_then(std::path::Path::parent)
        .map(|data_root| data_root.join("supervised"))
        .unwrap_or_else(|| {
            cron_store
                .parent()
                .map(|parent| parent.join("supervised"))
                .unwrap_or_else(|| PathBuf::from("supervised"))
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
    file_manager: Arc<FileManager>,
    fabric: FabricHandle,
) -> Result<ChannelBootstrap> {
    let attachments =
        Arc::new(crate::channel_attachment_store::FileManagerAttachmentStore::new(file_manager));
    let services = agent_diva_channels::adapter::AdapterServices::new(attachments);
    let channel_runtime = ChannelRuntime::start(config, services, fabric)
        .await
        .map_err(|error| anyhow::anyhow!("failed to start native channel runtime: {error}"))?;
    Ok(ChannelBootstrap { channel_runtime })
}

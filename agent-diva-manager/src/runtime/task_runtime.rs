use super::*;
use crate::{run_server, AppState, Manager};
use agent_diva_agent::subagent_run_handler::SubagentRunHandler;
use agent_diva_channels::runtime::ChannelRuntime;
use agent_diva_core::channel::ChannelCommand;
use agent_diva_core::supervised::{RunKind, TaskExecutor};

pub(super) async fn start_runtime_tasks(
    bootstrap: GatewayBootstrap,
    channel_bootstrap: ChannelBootstrap,
) -> GatewayTasks {
    start_runtime_tasks_inner(bootstrap, channel_bootstrap, ServerRuntime::BoundPort).await
}

pub(super) async fn start_embedded_runtime_tasks(
    bootstrap: GatewayBootstrap,
    channel_bootstrap: ChannelBootstrap,
    listener: tokio::net::TcpListener,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> GatewayTasks {
    start_runtime_tasks_inner(
        bootstrap,
        channel_bootstrap,
        ServerRuntime::Embedded {
            listener,
            shutdown_rx,
        },
    )
    .await
}

enum ServerRuntime {
    BoundPort,
    Embedded {
        listener: tokio::net::TcpListener,
        shutdown_rx: tokio::sync::watch::Receiver<bool>,
    },
}

async fn start_runtime_tasks_inner(
    bootstrap: GatewayBootstrap,
    channel_bootstrap: ChannelBootstrap,
    server_runtime: ServerRuntime,
) -> GatewayTasks {
    let GatewayBootstrap {
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
    } = bootstrap;
    let ChannelBootstrap { channel_runtime } = channel_bootstrap;
    let workspace_root = workspace.root.clone();
    let config_dir = loader.config_dir().to_path_buf();

    if let Err(error) = agent_diva_core::audit_sink::ensure_workspace_jsonl_sink(&workspace_root) {
        tracing::error!(
            "failed to initialize workspace audit sink for {}: {}",
            workspace_root.display(),
            error
        );
    }

    let (api_tx, api_rx) = mpsc::channel(100);
    let planning_service = Arc::new(crate::planning_service::PlanningService::governed(
        workspace_root.clone(),
        governance.clone(),
        agent_diva_core::workspace_identity::canonical_workspace_id(&workspace_root),
    ));
    let recovered_plans = planning_service
        .recover_incomplete()
        .await
        .expect("Plan approval recovery must complete before serving requests");
    if recovered_plans > 0 {
        tracing::warn!(
            recovered_plans,
            "recovered incomplete Plan approvals at startup"
        );
    }
    let runtime_control_tx_for_state = runtime_control_tx.clone();
    let pending_admissions = crate::channel_fabric_runtime::pending_admissions();
    let neuro_link_runtime = Arc::new(crate::channel_fabric_runtime::FabricNeuroLinkRuntime::new(
        fabric_handle.clone(),
        pending_admissions.clone(),
        runtime_control_tx.clone(),
    ));
    let fabric_ingress_handle = crate::channel_fabric_runtime::spawn_fabric_ingress(
        fabric_consumer,
        pending_admissions,
        runtime_control_tx.clone(),
    );
    let manager = Manager::new(
        api_rx,
        bus.clone(),
        dynamic_provider,
        loader,
        config.agents.defaults.provider.clone(),
        config.agents.defaults.model.clone(),
        provider_api_key,
        provider_api_base,
        Some(channel_runtime.clone()),
        Some(runtime_control_tx),
        Some(fabric_handle.clone()),
        Arc::clone(&cron_service),
        file_manager.clone(),
        workspace_root,
        governance.clone(),
        Some(Arc::clone(&planning_service)),
    );
    let api_tx_keepalive = api_tx.clone();

    let outbound_dispatch_handle =
        spawn_native_outbound_dispatch(egress_rx, channel_runtime.clone()).await;
    let supervised_executor_cancel = tokio_util::sync::CancellationToken::new();
    let supervised_executor_handle = spawn_supervised_executor(
        run_store,
        agent.subagent_manager(),
        supervised_executor_cancel.clone(),
    );
    let agent_handle = spawn_agent_runtime(agent);
    let manager_handle = spawn_manager_runtime(manager);
    let app_state = AppState::new_with_runtime_governance_and_control(
        api_tx,
        bus.clone(),
        workspace,
        config_dir,
        memory_home,
        command_approvals,
        ask_user,
        governance,
        planning_service,
        runtime_control_tx_for_state,
    )
    .expect("manager AppState storage services initialize")
    .with_neuro_link_runtime(neuro_link_runtime)
    .with_fabric_handle(fabric_handle.clone())
    .with_attachment_authority(file_manager.clone());
    app_state.health.mark_cron_ready();
    let (server_shutdown_tx, server_handle) = match server_runtime {
        ServerRuntime::BoundPort => spawn_server_runtime(port, app_state),
        ServerRuntime::Embedded {
            listener,
            shutdown_rx,
        } => spawn_embedded_server_runtime(app_state, listener, shutdown_rx),
    };

    GatewayTasks {
        cron_service,
        channel_runtime,
        server_shutdown_tx,
        fabric_ingress_handle,
        outbound_dispatch_handle,
        agent_handle,
        supervised_executor_cancel,
        supervised_executor_handle,
        manager_handle,
        server_handle,
        _api_tx_keepalive: api_tx_keepalive,
    }
}

async fn spawn_native_outbound_dispatch(
    mut outbound_rx: mpsc::Receiver<ChannelCommand>,
    channel_runtime: Arc<ChannelRuntime>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(command) = outbound_rx.recv().await {
            let channel = command
                .target_channel()
                .map(|id| id.to_string())
                .unwrap_or_else(|_| "unknown".to_string());
            let cancel = tokio_util::sync::CancellationToken::new();
            match channel_runtime.execute(command, &cancel).await {
                Ok(receipt) => tracing::info!(
                    %channel,
                    status = ?receipt.status,
                    platform_message_id = ?receipt.platform_message_id,
                    "native channel delivery receipt"
                ),
                Err(error) => tracing::error!(
                    %channel,
                    %error,
                    "native channel delivery failed"
                ),
            }
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

fn spawn_supervised_executor(
    run_store: Arc<RunStore>,
    subagent_manager: Arc<agent_diva_agent::subagent::SubagentManager>,
    cancel: tokio_util::sync::CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut executor = TaskExecutor::new((*run_store).clone(), "manager-subagent-worker");
        executor.register_handler(
            RunKind::Subagent,
            Arc::new(SubagentRunHandler::new(subagent_manager)),
        );
        executor.run(cancel).await;
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

fn spawn_embedded_server_runtime(
    state: AppState,
    listener: tokio::net::TcpListener,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> (broadcast::Sender<()>, JoinHandle<()>) {
    let (server_shutdown_tx, _) = broadcast::channel(1);
    let server_handle = tokio::spawn(async move {
        if let Err(e) = crate::server::run_server_with_listener(state, listener, shutdown_rx).await
        {
            tracing::error!("API Server error: {}", e);
        }
    });
    (server_shutdown_tx, server_handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_native_command_keeps_envelope_identity() {
        let mut correlation = agent_diva_core::channel::Correlation::new("runtime/session");
        correlation.reply_to = Some("message-1".to_string());
        let command = ChannelCommand::Send {
            envelope: agent_diva_core::channel::ChannelEnvelopeV1::new(
                agent_diva_core::channel::ChannelDirection::Egress,
                agent_diva_core::channel::ChannelAddress::new("telegram", "chat-1"),
                correlation,
                agent_diva_core::channel::ChannelOrigin::Runtime,
                agent_diva_core::channel::ChannelPayloadV1::Message {
                    parts: vec![agent_diva_core::channel::ContentPart::Markdown {
                        markdown: "hello".to_string(),
                    }],
                    subject: None,
                    locale: None,
                    context: None,
                },
            ),
            idempotency_key: None,
        };
        let ChannelCommand::Send { envelope, .. } = command else {
            panic!("expected send command");
        };
        assert_eq!(envelope.address.channel, "telegram");
        assert_eq!(envelope.address.chat_id, "chat-1");
        assert_eq!(envelope.correlation.session_key, "runtime/session");
        assert_eq!(envelope.correlation.reply_to.as_deref(), Some("message-1"));
    }
}

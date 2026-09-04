use super::{
    AdapterPacingHandle, AdapterPacingLane, AdapterRegistry, AdapterSupervisor, PacingError,
};
use crate::adapter::{
    build_active_adapters, probe_candidate, AdapterBuildError, AdapterContext, AdapterServices,
    ChannelAdapter, ChannelProbeError,
};
use agent_diva_core::channel::{
    ChannelCommand, ChannelHealthStatus, DeliveryReceipt, FabricHandle,
};
use agent_diva_core::config::Config;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

const EGRESS_ADMISSION_DEADLINE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChannelRuntimeStatus {
    pub name: String,
    pub registered: bool,
    pub lifecycle: String,
    pub health: ChannelHealthStatus,
    pub capabilities: Vec<String>,
    pub egress_remaining_capacity: usize,
    pub consecutive_failures: u32,
    pub diagnosis: Option<String>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum ChannelRuntimeError {
    #[error(transparent)]
    Build(#[from] AdapterBuildError),
    #[error("channel runtime registration failed: {0}")]
    Registry(String),
    #[error("channel runtime is not active: {0}")]
    Unknown(String),
    #[error(transparent)]
    Pacing(#[from] PacingError),
}

struct RuntimeEntry {
    adapter: Arc<dyn ChannelAdapter>,
    pacing: AdapterPacingLane,
    pacing_handle: AdapterPacingHandle,
    supervisor: AdapterSupervisor,
}

/// Production owner for native adapters, their listener supervisors, and bounded egress lanes.
pub struct ChannelRuntime {
    services: AdapterServices,
    fabric: FabricHandle,
    registry: Arc<AdapterRegistry>,
    cancel: CancellationToken,
    entries: Mutex<BTreeMap<String, RuntimeEntry>>,
}

impl std::fmt::Debug for ChannelRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChannelRuntime")
            .field("registry", &self.registry)
            .finish_non_exhaustive()
    }
}

impl ChannelRuntime {
    pub async fn start(
        config: &Config,
        services: AdapterServices,
        fabric: FabricHandle,
    ) -> Result<Arc<Self>, ChannelRuntimeError> {
        let runtime = Arc::new(Self {
            services,
            fabric,
            registry: Arc::new(AdapterRegistry::new()),
            cancel: CancellationToken::new(),
            entries: Mutex::new(BTreeMap::new()),
        });
        runtime.reconfigure(config).await?;
        Ok(runtime)
    }

    /// Construct every candidate before replacing the live set.
    pub async fn reconfigure(&self, config: &Config) -> Result<(), ChannelRuntimeError> {
        let candidates = build_active_adapters(config, self.services.clone())?;
        self.stop_entries().await;
        let mut installed = BTreeMap::new();
        for adapter in candidates {
            let id = adapter.name();
            self.registry
                .register(adapter.clone())
                .await
                .map_err(|error| ChannelRuntimeError::Registry(error.to_string()))?;
            self.registry
                .mark_running(&id)
                .await
                .map_err(|error| ChannelRuntimeError::Registry(error.to_string()))?;
            let pacing = AdapterPacingLane::spawn(adapter.clone());
            let pacing_handle = pacing.handle();
            let supervisor = AdapterSupervisor::spawn(
                adapter.clone(),
                AdapterContext {
                    fabric: self.fabric.clone(),
                    cancel: self.cancel.child_token(),
                },
            );
            installed.insert(
                id.to_string(),
                RuntimeEntry {
                    adapter,
                    pacing,
                    pacing_handle,
                    supervisor,
                },
            );
        }
        *self.entries.lock().await = installed;
        Ok(())
    }

    pub async fn execute(
        &self,
        command: ChannelCommand,
        cancel: &CancellationToken,
    ) -> Result<DeliveryReceipt, ChannelRuntimeError> {
        let channel = command
            .target_channel()
            .map_err(|error| ChannelRuntimeError::Registry(error.to_string()))?
            .to_string();
        let handle = self
            .entries
            .lock()
            .await
            .get(&channel)
            .map(|entry| entry.pacing_handle.clone())
            .ok_or(ChannelRuntimeError::Unknown(channel))?;
        Ok(handle
            .submit(command, EGRESS_ADMISSION_DEADLINE, cancel)
            .await?)
    }

    /// Probe a candidate configuration without changing the registered
    /// runtime set or starting a listener.
    pub async fn probe_candidate(
        &self,
        config: &Config,
        channel: &str,
        timeout: Duration,
    ) -> Result<DeliveryReceipt, ChannelProbeError> {
        probe_candidate(config, channel, self.services.clone(), timeout).await
    }

    pub async fn statuses(&self) -> Vec<ChannelRuntimeStatus> {
        self.entries
            .lock()
            .await
            .iter()
            .map(|(name, entry)| {
                let health = entry.supervisor.health();
                ChannelRuntimeStatus {
                    name: name.clone(),
                    registered: true,
                    lifecycle: match health.status {
                        ChannelHealthStatus::Healthy => "running",
                        ChannelHealthStatus::Degraded => "degraded",
                        ChannelHealthStatus::Down => "down",
                        ChannelHealthStatus::Unknown => "starting",
                    }
                    .to_string(),
                    health: health.status,
                    capabilities: entry
                        .adapter
                        .capabilities()
                        .supported
                        .iter()
                        .map(|capability| format!("{capability:?}"))
                        .collect(),
                    egress_remaining_capacity: entry.pacing_handle.remaining_capacity(),
                    consecutive_failures: health.consecutive_failures,
                    diagnosis: health.diagnosis,
                    checked_at: health.checked_at,
                }
            })
            .collect()
    }

    pub async fn shutdown(&self) {
        self.cancel.cancel();
        self.stop_entries().await;
    }

    async fn stop_entries(&self) {
        let entries = std::mem::take(&mut *self.entries.lock().await);
        for (name, entry) in entries {
            entry.supervisor.shutdown().await;
            entry.pacing.shutdown().await;
            let id = entry.adapter.name();
            self.registry.mark_stopped(&id).await;
            if let Err(error) = self.registry.unregister(&id).await {
                tracing::warn!(channel = %name, %error, "failed to unregister native adapter");
            }
        }
    }
}

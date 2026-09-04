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
use futures::future::join_all;
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
    #[error("channel runtime lifecycle failed: {0}")]
    Lifecycle(String),
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
    /// Serializes runtime transitions with all reads/submissions. The entry
    /// map is only observed while this guard is held, so a transition cannot
    /// expose a partially installed set to egress or status callers.
    lifecycle: Mutex<()>,
    /// Last successfully installed configuration, used to rebuild the live
    /// set if a candidate transition fails after old workers are stopped.
    active_config: Mutex<Option<Config>>,
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
            lifecycle: Mutex::new(()),
            active_config: Mutex::new(None),
        });
        runtime.reconfigure(config).await?;
        Ok(runtime)
    }

    /// Construct every candidate before replacing the live set.
    pub async fn reconfigure(&self, config: &Config) -> Result<(), ChannelRuntimeError> {
        let candidates = build_active_adapters(config, self.services.clone())?;
        let _lifecycle = self.lifecycle.lock().await;
        let previous_config = self.active_config.lock().await.clone();
        self.stop_entries_locked().await;
        let installed = match self.install_candidates(candidates).await {
            Ok(installed) => installed,
            Err(error) => {
                // install_candidates owns cleanup of every registered item,
                // including an adapter whose pacing/supervisor was only
                // partially created. Rebuild the last known good set while
                // the lifecycle guard is still held, so readers never see an
                // empty/half-installed runtime after a failed transition.
                if let Some(previous_config) = previous_config {
                    let restore_candidates = match build_active_adapters(
                        &previous_config,
                        self.services.clone(),
                    ) {
                        Ok(candidates) => candidates,
                        Err(restore_error) => {
                            return Err(ChannelRuntimeError::Lifecycle(format!(
                                "{error}; runtime recovery candidate construction failed: {restore_error}"
                            )));
                        }
                    };
                    match self.install_candidates(restore_candidates).await {
                        Ok(restored) => {
                            *self.entries.lock().await = restored;
                        }
                        Err(restore_error) => {
                            return Err(ChannelRuntimeError::Lifecycle(format!(
                                "{error}; runtime recovery failed: {restore_error}"
                            )));
                        }
                    }
                }
                return Err(error);
            }
        };
        *self.entries.lock().await = installed;
        *self.active_config.lock().await = Some(config.clone());
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
        let handle = {
            // The lifecycle guard protects the map lookup. The cloned handle
            // remains cancellation-aware, allowing a transition to stop the
            // lane promptly instead of waiting on an in-flight transport.
            let _lifecycle = self.lifecycle.lock().await;
            self.entries
                .lock()
                .await
                .get(&channel)
                .map(|entry| entry.pacing_handle.clone())
                .ok_or(ChannelRuntimeError::Unknown(channel))?
        };
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
        let _lifecycle = self.lifecycle.lock().await;
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
        let _lifecycle = self.lifecycle.lock().await;
        self.stop_entries_locked().await;
    }

    async fn install_candidates(
        &self,
        candidates: Vec<Arc<dyn ChannelAdapter>>,
    ) -> Result<BTreeMap<String, RuntimeEntry>, ChannelRuntimeError> {
        let mut installed = BTreeMap::new();
        for adapter in candidates {
            let id = adapter.name();
            if let Err(error) = self.registry.register(adapter.clone()).await {
                self.shutdown_entries(installed).await;
                return Err(ChannelRuntimeError::Registry(error.to_string()));
            }
            if let Err(error) = self.registry.mark_running(&id).await {
                self.registry.mark_stopped(&id).await;
                self.unregister_adapter(&id, "mark-running rollback").await;
                self.shutdown_entries(installed).await;
                return Err(ChannelRuntimeError::Registry(error.to_string()));
            }

            let pacing = match AdapterPacingLane::try_spawn(adapter.clone()) {
                Ok(pacing) => pacing,
                Err(error) => {
                    self.registry.mark_stopped(&id).await;
                    self.unregister_adapter(&id, "pacing spawn rollback").await;
                    self.shutdown_entries(installed).await;
                    return Err(ChannelRuntimeError::Pacing(error));
                }
            };
            let pacing_handle = pacing.handle();
            let supervisor = match AdapterSupervisor::try_spawn(
                adapter.clone(),
                AdapterContext {
                    fabric: self.fabric.clone(),
                    cancel: self.cancel.child_token(),
                },
            ) {
                Ok(supervisor) => supervisor,
                Err(error) => {
                    pacing.shutdown().await;
                    self.registry.mark_stopped(&id).await;
                    self.unregister_adapter(&id, "supervisor spawn rollback")
                        .await;
                    self.shutdown_entries(installed).await;
                    return Err(ChannelRuntimeError::Lifecycle(error.to_string()));
                }
            };
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
        Ok(installed)
    }

    async fn stop_entries_locked(&self) {
        let entries = std::mem::take(&mut *self.entries.lock().await);
        self.shutdown_entries(entries).await;
    }

    async fn shutdown_entries(&self, entries: BTreeMap<String, RuntimeEntry>) {
        let stopped = join_all(entries.into_iter().map(|(name, entry)| async move {
            let id = entry.adapter.name();
            self.registry.mark_stopped(&id).await;
            let RuntimeEntry {
                pacing, supervisor, ..
            } = entry;
            let _ = tokio::join!(supervisor.shutdown(), pacing.shutdown());
            (name, id)
        }))
        .await;
        for (name, id) in stopped {
            self.unregister_adapter(&id, &format!("{name} shutdown"))
                .await;
        }
    }

    async fn unregister_adapter(&self, id: &agent_diva_core::channel::ChannelId, operation: &str) {
        if let Err(error) = self.registry.unregister(id).await {
            tracing::warn!(channel = %id, %error, operation, "failed to unregister native adapter");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{
        AttachmentStoreError, ChannelAttachmentStore, IngressAttachment, StoredAttachment,
    };
    use agent_diva_core::channel::{
        AttachmentRef, ChannelCapabilities, ChannelHealth, ChannelHealthStatus, FabricKernel,
    };
    use async_trait::async_trait;

    #[derive(Default)]
    struct TestAttachments;

    #[async_trait]
    impl ChannelAttachmentStore for TestAttachments {
        async fn put(
            &self,
            _input: IngressAttachment,
        ) -> Result<AttachmentRef, AttachmentStoreError> {
            Err(AttachmentStoreError::Backend {
                diagnosis: "test attachment store does not persist values".to_string(),
            })
        }

        async fn get(
            &self,
            reference: &AttachmentRef,
        ) -> Result<StoredAttachment, AttachmentStoreError> {
            Err(AttachmentStoreError::NotFound {
                uri: reference.uri.clone(),
            })
        }
    }

    struct TestAdapter {
        id: agent_diva_core::channel::ChannelId,
    }

    #[async_trait]
    impl ChannelAdapter for TestAdapter {
        fn name(&self) -> agent_diva_core::channel::ChannelId {
            self.id.clone()
        }

        fn capabilities(&self) -> agent_diva_core::channel::ChannelCapabilities {
            ChannelCapabilities::new(std::iter::empty())
        }

        async fn start(&self, context: AdapterContext) -> Result<(), crate::adapter::AdapterError> {
            context.cancel.cancelled().await;
            Ok(())
        }

        async fn execute(
            &self,
            _command: ChannelCommand,
        ) -> Result<DeliveryReceipt, crate::adapter::AdapterError> {
            Err(crate::adapter::AdapterError::Stopped)
        }

        fn health(&self) -> agent_diva_core::channel::ChannelHealth {
            ChannelHealth::new(ChannelHealthStatus::Healthy)
        }

        async fn stop(&self) -> Result<(), crate::adapter::AdapterError> {
            Ok(())
        }
    }

    fn test_runtime() -> ChannelRuntime {
        let (fabric, _consumer) = FabricKernel::new().into_parts();
        ChannelRuntime {
            services: AdapterServices::new(Arc::new(TestAttachments)),
            fabric,
            registry: Arc::new(AdapterRegistry::new()),
            cancel: CancellationToken::new(),
            entries: Mutex::new(BTreeMap::new()),
            lifecycle: Mutex::new(()),
            active_config: Mutex::new(None),
        }
    }

    fn test_adapter(name: &str) -> Arc<TestAdapter> {
        Arc::new(TestAdapter {
            id: agent_diva_core::channel::ChannelId::new(name).unwrap(),
        })
    }

    #[tokio::test]
    async fn failed_partial_install_cleans_registry_and_owned_workers() {
        let runtime = test_runtime();
        let adapter = test_adapter("partial");
        let result = runtime
            .install_candidates(vec![adapter.clone(), adapter.clone()])
            .await;

        assert!(matches!(result, Err(ChannelRuntimeError::Registry(_))));
        assert!(runtime.registry.list().await.is_empty());
        assert!(runtime.entries.lock().await.is_empty());
    }

    #[tokio::test]
    async fn lifecycle_gate_blocks_status_observation_during_transition() {
        let runtime = Arc::new(test_runtime());
        let guard = runtime.lifecycle.lock().await;
        let status_runtime = runtime.clone();
        let mut status_task = tokio::spawn(async move { status_runtime.statuses().await });
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut status_task)
                .await
                .is_err()
        );
        drop(guard);
        assert!(status_task.await.unwrap().is_empty());
    }
}

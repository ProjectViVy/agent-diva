use super::supervisor::{StopCleanupRegistry, SupervisorPolicy};
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
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const EGRESS_ADMISSION_DEADLINE: Duration = Duration::from_secs(2);
const RECONFIGURE_LIFECYCLE_LOCK_TIMEOUT: Duration = Duration::from_secs(1);

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

/// Owns one runtime lifecycle transaction independently of its caller.
///
/// Reconfiguration moves the live entry map out, awaits supervisor shutdown,
/// then installs a new generation. A caller may be cancelled at any of those
/// await points; this slot keeps the spawned owner alive so the map cannot be
/// left taken and the old JoinHandles cannot be detached. The reservation is
/// synchronous to close the spawn-to-owner handoff race.
struct RuntimeOwnerSlot {
    state: StdMutex<RuntimeOwnerState>,
}

struct RuntimeOwnerState {
    reserved: bool,
    /// The owner has completed all runtime work and will only return from its
    /// task body. It is safe for the next transaction to replace this
    /// completed handle even if Tokio has not marked it finished yet.
    releasable: bool,
    task: Option<JoinHandle<()>>,
}

impl RuntimeOwnerSlot {
    fn new() -> Self {
        Self {
            state: StdMutex::new(RuntimeOwnerState {
                reserved: false,
                releasable: false,
                task: None,
            }),
        }
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, RuntimeOwnerState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Reserve the slot before spawning. This is a non-awaiting admission
    /// boundary, so a competing command cannot observe an unowned task.
    fn try_reserve(&self) -> bool {
        let mut state = self.lock_state();
        if state.releasable {
            state.task.take();
            state.releasable = false;
        }
        if state
            .task
            .as_ref()
            .is_some_and(tokio::task::JoinHandle::is_finished)
        {
            state.task.take();
        }
        if state.reserved || state.task.is_some() {
            return false;
        }
        state.reserved = true;
        true
    }

    fn adopt(&self, task: JoinHandle<()>) {
        let mut state = self.lock_state();
        debug_assert!(state.reserved);
        debug_assert!(state.task.is_none());
        state.reserved = false;
        state.releasable = false;
        state.task = Some(task);
    }

    /// Mark the owner available only after its transaction result has been
    /// sent. No mutable runtime work follows this call, so replacing the
    /// completed JoinHandle cannot detach an in-flight transition.
    fn mark_releasable(&self) {
        let mut state = self.lock_state();
        debug_assert!(!state.reserved);
        debug_assert!(state.task.is_some());
        state.releasable = true;
    }

    #[cfg(test)]
    fn is_owned(&self) -> bool {
        let state = self.lock_state();
        state.reserved || state.task.is_some()
    }

    #[cfg(test)]
    fn is_finished(&self) -> bool {
        let state = self.lock_state();
        state
            .task
            .as_ref()
            .is_some_and(tokio::task::JoinHandle::is_finished)
    }
}

/// Production owner for native adapters, their listener supervisors, and bounded egress lanes.
pub struct ChannelRuntime {
    services: AdapterServices,
    fabric: FabricHandle,
    registry: Arc<AdapterRegistry>,
    cancel: CancellationToken,
    entries: Arc<Mutex<BTreeMap<String, RuntimeEntry>>>,
    /// Serializes runtime transitions with all reads/submissions. The entry
    /// map is only observed while this guard is held, so a transition cannot
    /// expose a partially installed set to egress or status callers.
    lifecycle: Arc<Mutex<()>>,
    /// Last successfully installed configuration, used to rebuild the live
    /// set if a candidate transition fails after old workers are stopped.
    active_config: Arc<Mutex<Option<Config>>>,
    /// Owns stop futures that can outlive a supervisor's bounded shutdown.
    stop_cleanup: Arc<StopCleanupRegistry>,
    /// Keeps a reconfiguration transaction alive if its Manager/API waiter is
    /// cancelled while stopping or installing a generation.
    transition_owner: Arc<RuntimeOwnerSlot>,
    /// Shutdown has a separate owner because it may be requested while a
    /// transition owner is still draining the previous generation.
    shutdown_owner: Arc<RuntimeOwnerSlot>,
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
            entries: Arc::new(Mutex::new(BTreeMap::new())),
            lifecycle: Arc::new(Mutex::new(())),
            active_config: Arc::new(Mutex::new(None)),
            stop_cleanup: Arc::new(StopCleanupRegistry::new()),
            transition_owner: Arc::new(RuntimeOwnerSlot::new()),
            shutdown_owner: Arc::new(RuntimeOwnerSlot::new()),
        });
        runtime.reconfigure(config).await?;
        Ok(runtime)
    }

    /// Construct every candidate before replacing the live set.
    ///
    /// The public waiter only receives the owner result. The full transaction
    /// is retained by `transition_owner`, so dropping this future cannot
    /// cancel a `mem::take`/install sequence or detach an adapter worker.
    pub async fn reconfigure(&self, config: &Config) -> Result<(), ChannelRuntimeError> {
        if !self.transition_owner.try_reserve() {
            return Err(ChannelRuntimeError::Lifecycle(
                "channel runtime transition is busy; retry later".to_string(),
            ));
        }
        let (result_sender, result_receiver) = tokio::sync::oneshot::channel();
        let runtime = self.owner_arc();
        let owner_slot = self.transition_owner.clone();
        let config = config.clone();
        let owner = tokio::spawn(async move {
            let result = runtime.reconfigure_owned(&config).await;
            let _ = result_sender.send(result);
            owner_slot.mark_releasable();
        });
        self.transition_owner.adopt(owner);
        result_receiver.await.map_err(|_| {
            ChannelRuntimeError::Lifecycle(
                "channel runtime transaction owner stopped unexpectedly".to_string(),
            )
        })?
    }

    async fn reconfigure_owned(&self, config: &Config) -> Result<(), ChannelRuntimeError> {
        let candidates = build_active_adapters(config, self.services.clone())?;
        // Once acquired, the transition below consists only of bounded
        // supervisor/pacing shutdowns and synchronous candidate assembly.
        // Bound the only externally-contended lock separately so callers do
        // not need to cancel a transaction halfway through installation.
        let _lifecycle =
            tokio::time::timeout(RECONFIGURE_LIFECYCLE_LOCK_TIMEOUT, self.lifecycle.lock())
                .await
                .map_err(|_| {
                    ChannelRuntimeError::Lifecycle(
                        "channel runtime transition is busy; retry later".to_string(),
                    )
                })?;
        if self.stop_cleanup.pending().await > 0 {
            return Err(ChannelRuntimeError::Lifecycle(
                "channel runtime cleanup is still pending; retry later".to_string(),
            ));
        }
        let previous_config = self.active_config.lock().await.clone();
        self.stop_entries_locked().await?;
        let installed = match self.install_candidates(candidates).await {
            Ok(installed) => installed,
            Err(error) => {
                // install_candidates owns cleanup of every registered item,
                // including an adapter whose pacing/supervisor was only
                // partially created. Rebuild the last known good set while
                // the lifecycle guard is still held, so readers never see an
                // empty/half-installed runtime after a failed transition.
                if self.stop_cleanup.pending().await > 0 {
                    return Err(ChannelRuntimeError::Lifecycle(format!(
                        "{error}; runtime cleanup is still pending and no new generation was installed"
                    )));
                }
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
                    diagnosis: public_health_diagnosis(health.status),
                    checked_at: health.checked_at,
                }
            })
            .collect()
    }

    pub async fn shutdown(&self) {
        self.cancel.cancel();
        // Shutdown is idempotent. If another shutdown owner is already
        // draining, it remains the sole owner and this caller can return.
        if !self.shutdown_owner.try_reserve() {
            return;
        }
        let (done_sender, done_receiver) = tokio::sync::oneshot::channel();
        let runtime = self.owner_arc();
        let owner_slot = self.shutdown_owner.clone();
        let owner = tokio::spawn(async move {
            let _lifecycle = runtime.lifecycle.lock().await;
            let _ = runtime.stop_entries_locked().await;
            let _ = done_sender.send(());
            owner_slot.mark_releasable();
        });
        self.shutdown_owner.adopt(owner);
        let _ = done_receiver.await;
    }

    /// Build an owner handle that shares every mutable runtime resource while
    /// allowing the caller-facing `&self` future to be dropped safely.
    fn owner_arc(&self) -> Arc<Self> {
        Arc::new(Self {
            services: self.services.clone(),
            fabric: self.fabric.clone(),
            registry: self.registry.clone(),
            cancel: self.cancel.clone(),
            entries: self.entries.clone(),
            lifecycle: self.lifecycle.clone(),
            active_config: self.active_config.clone(),
            stop_cleanup: self.stop_cleanup.clone(),
            transition_owner: self.transition_owner.clone(),
            shutdown_owner: self.shutdown_owner.clone(),
        })
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
            let supervisor = match AdapterSupervisor::try_spawn_with_policy_and_cleanup(
                adapter.clone(),
                AdapterContext {
                    fabric: self.fabric.clone(),
                    cancel: self.cancel.child_token(),
                },
                SupervisorPolicy::default(),
                self.stop_cleanup.clone(),
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

    async fn stop_entries_locked(&self) -> Result<(), ChannelRuntimeError> {
        let entries = std::mem::take(&mut *self.entries.lock().await);
        self.shutdown_entries(entries).await;
        if self.stop_cleanup.pending().await > 0 {
            // The live map is deliberately empty here: the old generation
            // has been unregistered, while active_config remains the last
            // known-good rebuild source.  Do not install a new generation
            // until every old stop owner has drained; callers receive a
            // definite unavailable result and may retry the transition.
            return Err(ChannelRuntimeError::Lifecycle(
                "channel adapter cleanup exceeded its bounded wait; runtime generation is unavailable"
                    .to_string(),
            ));
        }
        Ok(())
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

/// Runtime status is an operator-facing API.  Adapter diagnostics are kept
/// internally for logs, but transport errors can contain endpoint URLs,
/// response bodies, or credentials.  Expose only a stable state summary.
fn public_health_diagnosis(status: ChannelHealthStatus) -> Option<String> {
    match status {
        ChannelHealthStatus::Healthy => None,
        ChannelHealthStatus::Degraded => Some("adapter is degraded".to_string()),
        ChannelHealthStatus::Down => Some("adapter is unavailable".to_string()),
        ChannelHealthStatus::Unknown => Some("adapter is starting".to_string()),
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
            entries: Arc::new(Mutex::new(BTreeMap::new())),
            lifecycle: Arc::new(Mutex::new(())),
            active_config: Arc::new(Mutex::new(None)),
            stop_cleanup: Arc::new(StopCleanupRegistry::new()),
            transition_owner: Arc::new(RuntimeOwnerSlot::new()),
            shutdown_owner: Arc::new(RuntimeOwnerSlot::new()),
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

    #[tokio::test]
    async fn reconfigure_fails_fast_when_lifecycle_transition_is_busy() {
        let runtime = Arc::new(test_runtime());
        let guard = runtime.lifecycle.lock().await;
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            runtime.reconfigure(&Config::default()),
        )
        .await
        .expect("busy transition must have a bounded result");

        assert!(matches!(
            result,
            Err(ChannelRuntimeError::Lifecycle(message))
                if message == "channel runtime transition is busy; retry later"
        ));
        drop(guard);
    }

    #[tokio::test]
    async fn completed_transition_owner_is_reusable_immediately() {
        let runtime = Arc::new(test_runtime());
        runtime
            .reconfigure(&Config::default())
            .await
            .expect("first transition");
        // The first owner sends its result before Tokio necessarily marks its
        // JoinHandle finished. A rollback issued immediately by Manager must
        // still be admitted once all transition work is complete.
        runtime
            .reconfigure(&Config::default())
            .await
            .expect("immediate rollback transition");
        assert!(runtime.entries.lock().await.is_empty());
    }

    #[tokio::test]
    async fn cancelled_reconfigure_owner_finishes_late_without_partial_runtime() {
        let runtime = Arc::new(test_runtime());
        let guard = runtime.lifecycle.lock().await;
        let caller_runtime = runtime.clone();
        let caller =
            tokio::spawn(async move { caller_runtime.reconfigure(&Config::default()).await });

        tokio::time::timeout(Duration::from_secs(1), async {
            while !runtime.transition_owner.is_owned() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("transition owner was registered before waiting");
        caller.abort();
        assert!(caller
            .await
            .expect_err("caller task was cancelled")
            .is_cancelled());
        drop(guard);

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if runtime.active_config.lock().await.is_some()
                    && runtime.transition_owner.is_finished()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("registry-held transaction completed after caller cancellation");
        assert!(runtime.entries.lock().await.is_empty());
    }

    #[tokio::test]
    async fn cancelled_shutdown_waiter_keeps_shutdown_owner() {
        let runtime = Arc::new(test_runtime());
        let guard = runtime.lifecycle.lock().await;
        let shutdown_runtime = runtime.clone();
        let shutdown = tokio::spawn(async move { shutdown_runtime.shutdown().await });

        tokio::time::timeout(Duration::from_secs(1), async {
            while !runtime.shutdown_owner.is_owned() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("shutdown owner was registered before waiting");
        shutdown.abort();
        assert!(shutdown
            .await
            .expect_err("shutdown caller task was cancelled")
            .is_cancelled());
        drop(guard);

        tokio::time::timeout(Duration::from_secs(1), async {
            while !runtime.shutdown_owner.is_finished() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("registry-held shutdown owner completed");
        assert!(runtime.cancel.is_cancelled());
        assert!(runtime.entries.lock().await.is_empty());
    }

    #[tokio::test]
    async fn pending_stop_cleanup_rejects_new_generation_without_installing_workers() {
        let runtime = Arc::new(test_runtime());
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let cleanup_task = tokio::spawn(async move {
            let _ = release_rx.await;
            Ok::<(), crate::adapter::AdapterError>(())
        });
        let reservation = runtime
            .stop_cleanup
            .try_reserve("email")
            .await
            .expect("cleanup slot");
        runtime
            .stop_cleanup
            .adopt("email".to_string(), cleanup_task, reservation);

        let result = runtime.reconfigure(&Config::default()).await;
        assert!(matches!(result, Err(ChannelRuntimeError::Lifecycle(_))));
        assert!(runtime.registry.list().await.is_empty());
        assert!(runtime.entries.lock().await.is_empty());

        release_tx.send(()).expect("release cleanup owner");
        tokio::time::sleep(Duration::from_millis(40)).await;
    }

    #[test]
    fn public_runtime_diagnosis_is_a_state_summary_only() {
        assert_eq!(public_health_diagnosis(ChannelHealthStatus::Healthy), None);
        assert_eq!(
            public_health_diagnosis(ChannelHealthStatus::Down),
            Some("adapter is unavailable".to_string())
        );
    }
}

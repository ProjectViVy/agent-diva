use crate::adapter::{AdapterContext, ChannelAdapter};
use agent_diva_core::channel::{ChannelHealth, ChannelHealthStatus};
use chrono::Utc;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::sync::{oneshot, watch, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const ADAPTER_STOP_TIMEOUT: Duration = Duration::from_secs(5);
const SUPERVISOR_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(6);
const MAX_PENDING_STOP_CLEANUPS: usize = 6;
const MAX_PENDING_STOP_CLEANUPS_PER_CHANNEL: usize = 1;

/// Owns adapter stop futures that outlive the supervisor's bounded shutdown
/// wait.  The runtime keeps one registry for its whole lifetime, so a
/// reconfigure cannot drop a timed-out `EmailAdapter::stop` future with its
/// blocking-task owner still inside it.
pub(super) struct StopCleanupRegistry {
    next_id: AtomicU64,
    slots: Arc<Semaphore>,
    channels: [Arc<Semaphore>; 6],
    tasks: StdMutex<BTreeMap<u64, StopCleanupTask>>,
    reaper_started: std::sync::atomic::AtomicBool,
}

pub(super) struct StopCleanupReservation {
    _global: OwnedSemaphorePermit,
    _channel: Option<OwnedSemaphorePermit>,
}

struct StopCleanupTask {
    channel: String,
    _reservation: StopCleanupReservation,
    task: JoinHandle<Result<(), crate::adapter::AdapterError>>,
}

fn cleanup_channel_index(channel: &str) -> Option<usize> {
    match channel {
        "telegram" => Some(0),
        "discord" => Some(1),
        "feishu" => Some(2),
        "dingtalk" => Some(3),
        "email" => Some(4),
        "qq" => Some(5),
        _ => None,
    }
}

impl StopCleanupRegistry {
    pub(super) fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            slots: Arc::new(Semaphore::new(MAX_PENDING_STOP_CLEANUPS)),
            channels: std::array::from_fn(|_| {
                Arc::new(Semaphore::new(MAX_PENDING_STOP_CLEANUPS_PER_CHANNEL))
            }),
            tasks: StdMutex::new(BTreeMap::new()),
            reaper_started: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Reserve a cleanup slot before spawning an adapter stop operation.
    /// Completed owners are purged first so a cancelled caller cannot make a
    /// slot permanently unavailable.  A full registry is a fail-fast result;
    /// callers must not start a stop future they cannot transfer to an owner.
    pub(super) async fn try_reserve(&self, channel: &str) -> Option<StopCleanupReservation> {
        let mut tasks = self
            .tasks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let finished = tasks
            .iter()
            .filter_map(|(id, entry)| entry.task.is_finished().then_some(*id))
            .collect::<Vec<_>>();
        for id in finished {
            tasks.remove(&id);
        }
        let channel_permit = match cleanup_channel_index(channel) {
            Some(index) => Some(self.channels[index].clone().try_acquire_owned().ok()?),
            None => None,
        };
        let global_permit = self.slots.clone().try_acquire_owned().ok()?;
        Some(StopCleanupReservation {
            _global: global_permit,
            _channel: channel_permit,
        })
    }

    /// Transfer a timed-out stop owner without an await point. A supervisor
    /// cancellation immediately after the bounded wait therefore cannot drop
    /// the JoinHandle and detach the adapter stop future.
    pub(super) fn adopt(
        self: &Arc<Self>,
        channel: String,
        task: JoinHandle<Result<(), crate::adapter::AdapterError>>,
        reservation: StopCleanupReservation,
    ) {
        let mut tasks = self
            .tasks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let finished = tasks
            .iter()
            .filter_map(|(id, entry)| entry.task.is_finished().then_some(*id))
            .collect::<Vec<_>>();
        for id in finished {
            tasks.remove(&id);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        tasks.insert(
            id,
            StopCleanupTask {
                channel,
                _reservation: reservation,
                task,
            },
        );
        debug_assert!(tasks.values().all(|entry| !entry.channel.is_empty()));
        drop(tasks);
        self.start_reaper();
    }

    pub(super) async fn pending(&self) -> usize {
        self.tasks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .filter(|entry| !entry.task.is_finished())
            .count()
    }

    fn start_reaper(self: &Arc<Self>) {
        if self
            .reaper_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        let registry = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(20)).await;
                let finished = {
                    let tasks = registry
                        .tasks
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    tasks
                        .iter()
                        .filter_map(|(id, entry)| entry.task.is_finished().then_some(*id))
                        .collect::<Vec<_>>()
                };
                for id in finished {
                    let task = registry
                        .tasks
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .remove(&id);
                    if let Some(task) = task {
                        let _ = task.task.await;
                    }
                }
                if registry
                    .tasks
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .is_empty()
                {
                    registry.reaper_started.store(false, Ordering::Release);
                    if registry
                        .tasks
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .is_empty()
                    {
                        break;
                    }
                    if registry
                        .reaper_started
                        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                        .is_err()
                    {
                        break;
                    }
                }
            }
        });
    }

    #[cfg(test)]
    async fn pending_for(&self, channel: &str) -> usize {
        self.tasks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .filter(|entry| entry.channel == channel && !entry.task.is_finished())
            .count()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SupervisorError {
    #[error("adapter supervisor worker could not be spawned: {0}")]
    WorkerSpawn(String),
}

/// Code-level listener restart policy; it is intentionally not user configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupervisorPolicy {
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub down_after_failures: u32,
    pub jitter_percent: u8,
}

impl Default for SupervisorPolicy {
    fn default() -> Self {
        Self {
            initial_backoff: Duration::from_millis(250),
            max_backoff: Duration::from_secs(30),
            down_after_failures: 3,
            jitter_percent: 20,
        }
    }
}

/// Owned supervisor for one independent adapter listener.
pub struct AdapterSupervisor {
    channel: String,
    cancel: CancellationToken,
    health_tx: watch::Sender<ChannelHealth>,
    health_rx: watch::Receiver<ChannelHealth>,
    consecutive_failures: Arc<AtomicU32>,
    task: JoinHandle<()>,
}

impl std::fmt::Debug for AdapterSupervisor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdapterSupervisor")
            .field("channel", &self.channel)
            .field(
                "consecutive_failures",
                &self.consecutive_failures.load(Ordering::Acquire),
            )
            .finish_non_exhaustive()
    }
}

impl AdapterSupervisor {
    pub fn spawn(adapter: Arc<dyn ChannelAdapter>, context: AdapterContext) -> Self {
        match Self::try_spawn(adapter, context) {
            Ok(supervisor) => supervisor,
            Err(error) => panic!("{error}"),
        }
    }

    /// Spawn a listener supervisor while reporting runtime allocation errors.
    pub fn try_spawn(
        adapter: Arc<dyn ChannelAdapter>,
        context: AdapterContext,
    ) -> Result<Self, SupervisorError> {
        Self::try_spawn_with_policy(adapter, context, SupervisorPolicy::default())
    }

    pub fn spawn_with_policy(
        adapter: Arc<dyn ChannelAdapter>,
        context: AdapterContext,
        policy: SupervisorPolicy,
    ) -> Self {
        match Self::try_spawn_with_policy(adapter, context, policy) {
            Ok(supervisor) => supervisor,
            Err(error) => panic!("{error}"),
        }
    }

    pub fn try_spawn_with_policy(
        adapter: Arc<dyn ChannelAdapter>,
        context: AdapterContext,
        policy: SupervisorPolicy,
    ) -> Result<Self, SupervisorError> {
        Self::try_spawn_with_policy_and_cleanup(
            adapter,
            context,
            policy,
            Arc::new(StopCleanupRegistry::new()),
        )
    }

    pub(super) fn try_spawn_with_policy_and_cleanup(
        adapter: Arc<dyn ChannelAdapter>,
        context: AdapterContext,
        policy: SupervisorPolicy,
        stop_cleanup: Arc<StopCleanupRegistry>,
    ) -> Result<Self, SupervisorError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|error| SupervisorError::WorkerSpawn(error.to_string()))?;
        let channel = adapter.name().to_string();
        let cancel = context.cancel.child_token();
        let initial_health = adapter.health();
        let (health_tx, health_rx) = watch::channel(initial_health);
        let consecutive_failures = Arc::new(AtomicU32::new(0));
        let task = runtime.spawn(run_supervisor(
            adapter,
            context,
            cancel.clone(),
            health_tx.clone(),
            consecutive_failures.clone(),
            policy,
            stop_cleanup.clone(),
        ));
        Ok(Self {
            channel,
            cancel,
            health_tx,
            health_rx,
            consecutive_failures,
            task,
        })
    }

    pub fn health(&self) -> ChannelHealth {
        self.health_rx.borrow().clone()
    }

    pub fn subscribe_health(&self) -> watch::Receiver<ChannelHealth> {
        self.health_rx.clone()
    }

    /// A successful explicit probe resets restart escalation.
    pub fn record_healthy_probe(&self, mut health: ChannelHealth) {
        if health.status == ChannelHealthStatus::Healthy {
            self.consecutive_failures.store(0, Ordering::Release);
            health.consecutive_failures = 0;
        }
        health.checked_at = Utc::now();
        self.health_tx.send_replace(health);
    }

    /// Interrupt listener reads/reconnect sleep and wait for clean ownership release.
    pub async fn shutdown(self) {
        self.cancel.cancel();
        let mut task = self.task;
        if tokio::time::timeout(SUPERVISOR_SHUTDOWN_TIMEOUT, &mut task)
            .await
            .is_err()
        {
            task.abort();
            let _ = task.await;
        }
    }
}

async fn run_supervisor(
    adapter: Arc<dyn ChannelAdapter>,
    context: AdapterContext,
    cancel: CancellationToken,
    health_tx: watch::Sender<ChannelHealth>,
    consecutive_failures: Arc<AtomicU32>,
    policy: SupervisorPolicy,
    stop_cleanup: Arc<StopCleanupRegistry>,
) {
    loop {
        if cancel.is_cancelled() {
            break;
        }
        let listener_cancel = cancel.child_token();
        let listener_context = context.with_cancel(listener_cancel.clone());
        let listener_adapter = adapter.clone();
        let mut listener =
            tokio::spawn(async move { listener_adapter.start(listener_context).await });

        let mut running_health = adapter.health();
        running_health.consecutive_failures = consecutive_failures.load(Ordering::Acquire);
        running_health.checked_at = Utc::now();
        health_tx.send_replace(running_health);

        let outcome = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                listener_cancel.cancel();
                let stop = stop_adapter(adapter.clone(), stop_cleanup.clone());
                let wait_listener = async {
                    if tokio::time::timeout(ADAPTER_STOP_TIMEOUT, &mut listener)
                        .await
                        .is_err()
                    {
                        listener.abort();
                        let _ = listener.await;
                    }
                };
                let _ = tokio::join!(stop, wait_listener);
                break;
            }
            outcome = &mut listener => outcome,
        };
        listener_cancel.cancel();
        let stop_outcome = stop_adapter(adapter.clone(), stop_cleanup.clone()).await;
        if matches!(stop_outcome, StopOutcome::Pending) {
            // A stop future that exceeded its local budget is now owned by the
            // registry.  Restarting the listener would permit another native
            // operation for the same adapter while the old one is still
            // unwinding, so this supervisor remains down until a fresh runtime
            // generation is installed.
            break;
        }

        let failure_count = consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1;
        let (panic, diagnosis) = match outcome {
            Ok(Ok(())) => (false, "adapter listener exited normally".to_string()),
            Ok(Err(error)) => (false, safe_listener_diagnosis(&error)),
            Err(error) => (error.is_panic(), "adapter listener task failed".to_string()),
        };
        let status = if panic || failure_count >= policy.down_after_failures.max(1) {
            ChannelHealthStatus::Down
        } else {
            ChannelHealthStatus::Degraded
        };
        health_tx.send_replace(ChannelHealth {
            status,
            diagnosis: Some(diagnosis.clone()),
            consecutive_failures: failure_count,
            checked_at: Utc::now(),
        });
        if should_log_failure(failure_count) {
            tracing::warn!(
                channel = %adapter.name(),
                failure_count,
                ?status,
                diagnosis = %diagnosis,
                "channel adapter listener terminated; scheduling bounded restart"
            );
        }

        let delay = backoff_delay(policy, failure_count, adapter.name().as_str());
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(delay) => {}
        }
    }

    let mut health = adapter.health();
    health.status = ChannelHealthStatus::Down;
    health.diagnosis = Some("adapter supervisor stopped".to_string());
    health.consecutive_failures = consecutive_failures.load(Ordering::Acquire);
    health.checked_at = Utc::now();
    health_tx.send_replace(health);
}

enum StopOutcome {
    Completed,
    Pending,
}

async fn stop_adapter(
    adapter: Arc<dyn ChannelAdapter>,
    stop_cleanup: Arc<StopCleanupRegistry>,
) -> StopOutcome {
    stop_adapter_with_budget(adapter, stop_cleanup, ADAPTER_STOP_TIMEOUT).await
}

async fn stop_adapter_with_budget(
    adapter: Arc<dyn ChannelAdapter>,
    stop_cleanup: Arc<StopCleanupRegistry>,
    budget: Duration,
) -> StopOutcome {
    let channel = adapter.name().to_string();
    let Some(reservation) = stop_cleanup.try_reserve(&channel).await else {
        tracing::warn!(
            channel = %channel,
            "channel adapter stop cleanup registry is full; refusing another native stop"
        );
        return StopOutcome::Pending;
    };
    let (result_sender, result_receiver) = oneshot::channel();
    let stop = tokio::spawn(async move {
        let result = adapter.stop().await;
        let _ = result_sender.send(result.clone());
        result
    });
    // Adopt before waiting on the bounded observation. If the supervisor or
    // its caller is cancelled during the adapter stop budget, the registry
    // still owns the JoinHandle and reservation instead of detaching native
    // cleanup work.
    stop_cleanup.adopt(channel.clone(), stop, reservation);
    match tokio::time::timeout(budget, result_receiver).await {
        Ok(Ok(Ok(()))) => StopOutcome::Completed,
        Ok(Ok(Err(error))) => {
            tracing::warn!(channel = %channel, code = %safe_adapter_code(&error), "channel adapter stop returned an error");
            StopOutcome::Completed
        }
        Ok(Err(error)) => {
            tracing::warn!(channel = %channel, error = %error, "channel adapter stop task failed");
            StopOutcome::Completed
        }
        Err(_) => {
            tracing::warn!(
                channel = %channel,
                "channel adapter stop exceeded its bounded wait; cleanup owner retained"
            );
            StopOutcome::Pending
        }
    }
}

fn safe_adapter_code(error: &crate::adapter::AdapterError) -> String {
    let code = error.code();
    if !code.is_empty()
        && code.len() <= 64
        && code.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
    {
        code.to_string()
    } else {
        "adapter_error".to_string()
    }
}

fn safe_listener_diagnosis(error: &crate::adapter::AdapterError) -> String {
    format!("adapter listener failed ({})", safe_adapter_code(error))
}

fn should_log_failure(failure_count: u32) -> bool {
    failure_count <= 3 || failure_count.is_power_of_two()
}

fn backoff_delay(policy: SupervisorPolicy, failure_count: u32, channel: &str) -> Duration {
    let exponent = failure_count.saturating_sub(1).min(16);
    let base = policy
        .initial_backoff
        .saturating_mul(1u32 << exponent)
        .min(policy.max_backoff);
    let jitter = policy.jitter_percent.min(100) as i64;
    if jitter == 0 || base.is_zero() {
        return base;
    }
    let hash = channel
        .bytes()
        .fold(u64::from(failure_count), |value, byte| {
            value
                .wrapping_mul(1_099_511_628_211)
                .wrapping_add(u64::from(byte))
        });
    let span = jitter * 2 + 1;
    let offset_percent = (hash % span as u64) as i64 - jitter;
    let millis = base.as_millis().min(i64::MAX as u128) as i64;
    let adjusted = millis.saturating_add(millis.saturating_mul(offset_percent) / 100);
    Duration::from_millis(adjusted.max(0) as u64).min(policy.max_backoff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AdapterError, ChannelAdapter};
    use agent_diva_core::channel::{
        ChannelCapabilities, ChannelCommand, ChannelId, DeliveryReceipt,
    };
    use async_trait::async_trait;
    use tokio::sync::Notify;

    struct BlockingStopAdapter {
        id: ChannelId,
        release: Arc<Notify>,
        started: Arc<std::sync::atomic::AtomicBool>,
    }

    #[async_trait]
    impl ChannelAdapter for BlockingStopAdapter {
        fn name(&self) -> ChannelId {
            self.id.clone()
        }

        fn capabilities(&self) -> ChannelCapabilities {
            ChannelCapabilities::new(std::iter::empty())
        }

        async fn start(&self, context: AdapterContext) -> Result<(), AdapterError> {
            context.cancel.cancelled().await;
            Ok(())
        }

        async fn execute(&self, _command: ChannelCommand) -> Result<DeliveryReceipt, AdapterError> {
            Err(AdapterError::Stopped)
        }

        fn health(&self) -> ChannelHealth {
            ChannelHealth::new(ChannelHealthStatus::Healthy)
        }

        async fn stop(&self) -> Result<(), AdapterError> {
            self.started.store(true, Ordering::Release);
            self.release.notified().await;
            Ok(())
        }
    }

    #[tokio::test]
    async fn timed_out_stop_remains_owned_until_reaper_observes_completion() {
        let cleanup = Arc::new(StopCleanupRegistry::new());
        let release = Arc::new(Notify::new());
        let adapter = Arc::new(BlockingStopAdapter {
            id: ChannelId::new("test").expect("channel id"),
            release: release.clone(),
            started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        });
        assert!(matches!(
            stop_adapter_with_budget(adapter, cleanup.clone(), Duration::from_millis(5)).await,
            StopOutcome::Pending
        ));
        assert_eq!(cleanup.pending_for("test").await, 1);
        release.notify_one();
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if cleanup.pending().await == 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("stop cleanup reaper");
    }

    #[tokio::test]
    async fn cancelled_stop_waiter_keeps_registry_owner() {
        let cleanup = Arc::new(StopCleanupRegistry::new());
        let release = Arc::new(Notify::new());
        let started = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let adapter = Arc::new(BlockingStopAdapter {
            id: ChannelId::new("test").expect("channel id"),
            release: release.clone(),
            started: started.clone(),
        });
        let caller_cleanup = cleanup.clone();
        let caller = tokio::spawn(async move {
            stop_adapter_with_budget(adapter, caller_cleanup, Duration::from_secs(30)).await
        });

        tokio::time::timeout(Duration::from_secs(1), async {
            while !started.load(Ordering::Acquire) {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("stop owner started before caller cancellation");
        caller.abort();
        match caller.await {
            Ok(_) => panic!("stop waiter should have been cancelled"),
            Err(error) => assert!(error.is_cancelled()),
        }
        assert_eq!(cleanup.pending_for("test").await, 1);

        release.notify_one();
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if cleanup.pending().await == 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("registry owner drained after caller cancellation");
    }

    #[tokio::test]
    async fn full_stop_cleanup_registry_fails_fast_without_starting_native_stop() {
        let cleanup = Arc::new(StopCleanupRegistry::new());
        let mut releases = Vec::new();
        for index in 0..MAX_PENDING_STOP_CLEANUPS {
            let (release_tx, release_rx) = tokio::sync::oneshot::channel();
            let task = tokio::spawn(async move {
                let _ = release_rx.await;
                Ok::<(), AdapterError>(())
            });
            let channel = format!("filled-{index}");
            let reservation = cleanup
                .try_reserve(&channel)
                .await
                .expect("cleanup capacity");
            cleanup.adopt(channel, task, reservation);
            releases.push(release_tx);
        }

        let started = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let adapter = Arc::new(BlockingStopAdapter {
            id: ChannelId::new("test").expect("channel id"),
            release: Arc::new(Notify::new()),
            started: started.clone(),
        });
        assert!(matches!(
            tokio::time::timeout(
                Duration::from_millis(100),
                stop_adapter_with_budget(adapter, cleanup.clone(), Duration::from_secs(5)),
            )
            .await
            .expect("full cleanup registry must fail fast"),
            StopOutcome::Pending
        ));
        assert!(!started.load(Ordering::Acquire));

        for release in releases {
            release.send(()).expect("release cleanup owner");
        }
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if cleanup.pending().await == 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("stop cleanup reaper");
    }

    #[tokio::test]
    async fn duplicate_channel_stop_cleanup_reservation_fails_fast() {
        let cleanup = StopCleanupRegistry::new();
        let reservation = cleanup
            .try_reserve("email")
            .await
            .expect("first channel reservation");
        assert!(cleanup.try_reserve("email").await.is_none());
        drop(reservation);
        assert!(cleanup.try_reserve("email").await.is_some());
    }

    #[test]
    fn backoff_is_bounded_and_deterministic() {
        let policy = SupervisorPolicy::default();
        assert_eq!(
            backoff_delay(
                SupervisorPolicy {
                    jitter_percent: 0,
                    ..policy
                },
                1,
                "test"
            ),
            Duration::from_millis(250)
        );
        assert!(backoff_delay(policy, 99, "test") <= Duration::from_secs(30));
        assert_eq!(
            backoff_delay(policy, 4, "test"),
            backoff_delay(policy, 4, "test")
        );
    }
}

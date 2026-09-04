use crate::adapter::{AdapterContext, ChannelAdapter};
use agent_diva_core::channel::{ChannelHealth, ChannelHealthStatus};
use chrono::Utc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const ADAPTER_STOP_TIMEOUT: Duration = Duration::from_secs(5);
const SUPERVISOR_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(12);

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
                stop_adapter(adapter.as_ref()).await;
                if tokio::time::timeout(ADAPTER_STOP_TIMEOUT, &mut listener).await.is_err() {
                    listener.abort();
                    let _ = listener.await;
                }
                break;
            }
            outcome = &mut listener => outcome,
        };
        listener_cancel.cancel();
        stop_adapter(adapter.as_ref()).await;

        let failure_count = consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1;
        let (panic, diagnosis) = match outcome {
            Ok(Ok(())) => (false, "adapter listener exited normally".to_string()),
            Ok(Err(error)) => (false, error.to_string()),
            Err(error) => (
                error.is_panic(),
                format!("adapter listener task failed: {error}"),
            ),
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

async fn stop_adapter(adapter: &dyn ChannelAdapter) {
    match tokio::time::timeout(ADAPTER_STOP_TIMEOUT, adapter.stop()).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => tracing::warn!(
            channel = %adapter.name(),
            error = %error,
            "channel adapter stop returned an error"
        ),
        Err(_) => tracing::warn!(
            channel = %adapter.name(),
            "channel adapter stop timed out"
        ),
    }
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

//! Lost Reaper — periodically scans for stale running tasks and marks them lost

use super::store::RunStore;
use chrono::Utc;
use tokio::time::{interval, Duration};
use tracing::{debug, info, warn};

/// Periodic scanner that marks stale supervised runs as lost.
///
/// The reaper runs on a fixed interval, calling `store.mark_lost()` with
/// a cutoff computed as `now - timeout_secs`.
///
/// # Example
///
/// ```rust,ignore
/// let reaper = Reaper::new(store, 30, 60);
/// reaper.run().await;
/// ```
pub struct Reaper {
    store: RunStore,
    interval_secs: u64,
    timeout_secs: u64,
}

impl Reaper {
    /// Create a new reaper.
    ///
    /// # Arguments
    ///
    /// * `store` — the `RunStore` to scan
    /// * `interval_secs` — how often to scan (seconds)
    /// * `timeout_secs` — heartbeat age threshold (seconds)
    pub fn new(store: RunStore, interval_secs: u64, timeout_secs: u64) -> Self {
        Self {
            store,
            interval_secs,
            timeout_secs,
        }
    }

    /// Run the reaper loop until the cancellation token fires.
    ///
    /// This is a background task; spawn it with `tokio::spawn`.
    pub async fn run(&self, cancel: tokio_util::sync::CancellationToken) {
        let mut ticker = interval(Duration::from_secs(self.interval_secs));
        info!(
            interval = self.interval_secs,
            timeout = self.timeout_secs,
            "reaper started"
        );

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let cutoff = Utc::now() - chrono::Duration::seconds(self.timeout_secs as i64);
                    match self.store.mark_lost(cutoff).await {
                        Ok(reaped) => {
                            if !reaped.is_empty() {
                                warn!(count = reaped.len(), "reaped stale runs");
                                for record in &reaped {
                                    debug!(run_id = %record.id, "marked lost");
                                }
                            } else {
                                debug!("no stale runs found");
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "mark_lost failed");
                        }
                    }
                }
                _ = cancel.cancelled() => {
                    info!("reaper shutting down");
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, RunStore) {
        let dir = TempDir::new().expect("tempdir");
        let store = RunStore::new(dir.path()).await.expect("store creation");
        (dir, store)
    }

    #[tokio::test]
    async fn test_reaper_reaps_stale_runs() {
        let (_dir, store) = setup().await;
        use super::super::types::{RunStatus, SupervisedRunSpec};

        let spec = SupervisedRunSpec::from_spec("stale reap test");
        let _created = store.create(&spec).await.expect("create");
        let claimed = store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim")
            .expect("claimed");

        // Manually set heartbeat to 120s ago
        let stale_time = Utc::now() - chrono::Duration::seconds(120);
        sqlx::query::<sqlx::Sqlite>("UPDATE supervised_runs SET heartbeat_at = ?1 WHERE id = ?2")
            .bind(stale_time.to_rfc3339())
            .bind(&claimed.id)
            .execute(&store.pool)
            .await
            .expect("update heartbeat");

        // Run one reaper tick directly
        let cutoff = Utc::now() - chrono::Duration::seconds(60);
        let reaped = store.mark_lost(cutoff).await.expect("mark_lost");
        assert_eq!(reaped.len(), 1);
        assert_eq!(reaped[0].id, claimed.id);

        let reloaded = store
            .get_record(&claimed.id)
            .await
            .expect("get")
            .expect("record");
        assert_eq!(reloaded.status, RunStatus::Lost);
    }

    #[tokio::test]
    async fn test_reaper_leaves_fresh_runs() {
        let (_dir, store) = setup().await;
        use super::super::types::SupervisedRunSpec;

        let spec = SupervisedRunSpec::from_spec("fresh reap test");
        store.create(&spec).await.expect("create");
        store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim")
            .expect("claimed");

        // Recent heartbeat — should NOT be reaped
        let cutoff = Utc::now() - chrono::Duration::seconds(60);
        let reaped = store.mark_lost(cutoff).await.expect("mark_lost");
        assert!(reaped.is_empty());
    }
}

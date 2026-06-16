use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct AutoDreamMetrics {
    runs_total: AtomicU64,
    failures_total: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoDreamMetricsSnapshot {
    pub autodream_runs_total: u64,
    pub autodream_failures_total: u64,
}

impl AutoDreamMetrics {
    pub const fn new() -> Self {
        Self {
            runs_total: AtomicU64::new(0),
            failures_total: AtomicU64::new(0),
        }
    }

    pub fn record_run(&self) {
        self.runs_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.failures_total.fetch_add(1, Ordering::Relaxed);
    }

    #[must_use]
    pub fn snapshot(&self) -> AutoDreamMetricsSnapshot {
        AutoDreamMetricsSnapshot {
            autodream_runs_total: self.runs_total.load(Ordering::Relaxed),
            autodream_failures_total: self.failures_total.load(Ordering::Relaxed),
        }
    }

    #[doc(hidden)]
    pub fn reset_for_test(&self) {
        self.runs_total.store(0, Ordering::Relaxed);
        self.failures_total.store(0, Ordering::Relaxed);
    }
}

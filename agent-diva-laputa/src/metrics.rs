use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct LaputaMetrics {
    writes_total: AtomicU64,
    write_errors_total: AtomicU64,
    rollbacks_total: AtomicU64,
    governance_failures_total: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaputaMetricsSnapshot {
    pub laputa_writes_total: u64,
    pub laputa_write_errors_total: u64,
    pub laputa_rollbacks_total: u64,
    pub laputa_governance_failures_total: u64,
}

impl LaputaMetrics {
    pub const fn new() -> Self {
        Self {
            writes_total: AtomicU64::new(0),
            write_errors_total: AtomicU64::new(0),
            rollbacks_total: AtomicU64::new(0),
            governance_failures_total: AtomicU64::new(0),
        }
    }

    pub fn record_write(&self) {
        self.writes_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_write_error(&self) {
        self.write_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_rollback(&self) {
        self.rollbacks_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_governance_failure(&self) {
        self.governance_failures_total
            .fetch_add(1, Ordering::Relaxed);
    }

    #[must_use]
    pub fn snapshot(&self) -> LaputaMetricsSnapshot {
        LaputaMetricsSnapshot {
            laputa_writes_total: self.writes_total.load(Ordering::Relaxed),
            laputa_write_errors_total: self.write_errors_total.load(Ordering::Relaxed),
            laputa_rollbacks_total: self.rollbacks_total.load(Ordering::Relaxed),
            laputa_governance_failures_total: self
                .governance_failures_total
                .load(Ordering::Relaxed),
        }
    }

    #[doc(hidden)]
    pub fn reset_for_test(&self) {
        self.writes_total.store(0, Ordering::Relaxed);
        self.write_errors_total.store(0, Ordering::Relaxed);
        self.rollbacks_total.store(0, Ordering::Relaxed);
        self.governance_failures_total.store(0, Ordering::Relaxed);
    }
}

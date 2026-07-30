use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

#[derive(Debug, Default)]
pub struct LaputaMetrics {
    writes_total: AtomicU64,
    write_errors_total: AtomicU64,
    rollbacks_total: AtomicU64,
    governance_failures_total: AtomicU64,
    governance_decisions_total: AtomicU64,
    governance_denials_total: AtomicU64,
    stale_receipts_total: AtomicU64,
    decision_latency_ms_total: AtomicU64,
    decision_latency_ms_max: AtomicU64,
    human_wait_ms_total: AtomicU64,
    human_wait_ms_max: AtomicU64,
    typed_applies_total: AtomicU64,
    typed_rollbacks_total: AtomicU64,
    correlation_events_total: AtomicU64,
    correlation_failures_total: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LaputaMetricsSnapshot {
    pub laputa_writes_total: u64,
    pub laputa_write_errors_total: u64,
    pub laputa_rollbacks_total: u64,
    pub laputa_governance_failures_total: u64,
    pub laputa_governance_decisions_total: u64,
    pub laputa_governance_denials_total: u64,
    pub laputa_stale_receipts_total: u64,
    pub laputa_decision_latency_ms_total: u64,
    pub laputa_decision_latency_ms_max: u64,
    pub laputa_human_wait_ms_total: u64,
    pub laputa_human_wait_ms_max: u64,
    pub laputa_typed_applies_total: u64,
    pub laputa_typed_rollbacks_total: u64,
    pub laputa_correlation_events_total: u64,
    pub laputa_correlation_failures_total: u64,
}

impl LaputaMetrics {
    pub const fn new() -> Self {
        Self {
            writes_total: AtomicU64::new(0),
            write_errors_total: AtomicU64::new(0),
            rollbacks_total: AtomicU64::new(0),
            governance_failures_total: AtomicU64::new(0),
            governance_decisions_total: AtomicU64::new(0),
            governance_denials_total: AtomicU64::new(0),
            stale_receipts_total: AtomicU64::new(0),
            decision_latency_ms_total: AtomicU64::new(0),
            decision_latency_ms_max: AtomicU64::new(0),
            human_wait_ms_total: AtomicU64::new(0),
            human_wait_ms_max: AtomicU64::new(0),
            typed_applies_total: AtomicU64::new(0),
            typed_rollbacks_total: AtomicU64::new(0),
            correlation_events_total: AtomicU64::new(0),
            correlation_failures_total: AtomicU64::new(0),
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

    pub fn record_governance_decision(
        &self,
        decision_latency_ms: u64,
        human_wait_ms: u64,
        denied: bool,
    ) {
        self.governance_decisions_total
            .fetch_add(1, Ordering::Relaxed);
        if denied {
            self.governance_denials_total
                .fetch_add(1, Ordering::Relaxed);
        }
        self.decision_latency_ms_total
            .fetch_add(decision_latency_ms, Ordering::Relaxed);
        self.decision_latency_ms_max
            .fetch_max(decision_latency_ms, Ordering::Relaxed);
        self.human_wait_ms_total
            .fetch_add(human_wait_ms, Ordering::Relaxed);
        self.human_wait_ms_max
            .fetch_max(human_wait_ms, Ordering::Relaxed);
    }

    pub fn record_stale_receipt(&self) {
        self.stale_receipts_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_typed_apply(&self) {
        self.typed_applies_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_typed_rollback(&self) {
        self.typed_rollbacks_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_correlation(&self, complete: bool) {
        self.correlation_events_total
            .fetch_add(1, Ordering::Relaxed);
        if !complete {
            self.correlation_failures_total
                .fetch_add(1, Ordering::Relaxed);
        }
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
            laputa_governance_decisions_total: self
                .governance_decisions_total
                .load(Ordering::Relaxed),
            laputa_governance_denials_total: self.governance_denials_total.load(Ordering::Relaxed),
            laputa_stale_receipts_total: self.stale_receipts_total.load(Ordering::Relaxed),
            laputa_decision_latency_ms_total: self
                .decision_latency_ms_total
                .load(Ordering::Relaxed),
            laputa_decision_latency_ms_max: self.decision_latency_ms_max.load(Ordering::Relaxed),
            laputa_human_wait_ms_total: self.human_wait_ms_total.load(Ordering::Relaxed),
            laputa_human_wait_ms_max: self.human_wait_ms_max.load(Ordering::Relaxed),
            laputa_typed_applies_total: self.typed_applies_total.load(Ordering::Relaxed),
            laputa_typed_rollbacks_total: self.typed_rollbacks_total.load(Ordering::Relaxed),
            laputa_correlation_events_total: self.correlation_events_total.load(Ordering::Relaxed),
            laputa_correlation_failures_total: self
                .correlation_failures_total
                .load(Ordering::Relaxed),
        }
    }

    #[doc(hidden)]
    pub fn reset_for_test(&self) {
        self.writes_total.store(0, Ordering::Relaxed);
        self.write_errors_total.store(0, Ordering::Relaxed);
        self.rollbacks_total.store(0, Ordering::Relaxed);
        self.governance_failures_total.store(0, Ordering::Relaxed);
        self.governance_decisions_total.store(0, Ordering::Relaxed);
        self.governance_denials_total.store(0, Ordering::Relaxed);
        self.stale_receipts_total.store(0, Ordering::Relaxed);
        self.decision_latency_ms_total.store(0, Ordering::Relaxed);
        self.decision_latency_ms_max.store(0, Ordering::Relaxed);
        self.human_wait_ms_total.store(0, Ordering::Relaxed);
        self.human_wait_ms_max.store(0, Ordering::Relaxed);
        self.typed_applies_total.store(0, Ordering::Relaxed);
        self.typed_rollbacks_total.store(0, Ordering::Relaxed);
        self.correlation_events_total.store(0, Ordering::Relaxed);
        self.correlation_failures_total.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn governance_snapshot_is_payload_free_and_tracks_latency_maxima() {
        let metrics = LaputaMetrics::new();
        metrics.record_governance_decision(12, 900, false);
        metrics.record_governance_decision(7, 1_200, true);
        metrics.record_stale_receipt();
        metrics.record_typed_apply();
        metrics.record_typed_rollback();
        metrics.record_correlation(true);
        metrics.record_correlation(false);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.laputa_governance_decisions_total, 2);
        assert_eq!(snapshot.laputa_governance_denials_total, 1);
        assert_eq!(snapshot.laputa_stale_receipts_total, 1);
        assert_eq!(snapshot.laputa_decision_latency_ms_total, 19);
        assert_eq!(snapshot.laputa_decision_latency_ms_max, 12);
        assert_eq!(snapshot.laputa_human_wait_ms_total, 2_100);
        assert_eq!(snapshot.laputa_human_wait_ms_max, 1_200);
        assert_eq!(snapshot.laputa_typed_applies_total, 1);
        assert_eq!(snapshot.laputa_typed_rollbacks_total, 1);
        assert_eq!(snapshot.laputa_correlation_events_total, 2);
        assert_eq!(snapshot.laputa_correlation_failures_total, 1);

        let json = serde_json::to_value(snapshot).expect("serialize metrics");
        let object = json.as_object().expect("metrics object");
        assert!(object
            .keys()
            .all(|key| { key.ends_with("_total") || key.ends_with("_max") }));
    }
}

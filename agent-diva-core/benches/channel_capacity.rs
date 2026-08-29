//! C1 capacity characterization for the future Fabric lanes.
//!
//! This benchmark models bounded admission only.  It does not replace the
//! C2 runtime and intentionally does not modify the legacy MessageBus.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tokio::sync::mpsc;

use agent_diva_core::channel::capacity;

fn bench_bounded_admission(c: &mut Criterion) {
    let mut group = c.benchmark_group("channel_capacity");
    for (lane, selected_capacity) in [
        ("control", capacity::CONTROL),
        ("ingress", capacity::INGRESS),
        ("durable_event", capacity::DURABLE_EVENT),
        ("transient_event", capacity::TRANSIENT_EVENT),
        ("adapter_egress", capacity::ADAPTER_EGRESS),
    ] {
        group.bench_with_input(
            BenchmarkId::new(lane, selected_capacity),
            &selected_capacity,
            |b, &queue_capacity| {
                b.iter(|| {
                    let (tx, mut rx) = mpsc::channel::<usize>(queue_capacity);
                    for value in 0..queue_capacity {
                        let _ = tx.try_send(value);
                    }
                    let mut drained = 0usize;
                    while rx.try_recv().is_ok() {
                        drained += 1;
                    }
                    black_box(drained)
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_bounded_admission);
criterion_main!(benches);

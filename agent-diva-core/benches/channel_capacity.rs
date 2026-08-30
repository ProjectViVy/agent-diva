//! Capacity characterization for the bounded Super Channel Fabric lanes.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use agent_diva_core::channel::{
    capacity, ChannelAddress, ChannelDirection, ChannelEnvelopeV1, ChannelOrigin, ChannelPayloadV1,
    Correlation, FabricKernel, TransientKey, TypingState,
};

fn envelope(sequence: usize) -> ChannelEnvelopeV1 {
    let mut correlation = Correlation::new(format!("bench-{sequence}"));
    correlation.request_id = Some(format!("request-{sequence}"));
    correlation.sequence = Some(sequence as u64);
    ChannelEnvelopeV1::new(
        ChannelDirection::Ingress,
        ChannelAddress::new("bench", format!("chat-{sequence}")),
        correlation,
        ChannelOrigin::Runtime,
        ChannelPayloadV1::Typing {
            state: TypingState::Started,
        },
    )
}

fn bench_bounded_admission(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("benchmark runtime");
    let mut group = c.benchmark_group("channel_capacity");
    for (lane, selected_capacity) in [
        ("control", capacity::CONTROL),
        ("ingress", capacity::INGRESS),
        ("durable_event", capacity::DURABLE_EVENT),
        ("transient_event", capacity::TRANSIENT_EVENT),
    ] {
        group.bench_with_input(
            BenchmarkId::new(lane, selected_capacity),
            &selected_capacity,
            |b, &queue_capacity| {
                b.iter(|| {
                    runtime.block_on(async {
                        let (handle, mut consumer) = FabricKernel::new().into_parts();
                        let cancel = CancellationToken::new();
                        for sequence in 0..queue_capacity {
                            let event = envelope(sequence);
                            match lane {
                                "control" => {
                                    handle.send_control(event, &cancel).await.unwrap();
                                }
                                "ingress" => {
                                    handle
                                        .admit_ingress(event, Duration::from_secs(1), &cancel)
                                        .await
                                        .unwrap();
                                }
                                "durable_event" => {
                                    handle.publish_durable(event, &cancel).await.unwrap();
                                }
                                "transient_event" => {
                                    let key = TransientKey::from_envelope(&event, "delta");
                                    handle.publish_transient(key, event).await.unwrap();
                                }
                                _ => unreachable!(),
                            }
                        }
                        let mut drained = 0usize;
                        for _ in 0..queue_capacity {
                            let item = match lane {
                                "control" | "ingress" => consumer.recv_ingress().await.is_some(),
                                "durable_event" => consumer.recv_durable().await.is_some(),
                                "transient_event" => consumer.recv_transient().await.is_some(),
                                _ => unreachable!(),
                            };
                            drained += usize::from(item);
                        }
                        black_box(drained)
                    })
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_bounded_admission);
criterion_main!(benches);

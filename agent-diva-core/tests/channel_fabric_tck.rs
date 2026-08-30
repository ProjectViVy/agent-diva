use agent_diva_core::channel::{
    capacity, ChannelAddress, ChannelDirection, ChannelEnvelopeV1, ChannelOrigin, ChannelPayloadV1,
    Correlation, FabricAdmissionError, FabricIngressItem, FabricIngressScheduler,
    FabricIngressSink, FabricKernel, FabricLane, FabricTransientItem, TransientKey,
    TransientPublishOutcome, TypingState,
};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, Semaphore};
use tokio_util::sync::CancellationToken;

fn envelope(session: &str, request: &str, sequence: u64) -> ChannelEnvelopeV1 {
    let mut correlation = Correlation::new(session);
    correlation.request_id = Some(request.to_string());
    correlation.sequence = Some(sequence);
    ChannelEnvelopeV1::new(
        ChannelDirection::Ingress,
        ChannelAddress::new("fabric-tck", session),
        correlation,
        ChannelOrigin::ExternalUser,
        ChannelPayloadV1::Typing {
            state: TypingState::Started,
        },
    )
}

#[tokio::test(start_paused = true)]
async fn ingress_is_bounded_and_returns_busy_without_consumption() {
    let (handle, _consumer) = FabricKernel::new().into_parts();
    let cancel = CancellationToken::new();
    for sequence in 0..capacity::INGRESS {
        handle
            .admit_ingress(
                envelope("full", "request", sequence as u64),
                Duration::from_secs(1),
                &cancel,
            )
            .await
            .unwrap();
    }

    let waiting_handle = handle.clone();
    let waiting_cancel = cancel.clone();
    let waiting = tokio::spawn(async move {
        waiting_handle
            .admit_ingress(
                envelope("full", "overflow", 999),
                Duration::from_secs(5),
                &waiting_cancel,
            )
            .await
    });
    tokio::task::yield_now().await;
    assert!(!waiting.is_finished());
    tokio::time::advance(Duration::from_secs(5)).await;

    assert!(matches!(
        waiting.await.unwrap(),
        Err(FabricAdmissionError::Busy {
            lane: FabricLane::Ingress,
            ..
        })
    ));
}

#[tokio::test]
async fn control_has_priority_over_ready_ingress_and_transient_saturation() {
    let (handle, mut consumer) = FabricKernel::new().into_parts();
    let cancel = CancellationToken::new();
    handle
        .admit_ingress(
            envelope("session", "ingress", 1),
            Duration::from_secs(1),
            &cancel,
        )
        .await
        .unwrap();
    for index in 0..capacity::TRANSIENT_EVENT {
        let event = envelope(&format!("transient-{index}"), "delta", 1);
        handle
            .publish_transient(TransientKey::from_envelope(&event, "delta"), event)
            .await
            .unwrap();
    }
    let control = envelope("session", "cancel", 2);
    handle.send_control(control.clone(), &cancel).await.unwrap();

    assert_eq!(
        consumer.recv_ingress().await,
        Some(FabricIngressItem::Control(control))
    );
    assert!(matches!(
        consumer.recv_ingress().await,
        Some(FabricIngressItem::Envelope(_))
    ));
}

#[tokio::test]
async fn durable_lane_applies_backpressure_instead_of_dropping() {
    let (handle, mut consumer) = FabricKernel::new().into_parts();
    let cancel = CancellationToken::new();
    for sequence in 0..capacity::DURABLE_EVENT {
        handle
            .publish_durable(envelope("durable", "request", sequence as u64), &cancel)
            .await
            .unwrap();
    }

    let waiting_handle = handle.clone();
    let waiting_cancel = cancel.clone();
    let waiting = tokio::spawn(async move {
        waiting_handle
            .publish_durable(envelope("durable", "overflow", 999), &waiting_cancel)
            .await
    });
    tokio::task::yield_now().await;
    assert!(!waiting.is_finished());
    assert!(consumer.recv_durable().await.is_some());
    tokio::time::timeout(Duration::from_secs(1), waiting)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn transient_lane_coalesces_and_reports_capacity_gap() {
    let (handle, consumer) = FabricKernel::new().into_parts();
    let first = envelope("same", "request", 1);
    let latest = envelope("same", "request", 2);
    let key = TransientKey::from_envelope(&first, "delta");
    assert_eq!(
        handle.publish_transient(key.clone(), first).await.unwrap(),
        TransientPublishOutcome::Queued
    );
    assert_eq!(
        handle.publish_transient(key, latest.clone()).await.unwrap(),
        TransientPublishOutcome::Coalesced
    );
    assert_eq!(
        consumer.recv_transient().await,
        Some(FabricTransientItem::Envelope(Box::new(latest)))
    );

    for index in 0..capacity::TRANSIENT_EVENT {
        let event = envelope(&format!("session-{index}"), "delta", 1);
        handle
            .publish_transient(TransientKey::from_envelope(&event, "delta"), event)
            .await
            .unwrap();
    }
    let overflow = envelope("overflow", "delta", 1);
    assert_eq!(
        handle
            .publish_transient(TransientKey::from_envelope(&overflow, "delta"), overflow,)
            .await
            .unwrap(),
        TransientPublishOutcome::GapScheduled
    );

    let mut gap = None;
    for _ in 0..=capacity::TRANSIENT_EVENT {
        if let Some(FabricTransientItem::Gap(candidate)) = consumer.recv_transient().await {
            gap = Some(candidate);
            break;
        }
    }
    let gap = gap.expect("transient overflow must emit a gap");
    assert_eq!(gap.session_key, "session-0");
    assert_eq!(gap.reason, "transient_capacity_eviction");
}

#[tokio::test]
async fn request_fence_drops_late_transient_delta() {
    let (handle, consumer) = FabricKernel::new().into_parts();
    handle.fence_request("session", "request", 4).await;
    let late = envelope("session", "request", 5);
    assert_eq!(
        handle
            .publish_transient(TransientKey::from_envelope(&late, "delta"), late)
            .await
            .unwrap(),
        TransientPublishOutcome::Fenced
    );

    let terminal = envelope("session", "request", 4);
    assert_eq!(
        handle
            .publish_transient(
                TransientKey::from_envelope(&terminal, "terminal"),
                terminal.clone(),
            )
            .await
            .unwrap(),
        TransientPublishOutcome::Queued
    );
    assert_eq!(
        consumer.recv_transient().await,
        Some(FabricTransientItem::Envelope(Box::new(terminal)))
    );
}

#[tokio::test]
async fn shutdown_rejects_new_work_and_drains_accepted_items() {
    let (handle, mut consumer) = FabricKernel::new().into_parts();
    let cancel = CancellationToken::new();
    let ingress = envelope("session", "ingress", 1);
    let durable = envelope("session", "durable", 2);
    let transient = envelope("session", "transient", 3);
    handle
        .admit_ingress(ingress.clone(), Duration::from_secs(1), &cancel)
        .await
        .unwrap();
    handle
        .publish_durable(durable.clone(), &cancel)
        .await
        .unwrap();
    handle
        .publish_transient(
            TransientKey::from_envelope(&transient, "delta"),
            transient.clone(),
        )
        .await
        .unwrap();

    consumer.begin_shutdown().await;
    assert!(handle.is_closed());
    assert!(matches!(
        handle
            .send_control(envelope("new", "new", 1), &cancel)
            .await,
        Err(FabricAdmissionError::Closed { .. })
    ));
    assert_eq!(
        consumer.recv_ingress().await,
        Some(FabricIngressItem::Envelope(ingress))
    );
    assert_eq!(consumer.recv_ingress().await, None);
    assert_eq!(consumer.recv_durable().await, Some(durable));
    assert_eq!(consumer.recv_durable().await, None);
    assert_eq!(
        consumer.recv_transient().await,
        Some(FabricTransientItem::Envelope(Box::new(transient)))
    );
    assert_eq!(consumer.recv_transient().await, None);
}

struct OrderedSink {
    started: mpsc::Sender<(String, u64)>,
    gates: Arc<Mutex<HashMap<String, Arc<Semaphore>>>>,
}

#[async_trait]
impl FabricIngressSink for OrderedSink {
    async fn submit(
        &self,
        envelope: ChannelEnvelopeV1,
    ) -> Result<(), agent_diva_core::channel::FabricDispatchError> {
        let session = envelope.correlation.session_key.clone();
        let sequence = envelope.correlation.sequence.unwrap();
        let gate = self.gates.lock().await.get(&session).unwrap().clone();
        self.started.send((session, sequence)).await.unwrap();
        gate.acquire().await.unwrap().forget();
        Ok(())
    }
}

#[tokio::test]
async fn ingress_scheduler_preserves_session_order_and_cross_session_concurrency() {
    let (started_tx, mut started_rx) = mpsc::channel(8);
    let session_a_gate = Arc::new(Semaphore::new(0));
    let session_b_gate = Arc::new(Semaphore::new(0));
    let gates = Arc::new(Mutex::new(HashMap::from([
        ("session-a".to_string(), session_a_gate.clone()),
        ("session-b".to_string(), session_b_gate.clone()),
    ])));
    let sink = Arc::new(OrderedSink {
        started: started_tx,
        gates,
    });
    let mut scheduler = FabricIngressScheduler::new(sink);
    scheduler
        .dispatch(envelope("session-a", "one", 1))
        .await
        .unwrap();
    scheduler
        .dispatch(envelope("session-a", "two", 2))
        .await
        .unwrap();
    scheduler
        .dispatch(envelope("session-b", "one", 1))
        .await
        .unwrap();

    let first_two = HashSet::from([
        tokio::time::timeout(Duration::from_secs(1), started_rx.recv())
            .await
            .unwrap()
            .unwrap(),
        tokio::time::timeout(Duration::from_secs(1), started_rx.recv())
            .await
            .unwrap()
            .unwrap(),
    ]);
    assert_eq!(
        first_two,
        HashSet::from([("session-a".to_string(), 1), ("session-b".to_string(), 1),])
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(25), started_rx.recv())
            .await
            .is_err()
    );

    session_a_gate.add_permits(2);
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(1), started_rx.recv())
            .await
            .unwrap(),
        Some(("session-a".to_string(), 2))
    );
    session_b_gate.add_permits(1);
    assert!(scheduler.shutdown().await.is_empty());
}

#[test]
fn fabric_production_module_contains_no_unbounded_channel() {
    let source = include_str!("../src/channel/fabric.rs");
    assert!(!source.contains("unbounded_channel"));
    assert!(!source.contains("UnboundedSender"));
    assert!(!source.contains("UnboundedReceiver"));
}

use agent_diva_channels::{
    AdapterContext, AdapterError, AdapterPacingLane, AdapterRegistry, AdapterRegistryError,
    AdapterSupervisor, ChannelAdapter, PacingError, SupervisorPolicy,
};
use agent_diva_core::channel::{
    capacity, ChannelAddress, ChannelCapabilities, ChannelCapability, ChannelCommand,
    ChannelDirection, ChannelEnvelopeV1, ChannelHealth, ChannelHealthStatus, ChannelId,
    ChannelOrigin, ChannelPayloadV1, ContentPart, Correlation, DeliveryReceipt, DeliveryStatus,
    FabricIngressItem, FabricKernel, ReactionOperation, TypingState,
};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
enum StartBehavior {
    Block,
    Exit,
    Error,
    Panic,
}

struct FakeAdapter {
    id: ChannelId,
    capabilities: Mutex<ChannelCapabilities>,
    health: Mutex<ChannelHealth>,
    start_behaviors: Mutex<VecDeque<StartBehavior>>,
    execute_results: Mutex<VecDeque<Result<DeliveryReceipt, AdapterError>>>,
    execute_gate: Mutex<Option<Arc<Semaphore>>>,
    commands: Mutex<Vec<ChannelCommand>>,
    start_count: AtomicUsize,
    stop_count: AtomicUsize,
    execute_count: AtomicUsize,
}

impl FakeAdapter {
    fn new(id: &str, capabilities: ChannelCapabilities) -> Self {
        Self {
            id: ChannelId::new(id).unwrap(),
            capabilities: Mutex::new(capabilities),
            health: Mutex::new(ChannelHealth::new(ChannelHealthStatus::Healthy)),
            start_behaviors: Mutex::new(VecDeque::new()),
            execute_results: Mutex::new(VecDeque::new()),
            execute_gate: Mutex::new(None),
            commands: Mutex::new(Vec::new()),
            start_count: AtomicUsize::new(0),
            stop_count: AtomicUsize::new(0),
            execute_count: AtomicUsize::new(0),
        }
    }

    fn with_start_behaviors(self, behaviors: impl IntoIterator<Item = StartBehavior>) -> Self {
        self.start_behaviors.lock().unwrap().extend(behaviors);
        self
    }

    fn with_execute_results(
        self,
        results: impl IntoIterator<Item = Result<DeliveryReceipt, AdapterError>>,
    ) -> Self {
        self.execute_results.lock().unwrap().extend(results);
        self
    }

    fn block_execute(&self) {
        *self.execute_gate.lock().unwrap() = Some(Arc::new(Semaphore::new(0)));
    }

    fn commands(&self) -> Vec<ChannelCommand> {
        self.commands.lock().unwrap().clone()
    }
}

#[async_trait]
impl ChannelAdapter for FakeAdapter {
    fn name(&self) -> ChannelId {
        self.id.clone()
    }

    fn capabilities(&self) -> ChannelCapabilities {
        self.capabilities.lock().unwrap().clone()
    }

    async fn start(&self, context: AdapterContext) -> Result<(), AdapterError> {
        self.start_count.fetch_add(1, Ordering::AcqRel);
        let behavior = self
            .start_behaviors
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(StartBehavior::Block);
        match behavior {
            StartBehavior::Block => {
                context.cancel.cancelled().await;
                Ok(())
            }
            StartBehavior::Exit => Ok(()),
            StartBehavior::Error => Err(AdapterError::Execution {
                code: "listener_failed".to_string(),
                diagnosis: "injected listener failure".to_string(),
                retry_after: None,
                retryable: true,
            }),
            StartBehavior::Panic => panic!("injected adapter panic"),
        }
    }

    async fn execute(&self, command: ChannelCommand) -> Result<DeliveryReceipt, AdapterError> {
        self.execute_count.fetch_add(1, Ordering::AcqRel);
        self.commands.lock().unwrap().push(command.clone());
        let gate = self.execute_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.acquire().await.unwrap().forget();
        }
        if let Some(result) = self.execute_results.lock().unwrap().pop_front() {
            return result;
        }
        Ok(delivered_receipt(&command))
    }

    fn health(&self) -> ChannelHealth {
        self.health.lock().unwrap().clone()
    }

    async fn stop(&self) -> Result<(), AdapterError> {
        self.stop_count.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}

fn capabilities(values: impl IntoIterator<Item = ChannelCapability>) -> ChannelCapabilities {
    ChannelCapabilities::new(values)
}

fn send_command(channel: &str, text: &str, idempotent: bool) -> ChannelCommand {
    let mut correlation = Correlation::new(format!("{channel}:chat"));
    correlation.request_id = Some(format!("request-{text}"));
    correlation.message_id = Some("logical-message".to_string());
    ChannelCommand::Send {
        envelope: ChannelEnvelopeV1::new(
            ChannelDirection::Egress,
            ChannelAddress::new(channel, "chat"),
            correlation,
            ChannelOrigin::Runtime,
            ChannelPayloadV1::Message {
                parts: vec![ContentPart::Text {
                    text: text.to_string(),
                }],
                subject: None,
                locale: None,
                context: None,
            },
        ),
        idempotency_key: idempotent.then(|| format!("idempotency-{text}")),
    }
}

fn send_part_command(channel: &str, part: ContentPart) -> ChannelCommand {
    ChannelCommand::Send {
        envelope: ChannelEnvelopeV1::new(
            ChannelDirection::Egress,
            ChannelAddress::new(channel, "chat"),
            Correlation::new(format!("{channel}:chat")),
            ChannelOrigin::Runtime,
            ChannelPayloadV1::Message {
                parts: vec![part],
                subject: None,
                locale: None,
                context: None,
            },
        ),
        idempotency_key: None,
    }
}

fn failed_receipt(command: &ChannelCommand, retry_after_ms: Option<u64>) -> DeliveryReceipt {
    let mut receipt = delivered_receipt(command);
    receipt.status = DeliveryStatus::Failed;
    receipt.platform_message_id = None;
    receipt.retry_after_ms = retry_after_ms;
    receipt.error_code = Some("injected_failure".to_string());
    receipt.diagnosis = Some("injected delivery failure".to_string());
    receipt
}

fn delivered_receipt(command: &ChannelCommand) -> DeliveryReceipt {
    let (channel, chat_id, thread_id) = match command {
        ChannelCommand::Send { envelope, .. } => (
            envelope.address.channel.clone(),
            envelope.address.chat_id.clone(),
            envelope.address.thread_id.clone(),
        ),
        ChannelCommand::Typing { address, .. }
        | ChannelCommand::Edit { address, .. }
        | ChannelCommand::Delete { address, .. }
        | ChannelCommand::React { address, .. }
        | ChannelCommand::FinalizeStream { address, .. } => (
            address.channel.clone(),
            address.chat_id.clone(),
            address.thread_id.clone(),
        ),
        ChannelCommand::ProbeHealth { channel } => (channel.to_string(), String::new(), None),
    };
    DeliveryReceipt {
        status: DeliveryStatus::Delivered,
        channel,
        chat_id,
        platform_message_id: Some("platform-message".to_string()),
        thread_id,
        retry_after_ms: None,
        error_code: None,
        diagnosis: None,
    }
}

async fn wait_for_count(counter: &AtomicUsize, expected: usize) {
    for _ in 0..100 {
        if counter.load(Ordering::Acquire) >= expected {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("counter did not reach {expected}");
}

async fn wait_for_failure_count(supervisor: &AdapterSupervisor, expected: u32) {
    for _ in 0..100 {
        if supervisor.health().consecutive_failures >= expected {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("supervisor did not reach {expected} failures");
}

#[tokio::test]
async fn registry_is_deterministic_and_rejects_invalid_lifecycle_operations() {
    let registry = AdapterRegistry::new();
    let adapter_b = Arc::new(FakeAdapter::new(
        "b",
        capabilities([ChannelCapability::EgressText]),
    ));
    let adapter_a = Arc::new(FakeAdapter::new(
        "a",
        capabilities([ChannelCapability::EgressText]),
    ));
    registry.register(adapter_b.clone()).await.unwrap();
    registry.register(adapter_a.clone()).await.unwrap();
    assert_eq!(
        registry.list().await,
        vec![ChannelId::new("a").unwrap(), ChannelId::new("b").unwrap()]
    );
    assert!(matches!(
        registry.register(adapter_a.clone()).await,
        Err(AdapterRegistryError::Duplicate(_))
    ));

    let id = adapter_a.name();
    registry.mark_running(&id).await.unwrap();
    assert!(matches!(
        registry.unregister(&id).await,
        Err(AdapterRegistryError::Running(_))
    ));
    registry.mark_stopped(&id).await;
    registry.unregister(&id).await.unwrap();
    assert_eq!(registry.list().await, vec![ChannelId::new("b").unwrap()]);
}

#[tokio::test]
async fn unsupported_capability_never_calls_adapter_execute() {
    let registry = AdapterRegistry::new();
    let adapter = Arc::new(FakeAdapter::new(
        "test",
        capabilities([ChannelCapability::EgressText]),
    ));
    registry.register(adapter.clone()).await.unwrap();
    let error = registry
        .execute_direct(ChannelCommand::Typing {
            address: ChannelAddress::new("test", "chat"),
            correlation: Correlation::new("test:chat"),
            state: TypingState::Started,
            idempotency_key: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        AdapterRegistryError::Adapter(AdapterError::UnsupportedCapability {
            capability: ChannelCapability::InteractionTyping
        })
    ));
    assert_eq!(adapter.execute_count.load(Ordering::Acquire), 0);
}

#[tokio::test]
async fn every_unsupported_command_is_rejected_before_transport() {
    let registry = AdapterRegistry::new();
    let adapter = Arc::new(FakeAdapter::new(
        "unsupported",
        capabilities([ChannelCapability::EgressText]),
    ));
    registry.register(adapter.clone()).await.unwrap();

    let address = ChannelAddress::new("unsupported", "chat");
    let correlation = Correlation::new("unsupported:chat");
    let commands = vec![
        ChannelCommand::Typing {
            address: address.clone(),
            correlation: correlation.clone(),
            state: TypingState::Started,
            idempotency_key: None,
        },
        ChannelCommand::Edit {
            address: address.clone(),
            correlation: correlation.clone(),
            target_message_id: "message".to_string(),
            parts: vec![ContentPart::Text {
                text: "edited".to_string(),
            }],
            idempotency_key: None,
        },
        ChannelCommand::Delete {
            address: address.clone(),
            correlation: correlation.clone(),
            target_message_id: "message".to_string(),
            idempotency_key: None,
        },
        ChannelCommand::React {
            address: address.clone(),
            correlation: correlation.clone(),
            target_message_id: "message".to_string(),
            operation: ReactionOperation::Add,
            emoji: "thumbsup".to_string(),
            idempotency_key: None,
        },
        ChannelCommand::FinalizeStream {
            address: address.clone(),
            correlation: correlation.clone(),
            parts: vec![ContentPart::Text {
                text: "final".to_string(),
            }],
            idempotency_key: None,
        },
        send_part_command(
            "unsupported",
            ContentPart::Markdown {
                markdown: "**markdown**".to_string(),
            },
        ),
        send_part_command(
            "unsupported",
            ContentPart::Image {
                attachment: test_attachment("image/png"),
            },
        ),
        send_part_command(
            "unsupported",
            ContentPart::Audio {
                attachment: test_attachment("audio/ogg"),
                transcript: None,
            },
        ),
        send_part_command(
            "unsupported",
            ContentPart::Video {
                attachment: test_attachment("video/mp4"),
            },
        ),
        send_part_command(
            "unsupported",
            ContentPart::File {
                attachment: test_attachment("application/pdf"),
            },
        ),
        send_part_command(
            "unsupported",
            ContentPart::Card {
                schema: "c5.card".to_string(),
                body: serde_json::json!({"title": "unsupported"}),
            },
        ),
    ];

    for command in commands {
        let before = adapter.execute_count.load(Ordering::Acquire);
        let error = registry.execute_direct(command).await.unwrap_err();
        assert!(matches!(
            error,
            AdapterRegistryError::Adapter(AdapterError::UnsupportedCapability { .. })
        ));
        assert_eq!(
            adapter.execute_count.load(Ordering::Acquire),
            before,
            "unsupported command reached the adapter transport"
        );
    }
}

fn test_attachment(media_type: &str) -> agent_diva_core::channel::AttachmentRef {
    agent_diva_core::channel::AttachmentRef {
        uri: "sha256:fixture".to_string(),
        media_type: media_type.to_string(),
        size_bytes: 7,
        sha256: "fixture".to_string(),
        file_name: Some("fixture.bin".to_string()),
    }
}

#[tokio::test(start_paused = true)]
async fn adapter_egress_is_bounded_and_shutdown_resolves_accepted_requests() {
    let adapter = Arc::new(FakeAdapter::new(
        "bounded",
        capabilities([ChannelCapability::EgressText]),
    ));
    adapter.block_execute();
    let lane = AdapterPacingLane::spawn(adapter.clone());
    let handle = lane.handle();
    let cancel = CancellationToken::new();
    let mut requests = Vec::new();
    for index in 0..=capacity::ADAPTER_EGRESS {
        let request_handle = handle.clone();
        let request_cancel = cancel.clone();
        requests.push(tokio::spawn(async move {
            request_handle
                .submit(
                    send_command("bounded", &index.to_string(), true),
                    Duration::from_secs(10),
                    &request_cancel,
                )
                .await
        }));
    }
    wait_for_count(&adapter.execute_count, 1).await;
    for _ in 0..100 {
        if handle.remaining_capacity() == 0 {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(handle.remaining_capacity(), 0);

    let overflow_handle = handle.clone();
    let overflow_cancel = cancel.clone();
    let overflow = tokio::spawn(async move {
        overflow_handle
            .submit(
                send_command("bounded", "overflow", true),
                Duration::from_millis(10),
                &overflow_cancel,
            )
            .await
    });
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_millis(10)).await;
    assert!(matches!(
        overflow.await.unwrap(),
        Err(PacingError::Busy { .. })
    ));

    lane.shutdown().await;
    for request in requests {
        assert!(request.await.is_ok());
    }
}

#[tokio::test(start_paused = true)]
async fn retry_after_is_honored_only_for_retry_safe_commands() {
    let idempotent = send_command("retry", "safe", true);
    let adapter = Arc::new(
        FakeAdapter::new("retry", capabilities([ChannelCapability::EgressText]))
            .with_execute_results([
                Ok(failed_receipt(&idempotent, Some(1_000))),
                Ok(delivered_receipt(&idempotent)),
            ]),
    );
    let lane = AdapterPacingLane::spawn(adapter.clone());
    let handle = lane.handle();
    let cancel = CancellationToken::new();
    let task = tokio::spawn(async move {
        handle
            .submit(idempotent, Duration::from_secs(1), &cancel)
            .await
    });
    wait_for_count(&adapter.execute_count, 1).await;
    tokio::time::advance(Duration::from_millis(999)).await;
    tokio::task::yield_now().await;
    assert_eq!(adapter.execute_count.load(Ordering::Acquire), 1);
    tokio::time::advance(Duration::from_millis(1)).await;
    wait_for_count(&adapter.execute_count, 2).await;
    assert_eq!(
        task.await.unwrap().unwrap().status,
        DeliveryStatus::Delivered
    );
    lane.shutdown().await;

    let unsafe_command = send_command("unsafe", "once", false);
    let unsafe_adapter = Arc::new(
        FakeAdapter::new("unsafe", capabilities([ChannelCapability::EgressText]))
            .with_execute_results([Ok(failed_receipt(&unsafe_command, Some(1_000)))]),
    );
    let unsafe_lane = AdapterPacingLane::spawn(unsafe_adapter.clone());
    let receipt = unsafe_lane
        .handle()
        .submit(
            unsafe_command,
            Duration::from_secs(1),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(receipt.status, DeliveryStatus::Failed);
    assert_eq!(unsafe_adapter.execute_count.load(Ordering::Acquire), 1);
    unsafe_lane.shutdown().await;
}

#[tokio::test]
async fn chunk_failure_stops_following_chunks_and_returns_partial_receipt() {
    let command = send_command("chunks", "abcdefg", true);
    let mut chunk_capabilities = capabilities([
        ChannelCapability::EgressText,
        ChannelCapability::EgressChunking,
    ]);
    chunk_capabilities.limits.max_text_chars = Some(3);
    let adapter = Arc::new(
        FakeAdapter::new("chunks", chunk_capabilities).with_execute_results([
            Ok(delivered_receipt(&command)),
            Ok(failed_receipt(&command, None)),
        ]),
    );
    let lane = AdapterPacingLane::spawn(adapter.clone());
    let receipt = lane
        .handle()
        .submit(command, Duration::from_secs(1), &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(receipt.status, DeliveryStatus::Failed);
    assert_eq!(receipt.error_code.as_deref(), Some("partial_delivery"));
    assert!(receipt
        .diagnosis
        .as_deref()
        .is_some_and(|value| value.contains("delivered 1/3")));

    let commands = adapter.commands();
    assert_eq!(commands.len(), 2);
    let envelopes: Vec<_> = commands
        .iter()
        .map(|command| match command {
            ChannelCommand::Send { envelope, .. } => envelope,
            _ => panic!("expected send chunk"),
        })
        .collect();
    assert_ne!(envelopes[0].envelope_id, envelopes[1].envelope_id);
    assert_eq!(
        envelopes[0].correlation.message_id,
        envelopes[1].correlation.message_id
    );
    assert_eq!(
        envelopes[0].extensions["agent-diva.chunk_count"],
        serde_json::json!(3)
    );
    lane.shutdown().await;
}

#[tokio::test]
async fn markdown_degrades_to_text_before_adapter_execution() {
    let adapter = Arc::new(FakeAdapter::new(
        "fallback",
        capabilities([ChannelCapability::EgressText]),
    ));
    let lane = AdapterPacingLane::spawn(adapter.clone());
    let mut envelope = match send_command("fallback", "placeholder", true) {
        ChannelCommand::Send { envelope, .. } => envelope,
        _ => unreachable!(),
    };
    envelope.payload = ChannelPayloadV1::Message {
        parts: vec![ContentPart::Markdown {
            markdown: "**hello**".to_string(),
        }],
        subject: None,
        locale: None,
        context: None,
    };
    lane.handle()
        .submit(
            ChannelCommand::Send {
                envelope,
                idempotency_key: Some("fallback-key".to_string()),
            },
            Duration::from_secs(1),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    let commands = adapter.commands();
    assert!(matches!(
        &commands[0],
        ChannelCommand::Send { envelope, .. }
            if matches!(
                &envelope.payload,
                ChannelPayloadV1::Message { parts, .. }
                    if matches!(&parts[0], ContentPart::Text { text } if text == "**hello**")
            )
    ));
    lane.shutdown().await;
}

#[tokio::test]
async fn a_blocked_adapter_does_not_block_another_adapter_lane() {
    let slow = Arc::new(FakeAdapter::new(
        "slow",
        capabilities([ChannelCapability::EgressText]),
    ));
    slow.block_execute();
    let fast = Arc::new(FakeAdapter::new(
        "fast",
        capabilities([ChannelCapability::EgressText]),
    ));
    let slow_lane = AdapterPacingLane::spawn(slow.clone());
    let fast_lane = AdapterPacingLane::spawn(fast.clone());
    let slow_handle = slow_lane.handle();
    let slow_task = tokio::spawn(async move {
        slow_handle
            .submit(
                send_command("slow", "blocked", true),
                Duration::from_secs(1),
                &CancellationToken::new(),
            )
            .await
    });
    wait_for_count(&slow.execute_count, 1).await;
    let fast_receipt = fast_lane
        .handle()
        .submit(
            send_command("fast", "ready", true),
            Duration::from_secs(1),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(fast_receipt.status, DeliveryStatus::Delivered);
    assert_eq!(fast.execute_count.load(Ordering::Acquire), 1);
    slow_lane.shutdown().await;
    assert!(slow_task.await.is_ok());
    fast_lane.shutdown().await;
}

#[tokio::test]
async fn fake_adapter_fabric_to_receipt_smoke() {
    let adapter = Arc::new(FakeAdapter::new(
        "smoke",
        capabilities([ChannelCapability::EgressText]),
    ));
    let registry = AdapterRegistry::new();
    registry.register(adapter.clone()).await.unwrap();
    let lane = AdapterPacingLane::spawn(
        registry
            .get(&ChannelId::new("smoke").unwrap())
            .await
            .unwrap(),
    );
    let (fabric, mut consumer) = FabricKernel::new().into_parts();
    let mut inbound = match send_command("smoke", "hello", true) {
        ChannelCommand::Send { envelope, .. } => envelope,
        _ => unreachable!(),
    };
    inbound.direction = ChannelDirection::Ingress;
    inbound.origin = ChannelOrigin::ExternalUser;
    fabric
        .admit_ingress(inbound, Duration::from_secs(1), &CancellationToken::new())
        .await
        .unwrap();
    let FabricIngressItem::Envelope(mut outbound) = consumer.recv_ingress().await.unwrap() else {
        panic!("smoke ingress must not be classified as control");
    };
    outbound.direction = ChannelDirection::Egress;
    outbound.origin = ChannelOrigin::Runtime;
    let receipt = lane
        .handle()
        .submit(
            ChannelCommand::Send {
                envelope: outbound,
                idempotency_key: Some("smoke-idempotency".to_string()),
            },
            Duration::from_secs(1),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(receipt.status, DeliveryStatus::Delivered);
    assert_eq!(receipt.channel, "smoke");
    assert_eq!(adapter.execute_count.load(Ordering::Acquire), 1);
    lane.shutdown().await;
}

#[tokio::test(start_paused = true)]
async fn supervisor_recovers_exit_error_and_panic_with_bounded_backoff() {
    let adapter = Arc::new(
        FakeAdapter::new(
            "supervised",
            capabilities([ChannelCapability::ReliabilitySupervisedRestart]),
        )
        .with_start_behaviors([
            StartBehavior::Exit,
            StartBehavior::Error,
            StartBehavior::Panic,
            StartBehavior::Block,
        ]),
    );
    let (fabric, _consumer) = FabricKernel::new().into_parts();
    let context = AdapterContext {
        fabric,
        cancel: CancellationToken::new(),
    };
    let supervisor = AdapterSupervisor::spawn_with_policy(
        adapter.clone(),
        context,
        SupervisorPolicy {
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(10),
            down_after_failures: 3,
            jitter_percent: 0,
        },
    );

    wait_for_count(&adapter.start_count, 1).await;
    wait_for_failure_count(&supervisor, 1).await;
    assert_eq!(supervisor.health().status, ChannelHealthStatus::Degraded);
    tokio::time::advance(Duration::from_secs(1)).await;
    wait_for_count(&adapter.start_count, 2).await;
    wait_for_failure_count(&supervisor, 2).await;
    tokio::time::advance(Duration::from_secs(2)).await;
    wait_for_count(&adapter.start_count, 3).await;
    wait_for_failure_count(&supervisor, 3).await;
    assert_eq!(supervisor.health().status, ChannelHealthStatus::Down);
    tokio::time::advance(Duration::from_secs(4)).await;
    wait_for_count(&adapter.start_count, 4).await;

    supervisor.shutdown().await;
    assert!(adapter.stop_count.load(Ordering::Acquire) >= 4);
}

#[tokio::test(start_paused = true)]
async fn supervisor_stop_interrupts_reconnect_backoff() {
    let adapter = Arc::new(
        FakeAdapter::new(
            "stop-backoff",
            capabilities([ChannelCapability::ReliabilitySupervisedRestart]),
        )
        .with_start_behaviors([StartBehavior::Error, StartBehavior::Block]),
    );
    let (fabric, _consumer) = FabricKernel::new().into_parts();
    let supervisor = AdapterSupervisor::spawn_with_policy(
        adapter.clone(),
        AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        },
        SupervisorPolicy {
            initial_backoff: Duration::from_secs(30),
            max_backoff: Duration::from_secs(30),
            down_after_failures: 3,
            jitter_percent: 0,
        },
    );
    wait_for_count(&adapter.start_count, 1).await;
    wait_for_failure_count(&supervisor, 1).await;
    supervisor.shutdown().await;
    assert_eq!(adapter.start_count.load(Ordering::Acquire), 1);
}

#[test]
fn native_adapter_runtime_contains_no_legacy_compatibility_bridge() {
    let adapter = include_str!("../src/adapter.rs");
    let registry = include_str!("../src/runtime/registry.rs");
    let pacing = include_str!("../src/runtime/pacing.rs");
    let supervisor = include_str!("../src/runtime/supervisor.rs");
    for source in [adapter, registry, pacing, supervisor] {
        assert!(!source.contains("ChannelHandler"));
        assert!(!source.contains("MessageBus"));
        assert!(!source.contains("unbounded_channel"));
    }
}

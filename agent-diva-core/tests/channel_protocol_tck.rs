use agent_diva_core::bus::{InboundMessage, MessageBus, OutboundMessage};
use agent_diva_core::channel::{
    capacity, ChannelEnvelopeV1, ChannelOrigin, ProtocolHelloParams, RpcRequestV1,
    CHANNEL_SCHEMA_VERSION_V1, JSON_RPC_VERSION, NEURO_LINK_PROTOCOL_V1,
};
use jsonschema::JSONSchema;
use serde_json::Value;

const SCHEMA: &str = include_str!("../../schemas/neuro-link/v1/protocol.schema.json");

const VALID_FIXTURES: &[(&str, &str)] = &[
    (
        "protocol-hello.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/valid/protocol-hello.json"),
    ),
    (
        "service-list.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/valid/service-list.json"),
    ),
    (
        "session-open.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/valid/session-open.json"),
    ),
    (
        "turn-cancel.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/valid/turn-cancel.json"),
    ),
    (
        "event-ack.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/valid/event-ack.json"),
    ),
    (
        "state-resume.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/valid/state-resume.json"),
    ),
    (
        "turn-start-all-content.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/valid/turn-start-all-content.json"),
    ),
    (
        "stream-notification.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/valid/stream-notification.json"),
    ),
    (
        "error-response.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/valid/error-response.json"),
    ),
];

const INVALID_FIXTURES: &[(&str, &str)] = &[
    (
        "unknown-field.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/invalid/unknown-field.json"),
    ),
    (
        "protocol-version-mismatch.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/invalid/protocol-version-mismatch.json"),
    ),
    (
        "legacy-pipe.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/invalid/legacy-pipe.json"),
    ),
    (
        "forged-identity.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/invalid/forged-identity.json"),
    ),
    (
        "bad-extension.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/invalid/bad-extension.json"),
    ),
    (
        "malformed-content.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/invalid/malformed-content.json"),
    ),
];

fn validator() -> JSONSchema {
    let schema: Value = serde_json::from_str(SCHEMA).expect("C1 protocol schema must be JSON");
    JSONSchema::compile(&schema).expect("C1 protocol schema must compile")
}

fn assert_valid(schema: &JSONSchema, name: &str, raw: &str) -> Value {
    let instance: Value = serde_json::from_str(raw)
        .unwrap_or_else(|error| panic!("valid fixture {name} must be JSON: {error}"));
    if let Err(errors) = schema.validate(&instance) {
        let details = errors.map(|error| error.to_string()).collect::<Vec<_>>();
        panic!("valid fixture {name} failed schema validation: {details:?}");
    }
    instance
}

#[test]
fn schema_compiles_and_all_positive_fixtures_validate() {
    let schema = validator();
    for (name, raw) in VALID_FIXTURES {
        assert_valid(&schema, name, raw);
    }
}

#[test]
fn all_negative_fixtures_are_rejected() {
    let schema = validator();
    for (name, raw) in INVALID_FIXTURES {
        let instance: Value = serde_json::from_str(raw)
            .unwrap_or_else(|error| panic!("invalid fixture {name} must be JSON: {error}"));
        assert!(
            !schema.is_valid(&instance),
            "invalid fixture {name} was accepted"
        );
    }
}

#[test]
fn rust_typed_request_and_envelope_round_trip_through_the_same_schema() {
    let schema = validator();

    let hello: RpcRequestV1<ProtocolHelloParams> = serde_json::from_str(include_str!(
        "../../schemas/neuro-link/v1/fixtures/valid/protocol-hello.json"
    ))
    .expect("hello fixture must deserialize into the typed Rust request");
    assert_eq!(hello.jsonrpc, JSON_RPC_VERSION);
    assert_eq!(hello.params.protocol, NEURO_LINK_PROTOCOL_V1);
    assert_eq!(hello.params.schema_version, CHANNEL_SCHEMA_VERSION_V1);

    let stream = assert_valid(
        &schema,
        "stream-notification.json",
        include_str!("../../schemas/neuro-link/v1/fixtures/valid/stream-notification.json"),
    );
    let envelope: ChannelEnvelopeV1 = serde_json::from_value(
        stream
            .get("params")
            .and_then(Value::as_object)
            .and_then(|params| params.get("envelope"))
            .cloned()
            .expect("stream fixture must carry params.envelope"),
    )
    .expect("stream envelope must deserialize into the typed Rust contract");
    assert_eq!(envelope.origin, ChannelOrigin::Runtime);
    envelope
        .validate()
        .expect("round-tripped envelope must satisfy transport-neutral invariants");

    let encoded = serde_json::json!({
        "jsonrpc": JSON_RPC_VERSION,
        "method": "conversation/stream",
        "params": { "envelope": envelope },
    });
    assert!(
        schema.is_valid(&encoded),
        "Rust serialization must remain accepted by the canonical schema"
    );
}

#[test]
fn legacy_pipe_cannot_be_decoded_as_a_v1_request() {
    let raw = include_str!("../../schemas/neuro-link/v1/fixtures/invalid/legacy-pipe.json");
    let decoded = serde_json::from_str::<RpcRequestV1<ProtocolHelloParams>>(raw);
    assert!(
        decoded.is_err(),
        "legacy pipe must not enter the v1 decoder"
    );
}

#[test]
fn c1_capacity_profile_is_explicit_bounded_and_not_octos_4096() {
    let capacities = [
        capacity::CONTROL,
        capacity::INGRESS,
        capacity::DURABLE_EVENT,
        capacity::TRANSIENT_EVENT,
        capacity::ADAPTER_EGRESS,
    ];
    assert!(capacities.iter().all(|capacity| *capacity > 0));
    assert!(capacities.iter().all(|capacity| *capacity <= 512));
    assert!(capacities.iter().all(|capacity| *capacity != 4096));
    assert!(capacity::CONTROL < capacity::INGRESS);
    assert!(capacity::TRANSIENT_EVENT < capacity::DURABLE_EVENT);
}

#[tokio::test]
async fn legacy_message_bus_characterization_preserves_fifo_and_single_receiver() {
    let bus = MessageBus::new();
    let mut receiver = bus
        .take_inbound_receiver()
        .await
        .expect("the legacy inbound receiver is single-consumer");
    assert!(bus.take_inbound_receiver().await.is_none());

    bus.publish_inbound(InboundMessage::new("telegram", "user-1", "chat-1", "one"))
        .expect("legacy inbound publish should accept an open bus");
    bus.publish_inbound(InboundMessage::new("telegram", "user-1", "chat-1", "two"))
        .expect("legacy inbound publish should preserve the open bus");

    assert_eq!(receiver.recv().await.expect("first inbound").content, "one");
    assert_eq!(
        receiver.recv().await.expect("second inbound").content,
        "two"
    );
}

#[tokio::test]
async fn legacy_outbound_bus_reports_closed_receiver() {
    let bus = MessageBus::new();
    let receiver = bus
        .take_outbound_receiver()
        .await
        .expect("the legacy outbound receiver is single-consumer");
    drop(receiver);

    let error = bus
        .publish_outbound(OutboundMessage::new("telegram", "chat-1", "reply"))
        .expect_err("publishing to a closed legacy receiver must be observable");
    assert!(error.to_string().contains("Outbound channel closed"));
}

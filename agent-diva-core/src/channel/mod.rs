//! Transport-neutral contracts for the Super Channel Fabric.
//!
//! This module is the Rust representation of the versioned wire contract in
//! `schemas/neuro-link/v1/protocol.schema.json`.  It deliberately does not
//! own listener lifecycle, queueing, or HTTP/WebSocket code; those concerns
//! land in later CHANNEL-EPIC batches.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

mod fabric;
mod runtime_contract;

pub use fabric::{
    FabricAdmissionError, FabricConsumer, FabricDispatchError, FabricHandle, FabricIngressItem,
    FabricIngressScheduler, FabricIngressSink, FabricKernel, FabricLane, FabricTransientGap,
    FabricTransientItem, TransientKey, TransientPublishOutcome,
};
pub use runtime_contract::{
    ChannelCapabilities, ChannelCapability, ChannelCapabilityProbe, ChannelCommand, ChannelHealth,
    ChannelId, ChannelLimits, ChannelLimitsProbe,
};

/// The exact protocol identifier accepted by Neuro-Link v1.
pub const NEURO_LINK_PROTOCOL_V1: &str = "neuro-link/v1";

/// The durable schema version carried by every Fabric envelope.
pub const CHANNEL_SCHEMA_VERSION_V1: u32 = 1;

/// The JSON-RPC version used by the Neuro-Link transport.
pub const JSON_RPC_VERSION: &str = "2.0";

/// The selected fixed queue capacities produced by the C1 characterization.
///
/// These are code-level constants, not user configuration.  C2 consumes this
/// profile when it introduces the Fabric lanes.
pub mod capacity {
    /// Control commands must remain available while transient events saturate.
    pub const CONTROL: usize = 64;
    /// Inbound user messages are admitted in a moderate bounded window.
    pub const INGRESS: usize = 256;
    /// Durable events have room for a burst but may never be silently lost.
    pub const DURABLE_EVENT: usize = 512;
    /// Transient events are intentionally smaller because coalescing is allowed.
    pub const TRANSIENT_EVENT: usize = 128;
    /// Each adapter receives a bounded pacing lane.
    pub const ADAPTER_EGRESS: usize = 128;
}

/// Direction of a Fabric envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelDirection {
    Ingress,
    Egress,
    InternalProjection,
}

/// Runtime provenance of a Fabric envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelOrigin {
    ExternalUser,
    OwnerFrontend,
    Runtime,
}

/// A platform and conversation address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelAddress {
    pub channel: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_id: Option<String>,
    pub chat_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

impl ChannelAddress {
    /// Build the minimum address required by the v1 contract.
    pub fn new(channel: impl Into<String>, chat_id: impl Into<String>) -> Self {
        Self {
            channel: channel.into(),
            account_id: None,
            sender_id: None,
            chat_id: chat_id.into(),
            thread_id: None,
        }
    }
}

/// Correlation identity attached to a Fabric event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Correlation {
    pub session_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u64>,
}

impl Correlation {
    /// Build the required session correlation with no turn-specific IDs.
    pub fn new(session_key: impl Into<String>) -> Self {
        Self {
            session_key: session_key.into(),
            request_id: None,
            trace_id: None,
            message_id: None,
            reply_to: None,
            sequence: None,
        }
    }
}

/// A bounded cursor into a durable projection stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CursorV1 {
    pub stream: String,
    pub sequence: u64,
}

/// A controlled reference to a file or media object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentRef {
    pub uri: String,
    pub media_type: String,
    pub size_bytes: u64,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
}

/// Typed content accepted by the Super Channel Fabric.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContentPart {
    Text {
        text: String,
    },
    Markdown {
        markdown: String,
    },
    Image {
        attachment: AttachmentRef,
    },
    Audio {
        attachment: AttachmentRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        transcript: Option<String>,
    },
    Video {
        attachment: AttachmentRef,
    },
    File {
        attachment: AttachmentRef,
    },
    Location {
        latitude: f64,
        longitude: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    Card {
        schema: String,
        body: Value,
    },
    Reference {
        uri: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
    },
}

/// State of a typing/listening indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypingState {
    Started,
    Stopped,
    Listening,
}

/// Stream lifecycle phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamPhase {
    Started,
    Delta,
    Finalized,
    Cancelled,
    Failed,
}

/// Reaction mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReactionOperation {
    Add,
    Remove,
}

/// Delivery outcome reported by an adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Accepted,
    Delivered,
    Rejected,
    Failed,
    Unsupported,
}

/// Adapter health state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelHealthStatus {
    Healthy,
    Degraded,
    Down,
    Unknown,
}

/// Unified payload union carried by a Fabric envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChannelPayloadV1 {
    Message {
        parts: Vec<ContentPart>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subject: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        locale: Option<String>,
    },
    Typing {
        state: TypingState,
    },
    Stream {
        phase: StreamPhase,
        parts: Vec<ContentPart>,
    },
    Reaction {
        operation: ReactionOperation,
        emoji: String,
    },
    Delete {
        target_message_id: String,
    },
    Delivery {
        receipt: DeliveryReceipt,
    },
    Health {
        status: ChannelHealthStatus,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        diagnosis: Option<String>,
    },
    Control {
        operation: String,
        body: Value,
    },
    Presentation {
        event: String,
        body: Value,
    },
    Gap {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_durable_cursor: Option<CursorV1>,
        reason: String,
    },
}

/// Result of an adapter delivery operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryReceipt {
    pub status: DeliveryStatus,
    pub channel: String,
    pub chat_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_message_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnosis: Option<String>,
}

/// Unified Fabric event envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelEnvelopeV1 {
    pub schema_version: u32,
    pub envelope_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub direction: ChannelDirection,
    pub address: ChannelAddress,
    pub correlation: Correlation,
    pub origin: ChannelOrigin,
    pub payload: ChannelPayloadV1,
    pub extensions: BTreeMap<String, Value>,
}

impl ChannelEnvelopeV1 {
    /// Construct a v1 envelope with server-neutral timestamp and identity.
    pub fn new(
        direction: ChannelDirection,
        address: ChannelAddress,
        correlation: Correlation,
        origin: ChannelOrigin,
        payload: ChannelPayloadV1,
    ) -> Self {
        Self {
            schema_version: CHANNEL_SCHEMA_VERSION_V1,
            envelope_id: Uuid::new_v4(),
            occurred_at: Utc::now(),
            direction,
            address,
            correlation,
            origin,
            payload,
            extensions: BTreeMap::new(),
        }
    }

    /// Validate invariants that are independent of transport.
    pub fn validate(&self) -> Result<(), ChannelContractError> {
        if self.schema_version != CHANNEL_SCHEMA_VERSION_V1 {
            return Err(ChannelContractError::SchemaVersion {
                expected: CHANNEL_SCHEMA_VERSION_V1,
                actual: self.schema_version,
            });
        }
        if self.address.channel.trim().is_empty() {
            return Err(ChannelContractError::EmptyField("address.channel"));
        }
        if self.address.chat_id.trim().is_empty() {
            return Err(ChannelContractError::EmptyField("address.chat_id"));
        }
        if self.correlation.session_key.trim().is_empty() {
            return Err(ChannelContractError::EmptyField("correlation.session_key"));
        }
        for key in self.extensions.keys() {
            if !is_namespaced_extension(key) {
                return Err(ChannelContractError::InvalidExtensionKey(key.clone()));
            }
        }
        Ok(())
    }
}

fn is_namespaced_extension(key: &str) -> bool {
    let mut pieces = key.split('.');
    let Some(namespace) = pieces.next() else {
        return false;
    };
    let Some(name) = pieces.next() else {
        return false;
    };
    !namespace.is_empty()
        && !name.is_empty()
        && namespace
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_lowercase())
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
}

/// Errors returned by transport-neutral contract validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChannelContractError {
    #[error("unsupported channel schema version: expected {expected}, got {actual}")]
    SchemaVersion { expected: u32, actual: u32 },
    #[error("required field is empty: {0}")]
    EmptyField(&'static str),
    #[error("extension key is not namespaced: {0}")]
    InvalidExtensionKey(String),
}

/// JSON-RPC request ID accepted by Neuro-Link v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RpcId {
    String(String),
    Number(i64),
}

/// A typed JSON-RPC request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcRequestV1<P> {
    pub jsonrpc: String,
    pub id: RpcId,
    pub method: String,
    pub params: P,
}

impl<P> RpcRequestV1<P> {
    /// Build a request with the fixed JSON-RPC version.
    pub fn new(id: RpcId, method: impl Into<String>, params: P) -> Self {
        Self {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

/// A typed JSON-RPC notification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcNotificationV1<P> {
    pub jsonrpc: String,
    pub method: String,
    pub params: P,
}

/// A typed JSON-RPC success response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcResponseV1<R> {
    pub jsonrpc: String,
    pub id: RpcId,
    pub result: R,
}

/// Stable protocol error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolErrorCode {
    InvalidRequest,
    InvalidParams,
    MethodNotFound,
    InternalError,
    ProtocolVersionMismatch,
    CursorOutOfRange,
    CursorInvalid,
    UnsupportedCapability,
    FrameTooLarge,
    AttachmentTooLarge,
}

/// A typed JSON-RPC error response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcErrorV1 {
    pub jsonrpc: String,
    pub id: RpcId,
    pub error: RpcErrorBody,
}

/// Error payload returned by the v1 protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcErrorBody {
    pub code: ProtocolErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// `protocol/hello` parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolHelloParams {
    pub protocol: String,
    pub schema_version: u32,
    pub frontend_instance_id: String,
    pub capabilities: Vec<String>,
}

/// `protocol/hello` result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolHelloResult {
    pub protocol: String,
    pub schema_version: u32,
    pub capabilities: Vec<String>,
}

/// `session/open` parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionOpenParams {
    pub session_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub durable_cursor: Option<CursorV1>,
}

/// `turn/start` parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurnStartParams {
    pub session_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    pub parts: Vec<ContentPart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_message_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
}

/// `turn/cancel` parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurnCancelParams {
    pub session_key: String,
    pub request_id: String,
}

/// `event/ack` parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventAckParams {
    pub session_key: String,
    pub cursor: CursorV1,
}

/// `state/resume` parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateResumeParams {
    pub session_key: String,
    pub cursor: CursorV1,
}

/// Service Binding exposed through the existing Manager HTTP handlers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceBindingV1 {
    pub service_id: String,
    pub schema_version: u32,
    pub methods: Vec<String>,
    pub http_base: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_frontend_capability: Option<String>,
}

/// Parameters shared by event notifications that carry a Fabric envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvelopeNotificationParams {
    pub envelope: ChannelEnvelopeV1,
}

/// `session/opened` notification parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionOpenedParams {
    pub session_key: String,
    pub cursor: CursorV1,
    pub services: Vec<ServiceBindingV1>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_constructor_and_validation_are_v1_native() {
        let mut address = ChannelAddress::new("neuro-link", "chat-1");
        address.sender_id = Some("frontend-1".to_string());
        let mut correlation = Correlation::new("neuro-link:chat-1");
        correlation.request_id = Some("request-1".to_string());
        let envelope = ChannelEnvelopeV1::new(
            ChannelDirection::Ingress,
            address,
            correlation,
            ChannelOrigin::OwnerFrontend,
            ChannelPayloadV1::Message {
                parts: vec![ContentPart::Text {
                    text: "hello".to_string(),
                }],
                subject: None,
                locale: None,
            },
        );

        assert_eq!(envelope.schema_version, CHANNEL_SCHEMA_VERSION_V1);
        assert!(envelope.validate().is_ok());
    }

    #[test]
    fn extensions_must_be_namespaced() {
        let mut envelope = ChannelEnvelopeV1::new(
            ChannelDirection::Ingress,
            ChannelAddress::new("telegram", "chat-1"),
            Correlation::new("telegram:chat-1"),
            ChannelOrigin::ExternalUser,
            ChannelPayloadV1::Typing {
                state: TypingState::Started,
            },
        );
        envelope.extensions.insert("raw".to_string(), Value::Null);

        assert!(matches!(
            envelope.validate(),
            Err(ChannelContractError::InvalidExtensionKey(key)) if key == "raw"
        ));
    }

    #[test]
    fn rpc_request_uses_exact_json_rpc_version() {
        let request = RpcRequestV1::new(
            RpcId::String("hello-1".to_string()),
            "protocol/hello",
            ProtocolHelloParams {
                protocol: NEURO_LINK_PROTOCOL_V1.to_string(),
                schema_version: CHANNEL_SCHEMA_VERSION_V1,
                frontend_instance_id: "frontend-1".to_string(),
                capabilities: vec!["typed_content".to_string()],
            },
        );

        assert_eq!(request.jsonrpc, JSON_RPC_VERSION);
        assert_eq!(request.method, "protocol/hello");
    }
}

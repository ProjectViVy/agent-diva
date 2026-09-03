//! Chat platform integrations for agent-diva
//!
//! This crate provides integrations for various chat platforms.
//!
//! Production transport is the native six-adapter runtime plus Neuro-Link v1.

pub mod adapter;
pub mod adapters;
pub mod runtime;

pub use adapter::{
    accepted_receipt, build_active_adapters, delivered_receipt, delivery_receipt, execution_error,
    external_message_envelope, is_sender_allowed, validate_attachment_reference, AdapterBuildError,
    AdapterContext, AdapterError, AdapterServices, AttachmentStoreError, ChannelAdapter,
    ChannelAttachmentStore, IngressAttachment, StoredAttachment,
};
pub use runtime::{
    AdapterPacingHandle, AdapterPacingLane, AdapterRegistry, AdapterRegistryError,
    AdapterSupervisor, ChannelRuntime, ChannelRuntimeError, ChannelRuntimeStatus, PacingError,
    SupervisorPolicy,
};

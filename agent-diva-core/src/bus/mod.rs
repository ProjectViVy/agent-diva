//! Message bus for decoupled communication
//!
//! The message bus provides a dual-queue system for inbound and outbound
//! messages, decoupling chat channels from the agent core.

pub mod events;
pub mod queue;

pub use events::{InboundMessage, OutboundMessage};
pub use queue::MessageBus;

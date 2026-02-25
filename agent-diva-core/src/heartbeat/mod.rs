//! Heartbeat service for periodic agent wake-up

pub mod service;
pub mod types;

pub use service::{HeartbeatDecideCallback, HeartbeatExecuteCallback, HeartbeatService};
pub use types::{
    heartbeat_tool_definition, is_heartbeat_empty, HeartbeatConfig, HeartbeatDecision,
    DEFAULT_HEARTBEAT_INTERVAL_S, HEARTBEAT_SYSTEM_PROMPT,
};

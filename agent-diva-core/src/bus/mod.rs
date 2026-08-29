//! Message bus for decoupled communication
//!
//! The message bus provides a dual-queue system for inbound and outbound
//! messages, decoupling chat channels from the agent core.

pub mod events;
pub mod queue;

pub use events::{
    AgentBusEvent, AgentEvent, InboundMessage, OutboundMessage, PlanApprovalResult,
    PlanRuntimeState, PlanRuntimeStep, PlanRuntimeTodo, PokeEvent, SessionAdmissionCode,
    SessionAdmissionObservation, SessionAdmissionPhase, SessionControlAction,
    SessionControlOutcome, SessionControlTargetState,
};
pub use queue::MessageBus;

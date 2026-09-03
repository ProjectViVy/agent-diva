//! Agent event fan-out for decoupled projection communication.
//!
//! Typed turn ingress and adapter egress are owned by the Channel Fabric.

pub mod events;
pub mod queue;

pub use events::{
    AgentBusEvent, AgentEvent, PlanApprovalResult, PlanRuntimeState, PlanRuntimeStep,
    PlanRuntimeTodo, PokeEvent, SessionAdmissionCode, SessionAdmissionObservation,
    SessionAdmissionPhase, SessionControlAction, SessionControlOutcome, SessionControlTargetState,
};
pub use queue::AgentEventBus;

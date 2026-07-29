//! Governed turn-stage contracts.
//!
//! These types keep the public [`super::AgentLoop`] facade stable while making
//! stage inputs explicit and preventing policy decisions from being inferred
//! independently at multiple side-effect boundaries.

pub(super) mod admission;
pub(super) mod context;
pub(super) mod finalize;
pub(super) mod iteration;
pub(super) mod policy;
pub(super) mod prompt;
pub(super) mod tool_step;

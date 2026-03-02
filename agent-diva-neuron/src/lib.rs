//! Neuron foundations for non-looping, single-turn LLM execution.
//!
//! This crate intentionally provides only building blocks:
//! - request/response contracts
//! - node trait and default provider-backed executor
//! - local event protocol for future graph-level orchestration

pub mod events;
pub mod executor;
pub mod node;
pub mod types;

pub use events::NeuronEvent;
pub use executor::LlmNeuron;
pub use node::{NeuronError, NeuronNode};
pub use types::{NeuronRequest, NeuronResponse};

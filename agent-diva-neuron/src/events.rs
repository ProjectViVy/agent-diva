//! Events emitted by neuron execution.

use serde::{Deserialize, Serialize};

/// Neuron-local event protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NeuronEvent {
    Started {
        neuron_id: String,
        trace_id: String,
    },
    TextDelta {
        neuron_id: String,
        delta: String,
    },
    ReasoningDelta {
        neuron_id: String,
        delta: String,
    },
    Completed {
        neuron_id: String,
        finish_reason: String,
    },
    Failed {
        neuron_id: String,
        error: String,
    },
}

//! Person-facing transcript seam (SWARM-MIG-02 / Story 6.6).
//!
//! Distinguishes content that may appear in the user-visible narrative from
//! internal swarm/subagent payloads that must not leak into that transcript (NFR-R2).

use serde::{Deserialize, Serialize};

/// Visibility of a message with respect to the **Person** / user-visible transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonSeamVisibility {
    /// Not shown in user-facing history slices (`Session::get_history`) or consolidation inputs.
    Internal,
    /// Normal user/assistant-visible turn (default when unset).
    PersonVisible,
}

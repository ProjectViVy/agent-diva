//! Shared, fail-closed governance contracts.
//!
//! The types in this module form a domain-neutral envelope for Plan, Sandbox,
//! and Memory decisions. Domain payloads remain owned by their respective
//! modules and are carried through [`ApprovalRequest`] generically.

mod policy;
mod types;

pub use crate::evolution::{EvidenceRef, EvidenceSource};
pub use policy::*;
pub use types::*;

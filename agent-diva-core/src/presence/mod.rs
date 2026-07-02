//! User presence state for the modular runtime.
//!
//! Tracks whether the user is actively engaged, distracted, or gone.
//! The three-state machine (`Active` / `Distracted` / `Gone`) drives
//! dynamic heartbeat interval adaptation.

pub mod detector;
pub mod service;
pub mod types;

pub use types::*;

//! Cognitive rule-file parsing (MEMRULES.MD).
//!
//! The machine-wide authority file lives at `{config_dir}/memory/MEMRULES.MD`
//! and is parsed by [`MemRules`]. The retired workspace WORLD governance
//! queue was removed by the cognitive clean break (S5); WORLD.MD authority
//! belongs to the persona workspace write core.

pub mod memrules;

pub use memrules::{MemRule, MemRules};

//! Native C5 channel adapters.
//!
//! Platform modules are added by their owning channel agents after the shared
//! adapter/services contract lands. This module intentionally contains no
//! placeholder or default-success adapter.

pub mod dingtalk;
pub mod discord;
pub mod email;
pub mod feishu;
pub mod qq;
pub mod telegram;

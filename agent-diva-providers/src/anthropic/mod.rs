//! Anthropic Messages API provider — native HTTP client.
//!
//! This module provides [`AnthropicClient`], a pure-HTTP driver for the
//! Anthropic Messages API with zero SDK dependency.

mod client;
mod dto;
mod stream;

pub use client::AnthropicClient;

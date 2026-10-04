//! diva-speech — DN-6A native speech state: versioned preferences with
//! revision CAS, OS-credential slots (presence-only readback, no plaintext
//! fallback), and bounded owned reference-asset storage with delete
//! leases. Domain logic is transport-free so it is unit-testable on any
//! host; the Tauri shell adds only a thin command wrapper.

pub mod assets;
pub mod config;
pub mod credentials;
pub mod providers;
pub mod registry;
pub mod service;
pub mod wav;

use serde::Serialize;

/// `diva.speech/v1` failure codes (subset reachable from DN-6A commands).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechCode {
    NativeUnavailable,
    NotConfigured,
    DeviceUnavailable,
    CredentialUnavailable,
    ProviderError,
    InvalidAudio,
    UnsupportedReference,
    AssetNotFound,
    StaleContext,
    InvalidInput,
    RevisionConflict,
    Busy,
    Cancelled,
    Timeout,
}

/// Error body surfaced to the frontend — SpeechFailure shape. Secrets,
/// audio bytes, raw text and provider URLs never appear in `message`.
#[derive(Debug, Clone, Serialize)]
pub struct SpeechError {
    pub code: SpeechCode,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<config::SpeechProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
}

impl SpeechError {
    pub fn new(code: SpeechCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retryable: false,
            provider: None,
            http_status: None,
        }
    }

    pub fn retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    pub fn provider(mut self, provider: config::SpeechProvider) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn http_status(mut self, status: u16) -> Self {
        self.http_status = Some(status);
        self
    }
}

impl std::fmt::Display for SpeechError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {}",
            serde_json::to_string(&self.code)
                .unwrap_or_default()
                .trim_matches('"'),
            self.message
        )
    }
}

impl std::error::Error for SpeechError {}

pub type SpeechResult<T> = Result<T, SpeechError>;

#[cfg(test)]
mod tests;

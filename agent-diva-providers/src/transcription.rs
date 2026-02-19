//! Voice transcription services using Groq Whisper API

use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use std::path::Path;
use thiserror::Error;
use tracing::{error, warn};

/// Transcription errors
#[derive(Error, Debug)]
pub enum TranscriptionError {
    #[error("API key not configured")]
    NoApiKey,

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error: {0}")]
    ApiError(String),
}

/// Groq API response
#[derive(Debug, Deserialize)]
struct GroqResponse {
    text: String,
}

/// Voice transcription service using Groq's Whisper API
///
/// Groq offers extremely fast transcription with a generous free tier.
#[derive(Clone)]
pub struct TranscriptionService {
    api_key: Option<String>,
    api_url: String,
    model: String,
}

impl TranscriptionService {
    /// Create a new transcription service
    ///
    /// # Arguments
    ///
    /// * `api_key` - Optional Groq API key. If None, will try to read from GROQ_API_KEY env var
    pub fn new(api_key: Option<String>) -> Self {
        let api_key = api_key.or_else(|| std::env::var("GROQ_API_KEY").ok());

        Self {
            api_key,
            api_url: "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
            model: "whisper-large-v3".to_string(),
        }
    }

    /// Create a new transcription service with custom settings
    pub fn with_settings(api_key: Option<String>, api_url: String, model: String) -> Self {
        let api_key = api_key.or_else(|| std::env::var("GROQ_API_KEY").ok());

        Self {
            api_key,
            api_url,
            model,
        }
    }

    /// Check if the service is configured
    pub fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }

    /// Transcribe an audio file using Groq
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the audio file
    ///
    /// # Returns
    ///
    /// The transcribed text, or an error
    ///
    /// # Supported formats
    ///
    /// - MP3
    /// - WAV
    /// - OGG
    /// - FLAC
    /// - M4A
    pub async fn transcribe<P: AsRef<Path>>(
        &self,
        file_path: P,
    ) -> Result<String, TranscriptionError> {
        let api_key = self.api_key.as_ref().ok_or(TranscriptionError::NoApiKey)?;

        let path = file_path.as_ref();
        if !path.exists() {
            return Err(TranscriptionError::FileNotFound(
                path.to_string_lossy().to_string(),
            ));
        }

        // Read file
        let file_bytes = tokio::fs::read(path).await?;
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.mp3")
            .to_string();

        // Build multipart form
        let file_part = Part::bytes(file_bytes).file_name(file_name);
        let form = Form::new()
            .part("file", file_part)
            .text("model", self.model.clone());

        // Send request
        let client = reqwest::Client::new();
        let response = client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await?;

        // Handle response
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Groq transcription failed: {} - {}", status, error_text);
            return Err(TranscriptionError::ApiError(format!(
                "{}: {}",
                status, error_text
            )));
        }

        let data: GroqResponse = response.json().await?;
        Ok(data.text)
    }

    /// Transcribe with fallback - returns empty string on error instead of Err
    ///
    /// This is useful when you want to continue even if transcription fails
    pub async fn transcribe_safe<P: AsRef<Path>>(&self, file_path: P) -> String {
        match self.transcribe(file_path).await {
            Ok(text) => text,
            Err(e) => {
                warn!("Transcription failed: {}", e);
                String::new()
            }
        }
    }
}

impl Default for TranscriptionService {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_service() {
        let service = TranscriptionService::new(Some("test_key".to_string()));
        assert!(service.is_configured());
        assert_eq!(service.model, "whisper-large-v3");
    }

    #[test]
    fn test_new_service_no_key() {
        let service = TranscriptionService::new(None);
        assert!(!service.is_configured());
    }

    #[test]
    fn test_with_settings() {
        let service = TranscriptionService::with_settings(
            Some("test_key".to_string()),
            "https://custom.api.com/v1/audio/transcriptions".to_string(),
            "custom-model".to_string(),
        );
        assert!(service.is_configured());
        assert_eq!(
            service.api_url,
            "https://custom.api.com/v1/audio/transcriptions"
        );
        assert_eq!(service.model, "custom-model");
    }

    #[tokio::test]
    async fn test_transcribe_no_api_key() {
        let service = TranscriptionService::new(None);
        let result = service.transcribe("test.mp3").await;
        assert!(matches!(result, Err(TranscriptionError::NoApiKey)));
    }

    #[tokio::test]
    async fn test_transcribe_file_not_found() {
        let service = TranscriptionService::new(Some("test_key".to_string()));
        let result = service.transcribe("/nonexistent/file.mp3").await;
        assert!(matches!(result, Err(TranscriptionError::FileNotFound(_))));
    }

    #[tokio::test]
    async fn test_transcribe_safe_returns_empty_on_error() {
        let service = TranscriptionService::new(None);
        let result = service.transcribe_safe("test.mp3").await;
        assert_eq!(result, "");
    }

    #[test]
    fn test_default() {
        let service = TranscriptionService::default();
        assert_eq!(service.model, "whisper-large-v3");
    }
}

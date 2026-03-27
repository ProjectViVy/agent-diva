//! Embedding provider abstractions for hybrid-ready local recall.

use agent_diva_core::{Error, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddingProviderConfig {
    pub provider: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub dimensions: usize,
}

impl Default for EmbeddingProviderConfig {
    fn default() -> Self {
        Self {
            provider: "noop".into(),
            base_url: "https://api.openai.com".into(),
            api_key: None,
            model: "text-embedding-3-small".into(),
            dimensions: 1536,
        }
    }
}

impl EmbeddingProviderConfig {
    pub fn from_env() -> Self {
        let provider =
            std::env::var("AGENT_DIVA_MEMORY_EMBEDDING_PROVIDER").unwrap_or_else(|_| "noop".into());
        let base_url = std::env::var("AGENT_DIVA_MEMORY_EMBEDDING_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com".into());
        let api_key = std::env::var("AGENT_DIVA_MEMORY_EMBEDDING_API_KEY").ok();
        let model = std::env::var("AGENT_DIVA_MEMORY_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".into());
        let dimensions = std::env::var("AGENT_DIVA_MEMORY_EMBEDDING_DIMENSIONS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1536);

        Self {
            provider,
            base_url,
            api_key,
            model,
            dimensions,
        }
    }
}

pub trait EmbeddingProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn dimensions(&self) -> usize;
    fn is_enabled(&self) -> bool;
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let mut results = self.embed(&[text])?;
        results
            .pop()
            .ok_or_else(|| Error::Internal("empty embedding response".into()))
    }
}

#[derive(Debug, Default)]
pub struct NoopEmbeddingProvider;

impl EmbeddingProvider for NoopEmbeddingProvider {
    fn name(&self) -> &str {
        "noop"
    }

    fn model(&self) -> &str {
        "noop"
    }

    fn dimensions(&self) -> usize {
        0
    }

    fn is_enabled(&self) -> bool {
        false
    }

    fn embed(&self, _texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        Ok(Vec::new())
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleEmbeddingProvider {
    config: EmbeddingProviderConfig,
    client: Client,
}

impl OpenAiCompatibleEmbeddingProvider {
    pub fn new(config: EmbeddingProviderConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|error| {
                Error::Internal(format!("failed to build embedding client: {error}"))
            })?;
        Ok(Self { config, client })
    }

    fn embeddings_url(&self) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        if base.ends_with("/embeddings") {
            base.to_string()
        } else if base.ends_with("/v1") || base.contains("/api/") || base.contains("/openai/") {
            format!("{base}/embeddings")
        } else {
            format!("{base}/v1/embeddings")
        }
    }
}

impl EmbeddingProvider for OpenAiCompatibleEmbeddingProvider {
    fn name(&self) -> &str {
        &self.config.provider
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    fn dimensions(&self) -> usize {
        self.config.dimensions
    }

    fn is_enabled(&self) -> bool {
        self.config
            .api_key
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let api_key = self
            .config
            .api_key
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| Error::Internal("embedding api key is missing".into()))?;
        let response = self
            .client
            .post(self.embeddings_url())
            .bearer_auth(api_key)
            .json(&serde_json::json!({
                "model": self.config.model,
                "input": texts,
            }))
            .send()
            .map_err(|error| Error::Internal(format!("embedding request failed: {error}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(Error::Internal(format!(
                "embedding api error {status}: {body}"
            )));
        }

        let body: serde_json::Value = response
            .json()
            .map_err(|error| Error::Internal(format!("invalid embedding response: {error}")))?;
        let data = body
            .get("data")
            .and_then(|value| value.as_array())
            .ok_or_else(|| Error::Internal("embedding response missing data array".into()))?;

        let mut embeddings = Vec::with_capacity(data.len());
        for item in data {
            let Some(values) = item.get("embedding").and_then(|value| value.as_array()) else {
                return Err(Error::Internal("embedding item missing vector".into()));
            };
            let vector = values
                .iter()
                .map(|value| value.as_f64().unwrap_or_default() as f32)
                .collect::<Vec<_>>();
            embeddings.push(vector);
        }
        Ok(embeddings)
    }
}

pub fn provider_from_config(
    config: &EmbeddingProviderConfig,
) -> Result<Box<dyn EmbeddingProvider>> {
    match config.provider.as_str() {
        "openai" | "openai_compatible" | "custom" => Ok(Box::new(
            OpenAiCompatibleEmbeddingProvider::new(config.clone())?,
        )),
        _ => Ok(Box::new(NoopEmbeddingProvider)),
    }
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }

    let mut dot = 0.0_f32;
    let mut left_norm = 0.0_f32;
    let mut right_norm = 0.0_f32;
    for (l, r) in left.iter().zip(right.iter()) {
        dot += l * r;
        left_norm += l * l;
        right_norm += r * r;
    }

    if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
        return 0.0;
    }

    dot / (left_norm.sqrt() * right_norm.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_provider_is_disabled() {
        let provider = NoopEmbeddingProvider;
        assert!(!provider.is_enabled());
        assert_eq!(provider.dimensions(), 0);
    }

    #[test]
    fn cosine_similarity_handles_basic_vectors() {
        let similarity = cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]);
        assert!(similarity > 0.99);
    }

    #[test]
    fn provider_from_env_defaults_to_noop() {
        let config = EmbeddingProviderConfig::default();
        let provider = provider_from_config(&config).unwrap();
        assert_eq!(provider.name(), "noop");
    }

    #[test]
    fn openai_provider_uses_v1_suffix_by_default() {
        let provider = OpenAiCompatibleEmbeddingProvider::new(EmbeddingProviderConfig {
            provider: "openai".into(),
            base_url: "https://api.openai.com".into(),
            api_key: Some("key".into()),
            model: "text-embedding-3-small".into(),
            dimensions: 1536,
        })
        .unwrap();
        assert_eq!(
            provider.embeddings_url(),
            "https://api.openai.com/v1/embeddings"
        );
    }
}

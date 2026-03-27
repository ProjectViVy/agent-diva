use crate::base::{
    LLMProvider, LLMResponse, LLMStreamEvent, Message, ProviderError, ProviderEventStream,
    ProviderResult,
};
use async_trait::async_trait;
use futures::stream;
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;

use agent_diva_core::auth::{extract_account_id_from_jwt, ProviderAuthService};

const DEFAULT_RESPONSES_URL: &str = "https://chatgpt.com/backend-api/codex/responses";

#[derive(Debug, Serialize)]
struct ResponsesRequest {
    model: String,
    input: Vec<ResponsesInputMessage>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ResponsesInputMessage {
    role: String,
    content: Vec<ResponsesContent>,
}

#[derive(Debug, Serialize)]
struct ResponsesContent {
    #[serde(rename = "type")]
    kind: String,
    text: String,
}

#[derive(Clone)]
pub struct OpenAiCodexProvider {
    client: Client,
    auth: ProviderAuthService,
    model: String,
    responses_url: String,
}

impl OpenAiCodexProvider {
    pub fn new(
        auth: ProviderAuthService,
        model: impl Into<String>,
        responses_url: Option<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            auth,
            model: model.into(),
            responses_url: responses_url.unwrap_or_else(|| DEFAULT_RESPONSES_URL.to_string()),
        }
    }

    async fn send_request(
        &self,
        messages: Vec<Message>,
        model: Option<String>,
    ) -> ProviderResult<LLMResponse> {
        let profile = self
            .auth
            .get_active_profile("openai-codex")
            .await
            .map_err(|err| ProviderError::ConfigError(err.to_string()))?;
        let access_token = self
            .auth
            .get_valid_openai_codex_access_token(None)
            .await
            .map_err(|err| ProviderError::ApiError(err.to_string()))?
            .ok_or_else(|| {
                ProviderError::ConfigError(
                    "OpenAI Codex auth profile not found. Run `agent-diva provider login openai-codex`.".into(),
                )
            })?;
        let account_id = profile
            .and_then(|profile| profile.account_id)
            .or_else(|| extract_account_id_from_jwt(&access_token));
        let request = ResponsesRequest {
            model: model.unwrap_or_else(|| self.model.clone()),
            input: messages
                .into_iter()
                .map(|message| ResponsesInputMessage {
                    role: message.role,
                    content: vec![ResponsesContent {
                        kind: "input_text".to_string(),
                        text: message.content,
                    }],
                })
                .collect(),
            stream: false,
        };

        let mut builder = self
            .client
            .post(&self.responses_url)
            .bearer_auth(&access_token)
            .header("OpenAI-Beta", "responses=experimental")
            .header("originator", "pi")
            .header("content-type", "application/json")
            .json(&request);
        if let Some(account_id) = account_id {
            builder = builder.header("chatgpt-account-id", account_id);
        }
        let response = builder.send().await.map_err(ProviderError::HttpError)?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!(
                "OpenAI Codex request failed ({status}): {body}"
            )));
        }
        let body = response.text().await.map_err(ProviderError::HttpError)?;
        let content = extract_text(&body).ok_or_else(|| {
            ProviderError::InvalidResponse("No text content in OpenAI Codex response".into())
        })?;
        Ok(LLMResponse {
            content: Some(content),
            tool_calls: vec![],
            finish_reason: "stop".to_string(),
            usage: HashMap::new(),
            reasoning_content: None,
        })
    }
}

fn extract_text(body: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    if let Some(value) = json.get("output_text").and_then(|value| value.as_str()) {
        return Some(value.to_string());
    }
    json.get("output")
        .and_then(|value| value.as_array())
        .and_then(|items| {
            items.iter().find_map(|item| {
                item.get("content")
                    .and_then(|value| value.as_array())
                    .and_then(|content| {
                        content.iter().find_map(|entry| {
                            entry
                                .get("text")
                                .and_then(|value| value.as_str())
                                .map(str::to_string)
                        })
                    })
            })
        })
}

#[async_trait]
impl LLMProvider for OpenAiCodexProvider {
    async fn chat(
        &self,
        messages: Vec<Message>,
        _tools: Option<Vec<serde_json::Value>>,
        model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        self.send_request(messages, model).await
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        _tools: Option<Vec<serde_json::Value>>,
        model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<ProviderEventStream> {
        let response = self.send_request(messages, model).await?;
        Ok(Box::pin(stream::iter(vec![Ok(LLMStreamEvent::Completed(
            response,
        ))])))
    }

    fn get_default_model(&self) -> String {
        self.model.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn runtime_without_token_returns_not_logged_in_error() {
        let dir = tempdir().unwrap();
        let provider = OpenAiCodexProvider::new(
            ProviderAuthService::new(dir.path()),
            "gpt-5-codex",
            Some("https://example.com".to_string()),
        );
        let err = provider
            .chat(vec![Message::user("hello")], None, None, 512, 0.0)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("provider login openai-codex"));
    }
}

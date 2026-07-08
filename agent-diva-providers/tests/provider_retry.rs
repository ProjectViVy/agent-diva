//! Integration tests for provider retry logic with exponential backoff.
//!
//! Uses mockito to simulate HTTP responses for testing retry behavior
//! (5xx retry, 429 rate limit, 4xx no-retry, network timeout).
//!
//! Tests cover:
//! - AC1: Exponential backoff (max 3 retries, base 1s, multiplier 2x)
//! - AC2: HTTP 429 �?ProviderError::RateLimited with Retry-After
//! - AC4: ProviderError::RateLimited variant with retry_after field

use agent_diva_providers::{LLMProvider, LLMResponse, Message, ProviderError};
use mockito::Server;

/// Helper: build a OpenAiCompatibleClient for testing with a mock server URL.
fn test_client(mock_url: &str) -> agent_diva_providers::OpenAiCompatibleClient {
    agent_diva_providers::OpenAiCompatibleClient::new(
        Some("sk-test-key".to_string()),
        Some(mock_url.to_string()),
        "test-model".to_string(),
        None,
        Some("test-provider".to_string()),
        None,
    )
}

#[tokio::test]
async fn test_retry_on_503_then_success() {
    let mut server = Server::new_async().await;

    // Mock: first two calls return 503, third returns 200
    let mock = server
        .mock("POST", "/chat/completions")
        .with_status(503)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Service Unavailable"}"#)
        .expect(2) // first two calls are 503
        .create_async()
        .await;

    // We need a second mock that takes priority for the third call
    // mockito processes mocks in order, but both match the same path
    // Use .expect() to limit the 503 mock to 2 calls

    let success_mock = server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "id": "test-123",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "test-model",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello!"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        }"#,
        )
        .expect(1)
        .create_async()
        .await;

    let client = test_client(&server.url());

    let messages = vec![Message::user("Hello")];
    let result: Result<LLMResponse, ProviderError> =
        client.chat(messages, None, None, 100, 0.7).await;

    assert!(
        result.is_ok(),
        "Should succeed after retries: {:?}",
        result.err()
    );
    mock.assert_async().await;
    success_mock.assert_async().await;
}

#[tokio::test]
async fn test_retry_max_attempts_exceeded() {
    let mut server = Server::new_async().await;

    // Mock always returns 503
    let mock = server
        .mock("POST", "/chat/completions")
        .with_status(503)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Service Unavailable"}"#)
        .expect(4) // 1 initial + 3 retries = 4 total
        .create_async()
        .await;

    let client = test_client(&server.url());

    let messages = vec![Message::user("Hello")];
    let result: Result<LLMResponse, ProviderError> =
        client.chat(messages, None, None, 100, 0.7).await;

    assert!(result.is_err(), "Should fail after max retries");
    let err = result.unwrap_err();
    assert!(
        matches!(err, ProviderError::ApiError(_)),
        "Should be ApiError after max retries, got: {:?}",
        err
    );
    mock.assert_async().await;
}

#[tokio::test]
async fn test_rate_limited_429_with_header() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/chat/completions")
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_header("Retry-After", "30")
        .with_body(r#"{"error": "Rate limit exceeded"}"#)
        .expect(1) // No retry on 429
        .create_async()
        .await;

    let client = test_client(&server.url());

    let messages = vec![Message::user("Hello")];
    let result: Result<LLMResponse, ProviderError> =
        client.chat(messages, None, None, 100, 0.7).await;

    assert!(result.is_err(), "429 should return error immediately");
    let err = result.unwrap_err();
    match err {
        ProviderError::RateLimited { retry_after } => {
            assert_eq!(
                retry_after,
                Some(30),
                "Retry-After should be 30s, got: {:?}",
                retry_after
            );
        }
        other => panic!("Expected RateLimited, got: {:?}", other),
    }
    mock.assert_async().await;
}

#[tokio::test]
async fn test_rate_limited_429_no_header() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/chat/completions")
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Too Many Requests"}"#)
        .expect(1)
        .create_async()
        .await;

    let client = test_client(&server.url());

    let messages = vec![Message::user("Hello")];
    let result: Result<LLMResponse, ProviderError> =
        client.chat(messages, None, None, 100, 0.7).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ProviderError::RateLimited { retry_after } => {
            assert_eq!(
                retry_after, None,
                "retry_after should be None when header is missing"
            );
        }
        other => panic!("Expected RateLimited, got: {:?}", other),
    }
    mock.assert_async().await;
}

#[tokio::test]
async fn test_non_retryable_400() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/chat/completions")
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Bad Request"}"#)
        .expect(1) // No retry on 400
        .create_async()
        .await;

    let client = test_client(&server.url());

    let messages = vec![Message::user("Hello")];
    let result: Result<LLMResponse, ProviderError> =
        client.chat(messages, None, None, 100, 0.7).await;

    assert!(result.is_err(), "400 should fail immediately without retry");
    let err = result.unwrap_err();
    assert!(
        matches!(err, ProviderError::ApiError(_)),
        "Should be ApiError for 400, got: {:?}",
        err
    );
    mock.assert_async().await;
}

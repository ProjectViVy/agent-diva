//! Retry logic with exponential backoff for provider HTTP requests.
//!
//! Implements exponential backoff with jitter for transient failures (5xx, network
//! errors) and rate-limit detection (HTTP 429 → `ProviderError::RateLimited`).
//!
//! Pattern references:
//! - codex: RetryConfig + exponential backoff with jitter
//! - openfang: BackoffStrategy enum + HealthMonitor integration
//! - OpenHarness: Simple for-loop with time.sleep(min(base * 2**attempt, max_delay))

use crate::base::{ProviderError, ProviderResult};
use reqwest::Response;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Maximum number of retry attempts (excluding the initial request).
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff in milliseconds (1 second).
const BASE_DELAY_MS: u64 = 1000;

/// Information about a single retry attempt, delivered to the optional
/// `on_retry` callback of [`send_with_retry`].
#[derive(Debug, Clone)]
pub struct RetryAttempt {
    pub model: String,
    /// 1-based retry attempt number.
    pub attempt: u32,
    pub max_retries: u32,
    pub delay_ms: u64,
    pub reason: String,
}

/// Callback invoked before each retry attempt so the caller can surface
/// retry status (e.g. as a GUI event) instead of waiting silently.
pub type RetryListener = Arc<dyn Fn(RetryAttempt) + Send + Sync>;

/// Calculate exponential backoff delay with ±20% jitter.
///
/// Delay = base × 2^(attempt-1) + jitter
/// Where jitter is random 0..(delay/5) to avoid thundering herd.
fn backoff_delay(attempt: u32) -> Duration {
    let base = BASE_DELAY_MS * 2u64.pow(attempt.saturating_sub(1));
    let jitter = rand::random::<u64>() % (base / 5);
    Duration::from_millis(base + jitter)
}

/// Check HTTP response status and return appropriate ProviderError handling.
///
/// - HTTP 429 → `ProviderError::RateLimited` with optional `Retry-After` header.
/// - HTTP 5xx → returns `false` (caller should retry).
/// - Other non-success → returns `Err(ProviderError::ApiError(...))`.
/// - Success → returns `Ok(true)`.
pub fn classify_response_status(
    status: reqwest::StatusCode,
    error_text: String,
) -> ProviderResult<bool> {
    if status.as_u16() == 429 {
        return Err(ProviderError::RateLimited { retry_after: None });
    }

    if status.is_server_error() {
        return Ok(false); // Retryable
    }

    if !status.is_success() {
        return Err(ProviderError::ApiError(format!(
            "HTTP {}: {}",
            status, error_text
        )));
    }

    Ok(true)
}

/// Extract `Retry-After` header value from HTTP 429 response.
pub fn parse_retry_after(response: &Response) -> Option<u64> {
    response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
}

/// Send an HTTP request with exponential backoff retry on 5xx and network errors.
///
/// The `send_fn` closure is called for each attempt and must return a `reqwest::Response`.
/// On success, the response is returned. On rate limit (HTTP 429), `RateLimited` is
/// returned immediately without retrying. On server errors and network failures,
/// the request is retried up to `MAX_RETRIES` additional times. When `on_retry` is
/// provided, it is invoked before each retry delay so callers can surface progress.
#[allow(clippy::needless_pass_by_value)]
pub async fn send_with_retry<F, Fut>(
    model: &str,
    on_retry: Option<&RetryListener>,
    send_fn: F,
) -> ProviderResult<Response>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<Response, reqwest::Error>>,
{
    let mut last_error: Option<ProviderError> = None;

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            let delay = backoff_delay(attempt);
            debug!(
                "Retry attempt {}/{} for model '{}' (delay: {:?})",
                attempt, MAX_RETRIES, model, delay
            );
            if let Some(listener) = on_retry {
                listener(RetryAttempt {
                    model: model.to_string(),
                    attempt,
                    max_retries: MAX_RETRIES,
                    delay_ms: delay.as_millis() as u64,
                    reason: last_error
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "unknown error".to_string()),
                });
            }
            tokio::time::sleep(delay).await;
        }

        let response = match send_fn().await {
            Ok(resp) => resp,
            Err(e) if attempt < MAX_RETRIES => {
                warn!(
                    "Request failed for model '{}' (attempt {}/{}), retrying: {}",
                    model,
                    attempt + 1,
                    MAX_RETRIES + 1,
                    e
                );
                last_error = Some(ProviderError::HttpError(e));
                continue;
            }
            Err(e) => return Err(ProviderError::HttpError(e)),
        };

        // Check for rate limiting (HTTP 429) — do NOT retry
        if response.status().as_u16() == 429 {
            let retry_after = parse_retry_after(&response);
            return Err(ProviderError::RateLimited { retry_after });
        }

        // Retry on server errors (HTTP 5xx)
        if response.status().is_server_error() && attempt < MAX_RETRIES {
            let status = response.status();
            warn!(
                "Server error HTTP {} for model '{}' (attempt {}/{}), retrying",
                status,
                model,
                attempt + 1,
                MAX_RETRIES + 1
            );
            last_error = Some(ProviderError::ApiError(format!("HTTP {}", status)));
            continue;
        }

        // Non-retryable error
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProviderError::ApiError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        // Success
        return Ok(response);
    }

    Err(last_error.unwrap_or_else(|| ProviderError::ApiError("Max retries exceeded".to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    #[test]
    fn test_backoff_delay_sequence() {
        // Attempt 1: 1s base delay
        let d1 = backoff_delay(1);
        assert!(d1.as_millis() >= 1000, "attempt 1 should be >= 1s");
        assert!(d1.as_millis() < 1200, "attempt 1 within jitter range");

        // Attempt 2: 2s base delay
        let d2 = backoff_delay(2);
        assert!(d2.as_millis() >= 2000, "attempt 2 should be >= 2s");

        // Attempt 3: 4s base delay
        let d3 = backoff_delay(3);
        assert!(d3.as_millis() >= 4000, "attempt 3 should be >= 4s");
    }

    #[test]
    fn test_classify_429_is_rate_limited() {
        let status = reqwest::StatusCode::from_u16(429).unwrap();
        let result = classify_response_status(status, "Too Many Requests".into());
        assert!(matches!(result, Err(ProviderError::RateLimited { .. })));
    }

    #[test]
    fn test_classify_503_is_retryable() {
        let status = reqwest::StatusCode::from_u16(503).unwrap();
        let result = classify_response_status(status, "Service Unavailable".into());
        assert!(!result.unwrap()); // false = retryable
    }

    #[test]
    fn test_classify_400_is_non_retryable() {
        let status = reqwest::StatusCode::from_u16(400).unwrap();
        let result = classify_response_status(status, "Bad Request".into());
        assert!(matches!(result, Err(ProviderError::ApiError(_))));
    }

    #[test]
    fn test_classify_200_is_success() {
        let status = reqwest::StatusCode::from_u16(200).unwrap();
        let result = classify_response_status(status, String::new());
        assert!(result.unwrap());
    }

    #[test]
    fn test_provider_error_rate_limited_format() {
        let err = ProviderError::RateLimited {
            retry_after: Some(30),
        };
        assert_eq!(err.to_string(), "Rate limited");

        let err = ProviderError::RateLimited { retry_after: None };
        assert_eq!(err.to_string(), "Rate limited");
    }

    #[tokio::test]
    async fn test_send_with_retry_invokes_listener_per_attempt() {
        // Local TCP mock: 500, 500, then 200, so two retries occur.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            for status in [500u16, 500, 200] {
                let (mut sock, _) = listener.accept().await.unwrap();
                let mut buf = [0u8; 4096];
                let _ = sock.read(&mut buf).await;
                let body = if status == 200 { "ok" } else { "err" };
                let resp = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = tokio::io::AsyncWriteExt::write_all(&mut sock, resp.as_bytes()).await;
            }
        });

        let url = format!("http://{}/retry", addr);
        let client = reqwest::Client::builder().no_proxy().build().unwrap();

        let attempts = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = attempts.clone();
        let listener_cb: RetryListener = std::sync::Arc::new(move |r: RetryAttempt| {
            captured.lock().unwrap().push(r);
        });

        let result = send_with_retry("test-model", Some(&listener_cb), || {
            let client = client.clone();
            let url = url.clone();
            async move { client.get(&url).send().await }
        })
        .await;

        assert!(result.is_ok(), "expected success on third attempt");
        let recorded = attempts.lock().unwrap();
        assert_eq!(recorded.len(), 2, "expected one listener call per retry");
        assert_eq!(recorded[0].model, "test-model");
        assert_eq!(recorded[0].attempt, 1);
        assert_eq!(recorded[1].attempt, 2);
        assert_eq!(recorded[0].max_retries, 3);
        assert!(recorded[0].delay_ms >= 1000, "attempt 1 delay >= 1s");
        assert!(recorded[1].delay_ms >= 2000, "attempt 2 delay >= 2s");
    }

    #[tokio::test]
    async fn test_send_with_retry_no_listener_still_succeeds() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let _ = sock.read(&mut buf).await;
            let body = "ok";
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = tokio::io::AsyncWriteExt::write_all(&mut sock, resp.as_bytes()).await;
        });

        let url = format!("http://{}/ok", addr);
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let result = send_with_retry("test-model", None, || {
            let client = client.clone();
            let url = url.clone();
            async move { client.get(&url).send().await }
        })
        .await;
        assert!(result.is_ok());
    }
}

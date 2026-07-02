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
use std::time::Duration;
use tracing::{debug, warn};

/// Maximum number of retry attempts (excluding the initial request).
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff in milliseconds (1 second).
const BASE_DELAY_MS: u64 = 1000;

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
/// the request is retried up to `MAX_RETRIES` additional times.
#[allow(clippy::needless_pass_by_value)]
pub async fn send_with_retry<F, Fut>(model: &str, send_fn: F) -> ProviderResult<Response>
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
        assert_eq!(result.unwrap(), false); // false = retryable
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
        assert_eq!(result.unwrap(), true);
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
}

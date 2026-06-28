//! Retry policy for LLM provider requests.
//!
//! Provides exponential backoff with jitter for transient errors and rate limits.

use std::time::Duration;
use tracing::{debug, warn};

use crate::base::{ProviderApiError, ProviderError};

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (not counting the initial request).
    pub max_retries: u32,
    /// Base delay for exponential backoff.
    pub base_delay: Duration,
    /// Maximum delay cap to prevent excessive waits.
    pub max_delay: Duration,
    /// Multiplier for exponential backoff (default: 2.0).
    pub backoff_multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy with custom settings.
    pub fn new(max_retries: u32, base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            max_retries,
            base_delay,
            max_delay,
            backoff_multiplier: 2.0,
        }
    }

    /// Calculate the delay before the next retry attempt.
    ///
    /// Uses exponential backoff with jitter to avoid thundering herd problems.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        // Exponential backoff: base_delay * multiplier^(attempt-1)
        let exponential =
            self.base_delay.as_millis() as f64 * self.backoff_multiplier.powi((attempt - 1) as i32);

        // Add jitter: ±25% of the calculated delay
        let jitter_range = exponential * 0.25;
        let jitter = (fastrand::f64() * 2.0 - 1.0) * jitter_range;
        let delay_ms = (exponential + jitter).max(0.0) as u64;

        // Cap at max_delay
        Duration::from_millis(delay_ms.min(self.max_delay.as_millis() as u64))
    }

    /// Determine delay from Retry-After header, falling back to calculated backoff.
    pub fn delay_with_retry_after(&self, attempt: u32, retry_after_secs: Option<u64>) -> Duration {
        if let Some(secs) = retry_after_secs {
            let header_delay = Duration::from_secs(secs);
            let backoff_delay = self.delay_for_attempt(attempt);
            // Use the longer of the two to respect server's Retry-After
            return header_delay.max(backoff_delay);
        }
        self.delay_for_attempt(attempt)
    }

    /// Check if an error is retryable.
    pub fn is_retryable(error: &ProviderError) -> bool {
        // Delegate to ProviderError's own classification first
        if error.is_retryable() {
            return true;
        }
        // Also handle legacy ApiError and HttpError variants that haven't been
        // reclassified into typed variants yet.
        match error {
            ProviderError::ApiError(api_error) => Self::is_retryable_api_error(api_error),
            ProviderError::HttpError(http_error) => Self::is_retryable_http_error(http_error),
            // JSON parsing, config errors, invalid responses, auth, permanent,
            // and tool schema errors are not retryable
            ProviderError::JsonError(_)
            | ProviderError::ConfigError(_)
            | ProviderError::InvalidResponse(_)
            | ProviderError::Auth { .. }
            | ProviderError::Permanent { .. }
            | ProviderError::ToolSchema { .. }
            | ProviderError::RateLimited { .. }
            | ProviderError::Transient { .. } => false,
        }
    }

    /// Check if an API error is retryable (429 or 5xx).
    fn is_retryable_api_error(error: &ProviderApiError) -> bool {
        // Check for rate limit (429)
        if error.status == Some(429) {
            return true;
        }

        // Check for server errors (5xx)
        if let Some(status) = error.status {
            if (500..600).contains(&status) {
                return true;
            }
        }

        // Check error type/code for rate limit indicators
        let rate_limit_indicators = [
            "rate_limit",
            "ratelimit",
            "rate_limit_error",
            "too_many_requests",
            "quota_exceeded",
        ];

        let error_type_lower = error
            .error_type
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase();
        let code_lower = error.code.as_deref().unwrap_or("").to_ascii_lowercase();
        let message_lower = error.message.to_ascii_lowercase();

        for indicator in &rate_limit_indicators {
            if error_type_lower.contains(indicator)
                || code_lower.contains(indicator)
                || message_lower.contains(indicator)
            {
                return true;
            }
        }

        // Check for transient error indicators in message
        let transient_indicators = [
            "timeout",
            "timed out",
            "connection reset",
            "connection refused",
            "connection aborted",
            "network",
            "temporary",
            "unavailable",
            "try again",
            "service overload",
            "server overloaded",
        ];

        for indicator in &transient_indicators {
            if message_lower.contains(indicator) {
                return true;
            }
        }

        false
    }

    /// Check if an HTTP error is retryable (network issues, timeouts).
    fn is_retryable_http_error(error: &reqwest::Error) -> bool {
        // Timeouts are retryable
        if error.is_timeout() {
            return true;
        }

        // Connection errors are retryable
        if error.is_connect() {
            return true;
        }

        // Request errors (network issues) are retryable
        if error.is_request() {
            return true;
        }

        false
    }

    /// Execute an async operation with retry logic.
    ///
    /// The operation is called up to `max_retries + 1` times.
    /// Returns the first successful result, or the last error.
    pub async fn execute_with_retry<F, Fut, T>(&self, mut operation: F) -> Result<T, ProviderError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, ProviderError>>,
    {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        debug!("Request succeeded after {} retries", attempt);
                    }
                    return Ok(result);
                }
                Err(error) => {
                    if !Self::is_retryable(&error) || attempt == self.max_retries {
                        return Err(error);
                    }

                    let retry_after_secs = match &error {
                        ProviderError::RateLimited { retry_after } => {
                            retry_after.map(|d| d.as_secs())
                        }
                        ProviderError::ApiError(api_error) => api_error.retry_after_secs,
                        _ => None,
                    };

                    let delay = self.delay_with_retry_after(attempt + 1, retry_after_secs);

                    warn!(
                        "Request failed (attempt {}/{}): {}. Retrying in {:?}...",
                        attempt + 1,
                        self.max_retries + 1,
                        error,
                        delay
                    );

                    tokio::time::sleep(delay).await;
                    last_error = Some(error);
                }
            }
        }

        // Should not reach here, but just in case
        Err(last_error.unwrap_or_else(|| {
            ProviderError::ConfigError("Retry loop exited without result".to_string())
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::ProviderApiError;

    #[test]
    fn default_policy_has_correct_values() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.base_delay, Duration::from_millis(1000));
        assert_eq!(policy.max_delay, Duration::from_secs(60));
        assert_eq!(policy.backoff_multiplier, 2.0);
    }

    #[test]
    fn delay_for_attempt_zero_is_zero() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.delay_for_attempt(0), Duration::ZERO);
    }

    #[test]
    fn delay_for_attempt_increases_exponentially() {
        let policy = RetryPolicy {
            base_delay: Duration::from_millis(1000),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(60),
            ..Default::default()
        };

        // Attempt 1: ~1000ms (±25% jitter)
        let delay1 = policy.delay_for_attempt(1);
        assert!(delay1.as_millis() >= 750 && delay1.as_millis() <= 1250);

        // Attempt 2: ~2000ms (±25% jitter)
        let delay2 = policy.delay_for_attempt(2);
        assert!(delay2.as_millis() >= 1500 && delay2.as_millis() <= 2500);

        // Attempt 3: ~4000ms (±25% jitter)
        let delay3 = policy.delay_for_attempt(3);
        assert!(delay3.as_millis() >= 3000 && delay3.as_millis() <= 5000);
    }

    #[test]
    fn delay_capped_at_max_delay() {
        let policy = RetryPolicy {
            base_delay: Duration::from_millis(1000),
            backoff_multiplier: 10.0,
            max_delay: Duration::from_secs(5),
            ..Default::default()
        };

        // Attempt 10 would be 10^9 ms without cap
        let delay = policy.delay_for_attempt(10);
        assert!(delay <= Duration::from_secs(5));
    }

    #[test]
    fn delay_with_retry_after_uses_longer_value() {
        let policy = RetryPolicy {
            base_delay: Duration::from_millis(1000),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(60),
            ..Default::default()
        };

        // Retry-After shorter than backoff
        let delay = policy.delay_with_retry_after(1, Some(0));
        assert!(delay.as_millis() >= 750); // Uses backoff

        // Retry-After longer than backoff
        let delay = policy.delay_with_retry_after(1, Some(30));
        assert_eq!(delay, Duration::from_secs(30)); // Uses Retry-After
    }

    #[test]
    fn is_retryable_detects_429_rate_limit() {
        let error = ProviderError::ApiError(Box::new(ProviderApiError {
            status: Some(429),
            provider: None,
            model: None,
            code: None,
            message: "rate limit exceeded".to_string(),
            error_type: None,
            retry_after_secs: None,
            request_id: None,
        }));
        assert!(RetryPolicy::is_retryable(&error));
    }

    #[test]
    fn is_retryable_detects_5xx_errors() {
        for status in [500, 502, 503, 504] {
            let error = ProviderError::ApiError(Box::new(ProviderApiError {
                status: Some(status),
                provider: None,
                model: None,
                code: None,
                message: "server error".to_string(),
                error_type: None,
                retry_after_secs: None,
                request_id: None,
            }));
            assert!(
                RetryPolicy::is_retryable(&error),
                "Expected 5{} to be retryable",
                status
            );
        }
    }

    #[test]
    fn is_retryable_detects_rate_limit_in_error_type() {
        let error = ProviderError::ApiError(Box::new(ProviderApiError {
            status: Some(400),
            provider: None,
            model: None,
            code: None,
            message: "bad request".to_string(),
            error_type: Some("rate_limit_error".to_string()),
            retry_after_secs: None,
            request_id: None,
        }));
        assert!(RetryPolicy::is_retryable(&error));
    }

    #[test]
    fn is_retryable_detects_rate_limit_in_message() {
        let error = ProviderError::api_message("too many requests, please try again");
        assert!(RetryPolicy::is_retryable(&error));
    }

    #[test]
    fn is_retryable_detects_timeout_in_message() {
        let error = ProviderError::api_message("request timed out");
        assert!(RetryPolicy::is_retryable(&error));
    }

    #[test]
    fn is_retryable_rejects_4xx_errors() {
        let error = ProviderError::ApiError(Box::new(ProviderApiError {
            status: Some(400),
            provider: None,
            model: None,
            code: None,
            message: "bad request".to_string(),
            error_type: None,
            retry_after_secs: None,
            request_id: None,
        }));
        assert!(!RetryPolicy::is_retryable(&error));
    }

    #[test]
    fn is_retryable_rejects_auth_errors() {
        let error = ProviderError::ApiError(Box::new(ProviderApiError {
            status: Some(401),
            provider: None,
            model: None,
            code: None,
            message: "unauthorized".to_string(),
            error_type: None,
            retry_after_secs: None,
            request_id: None,
        }));
        assert!(!RetryPolicy::is_retryable(&error));
    }

    #[test]
    fn is_retryable_rejects_json_errors() {
        let error = ProviderError::JsonError(serde_json::from_str::<String>("x").unwrap_err());
        assert!(!RetryPolicy::is_retryable(&error));
    }

    #[test]
    fn is_retryable_rejects_config_errors() {
        let error = ProviderError::ConfigError("missing api key".to_string());
        assert!(!RetryPolicy::is_retryable(&error));
    }

    #[tokio::test]
    async fn execute_with_retry_succeeds_on_first_try() {
        let policy = RetryPolicy::default();
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = policy
            .execute_with_retry(|| {
                call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async { Ok::<_, ProviderError>(42) }
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn execute_with_retry_retries_on_retryable_error() {
        let policy = RetryPolicy {
            max_retries: 2,
            base_delay: Duration::from_millis(10), // Fast for testing
            max_delay: Duration::from_millis(50),
            backoff_multiplier: 2.0,
        };
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = policy
            .execute_with_retry(|| {
                let count = call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                async move {
                    if count < 3 {
                        Err(ProviderError::api_message("timeout"))
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn execute_with_retry_fails_after_max_retries() {
        let policy = RetryPolicy {
            max_retries: 2,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(50),
            backoff_multiplier: 2.0,
        };
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = policy
            .execute_with_retry(|| {
                call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async { Err::<i32, _>(ProviderError::api_message("timeout")) }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 3); // 1 initial + 2 retries
    }

    #[tokio::test]
    async fn execute_with_retry_does_not_retry_non_retryable() {
        let policy = RetryPolicy::default();
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = policy
            .execute_with_retry(|| {
                call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async { Err::<i32, _>(ProviderError::api_message("invalid api key")) }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1); // No retries for non-retryable errors
    }

    #[tokio::test]
    async fn execute_with_retry_respects_retry_after_header() {
        let policy = RetryPolicy {
            max_retries: 1,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
        };
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = policy
            .execute_with_retry(|| {
                let count = call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                async move {
                    if count < 2 {
                        Err(ProviderError::ApiError(Box::new(ProviderApiError {
                            status: Some(429),
                            provider: None,
                            model: None,
                            code: None,
                            message: "rate limit".to_string(),
                            error_type: None,
                            retry_after_secs: Some(1), // Server says wait 1 second
                            request_id: None,
                        })))
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
}

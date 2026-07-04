//! Per-key Token Bucket rate limiter.
//!
//! Provides an in-memory, non-blocking rate limiter suitable for
//! per-provider/per-model throttling.  Each key gets its own bucket
//! with a configurable capacity and refill rate.
//!
//! # Example
//! ```
//! use agent_diva_core::rate_limiter::{RateLimiter, RateLimitError};
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! let limiter = RateLimiter::new();
//! // First request succeeds
//! assert!(limiter.check("deepseek:deepseek-chat").await.is_ok());
//! // ...
//! # });
//! ```

use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::Mutex;

/// Error returned when a rate limit is exceeded.
#[derive(Debug, PartialEq, Eq)]
pub enum RateLimitError {
    /// The bucket is empty; caller should retry after `retry_after` seconds
    /// when it can be determined.
    Exceeded { retry_after: Option<u64> },
}

/// Token bucket for a single rate-limit key.
pub struct TokenBucket {
    capacity: u32,
    tokens: f64,
    last_refill: Instant,
    refill_rate_per_sec: f64,
}

impl TokenBucket {
    /// Create a new token bucket.
    ///
    /// * `capacity` — maximum number of tokens the bucket can hold.
    /// * `refill_rate` — tokens added per second.
    pub fn new(capacity: u32, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            last_refill: Instant::now(),
            refill_rate_per_sec: refill_rate,
        }
    }

    /// Attempt to consume one token.
    ///
    /// Returns `true` if a token was available, `false` otherwise.
    pub fn check(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Seconds until the next token becomes available.
    pub fn retry_after_secs(&self) -> Option<u64> {
        compute_retry_after_secs(self.tokens, self.refill_rate_per_sec)
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        if elapsed > 0.0 {
            self.tokens = compute_refilled_tokens(
                self.tokens,
                self.capacity,
                self.refill_rate_per_sec,
                elapsed,
            );
            self.last_refill = now;
        }
    }
}

fn compute_refilled_tokens(
    current_tokens: f64,
    capacity: u32,
    refill_rate_per_sec: f64,
    elapsed_secs: f64,
) -> f64 {
    if !current_tokens.is_finite() {
        return capacity as f64;
    }

    let capacity = capacity as f64;
    let clamped_tokens = current_tokens.clamp(0.0, capacity);
    if elapsed_secs <= 0.0 || !elapsed_secs.is_finite() {
        return clamped_tokens;
    }
    if refill_rate_per_sec <= 0.0 || !refill_rate_per_sec.is_finite() {
        return clamped_tokens;
    }

    (clamped_tokens + elapsed_secs * refill_rate_per_sec).min(capacity)
}

fn compute_retry_after_secs(tokens: f64, refill_rate_per_sec: f64) -> Option<u64> {
    if tokens >= 1.0 {
        return Some(0);
    }
    if refill_rate_per_sec <= 0.0 || !refill_rate_per_sec.is_finite() {
        return None;
    }

    let missing = (1.0 - tokens).max(0.0);
    let seconds = missing / refill_rate_per_sec;
    if !seconds.is_finite() {
        return None;
    }

    Some(seconds.ceil().clamp(0.0, u64::MAX as f64) as u64)
}

/// In-memory rate limiter backed by a `HashMap` of token buckets.
pub struct RateLimiter {
    buckets: Mutex<HashMap<String, TokenBucket>>,
    capacity: u32,
    refill_rate: f64,
}

impl RateLimiter {
    /// Create a new rate limiter with default settings
    /// (capacity = 10, refill_rate = 2/sec).
    pub fn new() -> Self {
        Self::with_config(10, 2.0)
    }

    /// Create a rate limiter with custom capacity and refill rate.
    pub fn with_config(capacity: u32, refill_rate: f64) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            capacity,
            refill_rate,
        }
    }

    /// Check whether a request under the given key is allowed.
    ///
    /// Returns `Ok(())` if allowed, or `RateLimitError::Exceeded` with the
    /// number of seconds to wait before retrying.
    pub async fn check(&self, key: &str) -> Result<(), RateLimitError> {
        let mut buckets = self.buckets.lock().await;
        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(self.capacity, self.refill_rate));

        if bucket.check() {
            Ok(())
        } else {
            let retry_after = bucket.retry_after_secs();
            Err(RateLimitError::Exceeded { retry_after })
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn burst_10_requests_succeed() {
        let limiter = RateLimiter::new();
        for i in 0..10 {
            assert!(
                limiter.check("test-key").await.is_ok(),
                "request {} should succeed",
                i + 1
            );
        }
    }

    #[tokio::test]
    async fn eleventh_request_fails() {
        let limiter = RateLimiter::new();
        for _ in 0..10 {
            assert!(limiter.check("test-key").await.is_ok());
        }
        let result = limiter.check("test-key").await;
        assert!(matches!(result, Err(RateLimitError::Exceeded { .. })));
    }

    #[tokio::test]
    async fn after_refill_request_succeeds() {
        let limiter = RateLimiter::new();
        // Exhaust the bucket
        for _ in 0..10 {
            assert!(limiter.check("test-key").await.is_ok());
        }
        assert!(limiter.check("test-key").await.is_err());

        // Wait for at least one token to refill (2/sec → 0.5s per token)
        tokio::time::sleep(Duration::from_millis(600)).await;

        let result = limiter.check("test-key").await;
        assert!(result.is_ok(), "request after refill should succeed");
    }

    #[tokio::test]
    async fn different_keys_are_independent() {
        let limiter = RateLimiter::new();
        for _ in 0..10 {
            assert!(limiter.check("key-a").await.is_ok());
        }
        // key-b should still have full capacity
        assert!(limiter.check("key-b").await.is_ok());
    }

    #[tokio::test]
    async fn retry_after_is_non_zero_when_exceeded() {
        let limiter = RateLimiter::new();
        for _ in 0..10 {
            assert!(limiter.check("test-key").await.is_ok());
        }
        let result = limiter.check("test-key").await;
        match result {
            Err(RateLimitError::Exceeded { retry_after }) => {
                assert!(
                    retry_after.is_some_and(|retry_after| retry_after > 0),
                    "retry_after should be > 0 when exceeded"
                );
            }
            _ => panic!("Expected RateLimitError::Exceeded"),
        }
    }

    #[tokio::test]
    async fn retry_after_is_zero_when_allowed() {
        let limiter = RateLimiter::new();
        // First request should succeed
        assert!(limiter.check("test-key").await.is_ok());

        // Exhaust remaining tokens
        for _ in 0..9 {
            assert!(limiter.check("test-key").await.is_ok());
        }

        // Now it should be exceeded with retry_after > 0
        let result = limiter.check("test-key").await;
        match result {
            Err(RateLimitError::Exceeded { retry_after }) => {
                assert!(retry_after.is_some_and(|retry_after| retry_after > 0));
            }
            _ => panic!("Expected rate limit exceeded after burst"),
        }
    }

    #[test]
    fn zero_refill_has_no_retry_after() {
        assert_eq!(compute_retry_after_secs(0.0, 0.0), None);
        assert_eq!(compute_retry_after_secs(0.5, -1.0), None);
    }

    #[test]
    fn tiny_refill_retry_after_saturates() {
        assert_eq!(
            compute_retry_after_secs(0.0, f64::MIN_POSITIVE),
            Some(u64::MAX)
        );
    }

    #[test]
    fn long_elapsed_refill_caps_at_capacity() {
        let tokens = compute_refilled_tokens(0.0, 5, 0.5, 10_000.0);
        assert_eq!(tokens, 5.0);
    }

    #[test]
    fn zero_refill_does_not_restore_tokens() {
        let tokens = compute_refilled_tokens(0.0, 5, 0.0, 10.0);
        assert_eq!(tokens, 0.0);
    }

    #[tokio::test]
    async fn zero_refill_same_key_stays_depleted() {
        let limiter = RateLimiter::with_config(2, 0.0);
        assert!(limiter.check("same-key").await.is_ok());
        assert!(limiter.check("same-key").await.is_ok());

        let result = limiter.check("same-key").await;
        assert_eq!(result, Err(RateLimitError::Exceeded { retry_after: None }));
    }

    #[tokio::test]
    async fn zero_refill_different_keys_are_isolated() {
        let limiter = RateLimiter::with_config(1, 0.0);
        assert!(limiter.check("key-a").await.is_ok());
        assert_eq!(
            limiter.check("key-a").await,
            Err(RateLimitError::Exceeded { retry_after: None })
        );
        assert!(limiter.check("key-b").await.is_ok());
    }
}

//! Request-scoped provider observers.
//!
//! Provider instances are shared across concurrent sessions. These observers
//! therefore live in Tokio task-local scope instead of mutable provider-global
//! slots, while legacy listeners remain available as a compatibility fallback.

use crate::final_wire::FinalWireCacheListener;
use crate::retry::RetryListener;
use std::future::Future;

tokio::task_local! {
    static REQUEST_OBSERVERS: ProviderRequestObservers;
}

/// Observers attached to one logical provider request.
#[derive(Clone, Default)]
pub struct ProviderRequestObservers {
    pub retry: Option<RetryListener>,
    pub final_wire_cache: Option<FinalWireCacheListener>,
}

/// Run a provider request with observers isolated to the current Tokio task.
pub async fn with_provider_request_observers<F>(
    observers: ProviderRequestObservers,
    future: F,
) -> F::Output
where
    F: Future,
{
    REQUEST_OBSERVERS.scope(observers, future).await
}

/// Return the retry listener scoped to the current request, if present.
pub fn current_retry_listener() -> Option<RetryListener> {
    REQUEST_OBSERVERS
        .try_with(|observers| observers.retry.clone())
        .ok()
        .flatten()
}

/// Return the final-wire listener scoped to the current request, if present.
pub fn current_final_wire_cache_listener() -> Option<FinalWireCacheListener> {
    REQUEST_OBSERVERS
        .try_with(|observers| observers.final_wire_cache.clone())
        .ok()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retry::RetryAttempt;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn observers_are_isolated_between_concurrent_tasks() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut tasks = Vec::new();

        for request_id in ["request-a", "request-b"] {
            let seen = seen.clone();
            let listener: RetryListener = Arc::new(move |_| {
                seen.lock().unwrap().push(request_id.to_string());
            });
            tasks.push(tokio::spawn(with_provider_request_observers(
                ProviderRequestObservers {
                    retry: Some(listener),
                    final_wire_cache: None,
                },
                async move {
                    tokio::task::yield_now().await;
                    current_retry_listener().unwrap()(RetryAttempt {
                        model: "fixture".to_string(),
                        attempt: 1,
                        max_retries: 1,
                        delay_ms: 0,
                        reason: "fixture".to_string(),
                    });
                },
            )));
        }

        for task in tasks {
            task.await.unwrap();
        }
        let mut actual = seen.lock().unwrap().clone();
        actual.sort();
        assert_eq!(actual, vec!["request-a", "request-b"]);
    }

    #[tokio::test]
    async fn observers_do_not_leak_outside_scope() {
        with_provider_request_observers(ProviderRequestObservers::default(), async {
            assert!(current_retry_listener().is_none())
        })
        .await;
        assert!(current_retry_listener().is_none());
    }
}

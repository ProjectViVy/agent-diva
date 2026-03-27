use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct PendingProviderLogin<TSession, TStatus> {
    pub provider: String,
    pub profile: String,
    pub session: TSession,
    pub status: TStatus,
}

#[async_trait]
pub trait PendingProviderLoginStore<T>: Send + Sync
where
    T: Clone + Send + Sync + 'static,
{
    async fn insert(&self, key: String, value: T);
    async fn get(&self, key: &str) -> Option<T>;
    async fn update(&self, key: &str, value: T) -> Option<T>;
    async fn remove(&self, key: &str) -> Option<T>;
}

#[derive(Debug, Clone)]
pub struct InMemoryPendingProviderLoginStore<T>
where
    T: Clone + Send + Sync + 'static,
{
    inner: Arc<Mutex<HashMap<String, T>>>,
}

impl<T> Default for InMemoryPendingProviderLoginStore<T>
where
    T: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl<T> PendingProviderLoginStore<T> for InMemoryPendingProviderLoginStore<T>
where
    T: Clone + Send + Sync + 'static,
{
    async fn insert(&self, key: String, value: T) {
        self.inner.lock().await.insert(key, value);
    }

    async fn get(&self, key: &str) -> Option<T> {
        self.inner.lock().await.get(key).cloned()
    }

    async fn update(&self, key: &str, value: T) -> Option<T> {
        self.inner.lock().await.insert(key.to_string(), value)
    }

    async fn remove(&self, key: &str) -> Option<T> {
        self.inner.lock().await.remove(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_store_round_trip() {
        let store = InMemoryPendingProviderLoginStore::<String>::default();
        store.insert("openai".into(), "pending".into()).await;
        assert_eq!(store.get("openai").await.as_deref(), Some("pending"));
        store.update("openai", "complete".into()).await;
        assert_eq!(store.get("openai").await.as_deref(), Some("complete"));
        assert_eq!(store.remove("openai").await.as_deref(), Some("complete"));
        assert!(store.get("openai").await.is_none());
    }
}

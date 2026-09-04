use crate::adapter::{AdapterError, ChannelAdapter};
use agent_diva_core::channel::{
    ChannelCapability, ChannelCommand, ChannelHealth, ChannelId, DeliveryReceipt,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub enum AdapterRegistryError {
    #[error("adapter is already registered: {0}")]
    Duplicate(ChannelId),
    #[error("adapter is not registered: {0}")]
    Unknown(ChannelId),
    #[error("adapter is still running: {0}")]
    Running(ChannelId),
    #[error("invalid channel command: {0}")]
    InvalidCommand(String),
    #[error(transparent)]
    Adapter(#[from] AdapterError),
}

/// Thread-safe native adapter registry with deterministic listing and routing.
#[derive(Default)]
pub struct AdapterRegistry {
    state: Mutex<RegistryState>,
}

#[derive(Default)]
struct RegistryState {
    adapters: BTreeMap<ChannelId, Arc<dyn ChannelAdapter>>,
    running: BTreeSet<ChannelId>,
}

impl std::fmt::Debug for AdapterRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdapterRegistry")
            .finish_non_exhaustive()
    }
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register(
        &self,
        adapter: Arc<dyn ChannelAdapter>,
    ) -> Result<(), AdapterRegistryError> {
        let id = adapter.name();
        let mut state = self.state.lock().await;
        if state.adapters.contains_key(&id) {
            return Err(AdapterRegistryError::Duplicate(id));
        }
        state.adapters.insert(id, adapter);
        Ok(())
    }

    pub async fn unregister(&self, id: &ChannelId) -> Result<(), AdapterRegistryError> {
        let mut state = self.state.lock().await;
        if state.running.contains(id) {
            return Err(AdapterRegistryError::Running(id.clone()));
        }
        state
            .adapters
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| AdapterRegistryError::Unknown(id.clone()))
    }

    pub async fn mark_running(&self, id: &ChannelId) -> Result<(), AdapterRegistryError> {
        let mut state = self.state.lock().await;
        if !state.adapters.contains_key(id) {
            return Err(AdapterRegistryError::Unknown(id.clone()));
        }
        state.running.insert(id.clone());
        Ok(())
    }

    pub async fn mark_stopped(&self, id: &ChannelId) {
        self.state.lock().await.running.remove(id);
    }

    pub async fn list(&self) -> Vec<ChannelId> {
        self.state.lock().await.adapters.keys().cloned().collect()
    }

    pub async fn get(
        &self,
        id: &ChannelId,
    ) -> Result<Arc<dyn ChannelAdapter>, AdapterRegistryError> {
        self.state
            .lock()
            .await
            .adapters
            .get(id)
            .cloned()
            .ok_or_else(|| AdapterRegistryError::Unknown(id.clone()))
    }

    pub async fn health(&self, id: &ChannelId) -> Result<ChannelHealth, AdapterRegistryError> {
        Ok(self.get(id).await?.health())
    }

    pub async fn execute_direct(
        &self,
        command: ChannelCommand,
    ) -> Result<DeliveryReceipt, AdapterRegistryError> {
        let id = command
            .target_channel()
            .map_err(|error| AdapterRegistryError::InvalidCommand(error.to_string()))?;
        let adapter = self.get(&id).await?;
        let capabilities = adapter.capabilities();
        if let Some(capability) = command
            .required_capabilities()
            .into_iter()
            .find(|capability| !capabilities.supports(*capability))
        {
            return Err(AdapterError::UnsupportedCapability { capability }.into());
        }
        adapter.execute(command).await.map_err(Into::into)
    }

    pub async fn supports(
        &self,
        id: &ChannelId,
        capability: ChannelCapability,
    ) -> Result<bool, AdapterRegistryError> {
        Ok(self.get(id).await?.capabilities().supports(capability))
    }
}

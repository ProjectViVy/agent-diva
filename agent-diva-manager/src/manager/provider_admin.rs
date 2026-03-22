use agent_diva_providers::{
    CustomProviderUpsert, ProviderCatalogService, ProviderModelCatalogView, ProviderView,
};
use tokio::sync::oneshot;

use super::Manager;

impl Manager {
    pub(super) fn handle_get_providers(&self, reply: oneshot::Sender<Vec<ProviderView>>) {
        let config = self.loader.load().unwrap_or_default();
        let providers = ProviderCatalogService::new().list_provider_views(&config);
        let _ = reply.send(providers);
    }

    pub(super) fn handle_get_provider(
        &self,
        name: String,
        reply: oneshot::Sender<Result<Option<ProviderView>, String>>,
    ) {
        let config = self.loader.load().unwrap_or_default();
        let provider = ProviderCatalogService::new().get_provider_view(&config, &name);
        let _ = reply.send(Ok(provider));
    }

    pub(super) async fn handle_get_provider_models(
        &self,
        name: String,
        runtime: bool,
        reply: oneshot::Sender<ProviderModelCatalogView>,
    ) {
        let config = self.loader.load().unwrap_or_default();
        let fallback = ProviderModelCatalogView {
            provider: name.clone(),
            catalog_source: "error".to_string(),
            runtime_supported: false,
            api_base: None,
            models: vec![],
            custom_models: vec![],
            warnings: vec![],
            error: Some(format!("Unknown provider '{}'", name)),
        };
        let response = ProviderCatalogService::new()
            .list_provider_models(&config, &name, runtime, None)
            .await
            .unwrap_or(fallback);
        let _ = reply.send(response);
    }

    pub(super) fn handle_resolve_provider(
        &self,
        model: String,
        preferred_provider: Option<String>,
        reply: oneshot::Sender<Option<String>>,
    ) {
        let config = self.loader.load().unwrap_or_default();
        let provider_id = ProviderCatalogService::new().resolve_provider_id(
            &config,
            &model,
            preferred_provider.as_deref(),
        );
        let _ = reply.send(provider_id);
    }

    pub(super) async fn handle_add_provider_model(
        &self,
        name: String,
        model: String,
        reply: oneshot::Sender<Result<(), String>>,
    ) {
        let mut config = match self.loader.load() {
            Ok(config) => config,
            Err(error) => {
                let _ = reply.send(Err(error.to_string()));
                return;
            }
        };
        let catalog = ProviderCatalogService::new();
        let result = catalog.add_provider_model(&mut config, &name, &model);
        if let Err(error) = result {
            let _ = reply.send(Err(error));
            return;
        }
        if config.agents.defaults.provider.as_deref() == Some(name.as_str()) {
            config.agents.defaults.model = model;
        }
        let response = self.loader.save(&config).map_err(|e| e.to_string());
        let _ = reply.send(response);
    }

    pub(super) async fn handle_delete_provider_model(
        &self,
        name: String,
        model_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    ) {
        let mut config = match self.loader.load() {
            Ok(config) => config,
            Err(error) => {
                let _ = reply.send(Err(error.to_string()));
                return;
            }
        };
        let catalog = ProviderCatalogService::new();
        if let Err(error) = catalog.remove_provider_model(&mut config, &name, &model_id) {
            let _ = reply.send(Err(error));
            return;
        }
        if config.agents.defaults.provider.as_deref() == Some(name.as_str())
            && config.agents.defaults.model == model_id
        {
            match catalog
                .list_provider_models(&config, &name, false, None)
                .await
            {
                Ok(models) => {
                    if let Some(next_model) = models.models.first() {
                        config.agents.defaults.model = next_model.id.clone();
                    }
                }
                Err(error) => {
                    let _ = reply.send(Err(error));
                    return;
                }
            }
        }
        let response = self.loader.save(&config).map_err(|e| e.to_string());
        let _ = reply.send(response);
    }

    pub(super) async fn handle_create_provider(
        &self,
        payload: CustomProviderUpsert,
        reply: oneshot::Sender<Result<Option<ProviderView>, String>>,
    ) {
        let _ = reply.send(self.save_provider(payload).await);
    }

    pub(super) async fn handle_update_provider(
        &self,
        name: String,
        mut payload: CustomProviderUpsert,
        reply: oneshot::Sender<Result<Option<ProviderView>, String>>,
    ) {
        payload.id = name;
        let _ = reply.send(self.save_provider(payload).await);
    }

    pub(super) async fn handle_delete_provider(
        &self,
        name: String,
        reply: oneshot::Sender<Result<(), String>>,
    ) {
        let mut config = match self.loader.load() {
            Ok(config) => config,
            Err(error) => {
                let _ = reply.send(Err(error.to_string()));
                return;
            }
        };
        let catalog = ProviderCatalogService::new();
        if let Err(error) = catalog.delete_custom_provider(&mut config, &name) {
            let _ = reply.send(Err(error));
            return;
        }
        if config.agents.defaults.provider.as_deref() == Some(name.as_str()) {
            let fallback = catalog.list_provider_views(&config).into_iter().next();
            config.agents.defaults.provider = fallback.as_ref().map(|provider| provider.id.clone());
            if let Some(provider) = fallback {
                if let Some(model) = provider.default_model {
                    config.agents.defaults.model = model;
                }
            }
        }
        let response = self.loader.save(&config).map_err(|e| e.to_string());
        let _ = reply.send(response);
    }

    async fn save_provider(
        &self,
        payload: CustomProviderUpsert,
    ) -> Result<Option<ProviderView>, String> {
        let mut config = self.loader.load().map_err(|e| e.to_string())?;
        let provider_id = payload.id.clone();
        ProviderCatalogService::new()
            .save_custom_provider(&mut config, payload)
            .map_err(|e| e.to_string())?;
        self.loader.save(&config).map_err(|e| e.to_string())?;
        Ok(ProviderCatalogService::new().get_provider_view(&config, &provider_id))
    }
}

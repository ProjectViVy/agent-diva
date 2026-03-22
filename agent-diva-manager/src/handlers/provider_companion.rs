use agent_diva_providers::{CustomProviderUpsert, ProviderModelCatalogView, ProviderView};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use tokio::sync::oneshot;

use crate::state::{AppState, ManagerCommand, ProviderCommand};

pub async fn get_providers_handler(State(state): State<AppState>) -> Json<Vec<ProviderView>> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::Provider(ProviderCommand::GetProviders(tx)))
        .await
    {
        tracing::error!("Failed to send GetProviders request: {}", e);
        return Json(vec![]);
    }
    match rx.await {
        Ok(providers) => Json(providers),
        Err(e) => {
            tracing::error!("Failed to receive GetProviders response: {}", e);
            Json(vec![])
        }
    }
}

#[derive(serde::Deserialize)]
pub struct ProviderModelsQuery {
    #[serde(default = "default_provider_runtime_query")]
    pub runtime: bool,
}

fn default_provider_runtime_query() -> bool {
    true
}

#[derive(serde::Deserialize)]
pub struct ProviderModelMutation {
    pub model: String,
}

#[derive(serde::Deserialize)]
pub struct ResolveProviderRequest {
    pub model: String,
    pub preferred_provider: Option<String>,
}

pub async fn get_provider_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::Provider(ProviderCommand::GetProvider(
            name.clone(),
            tx,
        )))
        .await
    {
        tracing::error!("Failed to send GetProvider request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(Some(provider))) => Json(serde_json::json!(provider)),
        Ok(Ok(None)) => Json(serde_json::json!({
            "status": "error",
            "message": format!("Unknown provider '{}'", name),
        })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn get_provider_models_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<ProviderModelsQuery>,
) -> Json<ProviderModelCatalogView> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::Provider(ProviderCommand::GetProviderModels(
            name.clone(),
            query.runtime,
            tx,
        )))
        .await
    {
        tracing::error!("Failed to send GetProviderModels request: {}", e);
        return Json(ProviderModelCatalogView {
            provider: name,
            catalog_source: "error".to_string(),
            runtime_supported: false,
            api_base: None,
            models: vec![],
            custom_models: vec![],
            warnings: vec![],
            error: Some(e.to_string()),
        });
    }
    match rx.await {
        Ok(view) => Json(view),
        Err(e) => Json(ProviderModelCatalogView {
            provider: name,
            catalog_source: "error".to_string(),
            runtime_supported: false,
            api_base: None,
            models: vec![],
            custom_models: vec![],
            warnings: vec![],
            error: Some(e.to_string()),
        }),
    }
}

pub async fn add_provider_model_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<ProviderModelMutation>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::Provider(ProviderCommand::AddProviderModel(
            name,
            payload.model,
            tx,
        )))
        .await
    {
        tracing::error!("Failed to send AddProviderModel request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(())) => Json(serde_json::json!({ "status": "ok" })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn delete_provider_model_handler(
    State(state): State<AppState>,
    Path((name, model_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::Provider(ProviderCommand::DeleteProviderModel(
            name,
            model_id,
            tx,
        )))
        .await
    {
        tracing::error!("Failed to send DeleteProviderModel request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(())) => Json(serde_json::json!({ "status": "ok" })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn create_provider_handler(
    State(state): State<AppState>,
    Json(payload): Json<CustomProviderUpsert>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::Provider(ProviderCommand::CreateProvider(
            payload,
            tx,
        )))
        .await
    {
        tracing::error!("Failed to send CreateProvider request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(provider)) => Json(serde_json::json!({ "status": "ok", "provider": provider })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn update_provider_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<CustomProviderUpsert>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::Provider(ProviderCommand::UpdateProvider(
            name,
            payload,
            tx,
        )))
        .await
    {
        tracing::error!("Failed to send UpdateProvider request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(provider)) => Json(serde_json::json!({ "status": "ok", "provider": provider })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn delete_provider_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::Provider(ProviderCommand::DeleteProvider(
            name,
            tx,
        )))
        .await
    {
        tracing::error!("Failed to send DeleteProvider request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }
    match rx.await {
        Ok(Ok(())) => Json(serde_json::json!({ "status": "ok" })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

pub async fn resolve_provider_handler(
    State(state): State<AppState>,
    Json(payload): Json<ResolveProviderRequest>,
) -> Json<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::Provider(ProviderCommand::ResolveProvider(
            payload.model,
            payload.preferred_provider,
            tx,
        )))
        .await
    {
        tracing::error!("Failed to send ResolveProvider request: {}", e);
        return Json(serde_json::json!({ "provider_id": null, "message": e.to_string() }));
    }
    match rx.await {
        Ok(provider_id) => Json(serde_json::json!({ "provider_id": provider_id })),
        Err(e) => Json(serde_json::json!({ "provider_id": null, "message": e.to_string() })),
    }
}

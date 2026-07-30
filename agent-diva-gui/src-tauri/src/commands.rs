use crate::app_state::AgentState;
use crate::gateway_status::GatewayStatus;
use crate::notebook::{
    build_notebook_report_proposal, build_notebook_report_proposal_preview, load_notebook_reports,
    search_notebook_session_evidence, NotebookPeriod, NotebookProposalAction,
    NotebookProposalPreviewDto, NotebookReportDto, NotebookSessionSearchRequest,
};
use crate::process_utils;
use crate::shutdown_manager::ShutdownManager;
use agent_diva_agent::mask::{MaskRegistry, ToolPolicy};
use agent_diva_cli::cli_runtime::{collect_status_report, CliRuntime, StatusReport};
use agent_diva_core::bus::PlanRuntimeState;
use agent_diva_core::config::schema::{AgentMode, SubagentDefaults, ToolLimits};
use agent_diva_core::config::{Config, ConfigLoader};
use agent_diva_core::planning::{normalize_report_markdown, revision_hash, ExecutionContextPolicy};
use agent_diva_core::session::{SessionSearchHit, SessionSearchResponse};
use agent_diva_neuron::{LlmNeuron, NeuronNode, NeuronRequest};
use agent_diva_providers::{
    build_llm_provider, CustomProviderUpsert, LlmProviderBuildOptions, Message, ProviderAccess,
    ProviderCatalogService, ProviderModelCatalogView as SharedProviderModelCatalog,
    ProviderView as SharedProviderView,
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use eventsource_stream::Eventsource;
use futures::{SinkExt, StreamExt};
use http::header::{HeaderValue, AUTHORIZATION};
use native_tls::TlsConnector;
use serde::{Deserialize, Serialize};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, Window};
use tauri_plugin_store::StoreExt;
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::timeout;
use tokio_tungstenite::connect_async_tls_with_config;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
fn configure_background_command(command: &mut TokioCommand) {
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn configure_background_command(_command: &mut TokioCommand) {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSpec {
    pub name: String,
    pub display_name: String,
    pub api_type: String,
    pub source: String,
    pub configured: bool,
    pub ready: bool,
    pub default_api_base: String,
    pub default_model: Option<String>,
    pub models: Vec<String>,
    pub custom_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModelCatalog {
    pub provider: String,
    pub source: String,
    pub runtime_supported: bool,
    pub api_base: Option<String>,
    pub models: Vec<String>,
    pub custom_models: Vec<String>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModelTestResult {
    pub ok: bool,
    pub message: String,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderPayload {
    pub id: String,
    pub display_name: String,
    pub api_type: String,
    pub api_key: String,
    pub api_base: Option<String>,
    pub default_model: Option<String>,
    pub models: Vec<String>,
    pub extra_headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfigSnapshot {
    pub provider: Option<String>,
    pub api_base: Option<String>,
    pub model: String,
    pub has_api_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSessionTitlePayload {
    pub first_user_message: String,
    pub first_assistant_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDto {
    pub name: String,
    pub description: String,
    pub source: String,
    pub available: bool,
    pub active: bool,
    pub path: String,
    pub can_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskEntryDto {
    pub name: String,
    pub icon: String,
    pub description: String,
    pub mode: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskPayload {
    pub id: Option<String>,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub mode: Option<String>,
    pub model: Option<String>,
    pub subagent_defaults: SubagentDefaults,
    pub tool_limits: ToolLimits,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAttachmentDto {
    pub file_id: String,
    pub filename: String,
    pub size: u64,
    pub mime_type: Option<String>,
    pub channel: String,
    pub message_id: Option<String>,
    pub uploaded_by: Option<String>,
    pub stored_at: String,
    pub ref_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConnectionStatusDto {
    pub state: String,
    pub connected: bool,
    pub applied: bool,
    pub tool_count: usize,
    pub error: Option<String>,
    pub checked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerDto {
    pub name: String,
    pub enabled: bool,
    pub transport: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub url: String,
    pub tool_timeout: u64,
    pub status: McpConnectionStatusDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerPayload {
    pub name: String,
    pub enabled: bool,
    pub command: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub url: String,
    pub tool_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WipeSummary {
    pub removed_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaputaApplyPayload {
    pub governance_request_id: String,
    pub expected_version: u64,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaputaDecisionPayload {
    pub decision: String,
    pub grant: String,
    pub expected_version: u64,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaputaRollbackPayload {
    pub reason: String,
    pub expected_current: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaputaEditPayload {
    pub proposed_patch: Option<String>,
    pub evidence_refs: Option<serde_json::Value>,
    pub risk_level: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaputaTransitionPayload {
    pub state: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoDreamTriggerPayload {
    pub trigger: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfEvolutionConfigPayload {
    pub enabled: bool,
    pub autodream_frequency: String,
    pub trigger_threshold_sessions: u32,
    pub trigger_threshold_messages: u32,
    pub auto_merge_confidence: f32,
    pub require_confirmation_for: Vec<String>,
}

// Manager API bridge commands. These proxy companion/runtime HTTP APIs without
// depending on manager internals from the GUI host process.
#[tauri::command]
pub async fn get_providers(state: State<'_, AgentState>) -> Result<Vec<ProviderSpec>, String> {
    let views = state.get_provider_views().await?;
    let mut providers = Vec::with_capacity(views.len());
    for view in views {
        let models = state
            .get_provider_model_catalog(&view.id, false)
            .await
            .map(provider_models_from_catalog)
            .unwrap_or_default();
        providers.push(provider_spec_from_view(view, models.0, models.1));
    }

    Ok(providers)
}

#[tauri::command]
pub async fn create_custom_provider(
    payload: CustomProviderPayload,
    state: State<'_, AgentState>,
) -> Result<ProviderSpec, String> {
    let provider_id = payload.id.trim().to_string();
    let view = state
        .create_custom_provider(&CustomProviderUpsert {
            id: payload.id,
            display_name: payload.display_name,
            api_type: payload.api_type,
            api_key: payload.api_key,
            api_base: payload.api_base,
            default_model: payload.default_model,
            models: payload.models,
            extra_headers: payload.extra_headers,
        })
        .await
        .and_then(|provider| {
            provider.ok_or_else(|| format!("provider '{provider_id}' not found after save"))
        })?;
    let models = state
        .get_provider_model_catalog(&view.id, false)
        .await
        .map(provider_models_from_catalog)
        .unwrap_or_default();

    Ok(provider_spec_from_view(view, models.0, models.1))
}

#[tauri::command]
pub async fn delete_custom_provider(
    provider: String,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.delete_custom_provider(provider.trim()).await
}

#[tauri::command]
pub async fn add_provider_model(
    provider: String,
    model: String,
    state: State<'_, AgentState>,
) -> Result<ProviderModelCatalog, String> {
    let provider_id = provider.trim().to_string();
    let model_id = model.trim().to_string();
    state.add_provider_model(&provider_id, &model_id).await?;
    let updated = state
        .get_provider_model_catalog(&provider_id, false)
        .await?;
    Ok(provider_model_catalog_dto(updated))
}

#[tauri::command]
pub async fn remove_provider_model(
    provider: String,
    model: String,
    state: State<'_, AgentState>,
) -> Result<ProviderModelCatalog, String> {
    let provider_id = provider.trim().to_string();
    let model_id = model.trim().to_string();
    state.remove_provider_model(&provider_id, &model_id).await?;
    let updated = state
        .get_provider_model_catalog(&provider_id, false)
        .await?;
    Ok(provider_model_catalog_dto(updated))
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
pub async fn get_plans(state: State<'_, AgentState>) -> Result<serde_json::Value, String> {
    let reports = get_plan_reports(state).await?;
    let summaries = reports
        .as_array()
        .map(|reports| {
            reports
                .iter()
                .map(plan_report_summary_projection)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(serde_json::Value::Array(summaries))
}

#[tauri::command]
pub async fn get_plan(
    #[allow(non_snake_case)] planId: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let reports = get_plan_reports(state).await?;
    let plan_id = planId.trim();
    let detail = reports
        .as_array()
        .and_then(|reports| {
            reports
                .iter()
                .find(|report| plan_report_id(report) == Some(plan_id))
                .cloned()
        })
        .map(|report| plan_report_detail_projection(&report))
        .unwrap_or(serde_json::Value::Null);
    Ok(detail)
}

#[tauri::command]
pub async fn delete_plan(
    #[allow(non_snake_case)] planId: String,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    let url = format!(
        "{}/plans/{}",
        state.api_base_url(),
        urlencoding::encode(planId.trim())
    );
    let response = state
        .client
        .delete(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to delete plan: {}", e))?;
    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Failed to delete plan ({}): {}", status, body))
    }
}

async fn set_plan_todo_state(
    plan_id: String,
    todo_id: String,
    action: &str,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    let url = format!(
        "{}/plans/{}/todos/{}/{}",
        state.api_base_url(),
        urlencoding::encode(plan_id.trim()),
        urlencoding::encode(todo_id.trim()),
        action
    );
    let response = state
        .client
        .post(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to {} plan todo: {}", action, e))?;
    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!(
            "Failed to {} plan todo ({}): {}",
            action, status, body
        ))
    }
}

#[tauri::command]
pub async fn delete_plan_todo(
    #[allow(non_snake_case)] planId: String,
    #[allow(non_snake_case)] todoId: String,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    set_plan_todo_state(planId, todoId, "delete", state).await
}

#[tauri::command]
pub async fn restore_plan_todo(
    #[allow(non_snake_case)] planId: String,
    #[allow(non_snake_case)] todoId: String,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    set_plan_todo_state(planId, todoId, "restore", state).await
}

#[tauri::command]
pub async fn get_active_plan(
    #[allow(non_snake_case)] sessionKey: Option<String>,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let reports = get_plan_reports(state).await?;
    let session_key = sessionKey
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty());
    let active = reports
        .as_array()
        .and_then(|reports| {
            reports
                .iter()
                .find(|report| {
                    report_matches_session(report, session_key)
                        && plan_report_status(report) == "AwaitingApproval"
                })
                .or_else(|| {
                    reports.iter().find(|report| {
                        report_matches_session(report, session_key)
                            && plan_report_status(report) == "Approved"
                    })
                })
        })
        .map(plan_report_detail_projection)
        .unwrap_or(serde_json::Value::Null);
    Ok(active)
}

#[tauri::command]
pub async fn laputa_get_snapshot(
    since: Option<String>,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let mut url = format!("{}/laputa/snapshot", state.api_base_url());
    if let Some(since) = non_empty_query_value(since) {
        url.push_str(&format!("?since={}", urlencoding::encode(&since)));
    }
    get_laputa_payload(&state, &url, "snapshot").await
}

#[tauri::command]
pub async fn laputa_get_section(
    name: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!(
        "{}/laputa/section/{}",
        state.api_base_url(),
        urlencoding::encode(name.trim())
    );
    get_laputa_payload(&state, &url, "section").await
}

#[tauri::command]
pub async fn laputa_write_section(
    name: String,
    content: String,
    #[allow(non_snake_case)] summary: Option<String>,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!(
        "{}/laputa/section/{}/write",
        state.api_base_url(),
        urlencoding::encode(name.trim())
    );
    let payload = serde_json::json!({
        "content": content,
        "actor": "gui-user",
        "summary": summary,
    });
    post_laputa_full_response(&state, &url, &payload).await
}

#[tauri::command]
pub async fn laputa_list_proposals(
    since: Option<String>,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let mut url = format!("{}/laputa/proposals", state.api_base_url());
    if let Some(since) = non_empty_query_value(since) {
        url.push_str(&format!("?since={}", urlencoding::encode(&since)));
    }
    let mut response = get_laputa_full_response(&state, &url).await?;
    let governance = response
        .get("governance")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let proposals = response
        .get_mut("proposals")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| laputa_string_error("Laputa response missing proposals".into()))?;
    for proposal in proposals.iter_mut() {
        if let Some(id) = proposal.get("id").and_then(serde_json::Value::as_str) {
            if let Some(view) = governance.get(id) {
                proposal["governance"] = view.clone();
            }
        }
    }
    Ok(serde_json::Value::Array(std::mem::take(proposals)))
}

#[tauri::command]
pub async fn laputa_create_proposal(
    payload: serde_json::Value,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!("{}/laputa/proposals", state.api_base_url());
    post_laputa_payload(&state, &url, &payload, "proposal").await
}

#[tauri::command]
pub async fn laputa_get_proposal(
    id: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!(
        "{}/laputa/proposals/{}",
        state.api_base_url(),
        urlencoding::encode(id.trim())
    );
    let response = get_laputa_full_response(&state, &url).await?;
    let mut proposal = response
        .get("proposal")
        .cloned()
        .ok_or_else(|| laputa_string_error("Laputa response missing proposal".into()))?;
    if let Some(governance) = response.get("governance") {
        proposal["governance"] = governance.clone();
    }
    Ok(proposal)
}

#[tauri::command]
pub async fn laputa_apply_proposal(
    id: String,
    payload: LaputaApplyPayload,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!(
        "{}/laputa/proposals/{}/apply",
        state.api_base_url(),
        urlencoding::encode(id.trim())
    );
    let payload = serde_json::json!({
        "governance_request_id": payload.governance_request_id,
        "expected_version": payload.expected_version,
        "idempotency_key": payload.idempotency_key,
    });
    post_laputa_full_response(&state, &url, &payload).await
}

#[tauri::command]
pub async fn laputa_decide_proposal(
    id: String,
    payload: LaputaDecisionPayload,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!(
        "{}/laputa/proposals/{}/decision",
        state.api_base_url(),
        urlencoding::encode(id.trim())
    );
    let payload = serde_json::json!({
        "decision": payload.decision,
        "grant": payload.grant,
        "expected_version": payload.expected_version,
        "idempotency_key": payload.idempotency_key,
    });
    post_laputa_full_response(&state, &url, &payload).await
}

#[tauri::command]
pub async fn laputa_edit_proposal(
    id: String,
    payload: LaputaEditPayload,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!(
        "{}/laputa/proposals/{}",
        state.api_base_url(),
        urlencoding::encode(id.trim())
    );
    let payload = serde_json::json!({
        "proposed_patch": payload.proposed_patch,
        "evidence_refs": payload.evidence_refs,
        "risk_level": payload.risk_level,
        "updated_at": payload.updated_at,
    });
    let response = put_laputa_full_response(&state, &url, &payload).await?;
    proposal_with_governance(&response)
}

#[tauri::command]
pub async fn laputa_transition_proposal(
    id: String,
    payload: LaputaTransitionPayload,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!(
        "{}/laputa/proposals/{}/transition",
        state.api_base_url(),
        urlencoding::encode(id.trim())
    );
    let payload = serde_json::json!({
        "state": payload.state,
        "updated_at": payload.updated_at,
    });
    post_laputa_payload(&state, &url, &payload, "proposal").await
}

#[tauri::command]
pub async fn laputa_list_changelog(
    page: Option<usize>,
    page_size: Option<usize>,
    proposal_id: Option<String>,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let mut query = Vec::new();
    if let Some(page) = page {
        query.push(format!("page={page}"));
    }
    if let Some(page_size) = page_size {
        query.push(format!("page_size={page_size}"));
    }
    if let Some(proposal_id) = non_empty_query_value(proposal_id) {
        query.push(format!("proposal_id={}", urlencoding::encode(&proposal_id)));
    }
    let suffix = if query.is_empty() {
        String::new()
    } else {
        format!("?{}", query.join("&"))
    };
    let url = format!("{}/laputa/changelog{}", state.api_base_url(), suffix);
    get_laputa_payload(&state, &url, "changelog").await
}

#[tauri::command]
pub async fn laputa_get_changelog(
    id: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!(
        "{}/laputa/changelog/{}",
        state.api_base_url(),
        urlencoding::encode(id.trim())
    );
    get_laputa_payload(&state, &url, "record").await
}

#[tauri::command]
pub async fn laputa_rollback_changelog(
    id: String,
    payload: LaputaRollbackPayload,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!(
        "{}/laputa/changelog/{}/rollback",
        state.api_base_url(),
        urlencoding::encode(id.trim())
    );
    post_laputa_payload(&state, &url, &payload, "outcome").await
}

#[tauri::command]
pub async fn laputa_poll_events(
    kind: String,
    since: Option<String>,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let mut url = format!(
        "{}/laputa/events/{}/poll",
        state.api_base_url(),
        urlencoding::encode(kind.trim())
    );
    if let Some(since) = non_empty_query_value(since) {
        url.push_str(&format!("?since={}", urlencoding::encode(&since)));
    }
    get_laputa_payload(&state, &url, "events").await
}

#[tauri::command]
pub async fn trigger_autodream(
    payload: Option<AutoDreamTriggerPayload>,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!("{}/autodream/runs", state.api_base_url());
    let payload = payload.unwrap_or(AutoDreamTriggerPayload {
        trigger: Some("manual".to_string()),
    });
    post_laputa_payload(&state, &url, &payload, "run").await
}

#[tauri::command]
pub async fn get_notebook_reports(period: String) -> Result<Vec<NotebookReportDto>, String> {
    let loader = ConfigLoader::new();
    let config = loader.load().unwrap_or_default();
    let runtime = CliRuntime::from_paths(None, Some(loader.config_dir().to_path_buf()), None);
    let workspace = runtime.effective_workspace(&config);
    let period = NotebookPeriod::parse(&period)?;
    load_notebook_reports(&workspace, period)
}

#[tauri::command]
pub async fn trigger_notebook_report_generation(
    period: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let period = NotebookPeriod::parse(&period)?;
    match period {
        NotebookPeriod::Daily | NotebookPeriod::Weekly | NotebookPeriod::Monthly => {
            let trigger = match period {
                NotebookPeriod::Daily => "notebook-daily",
                NotebookPeriod::Weekly => "notebook-weekly",
                NotebookPeriod::Monthly => "notebook-monthly",
            };
            let url = format!("{}/autodream/runs", state.api_base_url());
            post_laputa_payload(
                &state,
                &url,
                &AutoDreamTriggerPayload {
                    trigger: Some(trigger.to_string()),
                },
                "run",
            )
            .await
            .map_err(laputa_error_message)
        }
    }
}

#[tauri::command]
pub async fn preview_notebook_report_proposal(
    report_id: String,
    action: String,
    session_hits: Option<Vec<SessionSearchHit>>,
) -> Result<NotebookProposalPreviewDto, String> {
    let loader = ConfigLoader::new();
    let config = loader.load().unwrap_or_default();
    let runtime = CliRuntime::from_paths(None, Some(loader.config_dir().to_path_buf()), None);
    let workspace = runtime.effective_workspace(&config);
    let action = NotebookProposalAction::parse(&action)?;
    build_notebook_report_proposal_preview(&workspace, &report_id, action, session_hits)
}

#[tauri::command]
pub async fn create_notebook_report_proposal(
    report_id: String,
    action: String,
    session_hits: Option<Vec<SessionSearchHit>>,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let loader = ConfigLoader::new();
    let config = loader.load().unwrap_or_default();
    let runtime = CliRuntime::from_paths(None, Some(loader.config_dir().to_path_buf()), None);
    let workspace = runtime.effective_workspace(&config);
    let action = NotebookProposalAction::parse(&action).map_err(laputa_string_error)?;
    let proposal =
        build_notebook_report_proposal(&workspace, &report_id, action, "notebook", session_hits)
            .map_err(laputa_string_error)?;
    let url = format!("{}/laputa/proposals", state.api_base_url());
    post_laputa_payload(&state, &url, &proposal, "proposal").await
}

#[tauri::command]
pub async fn search_notebook_session_evidence_command(
    request: NotebookSessionSearchRequest,
) -> Result<SessionSearchResponse, String> {
    let loader = ConfigLoader::new();
    let config = loader.load().unwrap_or_default();
    let runtime = CliRuntime::from_paths(None, Some(loader.config_dir().to_path_buf()), None);
    let workspace = runtime.effective_workspace(&config);
    search_notebook_session_evidence(&workspace, request)
}

#[tauri::command]
pub async fn get_autodream_run_status(
    id: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!(
        "{}/autodream/runs/{}",
        state.api_base_url(),
        urlencoding::encode(id.trim())
    );
    get_laputa_payload(&state, &url, "run").await
}

#[tauri::command]
pub async fn cancel_autodream_run(
    id: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!(
        "{}/autodream/runs/{}/cancel",
        state.api_base_url(),
        urlencoding::encode(id.trim())
    );
    post_laputa_payload(&state, &url, &serde_json::json!({}), "run").await
}

#[tauri::command]
pub async fn list_autodream_run_records(
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!("{}/autodream/runs", state.api_base_url());
    get_laputa_payload(&state, &url, "runs").await
}

#[tauri::command]
pub async fn list_recall_feedback(
    limit: Option<usize>,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!(
        "{}/laputa/recall-feedback?limit={}",
        state.api_base_url(),
        limit.unwrap_or(50).min(200)
    );
    get_laputa_payload(&state, &url, "feedback").await
}

#[tauri::command]
pub async fn get_evolution_health(
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!("{}/health", state.api_base_url());
    let response = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|error| laputa_string_error(format!("health request failed: {error}")))?;
    response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| laputa_string_error(format!("health response invalid: {error}")))
}

#[tauri::command]
pub async fn get_self_evolution_config(
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!("{}/config/self-evolution", state.api_base_url());
    get_laputa_payload(&state, &url, "config").await
}

#[tauri::command]
pub async fn save_self_evolution_config(
    config: SelfEvolutionConfigPayload,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, serde_json::Value> {
    let url = format!("{}/config/self-evolution", state.api_base_url());
    post_laputa_payload(&state, &url, &config, "config").await
}

fn non_empty_query_value(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn laputa_error_message(value: serde_json::Value) -> String {
    value
        .get("message")
        .and_then(|message| message.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| value.to_string())
}

fn laputa_string_error(message: String) -> serde_json::Value {
    serde_json::json!({ "status": "error", "message": message })
}

async fn get_laputa_payload(
    state: &State<'_, AgentState>,
    url: &str,
    field: &str,
) -> Result<serde_json::Value, serde_json::Value> {
    let response = state
        .client
        .get(url)
        .send()
        .await
        .map_err(|e| laputa_transport_error(format!("Failed to fetch Laputa API: {e}")))?;
    parse_laputa_response(response, field).await
}

async fn get_laputa_full_response(
    state: &State<'_, AgentState>,
    url: &str,
) -> Result<serde_json::Value, serde_json::Value> {
    let response = state
        .client
        .get(url)
        .send()
        .await
        .map_err(|e| laputa_transport_error(format!("Failed to fetch Laputa API: {e}")))?;
    parse_laputa_response(response, "").await
}

async fn post_laputa_payload<T: Serialize + ?Sized>(
    state: &State<'_, AgentState>,
    url: &str,
    payload: &T,
    field: &str,
) -> Result<serde_json::Value, serde_json::Value> {
    let response = state
        .client
        .post(url)
        .json(payload)
        .send()
        .await
        .map_err(|e| laputa_transport_error(format!("Failed to call Laputa API: {e}")))?;
    parse_laputa_response(response, field).await
}

async fn put_laputa_full_response<T: Serialize + ?Sized>(
    state: &State<'_, AgentState>,
    url: &str,
    payload: &T,
) -> Result<serde_json::Value, serde_json::Value> {
    let response = state
        .client
        .put(url)
        .json(payload)
        .send()
        .await
        .map_err(|e| laputa_transport_error(format!("Failed to call Laputa API: {e}")))?;
    parse_laputa_response(response, "").await
}

fn proposal_with_governance(
    response: &serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
    let mut proposal = response
        .get("proposal")
        .cloned()
        .ok_or_else(|| laputa_string_error("Laputa response missing proposal".into()))?;
    if let Some(governance) = response.get("governance") {
        proposal["governance"] = governance.clone();
    }
    Ok(proposal)
}

async fn post_laputa_full_response<T: Serialize + ?Sized>(
    state: &State<'_, AgentState>,
    url: &str,
    payload: &T,
) -> Result<serde_json::Value, serde_json::Value> {
    let response = state
        .client
        .post(url)
        .json(payload)
        .send()
        .await
        .map_err(|e| laputa_transport_error(format!("Failed to call Laputa API: {e}")))?;
    parse_laputa_response(response, "").await
}

async fn parse_laputa_response(
    response: reqwest::Response,
    field: &str,
) -> Result<serde_json::Value, serde_json::Value> {
    let status = response.status();
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| laputa_transport_error(format!("Invalid Laputa API response: {e}")))?;
    if !status.is_success() || value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(serde_json::json!({
            "status": "error",
            "http_status": status.as_u16(),
            "code": value
                .get("code")
                .and_then(|v| v.as_str())
                .unwrap_or("laputa_error"),
            "message": value
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown Laputa API error"),
            "body": value,
        }));
    }
    if field.is_empty() {
        return Ok(value);
    }
    Ok(value.get(field).cloned().unwrap_or(serde_json::Value::Null))
}

fn laputa_transport_error(message: String) -> serde_json::Value {
    serde_json::json!({
        "status": "error",
        "code": "transport_error",
        "message": message,
    })
}

#[derive(Deserialize, Serialize, Clone)]
struct ToolStartEvent {
    name: String,
    #[serde(alias = "args")]
    args_preview: String,
    #[serde(alias = "id")]
    call_id: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
struct ToolFinishEvent {
    name: String,
    result: String,
    #[serde(alias = "error")]
    is_error: Option<bool>,
    #[serde(alias = "id")]
    call_id: Option<String>,
}

#[derive(Deserialize)]
struct ToolDeltaEvent {
    delta: String,
}

#[derive(Deserialize)]
struct BackgroundFinalEvent {
    content: String,
}

#[derive(Serialize, Clone)]
struct StreamTextPayload {
    request_id: String,
    data: String,
}

#[derive(Serialize, Clone)]
struct StreamToolStartPayload {
    request_id: String,
    name: String,
    args_preview: String,
    call_id: Option<String>,
}

#[derive(Serialize, Clone)]
struct StreamToolFinishPayload {
    request_id: String,
    name: String,
    result: String,
    is_error: Option<bool>,
    call_id: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
struct PlanStreamEvent {
    plan: PlanRuntimeState,
    todo: Option<agent_diva_core::bus::PlanRuntimeTodo>,
}

#[derive(Deserialize, Serialize, Clone)]
struct TurnPlanUpdatedEvent {
    explanation: Option<String>,
    plan: Vec<TurnPlanItem>,
}

#[derive(Deserialize, Serialize, Clone)]
struct TurnPlanItem {
    step: String,
    status: String,
}

#[derive(Serialize, Clone)]
struct StreamPlanPayload {
    request_id: String,
    data: PlanStreamEvent,
}

#[derive(Serialize, Clone)]
struct StreamTurnPlanPayload {
    request_id: String,
    data: TurnPlanUpdatedEvent,
}

#[derive(Serialize, Clone)]
struct StreamJsonPayload {
    request_id: String,
    data: serde_json::Value,
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn send_message(
    message: String,
    channel: Option<String>,
    #[allow(non_snake_case)] chatId: Option<String>,
    attachments: Option<Vec<String>>,
    mode: Option<String>,
    execution: Option<serde_json::Value>,
    #[allow(non_snake_case)] streamRequestId: String,
    window: Window,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    // Tauri v2 uses camelCase from frontend, convert to snake_case internally
    let chat_id = chatId;
    let stream_request_id = streamRequestId;
    info!("Sending message to API: {}", message);
    info!("Attachments: {:?}", attachments);

    let client = &state.client;
    let url = format!("{}/chat", state.api_base_url());

    let response = client
        .post(&url)
        .json(&serde_json::json!({
            "message": message,
            "channel": channel,
            "chat_id": chat_id,
            "attachments": attachments,
            "mode": mode,
            "execution_start": execution.as_ref().map(|_| true),
            "plan_id": execution.as_ref().and_then(|value| value.get("plan_id")),
            "plan_revision": execution.as_ref().and_then(|value| value.get("revision")),
            "execution_id": execution.as_ref().and_then(|value| value.get("execution_id"))
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to agent server: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let mut stream = response.bytes_stream().eventsource();

    while let Some(event) = stream.next().await {
        match event {
            Ok(event) => {
                match event.event.as_str() {
                    "delta" => {
                        let _ = window.emit(
                            "agent-response-delta",
                            StreamTextPayload {
                                request_id: stream_request_id.clone(),
                                data: event.data,
                            },
                        );
                    }
                    "reasoning_delta" => {
                        let _ = window.emit(
                            "agent-reasoning-delta",
                            StreamTextPayload {
                                request_id: stream_request_id.clone(),
                                data: event.data,
                            },
                        );
                    }
                    "tool_delta" => {
                        if let Ok(data) = serde_json::from_str::<ToolDeltaEvent>(&event.data) {
                            let _ = window.emit(
                                "agent-tool-delta",
                                StreamTextPayload {
                                    request_id: stream_request_id.clone(),
                                    data: data.delta,
                                },
                            );
                        }
                    }
                    "final" => {
                        let _ = window.emit(
                            "agent-response-complete",
                            StreamTextPayload {
                                request_id: stream_request_id.clone(),
                                data: event.data,
                            },
                        );
                    }
                    "tool_start" => {
                        if let Ok(data) = serde_json::from_str::<ToolStartEvent>(&event.data) {
                            let _ = window.emit(
                                "agent-tool-start",
                                StreamToolStartPayload {
                                    request_id: stream_request_id.clone(),
                                    name: data.name,
                                    args_preview: data.args_preview,
                                    call_id: data.call_id,
                                },
                            );
                        } else {
                            // Fallback if parsing fails
                            let _ = window.emit(
                                "agent-tool-start",
                                StreamToolStartPayload {
                                    request_id: stream_request_id.clone(),
                                    name: "unknown".to_string(),
                                    args_preview: event.data,
                                    call_id: None,
                                },
                            );
                        }
                    }
                    "tool_finish" => {
                        if let Ok(data) = serde_json::from_str::<ToolFinishEvent>(&event.data) {
                            let _ = window.emit(
                                "agent-tool-end",
                                StreamToolFinishPayload {
                                    request_id: stream_request_id.clone(),
                                    name: data.name,
                                    result: data.result,
                                    is_error: data.is_error,
                                    call_id: data.call_id,
                                },
                            );
                        } else {
                            let _ = window.emit(
                                "agent-tool-end",
                                StreamToolFinishPayload {
                                    request_id: stream_request_id.clone(),
                                    name: "unknown".to_string(),
                                    result: event.data,
                                    is_error: Some(false),
                                    call_id: None,
                                },
                            );
                        }
                    }
                    "error" => {
                        let _ = window.emit(
                            "agent-error",
                            StreamTextPayload {
                                request_id: stream_request_id.clone(),
                                data: event.data,
                            },
                        );
                    }
                    "todo_created" => {
                        if let Ok(data) = serde_json::from_str::<PlanStreamEvent>(&event.data) {
                            let _ = window.emit(
                                "agent-plan-todo-created",
                                StreamPlanPayload {
                                    request_id: stream_request_id.clone(),
                                    data,
                                },
                            );
                        }
                    }
                    "todo_step_updated" => {
                        if let Ok(data) = serde_json::from_str::<PlanStreamEvent>(&event.data) {
                            let _ = window.emit(
                                "agent-plan-todo-updated",
                                StreamPlanPayload {
                                    request_id: stream_request_id.clone(),
                                    data,
                                },
                            );
                        }
                    }
                    "todo_completed" => {
                        if let Ok(data) = serde_json::from_str::<PlanStreamEvent>(&event.data) {
                            let _ = window.emit(
                                "agent-plan-todo-completed",
                                StreamPlanPayload {
                                    request_id: stream_request_id.clone(),
                                    data,
                                },
                            );
                        }
                    }
                    "todo_cancelled" => {
                        if let Ok(data) = serde_json::from_str::<PlanStreamEvent>(&event.data) {
                            let _ = window.emit(
                                "agent-plan-todo-cancelled",
                                StreamPlanPayload {
                                    request_id: stream_request_id.clone(),
                                    data,
                                },
                            );
                        }
                    }
                    "plan_ready_for_approval" => {
                        if let Ok(data) = serde_json::from_str::<PlanStreamEvent>(&event.data) {
                            let _ = window.emit(
                                "agent-plan-ready",
                                StreamPlanPayload {
                                    request_id: stream_request_id.clone(),
                                    data,
                                },
                            );
                        }
                    }
                    "plan_report_ready_for_approval" => {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&event.data) {
                            let _ = window.emit(
                                "agent-plan-report-ready",
                                StreamJsonPayload {
                                    request_id: stream_request_id.clone(),
                                    data,
                                },
                            );
                        }
                    }
                    "turn_plan_updated" => {
                        if let Ok(data) = serde_json::from_str::<TurnPlanUpdatedEvent>(&event.data)
                        {
                            let _ = window.emit(
                                "agent-turn-plan-updated",
                                StreamTurnPlanPayload {
                                    request_id: stream_request_id.clone(),
                                    data,
                                },
                            );
                        }
                    }
                    _ => {}
                }
            }
            Err(e) => {
                error!("Stream error: {}", e);
                let _ = window.emit(
                    "agent-error",
                    StreamTextPayload {
                        request_id: stream_request_id.clone(),
                        data: e.to_string(),
                    },
                );
            }
        }
    }

    Ok(())
}

/// Continues a previously approved plan without requiring the frontend to
/// synthesize a visible user chat message.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn continue_approved_plan_execution(
    channel: Option<String>,
    #[allow(non_snake_case)] chatId: Option<String>,
    #[allow(non_snake_case)] streamRequestId: String,
    plan_id: Option<String>,
    revision: Option<i64>,
    execution_id: Option<String>,
    window: Window,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    send_message(
        "Continue the approved plan execution from its persisted plan and execution context."
            .to_string(),
        channel,
        chatId,
        None,
        Some("agent".to_string()),
        Some(serde_json::json!({
            "plan_id": plan_id,
            "revision": revision,
            "execution_id": execution_id,
        })),
        streamRequestId,
        window,
        state,
    )
    .await
}

#[tauri::command]
pub async fn approve_active_plan_execution(
    request: serde_json::Value,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let report_id = request
        .get("plan_id")
        .or_else(|| request.get("planId"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "plan_id is required".to_string())?;
    let session_key = request
        .get("session_key")
        .or_else(|| request.get("sessionKey"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "session_key is required".to_string())?;
    let revision = request
        .get("expected_revision")
        .or_else(|| request.get("expectedRevision"))
        .and_then(|value| value.as_i64())
        .ok_or_else(|| "expected_revision is required".to_string())?;
    let markdown = request
        .get("markdown")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "markdown is required".to_string())?;
    // Agent stores normalize_report_markdown(body). Display paths may trim the
    // body (stripping the trailing newline), which must not change the hash.
    let markdown = normalize_report_markdown(markdown);
    let context_policy = match request
        .get("context_policy")
        .or_else(|| request.get("contextPolicy"))
        .and_then(|value| value.as_str())
        .unwrap_or("compact")
    {
        "retain" | "Retain" => ExecutionContextPolicy::Retain,
        "clear" | "Clear" => ExecutionContextPolicy::Clear,
        _ => ExecutionContextPolicy::Compact,
    };
    let todo_policy = request
        .get("todo_policy")
        .or_else(|| request.get("todoPolicy"))
        .and_then(|value| value.as_str())
        .unwrap_or("Optional");
    if !matches!(todo_policy, "Never" | "Optional" | "Always") {
        return Err("todo_policy must be Never, Optional, or Always".to_string());
    }
    let materialize_todos = request
        .get("materialize_todos")
        .or_else(|| request.get("materializeTodos"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let payload = serde_json::json!({
        "session_key": session_key,
        "revision": revision,
        "revision_hash": revision_hash(&markdown),
        "context_policy": context_policy,
        "compacted_context": null,
        "todo_policy": todo_policy,
        "materialize_todos": materialize_todos,
    });
    let execution = approve_plan_report(report_id.to_string(), payload, state.clone()).await?;
    let todos = if let Some(execution_id) = execution.get("id").and_then(|value| value.as_str()) {
        let url = format!(
            "{}/plan-executions/{}/todos",
            state.api_base_url(),
            execution_id
        );
        state
            .client
            .get(&url)
            .send()
            .await
            .map_err(|error| format!("Failed to load approved execution TODOs: {error}"))?
            .json::<serde_json::Value>()
            .await
            .map_err(|error| format!("Invalid execution TODO response: {error}"))?
            .get("todos")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]))
    } else {
        serde_json::json!([])
    };

    let plan = serde_json::json!({
        "plan_id": report_id, "revision": revision, "phase": "Execute",
        "status": "InProgress", "markdown": markdown, "summary": markdown,
        "strategy": markdown, "steps": [], "todos": todos,
        "execution_id": execution.get("id"),
        "initialization_status": execution.get("initialization_status"),
        "initialization_error": execution.get("initialization_error")
    });

    let approved_at = execution
        .get("created_at")
        .cloned()
        .unwrap_or_else(|| serde_json::json!(chrono::Utc::now().to_rfc3339()));

    Ok(serde_json::json!({
        "plan": plan,
        "receipt": {
            "plan_id": report_id,
            "revision": revision,
            "approved_by": "desktop-ui",
            "approved_at": approved_at,
            "todo_policy": todo_policy,
            "todos_materialized": todo_policy == "Always"
                || (todo_policy == "Optional" && materialize_todos),
        },
    }))
}

#[tauri::command]
pub async fn return_active_plan_to_draft(
    #[allow(non_snake_case)] sessionKey: Option<String>,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    // GUI only needs the JSON shape; do not force PlanRuntimeState enum decode
    // (report status AwaitingApproval is not a PlanStatus variant).
    let value = get_active_plan(sessionKey, state).await?;
    if value.is_null() {
        return Err("No active plan to return to draft".to_string());
    }
    Ok(value)
}

#[tauri::command]
pub async fn get_plan_reports(state: State<'_, AgentState>) -> Result<serde_json::Value, String> {
    let url = format!("{}/plan-reports", state.api_base_url());
    let value: serde_json::Value = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|error| format!("Failed to get plan reports: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Invalid plan report response: {error}"))?;
    value
        .get("reports")
        .cloned()
        .ok_or_else(|| "Missing plan reports payload".to_string())
}

fn plan_report_id(report: &serde_json::Value) -> Option<&str> {
    report
        .pointer("/report/id")
        .and_then(|value| value.as_str())
        .or_else(|| {
            report
                .pointer("/report/id/0")
                .and_then(|value| value.as_str())
        })
}

fn report_matches_session(report: &serde_json::Value, session_key: Option<&str>) -> bool {
    session_key
        .map(|key| {
            report
                .pointer("/report/session_key")
                .and_then(|value| value.as_str())
                == Some(key)
        })
        .unwrap_or(true)
}

#[cfg(test)]
mod plan_session_tests {
    use super::report_matches_session;

    #[test]
    fn active_plan_matching_is_limited_to_the_requested_session() {
        let report = serde_json::json!({
            "report": { "session_key": "gui:plan-chat" }
        });

        assert!(report_matches_session(&report, Some("gui:plan-chat")));
        assert!(!report_matches_session(&report, Some("gui:agent-chat")));
        assert!(report_matches_session(&report, None));
    }
}

fn plan_report_revision(report: &serde_json::Value) -> Option<i64> {
    report
        .pointer("/revision/revision")
        .and_then(|value| value.as_i64())
}

fn plan_report_title(report: &serde_json::Value) -> String {
    let raw = report
        .pointer("/revision/title")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let cleaned: String = raw.chars().filter(|ch| *ch != '\u{FFFD}').collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        // Fall back to first markdown H1.
        if let Some(title) = plan_report_markdown(report)
            .lines()
            .find_map(|line| line.trim().strip_prefix("# "))
        {
            let t: String = title.chars().filter(|ch| *ch != '\u{FFFD}').collect();
            let t = t.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
        return "计划报告".to_string();
    }
    // Recover truncated mojibake titles such as "计报告".
    if cleaned.contains('报') && cleaned.chars().count() < 4 {
        return "计划报告".to_string();
    }
    cleaned.to_string()
}

fn plan_report_markdown(report: &serde_json::Value) -> String {
    report
        .pointer("/revision/markdown")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string()
}

fn plan_report_status(report: &serde_json::Value) -> String {
    report
        .pointer("/report/status")
        .and_then(|value| value.as_str())
        .unwrap_or("Draft")
        .to_string()
}

fn plan_report_summary_projection(report: &serde_json::Value) -> serde_json::Value {
    let id = plan_report_id(report).unwrap_or_default();
    let status = plan_report_status(report);
    let phase = if status == "Approved" {
        "Execute"
    } else {
        status.as_str()
    };
    serde_json::json!({
        "id": id,
        "title": plan_report_title(report),
        "goal": plan_report_markdown(report).lines().find(|line| !line.trim().is_empty()).unwrap_or("Markdown plan report"),
        "phase": phase,
        "status": status,
        "todo_count": 0,
        "todo_completed": 0,
        "is_active": matches!(status.as_str(), "AwaitingApproval" | "Approved"),
    })
}

fn plan_report_detail_projection(report: &serde_json::Value) -> serde_json::Value {
    let id = plan_report_id(report).unwrap_or_default();
    let report_status = plan_report_status(report);
    // GUI uses `phase` for card/bar routing (PlanPhase).
    // `status` must stay compatible with PlanStatus when any Rust path deserializes
    // PlanRuntimeState (Pending/InProgress/… — never report statuses like AwaitingApproval).
    let (phase, status_out) = match report_status.as_str() {
        "Approved" => ("Execute", "InProgress"),
        "Closed" => ("Completed", "Completed"),
        "Draft" | "AwaitingApproval" => ("AwaitingApproval", "Pending"),
        other => (other, "Pending"),
    };
    let markdown = plan_report_markdown(report);
    let title = plan_report_title(report);
    let goal = markdown
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .unwrap_or(title.as_str())
        .to_string();
    let created_at = report
        .pointer("/report/created_at")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::String(String::new()));
    let updated_at = report
        .pointer("/report/updated_at")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::String(String::new()));
    serde_json::json!({
        "id": id,
        "plan_id": id,
        "revision": plan_report_revision(report),
        "title": title,
        "goal": goal,
        "phase": phase,
        "status": status_out,
        "report_status": report_status,
        "strategy": markdown,
        "summary": markdown,
        "markdown": markdown,
        "assumptions": [],
        "risks": [],
        "open_questions": [],
        "verification_verdict": null,
        "steps": [],
        "todos": [],
        "created_at": created_at,
        "updated_at": updated_at,
    })
}

#[tauri::command]
pub async fn approve_plan_report(
    report_id: String,
    payload: serde_json::Value,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/plan-reports/{}/approve",
        state.api_base_url(),
        urlencoding::encode(report_id.trim())
    );
    let value: serde_json::Value = state
        .client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|error| format!("Failed to approve plan report: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Invalid plan report approval response: {error}"))?;
    if value.get("status").and_then(|status| status.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|message| message.as_str())
            .unwrap_or("Plan report approval failed")
            .to_string());
    }
    value
        .get("execution")
        .cloned()
        .ok_or_else(|| "Missing execution payload".to_string())
}

#[tauri::command]
pub async fn get_active_plan_execution(
    session_key: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/plan-executions/active?session_key={}",
        state.api_base_url(),
        urlencoding::encode(session_key.trim())
    );
    let value: serde_json::Value = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|error| format!("Failed to get active plan execution: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Invalid active execution response: {error}"))?;
    if value.get("status").and_then(|status| status.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|message| message.as_str())
            .unwrap_or("Failed to get active plan execution")
            .to_string());
    }
    Ok(value
        .get("execution")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn get_execution_todos(
    execution_id: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/plan-executions/{}/todos",
        state.api_base_url(),
        urlencoding::encode(execution_id.trim())
    );
    let value: serde_json::Value = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|error| format!("Failed to get execution todos: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Invalid execution todos response: {error}"))?;
    if value.get("status").and_then(|status| status.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|message| message.as_str())
            .unwrap_or("Failed to get execution todos")
            .to_string());
    }
    value
        .get("todos")
        .cloned()
        .ok_or_else(|| "Missing execution todos payload".to_string())
}

#[tauri::command]
pub async fn update_execution_todo(
    execution_id: String,
    todo_id: String,
    payload: serde_json::Value,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/plan-executions/{}/todos/{}",
        state.api_base_url(),
        urlencoding::encode(execution_id.trim()),
        urlencoding::encode(todo_id.trim())
    );
    let value: serde_json::Value = state
        .client
        .patch(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|error| format!("Failed to update execution todo: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Invalid execution todo response: {error}"))?;
    if value.get("status").and_then(|status| status.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|message| message.as_str())
            .unwrap_or("Failed to update execution todo")
            .to_string());
    }
    value
        .get("todo")
        .cloned()
        .ok_or_else(|| "Missing execution todo payload".to_string())
}

#[tauri::command]
pub async fn stop_generation(
    channel: Option<String>,
    chat_id: Option<String>,
    state: State<'_, AgentState>,
) -> Result<bool, String> {
    let url = format!("{}/chat/stop", state.api_base_url());
    let payload = serde_json::json!({
        "channel": channel,
        "chat_id": chat_id
    });

    let response = state
        .client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to request stop: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid stop response: {}", e))?;

    let status_ok = value.get("status").and_then(|v| v.as_str()) == Some("ok");
    if !status_ok {
        let message = value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Stop request rejected: {}", message));
    }

    Ok(value
        .get("stopped")
        .and_then(|v| v.as_bool())
        .unwrap_or(true))
}

#[tauri::command]
pub async fn reset_session(
    channel: Option<String>,
    chat_id: Option<String>,
    state: State<'_, AgentState>,
) -> Result<bool, String> {
    let url = format!("{}/sessions/reset", state.api_base_url());
    let payload = serde_json::json!({
        "channel": channel,
        "chat_id": chat_id
    });

    let response = state
        .client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to request session reset: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid reset response: {}", e))?;

    let status_ok = value.get("status").and_then(|v| v.as_str()) == Some("ok");
    if !status_ok {
        let message = value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Reset request rejected: {}", message));
    }

    Ok(value.get("reset").and_then(|v| v.as_bool()).unwrap_or(true))
}

#[tauri::command]
pub async fn delete_session(chat_id: String, state: State<'_, AgentState>) -> Result<bool, String> {
    let id_encoded = urlencoding::encode(&chat_id);
    // Use POST /sessions/:id (same path as DELETE) - more reliable in some environments
    let url = format!("{}/sessions/{}", state.api_base_url(), id_encoded);

    let response = state
        .client
        .post(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to delete session: {}", e))?;

    if !response.status().is_success() {
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(false);
        }
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid delete session response: {}", e))?;

    let status_ok = value.get("status").and_then(|v| v.as_str()) == Some("ok");
    if !status_ok {
        let message = value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Delete session request rejected: {}", message));
    }

    Ok(value
        .get("deleted")
        .and_then(|v| v.as_bool())
        .unwrap_or(true))
}

#[tauri::command]
pub async fn get_sessions(state: State<'_, AgentState>) -> Result<serde_json::Value, String> {
    let url = format!("{}/sessions", state.api_base_url());

    let response = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch sessions: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid get sessions response: {}", e))?;

    let status_ok = value.get("status").and_then(|v| v.as_str()) == Some("ok");
    if !status_ok {
        let message = value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Get sessions request rejected: {}", message));
    }

    Ok(value
        .get("sessions")
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![])))
}

#[tauri::command]
pub async fn get_session_history(
    chat_id: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    // URL encode the chat_id in case it contains special characters like ':'
    let id_encoded = urlencoding::encode(&chat_id);
    let url = format!("{}/sessions/{}", state.api_base_url(), id_encoded);

    let response = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch session history: {}", e))?;

    if !response.status().is_success() {
        // A 404 or other failure could mean no history
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(serde_json::Value::Null);
        }
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid get session history response: {}", e))?;

    let status_ok = value.get("status").and_then(|v| v.as_str()) == Some("ok");

    if !status_ok {
        let message = value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Get session history request rejected: {}", message));
    }

    Ok(value
        .get("session")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn update_session_title(
    session_key: String,
    title: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/sessions/{}/title",
        state.api_base_url(),
        urlencoding::encode(session_key.trim())
    );
    let response = state
        .client
        .patch(&url)
        .json(&serde_json::json!({ "title": title }))
        .send()
        .await
        .map_err(|e| format!("Failed to update session title: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid update session title response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    Ok(value)
}

#[tauri::command]
pub async fn generate_session_title(
    session_key: String,
    payload: GenerateSessionTitlePayload,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/sessions/{}/generate-title",
        state.api_base_url(),
        urlencoding::encode(session_key.trim())
    );
    let response = state
        .client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to generate session title: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid generate session title response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    Ok(value)
}

#[tauri::command]
pub async fn get_cron_jobs(state: State<'_, AgentState>) -> Result<serde_json::Value, String> {
    let url = format!("{}/cron/jobs", state.api_base_url());
    let response = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch cron jobs: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid get cron jobs response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(value
        .get("jobs")
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![])))
}

#[tauri::command]
pub async fn get_cron_job(
    job_id: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/cron/jobs/{}",
        state.api_base_url(),
        urlencoding::encode(&job_id)
    );
    let response = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch cron job: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid get cron job response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(value.get("job").cloned().unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn create_cron_job(
    payload: serde_json::Value,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!("{}/cron/jobs", state.api_base_url());
    let response = state
        .client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to create cron job: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid create cron job response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(value.get("job").cloned().unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn update_cron_job(
    job_id: String,
    payload: serde_json::Value,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/cron/jobs/{}",
        state.api_base_url(),
        urlencoding::encode(&job_id)
    );
    let response = state
        .client
        .put(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to update cron job: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid update cron job response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(value.get("job").cloned().unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn set_cron_job_enabled(
    job_id: String,
    enabled: bool,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/cron/jobs/{}/enable",
        state.api_base_url(),
        urlencoding::encode(&job_id)
    );
    let response = state
        .client
        .post(&url)
        .json(&serde_json::json!({ "enabled": enabled }))
        .send()
        .await
        .map_err(|e| format!("Failed to update cron job status: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid cron job status response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(value.get("job").cloned().unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn run_cron_job(
    job_id: String,
    force: bool,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/cron/jobs/{}/run",
        state.api_base_url(),
        urlencoding::encode(&job_id)
    );
    let response = state
        .client
        .post(&url)
        .json(&serde_json::json!({ "force": force }))
        .send()
        .await
        .map_err(|e| format!("Failed to run cron job: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid run cron job response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(value.get("job").cloned().unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn stop_cron_job_run(
    job_id: String,
    state: State<'_, AgentState>,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{}/cron/jobs/{}/stop",
        state.api_base_url(),
        urlencoding::encode(&job_id)
    );
    let response = state
        .client
        .post(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to stop cron job: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid stop cron job response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(value.get("run").cloned().unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub async fn delete_cron_job(job_id: String, state: State<'_, AgentState>) -> Result<(), String> {
    let url = format!(
        "{}/cron/jobs/{}",
        state.api_base_url(),
        urlencoding::encode(&job_id)
    );
    let response = state
        .client
        .delete(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to delete cron job: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid delete cron job response: {}", e))?;

    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }

    Ok(())
}

#[tauri::command]
pub async fn start_background_stream(
    window: Window,
    state: State<'_, AgentState>,
    shutdown_manager: State<'_, ShutdownManager>,
) -> Result<(), String> {
    let client = state.client.clone();
    let cancel_token = shutdown_manager.cancel_token();
    let url = format!(
        "{}/events?channel=api&chat_prefix=cron:",
        state.api_base_url()
    );

    tauri::async_runtime::spawn(async move {
        loop {
            let response = tokio::select! {
                _ = cancel_token.cancelled() => {
                    info!("Background stream cancelled before next connection attempt");
                    break;
                }
                response = client.get(&url).send() => response,
            };

            let response = match response {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Failed to connect background stream: {}", e);
                    tokio::select! {
                        _ = cancel_token.cancelled() => {
                            info!("Background stream cancelled during reconnect backoff");
                            break;
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}
                    }
                    continue;
                }
            };

            if !response.status().is_success() {
                error!("Background stream server error: {}", response.status());
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        info!("Background stream cancelled after server error");
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}
                }
                continue;
            }

            let mut stream = response.bytes_stream().eventsource();
            loop {
                let event = tokio::select! {
                    _ = cancel_token.cancelled() => {
                        info!("Background stream cancelled while reading events");
                        return;
                    }
                    event = stream.next() => event,
                };

                let Some(event) = event else {
                    break;
                };

                match event {
                    Ok(event) => match event.event.as_str() {
                        "final" => {
                            if let Ok(payload) =
                                serde_json::from_str::<BackgroundFinalEvent>(&event.data)
                            {
                                let _ = window.emit("agent-background-response", payload.content);
                            }
                        }
                        "error" => {
                            let _ = window.emit("agent-error", event.data);
                        }
                        _ => {}
                    },
                    Err(e) => {
                        error!("Background stream error: {}", e);
                        break;
                    }
                }
            }

            tokio::select! {
                _ = cancel_token.cancelled() => {
                    info!("Background stream cancelled before retry");
                    break;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
            }
        }

        info!("Background stream task exited");
    });

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandApprovalScopeDto {
    pub channel: String,
    pub chat_id: String,
    pub session_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandApprovalRequestDto {
    pub approval_id: String,
    pub command: String,
    pub cwd: String,
    pub reason: String,
    pub scope: CommandApprovalScopeDto,
    pub created_at: String,
    pub timeout_seconds: u64,
    pub suggested_prefix: Option<SafePrefixSuggestionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafePrefixSuggestionDto {
    pub pattern: Vec<String>,
    pub justification: String,
}

#[derive(Debug, Deserialize)]
struct CommandApprovalListResponse {
    requests: Vec<CommandApprovalRequestDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandApprovalResolutionDto {
    pub approval_id: String,
    pub decision: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandApprovalApiError {
    pub status: u16,
    pub code: String,
}

#[tauri::command]
pub async fn get_command_approvals(
    state: State<'_, AgentState>,
    session_key: Option<String>,
) -> Result<Vec<CommandApprovalRequestDto>, String> {
    let mut request = state.client.get(format!(
        "{}/command-approvals?channel=gui",
        state.api_base_url()
    ));
    if let Some(session_key) = session_key {
        let chat_id = session_key
            .strip_prefix("gui:")
            .unwrap_or(&session_key)
            .to_string();
        request = request.query(&[
            ("chat_id", chat_id.as_str()),
            ("session_key", session_key.as_str()),
        ]);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("Failed to query command approvals: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Command approval query failed with HTTP {}",
            response.status().as_u16()
        ));
    }
    response
        .json::<CommandApprovalListResponse>()
        .await
        .map(|body| {
            body.requests
                .into_iter()
                .filter(|request| request.scope.channel == "gui")
                .collect()
        })
        .map_err(|error| format!("Invalid command approval response: {error}"))
}

#[tauri::command]
pub async fn resolve_command_approval(
    state: State<'_, AgentState>,
    approval_id: String,
    decision: String,
) -> Result<CommandApprovalResolutionDto, CommandApprovalApiError> {
    if !matches!(
        decision.as_str(),
        "approve_once" | "approve_session" | "approve_global" | "reject"
    ) {
        return Err(CommandApprovalApiError {
            status: 422,
            code: "invalid_decision".into(),
        });
    }
    let response = state
        .client
        .post(format!(
            "{}/command-approvals/{}",
            state.api_base_url(),
            approval_id
        ))
        .json(&serde_json::json!({ "decision": decision }))
        .send()
        .await
        .map_err(|error| CommandApprovalApiError {
            status: 0,
            code: format!("transport_error:{error}"),
        })?;
    let status = response.status();
    if !status.is_success() {
        let code = response
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|body| {
                body.get("error")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| format!("http_{}", status.as_u16()));
        return Err(CommandApprovalApiError {
            status: status.as_u16(),
            code,
        });
    }
    response
        .json()
        .await
        .map_err(|error| CommandApprovalApiError {
            status: 0,
            code: format!("invalid_response:{error}"),
        })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRuleDto {
    pub id: String,
    pub pattern: Vec<String>,
    pub decision: String,
    pub enabled: bool,
    pub source: String,
    pub justification: String,
    pub created_at: String,
    pub revision: u64,
}

#[derive(Debug, Deserialize)]
struct CommandRuleListResponse {
    rules: Vec<CommandRuleDto>,
}

#[tauri::command]
pub async fn get_command_rules(
    state: State<'_, AgentState>,
) -> Result<Vec<CommandRuleDto>, String> {
    let response = state
        .client
        .get(format!("{}/command-rules", state.api_base_url()))
        .send()
        .await
        .map_err(|error| format!("Failed to query command rules: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Command rule query failed with HTTP {}",
            response.status().as_u16()
        ));
    }
    response
        .json::<CommandRuleListResponse>()
        .await
        .map(|body| body.rules)
        .map_err(|error| format!("Invalid command rule response: {error}"))
}

#[tauri::command]
pub async fn set_command_rule_enabled(
    state: State<'_, AgentState>,
    rule_id: String,
    revision: u64,
    enabled: bool,
) -> Result<CommandRuleDto, CommandApprovalApiError> {
    let response = state
        .client
        .patch(format!(
            "{}/command-rules/{}",
            state.api_base_url(),
            rule_id
        ))
        .json(&serde_json::json!({ "revision": revision, "enabled": enabled }))
        .send()
        .await
        .map_err(command_rule_transport_error)?;
    parse_command_rule_response(response).await
}

#[tauri::command]
pub async fn delete_command_rule(
    state: State<'_, AgentState>,
    rule_id: String,
    revision: u64,
) -> Result<(), CommandApprovalApiError> {
    let response = state
        .client
        .delete(format!(
            "{}/command-rules/{}?revision={}",
            state.api_base_url(),
            rule_id,
            revision
        ))
        .send()
        .await
        .map_err(command_rule_transport_error)?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(command_rule_api_error(response).await)
    }
}

fn command_rule_transport_error(error: reqwest::Error) -> CommandApprovalApiError {
    CommandApprovalApiError {
        status: 0,
        code: format!("transport_error:{error}"),
    }
}

async fn parse_command_rule_response(
    response: reqwest::Response,
) -> Result<CommandRuleDto, CommandApprovalApiError> {
    if !response.status().is_success() {
        return Err(command_rule_api_error(response).await);
    }
    response
        .json()
        .await
        .map_err(|error| CommandApprovalApiError {
            status: 0,
            code: format!("invalid_response:{error}"),
        })
}

async fn command_rule_api_error(response: reqwest::Response) -> CommandApprovalApiError {
    let status = response.status().as_u16();
    let code = response
        .json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|body| {
            body.get("error")
                .and_then(|value| value.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("http_{status}"));
    CommandApprovalApiError { status, code }
}

#[tauri::command]
pub async fn start_command_approval_stream(
    window: Window,
    state: State<'_, AgentState>,
    shutdown_manager: State<'_, ShutdownManager>,
) -> Result<(), String> {
    let client = state.client.clone();
    let cancel_token = shutdown_manager.cancel_token();
    let url = format!(
        "{}/command-approvals/events?channel=gui",
        state.api_base_url()
    );
    tauri::async_runtime::spawn(async move {
        loop {
            let response = tokio::select! {
                _ = cancel_token.cancelled() => break,
                response = client.get(&url).send() => response,
            };
            let response = match response {
                Ok(response) if response.status().is_success() => response,
                Ok(response) => {
                    error!(
                        "Command approval stream server error: {}",
                        response.status()
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    continue;
                }
                Err(error) => {
                    error!("Failed to connect command approval stream: {error}");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    continue;
                }
            };
            let _ = window.emit("command-approval-stream-connected", ());
            let mut stream = response.bytes_stream().eventsource();
            while let Some(event) = tokio::select! {
                _ = cancel_token.cancelled() => None,
                event = stream.next() => event,
            } {
                match event {
                    Ok(event) if event.event == "command_approval_requested" => {
                        if let Ok(request) =
                            serde_json::from_str::<CommandApprovalRequestDto>(&event.data)
                        {
                            if request.scope.channel == "gui" {
                                let _ = window.emit("command-approval-requested", request);
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(error) => {
                        error!("Command approval stream error: {error}");
                        break;
                    }
                }
            }
            tokio::select! {
                _ = cancel_token.cancelled() => break,
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn check_health(state: State<'_, AgentState>) -> Result<bool, String> {
    let url = format!("{}/health", state.api_base_url());

    let client = &state.client;
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_millis(1000))
        .send()
        .await
        .map_err(|e| format!("Health check failed: {}", e))?;

    Ok(response.status().is_success())
}

#[tauri::command]
pub async fn update_config(
    api_base: Option<String>,
    api_key: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    info!(
        "Updating config via API: model={:?}, base={:?}",
        model, api_base
    );
    state.reconfigure(api_base, api_key, provider, model).await
}

#[tauri::command]
pub async fn get_tools_config(state: State<'_, AgentState>) -> Result<serde_json::Value, String> {
    state.get_tools_config().await
}

#[tauri::command]
pub async fn get_provider_models(
    provider: String,
    api_base: Option<String>,
    api_key: Option<String>,
    state: State<'_, AgentState>,
) -> Result<ProviderModelCatalog, String> {
    if api_base.is_none() && api_key.is_none() {
        let catalog = state
            .get_provider_model_catalog(provider.trim(), true)
            .await?;
        return Ok(provider_model_catalog_dto(catalog));
    }

    let loader = config_loader();
    let config = loader.load().unwrap_or_default();
    let mut access = ProviderCatalogService::new()
        .get_provider_access(&config, provider.trim())
        .unwrap_or_else(|| ProviderAccess::from_config(None));
    if let Some(api_base) = api_base
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
    {
        access.api_base = Some(api_base);
    }
    if let Some(api_key) = api_key
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        access.api_key = Some(api_key);
    }
    let catalog = ProviderCatalogService::new()
        .list_provider_models(&config, provider.trim(), true, Some(access))
        .await?;

    Ok(provider_model_catalog_dto(catalog))
}

#[tauri::command]
pub async fn test_provider_model(
    provider: String,
    model: String,
    api_base: Option<String>,
    api_key: Option<String>,
) -> Result<ProviderModelTestResult, String> {
    // Keep this local so the GUI can probe ad-hoc credentials without
    // introducing new manager API surface or mutating manager-managed config.
    let provider = provider.trim().to_string();
    let model = model.trim().to_string();
    if provider.is_empty() {
        return Err("provider must not be empty".to_string());
    }
    if model.is_empty() {
        return Err("model must not be empty".to_string());
    }

    let loader = config_loader();
    let config = loader.load().unwrap_or_default();
    let access = provider_access_for_test(&config, &provider, api_base, api_key);

    let spec = ProviderCatalogService::new()
        .provider_spec(&provider, &config.providers)
        .ok_or_else(|| format!("Unknown provider '{provider}'"))?;
    let client = build_llm_provider(LlmProviderBuildOptions {
        spec,
        access,
        model: model.clone(),
        reasoning_effort: config.agents.defaults.reasoning_effort.clone(),
        reasoning_config: None,
        response_protocol: config
            .providers
            .get(&provider)
            .map(|provider| provider.response_protocol)
            .unwrap_or_default(),
    })
    .map_err(|error| error.to_string())?;
    let neuron = LlmNeuron::with_id(client, format!("provider-test:{provider}:{model}"));
    let request = NeuronRequest::new(
        vec![Message::user(
            "Reply with a short connectivity confirmation for this model test.",
        )],
        16,
        0.0,
    )
    .with_model(model);

    let started = Instant::now();
    match neuron.run_once(request).await {
        Ok(_) => Ok(ProviderModelTestResult {
            ok: true,
            message: "Connection test succeeded.".to_string(),
            latency_ms: started.elapsed().as_millis() as u64,
        }),
        Err(error) => Ok(ProviderModelTestResult {
            ok: false,
            message: format!("Connection test failed: {error}"),
            latency_ms: started.elapsed().as_millis() as u64,
        }),
    }
}

#[tauri::command]
pub async fn get_skills(state: State<'_, AgentState>) -> Result<Vec<SkillDto>, String> {
    let value = state.get_skills().await?;
    serde_json::from_value(value).map_err(|e| format!("Invalid skills payload: {}", e))
}

#[tauri::command]
pub fn list_masks() -> Result<Vec<MaskEntryDto>, String> {
    let registry = load_mask_registry();
    let mut items: Vec<MaskEntryDto> = registry
        .list()
        .into_iter()
        .map(mask_entry_from_file)
        .collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(items)
}

#[tauri::command]
pub fn get_active_mask() -> Result<Option<MaskEntryDto>, String> {
    let registry = load_mask_registry();
    Ok(registry.current_mask().map(mask_entry_from_file))
}

#[tauri::command]
pub fn get_current_mask() -> Result<MaskEntryDto, String> {
    let registry = load_mask_registry();
    let mask = registry
        .current_mask()
        .cloned()
        .unwrap_or_else(agent_diva_agent::mask::MaskFile::default_mask);
    Ok(mask_entry_from_file(&mask))
}

#[tauri::command]
pub fn switch_mask(name: String) -> Result<MaskEntryDto, String> {
    let mut registry = load_mask_registry();
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed == agent_diva_agent::mask::MaskFile::DEFAULT_NAME {
        registry.switch_off();
        return Ok(mask_entry_from_file(
            &agent_diva_agent::mask::MaskFile::default_mask(),
        ));
    }

    let mask = registry
        .switch_to(trimmed)
        .map_err(|error| error.to_string())?
        .clone();
    Ok(mask_entry_from_file(&mask))
}

fn parse_agent_mode(mode: Option<String>) -> Option<AgentMode> {
    match mode? {
        value if value.eq_ignore_ascii_case("normal") => Some(AgentMode::Normal),
        value if value.eq_ignore_ascii_case("assist") => Some(AgentMode::Assist),
        _ => None,
    }
}

#[tauri::command]
pub fn create_or_update_mask(payload: MaskPayload) -> Result<MaskEntryDto, String> {
    let mut registry = load_mask_registry();
    let name = payload.name.trim();
    if name.is_empty() {
        return Err("mask name cannot be empty".to_string());
    }
    let frontmatter = agent_diva_core::config::schema::MaskConfig {
        id: payload.id,
        name: name.to_string(),
        icon: payload.icon,
        description: payload.description,
        mode: parse_agent_mode(payload.mode),
        model: payload.model,
        subagent_defaults: payload.subagent_defaults,
        tool_limits: payload.tool_limits,
    };
    let body = payload.body.unwrap_or_default().trim().to_string();
    let mask = agent_diva_agent::mask::MaskFile { frontmatter, body };
    let mask = registry
        .create_or_update(mask)
        .map_err(|error| error.to_string())?
        .clone();
    Ok(mask_entry_from_file(&mask))
}

#[tauri::command]
pub fn delete_mask(name: String) -> Result<(), String> {
    let mut registry = load_mask_registry();
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed == agent_diva_agent::mask::MaskFile::DEFAULT_NAME {
        return Err("cannot delete the default mask".to_string());
    }
    registry.delete(trimmed).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_mcps(state: State<'_, AgentState>) -> Result<Vec<McpServerDto>, String> {
    let value = state.get_mcps().await?;
    serde_json::from_value(value).map_err(|e| format!("Invalid MCP payload: {}", e))
}

#[tauri::command]
pub async fn create_mcp(
    payload: McpServerPayload,
    state: State<'_, AgentState>,
) -> Result<McpServerDto, String> {
    let url = format!("{}/mcps", state.api_base_url());
    let response = state
        .client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to create MCP: {}", e))?;
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid create MCP response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    serde_json::from_value(value.get("mcp").cloned().unwrap_or(serde_json::Value::Null))
        .map_err(|e| format!("Invalid created MCP payload: {}", e))
}

#[tauri::command]
pub async fn update_mcp(
    name: String,
    payload: McpServerPayload,
    state: State<'_, AgentState>,
) -> Result<McpServerDto, String> {
    let url = format!(
        "{}/mcps/{}",
        state.api_base_url(),
        urlencoding::encode(&name)
    );
    let response = state
        .client
        .put(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to update MCP: {}", e))?;
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid update MCP response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    serde_json::from_value(value.get("mcp").cloned().unwrap_or(serde_json::Value::Null))
        .map_err(|e| format!("Invalid updated MCP payload: {}", e))
}

#[tauri::command]
pub async fn delete_mcp(name: String, state: State<'_, AgentState>) -> Result<(), String> {
    let url = format!(
        "{}/mcps/{}",
        state.api_base_url(),
        urlencoding::encode(&name)
    );
    let response = state
        .client
        .delete(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to delete MCP: {}", e))?;
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid delete MCP response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn set_mcp_enabled(
    name: String,
    enabled: bool,
    state: State<'_, AgentState>,
) -> Result<McpServerDto, String> {
    let url = format!(
        "{}/mcps/{}/enable",
        state.api_base_url(),
        urlencoding::encode(&name)
    );
    let response = state
        .client
        .post(&url)
        .json(&serde_json::json!({ "enabled": enabled }))
        .send()
        .await
        .map_err(|e| format!("Failed to toggle MCP: {}", e))?;
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid toggle MCP response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    serde_json::from_value(value.get("mcp").cloned().unwrap_or(serde_json::Value::Null))
        .map_err(|e| format!("Invalid toggled MCP payload: {}", e))
}

#[tauri::command]
pub async fn refresh_mcp_status(
    name: String,
    state: State<'_, AgentState>,
) -> Result<McpServerDto, String> {
    let url = format!(
        "{}/mcps/{}/refresh",
        state.api_base_url(),
        urlencoding::encode(&name)
    );
    let response = state
        .client
        .post(&url)
        .json(&serde_json::json!({ "reapply": true }))
        .send()
        .await
        .map_err(|e| format!("Failed to refresh MCP: {}", e))?;
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid refresh MCP response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    serde_json::from_value(value.get("mcp").cloned().unwrap_or(serde_json::Value::Null))
        .map_err(|e| format!("Invalid refreshed MCP payload: {}", e))
}

#[tauri::command]
pub async fn upload_skill(
    file_name: String,
    bytes: Vec<u8>,
    state: State<'_, AgentState>,
) -> Result<SkillDto, String> {
    let url = format!("{}/skills", state.api_base_url());
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name)
        .mime_str("application/zip")
        .map_err(|e| format!("Failed to build upload part: {}", e))?;
    let form = reqwest::multipart::Form::new().part("file", part);
    let response = state
        .client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to upload skill: {}", e))?;

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid upload response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    serde_json::from_value(
        value
            .get("skill")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|e| format!("Invalid uploaded skill payload: {}", e))
}

#[tauri::command]
pub async fn upload_file(
    file_name: String,
    bytes: Vec<u8>,
    channel: String,
    message_id: Option<String>,
    state: State<'_, AgentState>,
) -> Result<FileAttachmentDto, String> {
    let url = format!("{}/files/upload", state.api_base_url());
    let file_part = reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name)
        .mime_str("application/octet-stream")
        .map_err(|e| format!("Failed to build file part: {}", e))?;
    let channel_part = reqwest::multipart::Part::text(channel);
    // Use provided message_id or generate a temporary one for GUI uploads
    let message_id = message_id.unwrap_or_else(|| {
        format!(
            "gui_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        )
    });
    let message_id_part = reqwest::multipart::Part::text(message_id);
    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .part("channel", channel_part)
        .part("message_id", message_id_part);
    let response = state
        .client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to upload file: {}", e))?;

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid upload response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    serde_json::from_value(
        value
            .get("attachment")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|e| format!("Invalid uploaded file payload: {}", e))
}

#[tauri::command]
pub async fn delete_skill(name: String, state: State<'_, AgentState>) -> Result<(), String> {
    let name = urlencoding::encode(&name);
    let url = format!("{}/skills/{}", state.api_base_url(), name);
    let response = state
        .client
        .delete(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to delete skill: {}", e))?;
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid delete skill response: {}", e))?;
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn update_tools_config(
    tools: serde_json::Value,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    state.update_tools_config(tools).await
}

#[tauri::command]
pub async fn get_channels(state: State<'_, AgentState>) -> Result<serde_json::Value, String> {
    let url = format!("{}/channels", state.api_base_url());

    let response = state
        .client
        .get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch channels: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server error: {}", response.status()));
    }

    let channels: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    Ok(channels)
}

#[tauri::command]
pub async fn update_channel(
    name: String,
    enabled: Option<bool>,
    config: serde_json::Value,
    state: State<'_, AgentState>,
) -> Result<(), String> {
    let url = format!("{}/channels", state.api_base_url());

    let payload = serde_json::json!({
        "name": name,
        "enabled": enabled,
        "config": config
    });

    let response = state
        .client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to update channel: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server error: {}", response.status()));
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeInfo {
    pub platform: String,
    pub is_bundled: bool,
    pub resource_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayProcessStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub executable_path: Option<String>,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatusPayload {
    pub installed: bool,
    pub running: bool,
    pub state: String,
    pub executable_path: Option<String>,
    pub details: Option<String>,
}

// Local runtime bridge helpers below operate on the desktop host process,
// bundled runtime assets, and local filesystem/process state.
fn runtime_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn is_bundled_app() -> bool {
    !cfg!(debug_assertions)
}

fn bundled_binary_name(base: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn config_loader() -> ConfigLoader {
    match std::env::var("AGENT_DIVA_CONFIG_DIR") {
        Ok(path) if !path.trim().is_empty() => ConfigLoader::with_dir(expand_user_path(&path)),
        _ => ConfigLoader::new(),
    }
}

/// Saves the gateway port to a configuration file
pub(crate) fn save_gateway_port_config(port: u16) -> Result<(), String> {
    let loader = config_loader();
    let config_dir = loader.config_dir();
    std::fs::create_dir_all(config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let port_file = config_dir.join("gateway.port");
    std::fs::write(&port_file, port.to_string())
        .map_err(|e| format!("Failed to write gateway port config: {}", e))?;

    info!("Saved gateway port {} to {}", port, port_file.display());
    Ok(())
}

#[cfg(test)]
mod gateway_status_tests {
    use super::{
        gateway_process_status_from_runtime, normalize_pet_voice_relative_path,
        normalize_tts_provider,
    };
    use crate::gateway_status::GatewayStatus;

    #[test]
    fn gateway_process_status_uses_embedded_runtime_state() {
        let status = GatewayStatus::new(3456);
        let process_status = gateway_process_status_from_runtime(&status);

        assert!(process_status.running);
        assert_eq!(process_status.pid, None);
        assert_eq!(process_status.executable_path, None);
        assert_eq!(
            process_status.details.as_deref(),
            Some("Gateway: Running (port: 3456)")
        );
    }

    #[test]
    fn pet_voice_relative_path_rejects_parent_escape() {
        let error = normalize_pet_voice_relative_path("voice_resource/../secret.mp3").unwrap_err();
        assert!(error.contains("voice_resource"));
    }

    #[test]
    fn pet_voice_relative_path_accepts_voice_resource_child() {
        let path = normalize_pet_voice_relative_path("voice_resource/custom/sample.mp3").unwrap();
        assert_eq!(path, "voice_resource/custom/sample.mp3");
    }

    #[test]
    fn normalize_tts_provider_accepts_minimax() {
        assert_eq!(normalize_tts_provider("minimax"), "minimax");
    }
}

/// Loads the gateway port from configuration file, defaults to 3000
#[allow(dead_code)]
fn load_gateway_port_config() -> u16 {
    let loader = config_loader();
    let port_file = loader.config_dir().join("gateway.port");

    match std::fs::read_to_string(&port_file) {
        Ok(content) => match content.trim().parse::<u16>() {
            Ok(port) => {
                debug!("Loaded gateway port {} from {}", port, port_file.display());
                port
            }
            Err(e) => {
                warn!("Invalid port in config file: {}. Using default 3000", e);
                3000
            }
        },
        Err(_) => {
            debug!("Gateway port config file not found. Using default 3000");
            3000
        }
    }
}

fn cli_runtime_from_loader(loader: &ConfigLoader) -> CliRuntime {
    CliRuntime::from_paths(
        Some(loader.config_path().to_path_buf()),
        Some(loader.config_dir().to_path_buf()),
        None,
    )
}

fn mask_workspace_dir() -> PathBuf {
    let loader = config_loader();
    let config = loader.load().unwrap_or_default();
    let runtime = cli_runtime_from_loader(&loader);
    runtime.effective_workspace(&config).join("masks")
}

fn load_mask_registry() -> MaskRegistry {
    MaskRegistry::new(mask_workspace_dir())
}

fn mask_entry_from_file(mask: &agent_diva_agent::mask::MaskFile) -> MaskEntryDto {
    let read_only = ToolPolicy::is_read_only_mode(mask);
    MaskEntryDto {
        name: mask.frontmatter.name.clone(),
        icon: mask.frontmatter.icon.clone().unwrap_or_else(|| {
            if read_only {
                "📝".to_string()
            } else {
                "😊".to_string()
            }
        }),
        description: mask.frontmatter.description.clone().unwrap_or_default(),
        mode: mask
            .frontmatter
            .mode
            .map(|mode| match mode {
                agent_diva_core::config::schema::AgentMode::Normal => "normal",
                agent_diva_core::config::schema::AgentMode::Assist => "assist",
            })
            .unwrap_or("normal")
            .to_string(),
        read_only,
    }
}

fn provider_access_for_test(
    config: &Config,
    provider: &str,
    api_base: Option<String>,
    api_key: Option<String>,
) -> ProviderAccess {
    let mut access = ProviderCatalogService::new()
        .get_provider_access(config, provider)
        .unwrap_or_else(|| ProviderAccess::from_config(None));
    if let Some(api_base) = api_base
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
    {
        access.api_base = Some(api_base);
    }
    if let Some(api_key) = api_key
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        access.api_key = Some(api_key);
    }
    access
}

fn provider_model_catalog_dto(catalog: SharedProviderModelCatalog) -> ProviderModelCatalog {
    ProviderModelCatalog {
        provider: catalog.provider,
        source: catalog.catalog_source,
        runtime_supported: catalog.runtime_supported,
        api_base: catalog.api_base,
        models: catalog.models.into_iter().map(|entry| entry.id).collect(),
        custom_models: catalog.custom_models,
        warnings: catalog.warnings,
        error: catalog.error,
    }
}

fn provider_models_from_catalog(catalog: SharedProviderModelCatalog) -> (Vec<String>, Vec<String>) {
    (
        catalog.models.into_iter().map(|entry| entry.id).collect(),
        catalog.custom_models,
    )
}

fn provider_spec_from_view(
    view: SharedProviderView,
    models: Vec<String>,
    custom_models: Vec<String>,
) -> ProviderSpec {
    ProviderSpec {
        name: view.id,
        display_name: view.display_name,
        api_type: view.api_type,
        source: serde_json::to_value(view.source)
            .ok()
            .and_then(|value| value.as_str().map(ToString::to_string))
            .unwrap_or_else(|| "builtin".to_string()),
        configured: view.configured,
        ready: view.ready,
        default_api_base: view.default_api_base.or(view.api_base).unwrap_or_default(),
        default_model: view.default_model,
        models,
        custom_models,
    }
}

fn expand_user_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn resolve_configured_path(path: &str, config_dir: &std::path::Path) -> PathBuf {
    let expanded = expand_user_path(path);
    if expanded.is_absolute() {
        expanded
    } else {
        config_dir.join(expanded)
    }
}

fn runtime_info_from_app(app: &AppHandle) -> RuntimeInfo {
    let resource_dir = app
        .path()
        .resolve(".", BaseDirectory::Resource)
        .ok()
        .map(|path| path.display().to_string());

    RuntimeInfo {
        platform: runtime_platform().to_string(),
        is_bundled: is_bundled_app(),
        resource_dir,
    }
}

fn ensure_bundled_runtime(app: &AppHandle) -> Result<RuntimeInfo, String> {
    let info = runtime_info_from_app(app);
    if !info.is_bundled {
        return Err("service management is only available in bundled app".to_string());
    }
    Ok(info)
}

fn candidate_cli_paths(app: &AppHandle) -> Vec<PathBuf> {
    let platform = runtime_platform();
    let binary_name = bundled_binary_name("agent-diva");
    let mut candidates = Vec::new();

    if let Ok(resource_path) = app.path().resolve(
        format!("bin/{platform}/{binary_name}"),
        BaseDirectory::Resource,
    ) {
        candidates.push(resource_path);
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            candidates.push(exe_dir.join(&binary_name));
            candidates.push(
                exe_dir
                    .join("resources")
                    .join("bin")
                    .join(platform)
                    .join(&binary_name),
            );
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .unwrap_or(manifest_dir);
    candidates.push(
        workspace_root
            .join("target")
            .join("release")
            .join(&binary_name),
    );
    candidates.push(
        workspace_root
            .join("target")
            .join("debug")
            .join(&binary_name),
    );
    if let Ok(path) = which::which(&binary_name) {
        candidates.push(path);
    }

    candidates
}

fn resolve_cli_binary(app: &AppHandle) -> Result<PathBuf, String> {
    candidate_cli_paths(app)
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| {
            format!(
                "unable to locate bundled agent-diva binary for platform {}",
                runtime_platform()
            )
        })
}

async fn run_service_cli(app: &AppHandle, args: &[&str]) -> Result<String, String> {
    if runtime_platform() != "windows" {
        return Err("service management is currently implemented for Windows only".to_string());
    }
    if cfg!(debug_assertions) {
        return Err("service management is only available in bundled app".to_string());
    }

    let cli_binary = resolve_cli_binary(app)?;
    let mut command = TokioCommand::new(&cli_binary);
    configure_background_command(&mut command);
    let output = command
        .arg("service")
        .args(args)
        .output()
        .await
        .map_err(|e| format!("failed to execute {}: {}", cli_binary.display(), e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let message = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("service command failed with status {}", output.status)
        };
        Err(message)
    }
}

async fn run_command_capture<I, S>(program: &str, args: I) -> Result<std::process::Output, String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut command = TokioCommand::new(program);
    configure_background_command(&mut command);
    command
        .args(args.into_iter().map(Into::into))
        .output()
        .await
        .map_err(|e| format!("failed to execute {program}: {e}"))
}

fn command_output_message(program: &str, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("{program} exited with status {}", output.status)
    }
}

fn resolve_resource_path(app: &AppHandle, relative: &str) -> Option<PathBuf> {
    app.path().resolve(relative, BaseDirectory::Resource).ok()
}

fn resolve_linux_service_script(app: &AppHandle, file_name: &str) -> Result<PathBuf, String> {
    if let Some(path) = resolve_resource_path(app, &format!("systemd/{file_name}")) {
        if path.exists() {
            return Ok(path);
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .unwrap_or(manifest_dir);
    let fallback = workspace_root
        .join("contrib")
        .join("systemd")
        .join(file_name);
    if fallback.exists() {
        return Ok(fallback);
    }

    Err(format!(
        "unable to locate Linux service script: {file_name}"
    ))
}

fn resolve_macos_launchd_script(app: &AppHandle, file_name: &str) -> Result<PathBuf, String> {
    if let Some(path) = resolve_resource_path(app, &format!("launchd/{file_name}")) {
        if path.exists() {
            return Ok(path);
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .unwrap_or(manifest_dir);
    let fallback = workspace_root
        .join("contrib")
        .join("launchd")
        .join(file_name);
    if fallback.exists() {
        return Ok(fallback);
    }

    Err(format!(
        "unable to locate macOS launchd script: {file_name}"
    ))
}

async fn run_macos_launchd_bash(_app: &AppHandle, script: &PathBuf) -> Result<(), String> {
    let script_dir = script
        .parent()
        .ok_or_else(|| "script has no parent directory".to_string())?;
    let bundle_root = script_dir
        .parent()
        .ok_or_else(|| "launchd script has no bundle root".to_string())?;
    let output = TokioCommand::new("bash")
        .arg(script)
        .current_dir(bundle_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            format!(
                "failed to execute launchd script {}: {}",
                script.display(),
                e
            )
        })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_output_message("bash", &output))
    }
}

async fn run_macos_launchctl(action: &str) -> Result<(), String> {
    let output = TokioCommand::new("launchctl")
        .arg(action)
        .arg("com.agent-diva.gateway")
        .output()
        .await
        .map_err(|e| format!("failed to execute launchctl {action}: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_output_message("launchctl", &output))
    }
}

async fn linux_service_status() -> Result<ServiceStatusPayload, String> {
    let load_state = run_command_capture(
        "systemctl",
        ["show", "agent-diva", "--property=LoadState", "--value"],
    )
    .await?;
    if !load_state.status.success() {
        return Err(command_output_message("systemctl", &load_state));
    }

    let load_state_value = String::from_utf8_lossy(&load_state.stdout)
        .trim()
        .to_string();
    let active_state = run_command_capture(
        "systemctl",
        ["show", "agent-diva", "--property=ActiveState", "--value"],
    )
    .await?;
    if !active_state.status.success() {
        return Err(command_output_message("systemctl", &active_state));
    }

    let active_state_value = String::from_utf8_lossy(&active_state.stdout)
        .trim()
        .to_string();
    let installed = !matches!(load_state_value.as_str(), "" | "not-found");
    let running = active_state_value == "active";
    let state = if installed {
        format!("{load_state_value}/{active_state_value}")
    } else {
        "NotInstalled".to_string()
    };

    Ok(ServiceStatusPayload {
        installed,
        running,
        state: state.clone(),
        executable_path: Some("/usr/bin/agent-diva".to_string()),
        details: Some(if installed {
            format!("systemd load={load_state_value}, active={active_state_value}")
        } else {
            "systemd unit not installed".to_string()
        }),
    })
}

async fn run_linux_privileged_bash(script: &PathBuf) -> Result<(), String> {
    let pkexec = which::which("pkexec").map_err(|_| {
        "pkexec not found; install policykit or run the bundled script manually with sudo"
            .to_string()
    })?;
    let output = TokioCommand::new(pkexec)
        .arg("bash")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            format!(
                "failed to execute privileged script {}: {}",
                script.display(),
                e
            )
        })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_output_message("pkexec", &output))
    }
}

async fn run_linux_systemctl(action: &str) -> Result<(), String> {
    let pkexec = which::which("pkexec").map_err(|_| {
        "pkexec not found; install policykit or run systemctl manually with sudo".to_string()
    })?;
    let output = TokioCommand::new(pkexec)
        .arg("systemctl")
        .arg(action)
        .arg("agent-diva")
        .output()
        .await
        .map_err(|e| format!("failed to execute systemctl {action}: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_output_message("systemctl", &output))
    }
}

async fn macos_service_status() -> Result<ServiceStatusPayload, String> {
    let plist_path = dirs::home_dir()
        .map(|home| {
            home.join("Library")
                .join("LaunchAgents")
                .join("com.agent-diva.gateway.plist")
        })
        .ok_or_else(|| "failed to resolve home directory for launchd status".to_string())?;
    let installed = plist_path.exists();

    let launchctl_output = run_command_capture("launchctl", ["list"]).await?;
    if !launchctl_output.status.success() {
        return Err(command_output_message("launchctl", &launchctl_output));
    }

    let stdout = String::from_utf8_lossy(&launchctl_output.stdout);
    let running = stdout.contains("com.agent-diva.gateway");
    let state = if installed {
        if running {
            "Loaded".to_string()
        } else {
            "Installed".to_string()
        }
    } else {
        "NotInstalled".to_string()
    };

    Ok(ServiceStatusPayload {
        installed,
        running,
        state: state.clone(),
        executable_path: Some(plist_path.display().to_string()),
        details: Some(if installed {
            if running {
                "launchd Loaded".to_string()
            } else {
                "launchd Installed".to_string()
            }
        } else {
            "launchd plist not found".to_string()
        }),
    })
}

fn resolve_log_directory(config: &Config, loader: &ConfigLoader) -> PathBuf {
    resolve_configured_path(&config.logging.dir, loader.config_dir())
}

fn latest_log_file(log_dir: &std::path::Path) -> Result<Option<PathBuf>, String> {
    if !log_dir.exists() {
        return Ok(None);
    }

    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let entries = std::fs::read_dir(log_dir)
        .map_err(|e| format!("failed to read log directory {}: {}", log_dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to inspect log directory entry: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !(name.starts_with("gateway.log") || name.starts_with("gateway-")) {
            continue;
        }

        let modified = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        match &newest {
            Some((current, _)) if modified <= *current => {}
            _ => newest = Some((modified, path)),
        }
    }

    Ok(newest.map(|(_, path)| path))
}

pub fn gateway_process_status_from_runtime(status: &GatewayStatus) -> GatewayProcessStatus {
    GatewayProcessStatus {
        running: status.running,
        pid: None,
        executable_path: None,
        details: Some(status.format_status()),
    }
}

const EMBEDDED_GATEWAY_AUTOMATIC_MESSAGE: &str =
    "embedded mode: gateway starts automatically with app";
const EMBEDDED_GATEWAY_COMPAT_STOP_MESSAGE: &str =
    "embedded mode: stop_gateway is a compatibility no-op; quit the app or use tray Quit to stop the embedded gateway";
const EMBEDDED_GATEWAY_COMPAT_UNINSTALL_MESSAGE: &str =
    "embedded mode: uninstall_gateway is deprecated and only performs compatibility cleanup for stray legacy gateway processes";

#[tauri::command]
pub fn get_runtime_info(app: AppHandle) -> RuntimeInfo {
    runtime_info_from_app(&app)
}

#[tauri::command]
pub async fn get_service_status(app: AppHandle) -> Result<ServiceStatusPayload, String> {
    let info = ensure_bundled_runtime(&app)?;
    match info.platform.as_str() {
        "windows" => {
            let output = run_service_cli(&app, &["status", "--json"]).await?;
            let mut payload: ServiceStatusPayload = serde_json::from_str(&output)
                .map_err(|e| format!("failed to parse service status payload: {}", e))?;
            if payload.details.is_none() {
                payload.details = Some(payload.state.clone());
            }
            Ok(payload)
        }
        "linux" => linux_service_status().await,
        "macos" => macos_service_status().await,
        _ => Err("unsupported platform".to_string()),
    }
}

#[tauri::command]
pub async fn install_service(app: AppHandle) -> Result<(), String> {
    let info = ensure_bundled_runtime(&app)?;
    match info.platform.as_str() {
        "windows" => {
            let _ = run_service_cli(&app, &["install", "--auto-start"]).await?;
            Ok(())
        }
        "linux" => {
            let script = resolve_linux_service_script(&app, "install.sh")?;
            run_linux_privileged_bash(&script).await
        }
        "macos" => {
            let script = resolve_macos_launchd_script(&app, "install.sh")?;
            run_macos_launchd_bash(&app, &script).await
        }
        _ => Err("unsupported platform".to_string()),
    }
}

#[tauri::command]
pub async fn uninstall_service(app: AppHandle) -> Result<(), String> {
    let info = ensure_bundled_runtime(&app)?;
    match info.platform.as_str() {
        "windows" => {
            let _ = run_service_cli(&app, &["uninstall"]).await?;
            Ok(())
        }
        "linux" => {
            let script = resolve_linux_service_script(&app, "uninstall.sh")?;
            run_linux_privileged_bash(&script).await
        }
        "macos" => {
            let script = resolve_macos_launchd_script(&app, "uninstall.sh")?;
            run_macos_launchd_bash(&app, &script).await
        }
        _ => Err("unsupported platform".to_string()),
    }
}

#[tauri::command]
pub async fn start_service(app: AppHandle) -> Result<(), String> {
    let info = ensure_bundled_runtime(&app)?;
    match info.platform.as_str() {
        "windows" => {
            let _ = run_service_cli(&app, &["start"]).await?;
            Ok(())
        }
        "linux" => run_linux_systemctl("start").await,
        "macos" => run_macos_launchctl("start").await,
        _ => Err("unsupported platform".to_string()),
    }
}

#[tauri::command]
pub async fn stop_service(app: AppHandle) -> Result<(), String> {
    let info = ensure_bundled_runtime(&app)?;
    match info.platform.as_str() {
        "windows" => {
            let _ = run_service_cli(&app, &["stop"]).await?;
            Ok(())
        }
        "linux" => run_linux_systemctl("stop").await,
        "macos" => run_macos_launchctl("stop").await,
        _ => Err("unsupported platform".to_string()),
    }
}

#[tauri::command]
pub async fn get_gateway_process_status(
    state: State<'_, AsyncMutex<GatewayStatus>>,
) -> Result<GatewayProcessStatus, String> {
    let status = state.lock().await.clone();
    Ok(gateway_process_status_from_runtime(&status))
}

#[tauri::command]
pub async fn get_gateway_status(
    state: State<'_, AsyncMutex<GatewayStatus>>,
) -> Result<GatewayStatus, String> {
    Ok(state.lock().await.clone())
}

#[tauri::command]
#[allow(dead_code)]
pub fn get_gateway_port() -> u16 {
    load_gateway_port_config()
}

#[tauri::command]
#[deprecated(note = "Embedded mode starts the gateway automatically; keep for compatibility only.")]
pub async fn start_gateway(_app: AppHandle, _bin_path: Option<String>) -> Result<u16, String> {
    warn!("start_gateway called through deprecated compatibility layer");
    Err(EMBEDDED_GATEWAY_AUTOMATIC_MESSAGE.to_string())
}

#[tauri::command]
#[deprecated(
    note = "Embedded mode manages gateway shutdown with app lifecycle; keep for compatibility only."
)]
pub async fn stop_gateway() -> Result<(), String> {
    warn!("stop_gateway called through deprecated compatibility layer");
    info!("{}", EMBEDDED_GATEWAY_COMPAT_STOP_MESSAGE);
    Ok(())
}

#[tauri::command]
#[deprecated(
    note = "Embedded mode uses in-process lifecycle management; keep only as a compatibility cleanup wrapper."
)]
pub async fn uninstall_gateway(app: AppHandle) -> Result<(), String> {
    warn!("uninstall_gateway called through deprecated compatibility layer");
    info!("{}", EMBEDDED_GATEWAY_COMPAT_UNINSTALL_MESSAGE);

    crate::shutdown_embedded_gateway(&app).await;

    process_utils::cleanup_legacy_gateway_processes()
        .map(|terminated| {
            info!(
                "Compatibility uninstall cleanup finished: terminated {} legacy gateway process(es)",
                terminated
            );
        })
        .map_err(|error| format!("{EMBEDDED_GATEWAY_COMPAT_UNINSTALL_MESSAGE}: {error}"))
}

#[tauri::command]
pub fn load_config() -> Result<String, String> {
    let loader = config_loader();
    let config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    serde_json::to_string_pretty(&config).map_err(|e| format!("failed to serialize config: {}", e))
}

#[tauri::command]
pub async fn get_config(state: State<'_, AgentState>) -> Result<RuntimeConfigSnapshot, String> {
    let url = format!("{}/config", state.api_base_url());
    let response = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch runtime config: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    response
        .json::<RuntimeConfigSnapshot>()
        .await
        .map_err(|e| format!("Invalid runtime config payload: {}", e))
}

#[tauri::command]
pub async fn get_config_status() -> Result<StatusReport, String> {
    let loader = config_loader();
    let runtime = cli_runtime_from_loader(&loader);
    collect_status_report(&runtime)
        .await
        .map_err(|e| format!("failed to collect config status: {}", e))
}

#[tauri::command]
pub fn save_config(raw: String) -> Result<(), String> {
    let loader = config_loader();
    let config: Config =
        serde_json::from_str(&raw).map_err(|e| format!("failed to parse config JSON: {}", e))?;
    loader
        .save(&config)
        .map_err(|e| format!("failed to save config: {}", e))
}

fn validate_wipe_config_root(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("config directory path is empty".to_string());
    }
    if path.components().count() < 2 {
        return Err("config directory path is too short to be safe".to_string());
    }
    if let Some(home) = dirs::home_dir() {
        if path == home.as_path() {
            return Err("refusing to delete user home as config directory".to_string());
        }
        if path.exists() {
            if let (Ok(h_canon), Ok(p_canon)) = (home.canonicalize(), path.canonicalize()) {
                if h_canon == p_canon {
                    return Err("refusing to delete user home as config directory".to_string());
                }
            }
        }
    }
    Ok(())
}

fn validate_external_workspace_delete(ws_canon: &Path) -> Result<(), String> {
    if let Some(home) = dirs::home_dir() {
        if let Ok(h) = home.canonicalize() {
            if ws_canon == h.as_path() {
                return Err("refusing to delete workspace: path is user home".to_string());
            }
        }
    }
    let lossy = ws_canon.to_string_lossy().to_lowercase();
    const BLOCKED: &[&str] = &[
        "\\program files\\",
        "\\program files (x86)\\",
        "\\windows\\",
        "\\programdata\\",
    ];
    for fragment in BLOCKED {
        if lossy.contains(fragment) {
            return Err(format!(
                "refusing to delete workspace under protected system location ({fragment})"
            ));
        }
    }
    Ok(())
}

fn wipe_local_disk_blocking(
    config_root: PathBuf,
    workspace: PathBuf,
) -> Result<WipeSummary, Vec<String>> {
    let mut removed_paths = Vec::new();
    let mut errors = Vec::new();

    let cr_exists = config_root.exists();
    let ws_still_exists = workspace.exists();

    let workspace_inside_config = if cr_exists && ws_still_exists {
        let cr_c = std::fs::canonicalize(&config_root).map_err(|e| {
            vec![format!(
                "failed to canonicalize config directory {}: {}",
                config_root.display(),
                e
            )]
        })?;
        let ws_c = std::fs::canonicalize(&workspace).map_err(|e| {
            vec![format!(
                "failed to canonicalize workspace {}: {}",
                workspace.display(),
                e
            )]
        })?;
        ws_c.starts_with(&cr_c)
    } else {
        false
    };

    if cr_exists {
        match std::fs::remove_dir_all(&config_root) {
            Ok(()) => removed_paths.push(config_root.display().to_string()),
            Err(e) => errors.push(format!(
                "failed to remove config directory {}: {}",
                config_root.display(),
                e
            )),
        }
    }

    if workspace.exists() && !workspace_inside_config {
        let ws_canon = std::fs::canonicalize(&workspace).map_err(|e| {
            vec![format!(
                "failed to canonicalize workspace {}: {}",
                workspace.display(),
                e
            )]
        })?;
        if let Err(msg) = validate_external_workspace_delete(&ws_canon) {
            errors.push(msg);
        } else {
            match std::fs::remove_dir_all(&workspace) {
                Ok(()) => {
                    let label = ws_canon.display().to_string();
                    if !removed_paths.iter().any(|p| p == &label) {
                        removed_paths.push(label);
                    }
                }
                Err(e) => errors.push(format!(
                    "failed to remove workspace {}: {}",
                    workspace.display(),
                    e
                )),
            }
        }
    }

    if errors.is_empty() {
        Ok(WipeSummary { removed_paths })
    } else {
        Err(errors)
    }
}

/// Stops the gateway, terminates stray gateway processes, then deletes the config directory
/// (and the workspace directory when it lies outside the config directory).
#[tauri::command]
pub async fn wipe_local_data(app: AppHandle) -> Result<WipeSummary, String> {
    crate::shutdown_embedded_gateway(&app).await;

    match process_utils::cleanup_legacy_gateway_processes() {
        Ok(terminated) if terminated > 0 => {
            info!(
                "wipe_local_data: terminated {} lingering legacy gateway process(es)",
                terminated
            );
        }
        Ok(_) => {}
        Err(error) => {
            warn!(
                "wipe_local_data: best-effort legacy gateway cleanup failed: {}",
                error
            );
        }
    }
    tokio::time::sleep(std::time::Duration::from_millis(450)).await;

    let loader = config_loader();
    validate_wipe_config_root(loader.config_dir())?;

    let runtime = cli_runtime_from_loader(&loader);
    let config = loader.load().unwrap_or_default();
    let config_root = loader.config_dir().to_path_buf();
    let workspace = runtime.effective_workspace(&config);

    tokio::task::spawn_blocking(move || wipe_local_disk_blocking(config_root, workspace))
        .await
        .map_err(|e| format!("wipe task join error: {}", e))?
        .map_err(|errs| errs.join("; "))
}

#[tauri::command]
pub fn tail_logs(lines: usize) -> Result<Vec<String>, String> {
    let loader = config_loader();
    let config = loader
        .load()
        .map_err(|e| format!("failed to load config for logs: {}", e))?;
    let log_dir = resolve_log_directory(&config, &loader);
    let Some(log_file) = latest_log_file(&log_dir)? else {
        return Ok(Vec::new());
    };

    let content = std::fs::read_to_string(&log_file)
        .map_err(|e| format!("failed to read log file {}: {}", log_file.display(), e))?;
    let mut all_lines: Vec<String> = content.lines().map(ToString::to_string).collect();
    let keep = lines.max(1);
    if all_lines.len() > keep {
        all_lines = all_lines.split_off(all_lines.len().saturating_sub(keep));
    }
    Ok(all_lines)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GuiPrefs {
    pub close_to_tray: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VrmModelInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub source: String,
    pub thumbnail: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetImportVrmModelPayload {
    pub base64_data: String,
    pub file_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetVrmModelFileData {
    pub base64_data: String,
    pub content_type: String,
    pub file_name: String,
}

const PET_VRM_DIR_NAME: &str = "vrm";
const PET_VRM_MODELS_DIR_NAME: &str = "models";
const PET_VRM_CUSTOM_DIR_NAME: &str = "custom";
const DEFAULT_PET_VRM_MODEL_PATH: &str = "/vrm/models/Alice.vrm";

/// Scans bundled VRM models and custom models under ~/.agent-diva/vrm/models/custom.
#[tauri::command]
pub async fn pet_list_vrm_models(app_handle: AppHandle) -> Result<Vec<VrmModelInfo>, String> {
    let mut models: Vec<VrmModelInfo> = Vec::new();
    append_builtin_vrm_models(&app_handle, &mut models);

    let loader = config_loader();
    append_custom_vrm_models(loader.config_dir(), &mut models)?;

    if !models
        .iter()
        .any(|model| model.path == DEFAULT_PET_VRM_MODEL_PATH)
    {
        models.push(VrmModelInfo {
            id: "Alice".to_string(),
            name: "Alice".to_string(),
            path: DEFAULT_PET_VRM_MODEL_PATH.to_string(),
            source: "builtin".to_string(),
            thumbnail: None,
        });
    }

    models.sort_by(|a, b| a.source.cmp(&b.source).then_with(|| a.name.cmp(&b.name)));
    models.dedup_by(|a, b| a.path == b.path);
    Ok(models)
}

#[tauri::command]
pub fn pet_import_vrm_model(payload: PetImportVrmModelPayload) -> Result<VrmModelInfo, String> {
    let loader = config_loader();
    let mut config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    let custom_dir = pet_vrm_custom_models_dir(loader.config_dir());
    let sanitized_name = sanitize_pet_vrm_file_name(&payload.file_name)?;
    let target_path = custom_dir.join(&sanitized_name);

    std::fs::create_dir_all(&custom_dir).map_err(|error| {
        format!(
            "failed to create VRM import directory {}: {}",
            custom_dir.display(),
            error
        )
    })?;

    let decoded = BASE64_STANDARD
        .decode(payload.base64_data.as_bytes())
        .map_err(|error| format!("failed to decode imported VRM file: {}", error))?;
    std::fs::write(&target_path, decoded).map_err(|error| {
        format!(
            "failed to write imported VRM file {}: {}",
            target_path.display(),
            error
        )
    })?;

    let relative_path = make_pet_vrm_relative_path(loader.config_dir(), &target_path)?;
    config.pet.vrm_model = relative_path.clone();
    loader
        .save(&config)
        .map_err(|e| format!("failed to save config: {}", e))?;

    let id = target_path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("custom")
        .to_string();
    Ok(VrmModelInfo {
        id: id.clone(),
        name: id,
        path: relative_path,
        source: "custom".to_string(),
        thumbnail: None,
    })
}

#[tauri::command]
pub fn pet_delete_vrm_model(relative_path: String) -> Result<(), String> {
    let normalized = normalize_pet_vrm_relative_path(&relative_path)?;
    if !normalized.starts_with("vrm/models/custom/") {
        return Err("only custom VRM models can be deleted".to_string());
    }

    let loader = config_loader();
    let mut config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    let target_path = resolve_pet_vrm_model_file(loader.config_dir(), &normalized)?;
    std::fs::remove_file(&target_path).map_err(|error| {
        format!(
            "failed to delete VRM model {}: {}",
            target_path.display(),
            error
        )
    })?;

    if config.pet.vrm_model == normalized {
        config.pet.vrm_model = DEFAULT_PET_VRM_MODEL_PATH.to_string();
        loader
            .save(&config)
            .map_err(|e| format!("failed to save config: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn pet_read_vrm_model(relative_path: String) -> Result<PetVrmModelFileData, String> {
    let loader = config_loader();
    let normalized = normalize_pet_vrm_relative_path(&relative_path)?;
    if !normalized.starts_with("vrm/models/custom/") {
        return Err("only custom VRM models can be read from the config directory".to_string());
    }

    let file_path = resolve_pet_vrm_model_file(loader.config_dir(), &normalized)?;
    let bytes = std::fs::read(&file_path).map_err(|error| {
        format!(
            "failed to read VRM model {}: {}",
            file_path.display(),
            error
        )
    })?;
    let file_name = file_path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("model.vrm")
        .to_string();

    Ok(PetVrmModelFileData {
        base64_data: BASE64_STANDARD.encode(bytes),
        content_type: "model/gltf-binary".to_string(),
        file_name,
    })
}

fn append_builtin_vrm_models(app_handle: &AppHandle, models: &mut Vec<VrmModelInfo>) {
    let models_dir = match app_handle
        .path()
        .resolve("vrm/models", BaseDirectory::Resource)
    {
        Ok(dir) => dir,
        Err(_) => return,
    };

    if !models_dir.exists() {
        return;
    }

    let entries = match std::fs::read_dir(&models_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("vrm") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(stem)
            .to_string();
        models.push(VrmModelInfo {
            id: stem.to_string(),
            name: stem.to_string(),
            path: format!("/vrm/models/{file_name}"),
            source: "builtin".to_string(),
            thumbnail: None,
        });
    }
}

const PET_VOICE_DIR_NAME: &str = "voice_resource";
const PET_VOICE_CUSTOM_DIR_NAME: &str = "custom";

// --- Voice Assets types ---

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetVoiceOption {
    pub id: String,
    pub label: String,
    pub relative_path: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetResolvedVoiceConfig {
    pub enabled: bool,
    pub provider: String,
    pub api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub siliconflow_api_key: Option<String>,
    pub minimax_api_key: Option<String>,
    pub base_url: String,
    pub model: Option<String>,
    pub voice_id: Option<String>,
    pub reference_voice: Option<String>,
    pub reference_text: Option<String>,
    pub speed: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetLoadedVoiceAssets {
    pub active_voice: PetResolvedVoiceConfig,
    pub config_directory_path: String,
    pub voice_options: Vec<PetVoiceOption>,
    pub voice_directory_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetSaveVoiceSelectionPayload {
    pub enabled: bool,
    pub provider: String,
    pub openai_api_key: Option<String>,
    pub siliconflow_api_key: Option<String>,
    pub minimax_api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub voice_id: Option<String>,
    pub reference_voice: Option<String>,
    pub reference_text: Option<String>,
    pub speed: f64,
    pub volume: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetImportVoiceFilePayload {
    pub base64_data: String,
    pub file_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetDeleteVoiceFilePayload {
    pub relative_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetVoiceFileData {
    pub base64_data: String,
    pub content_type: String,
    pub file_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetMiniMaxSynthesizePayload {
    pub text: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub voice_id: Option<String>,
    pub speed: Option<f64>,
    pub volume: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetMiniMaxSynthesizeResponse {
    pub base64_data: String,
    pub content_type: String,
}

type MiniMaxSocket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

struct MiniMaxSynthesizeResult {
    audio_bytes: Vec<u8>,
    chunk_count: usize,
    trace_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetSiliconFlowSynthesizePayload {
    pub text: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub voice: Option<String>,
    pub speed: Option<f64>,
    pub gain: Option<f64>,
    pub references: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetSiliconFlowSynthesizeResponse {
    pub base64_data: String,
    pub content_type: String,
}

#[tauri::command]
pub fn pet_load_voice_assets() -> Result<PetLoadedVoiceAssets, String> {
    let loader = config_loader();
    let config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    build_pet_voice_assets(loader.config_dir(), &config)
}

#[tauri::command]
pub fn pet_save_voice_selection(
    payload: PetSaveVoiceSelectionPayload,
) -> Result<PetLoadedVoiceAssets, String> {
    let loader = config_loader();
    let mut config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;

    config.pet.tts_enabled = payload.enabled;
    config.pet.tts_provider = normalize_tts_provider(&payload.provider);
    config.pet.tts_api_key = None;
    config.pet.tts_openai_api_key = payload
        .openai_api_key
        .filter(|value| !value.trim().is_empty());
    config.pet.tts_siliconflow_api_key = payload
        .siliconflow_api_key
        .filter(|value| !value.trim().is_empty());
    config.pet.tts_minimax_api_key = payload
        .minimax_api_key
        .filter(|value| !value.trim().is_empty());
    config.pet.tts_base_url = payload.base_url.unwrap_or_default();
    config.pet.tts_model = payload.model.filter(|value| !value.trim().is_empty());
    config.pet.tts_voice_id = payload.voice_id.filter(|value| !value.trim().is_empty());
    config.pet.tts_reference_voice = payload
        .reference_voice
        .as_deref()
        .map(normalize_pet_voice_relative_path)
        .transpose()?;
    config.pet.tts_reference_text = payload
        .reference_text
        .filter(|value| !value.trim().is_empty());
    config.pet.tts_speed = sanitized_pet_tts_speed(payload.speed);
    config.pet.tts_volume = sanitized_pet_tts_volume(payload.volume);

    loader
        .save(&config)
        .map_err(|e| format!("failed to save config: {}", e))?;
    build_pet_voice_assets(loader.config_dir(), &config)
}

#[tauri::command]
pub fn pet_import_voice_file(
    payload: PetImportVoiceFilePayload,
) -> Result<PetLoadedVoiceAssets, String> {
    let loader = config_loader();
    let mut config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    let config_dir = loader.config_dir();
    let voice_custom_dir = config_dir
        .join(PET_VOICE_DIR_NAME)
        .join(PET_VOICE_CUSTOM_DIR_NAME);
    let sanitized_name = sanitize_pet_voice_file_name(&payload.file_name);
    let target_path = voice_custom_dir.join(sanitized_name);

    std::fs::create_dir_all(&voice_custom_dir).map_err(|error| {
        format!(
            "failed to create voice import directory {}: {}",
            voice_custom_dir.display(),
            error
        )
    })?;

    let decoded_bytes = BASE64_STANDARD
        .decode(payload.base64_data.as_bytes())
        .map_err(|error| format!("failed to decode imported voice file: {}", error))?;
    std::fs::write(&target_path, decoded_bytes).map_err(|error| {
        format!(
            "failed to write imported voice file {}: {}",
            target_path.display(),
            error
        )
    })?;

    let relative_path = make_pet_voice_relative_path(config_dir, &target_path)?;
    config.pet.tts_reference_voice = Some(relative_path);
    if config.pet.tts_provider.trim().is_empty() || config.pet.tts_provider == "browser" {
        config.pet.tts_provider = "siliconflow".to_string();
    }
    if config.pet.tts_speed <= 0.0 || !config.pet.tts_speed.is_finite() {
        config.pet.tts_speed = 1.0;
    }
    if !config.pet.tts_volume.is_finite() || !(0.0..=2.0).contains(&config.pet.tts_volume) {
        config.pet.tts_volume = 1.0;
    }

    loader
        .save(&config)
        .map_err(|e| format!("failed to save config: {}", e))?;
    build_pet_voice_assets(config_dir, &config)
}

#[tauri::command]
pub fn pet_delete_voice_file(
    payload: PetDeleteVoiceFilePayload,
) -> Result<PetLoadedVoiceAssets, String> {
    let normalized_relative_path = normalize_pet_voice_relative_path(&payload.relative_path)?;
    if !normalized_relative_path.starts_with("voice_resource/custom/") {
        return Err("only custom voice files can be deleted".to_string());
    }

    let loader = config_loader();
    let mut config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    let target_path = resolve_pet_voice_file(loader.config_dir(), &normalized_relative_path)?;

    std::fs::remove_file(&target_path).map_err(|error| {
        format!(
            "failed to delete voice file {}: {}",
            target_path.display(),
            error
        )
    })?;

    if config.pet.tts_reference_voice.as_deref() == Some(normalized_relative_path.as_str()) {
        config.pet.tts_reference_voice = None;
        config.pet.tts_reference_text = None;
    }

    loader
        .save(&config)
        .map_err(|e| format!("failed to save config: {}", e))?;
    build_pet_voice_assets(loader.config_dir(), &config)
}

#[tauri::command]
pub fn pet_read_voice_file(relative_path: String) -> Result<PetVoiceFileData, String> {
    let loader = config_loader();
    let normalized_relative_path = normalize_pet_voice_relative_path(&relative_path)?;
    let file_path = resolve_pet_voice_file(loader.config_dir(), &normalized_relative_path)?;
    let file_bytes = std::fs::read(&file_path).map_err(|error| {
        format!(
            "failed to read voice file {}: {}",
            file_path.display(),
            error
        )
    })?;
    let file_name = file_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("voice.mp3")
        .to_string();

    Ok(PetVoiceFileData {
        base64_data: BASE64_STANDARD.encode(file_bytes),
        content_type: pet_voice_content_type(&file_name).to_string(),
        file_name,
    })
}

#[tauri::command]
pub async fn pet_minimax_synthesize(
    payload: PetMiniMaxSynthesizePayload,
) -> Result<PetMiniMaxSynthesizeResponse, String> {
    let api_key = payload.api_key.trim();
    if api_key.is_empty() {
        return Err("MiniMax API key is required".to_string());
    }

    let base_url = payload
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("https://api.minimaxi.com");
    let model = payload
        .model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("speech-2.8-hd");
    let voice_id = payload
        .voice_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("male-qn-qingse");
    let speed = payload
        .speed
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(1.0)
        .clamp(1.0, 2.0)
        .round() as i32;
    let volume = payload
        .volume
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(1.0)
        .clamp(1.0, 10.0)
        .round() as i32;
    let ws_url = minimax_websocket_url(base_url);

    info!(
        model = model,
        voice_id = voice_id,
        ws_url = %ws_url,
        key_prefix = %&api_key[..api_key.len().min(8)],
        key_len = api_key.len(),
        "calling MiniMax TTS websocket API"
    );

    let mut socket = minimax_establish_connection(base_url, api_key).await?;
    let result =
        minimax_synthesize_sync(&mut socket, model, voice_id, speed, volume, &payload.text).await;
    let finish_result = minimax_finish_socket(&mut socket).await;
    let result = result?;
    finish_result?;

    info!(
        model = model,
        voice_id = voice_id,
        chunk_count = result.chunk_count,
        audio_bytes = result.audio_bytes.len(),
        trace_id = result.trace_id.as_deref().unwrap_or("n/a"),
        "MiniMax TTS synthesis completed via websocket"
    );

    Ok(PetMiniMaxSynthesizeResponse {
        base64_data: BASE64_STANDARD.encode(result.audio_bytes),
        content_type: "audio/mpeg".to_string(),
    })
}

async fn minimax_establish_connection(
    base_url: &str,
    api_key: &str,
) -> Result<MiniMaxSocket, String> {
    let ws_url = minimax_websocket_url(base_url);
    let mut request = ws_url
        .clone()
        .into_client_request()
        .map_err(|error| format!("failed to build MiniMax websocket request: {}", error))?;
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|error| format!("failed to build MiniMax authorization header: {}", error))?,
    );

    let tls = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()
        .map_err(|error| format!("failed to build MiniMax TLS connector: {}", error))?;
    let connector = Connector::NativeTls(tls);

    let (mut socket, _) = timeout(
        Duration::from_secs(10),
        connect_async_tls_with_config(request, None, false, Some(connector)),
    )
    .await
    .map_err(|_| {
        format!(
            "timed out while connecting to MiniMax websocket: {}",
            ws_url
        )
    })?
    .map_err(|error| {
        format!(
            "failed to connect to MiniMax websocket: {} (input base_url: {}, resolved ws_url: {})",
            error, base_url, ws_url
        )
    })?;

    minimax_expect_event(&mut socket, "connected_success").await?;
    Ok(socket)
}

async fn minimax_synthesize_sync(
    socket: &mut MiniMaxSocket,
    model: &str,
    voice_id: &str,
    speed: i32,
    volume: i32,
    text: &str,
) -> Result<MiniMaxSynthesizeResult, String> {
    minimax_send_json(
        socket,
        serde_json::json!({
            "event": "task_start",
            "model": model,
            "voice_setting": {
                "voice_id": voice_id,
                "speed": speed,
                "vol": volume,
                "pitch": 0,
                "english_normalization": false
            },
            "audio_setting": {
                "sample_rate": 32000,
                "bitrate": 128000,
                "format": "mp3",
                "channel": 1
            }
        }),
    )
    .await?;
    minimax_expect_event(socket, "task_started").await?;

    minimax_send_json(
        socket,
        serde_json::json!({
            "event": "task_continue",
            "text": text
        }),
    )
    .await?;

    let mut audio_bytes = Vec::new();
    let mut chunk_count = 0usize;
    let mut trace_id = None;

    loop {
        let message = minimax_read_json_message(socket).await?;
        if trace_id.is_none() {
            trace_id = message
                .get("trace_id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned);
        }

        if let Some(audio_hex) = message
            .get("data")
            .and_then(|value| value.get("audio"))
            .and_then(serde_json::Value::as_str)
        {
            if !audio_hex.trim().is_empty() {
                let chunk = decode_hex_audio(audio_hex)?;
                chunk_count += 1;
                audio_bytes.extend_from_slice(&chunk);
            }
        }

        if let Some(event) = message.get("event").and_then(serde_json::Value::as_str) {
            if event.eq_ignore_ascii_case("error") || event.eq_ignore_ascii_case("task_failed") {
                let detail = message
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("MiniMax websocket returned an error event");
                return Err(detail.to_string());
            }
        }

        if message
            .get("is_final")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            break;
        }
    }

    if audio_bytes.is_empty() {
        return Err("MiniMax TTS completed without audio data".to_string());
    }

    Ok(MiniMaxSynthesizeResult {
        audio_bytes,
        chunk_count,
        trace_id,
    })
}

async fn minimax_send_json(
    socket: &mut MiniMaxSocket,
    payload: serde_json::Value,
) -> Result<(), String> {
    socket
        .send(WsMessage::Text(payload.to_string()))
        .await
        .map_err(|error| format!("failed to send MiniMax websocket message: {}", error))
}

async fn minimax_expect_event(socket: &mut MiniMaxSocket, expected: &str) -> Result<(), String> {
    let message = minimax_read_json_message(socket).await?;
    let actual = message
        .get("event")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "MiniMax websocket response is missing event field".to_string())?;

    if actual != expected {
        let detail = message
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        return Err(format!(
            "unexpected MiniMax websocket event: expected {}, got {}{}",
            expected,
            actual,
            if detail.is_empty() {
                String::new()
            } else {
                format!(" ({detail})")
            }
        ));
    }

    Ok(())
}

async fn minimax_read_json_message(
    socket: &mut MiniMaxSocket,
) -> Result<serde_json::Value, String> {
    loop {
        let frame = timeout(Duration::from_secs(30), socket.next())
            .await
            .map_err(|_| "timed out while waiting for MiniMax websocket message".to_string())?
            .ok_or_else(|| "MiniMax websocket closed unexpectedly".to_string())?
            .map_err(|error| format!("MiniMax websocket stream error: {}", error))?;

        match frame {
            WsMessage::Text(text) => {
                let value = serde_json::from_str::<serde_json::Value>(&text)
                    .map_err(|error| format!("invalid MiniMax websocket payload: {}", error))?;
                return Ok(value);
            }
            WsMessage::Ping(payload) => {
                socket
                    .send(WsMessage::Pong(payload))
                    .await
                    .map_err(|error| format!("failed to respond to MiniMax ping: {}", error))?;
            }
            WsMessage::Close(frame) => {
                let detail = frame
                    .map(|value| value.reason.to_string())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "unknown close frame".to_string());
                return Err(format!("MiniMax websocket closed: {}", detail));
            }
            WsMessage::Binary(_) => {
                return Err("MiniMax websocket returned unexpected binary frame".to_string());
            }
            WsMessage::Pong(_) | WsMessage::Frame(_) => {}
        }
    }
}

async fn minimax_finish_socket(socket: &mut MiniMaxSocket) -> Result<(), String> {
    let _ = minimax_send_json(socket, serde_json::json!({ "event": "task_finish" })).await;
    socket
        .close(None)
        .await
        .map_err(|error| format!("failed to close MiniMax websocket: {}", error))
}

fn minimax_websocket_url(base_url: &str) -> String {
    const DEFAULT_HOST: &str = "api.minimaxi.com";
    const WS_PATH: &str = "/ws/v1/t2a_v2";

    let trimmed = base_url.trim();
    let candidate = if trimmed.is_empty() {
        format!("https://{DEFAULT_HOST}")
    } else if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{}", trimmed.trim_matches('/'))
    };

    let mut url = match reqwest::Url::parse(&candidate) {
        Ok(url) => url,
        Err(_) => {
            return format!("wss://{DEFAULT_HOST}{WS_PATH}");
        }
    };

    match url.scheme() {
        "http" | "ws" => {
            let _ = url.set_scheme("ws");
        }
        "https" | "wss" => {
            let _ = url.set_scheme("wss");
        }
        _ => {
            let _ = url.set_scheme("wss");
        }
    }

    if matches!(url.host_str(), Some("platform.minimaxi.com")) {
        let _ = url.set_host(Some(DEFAULT_HOST));
    }

    url.set_path(WS_PATH);
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

#[cfg(test)]
mod minimax_url_tests {
    use super::minimax_websocket_url;

    #[test]
    fn minimax_websocket_url_normalizes_api_base() {
        assert_eq!(
            minimax_websocket_url("https://api.minimaxi.com"),
            "wss://api.minimaxi.com/ws/v1/t2a_v2"
        );
        assert_eq!(
            minimax_websocket_url("https://api.minimaxi.com/v1"),
            "wss://api.minimaxi.com/ws/v1/t2a_v2"
        );
    }

    #[test]
    fn minimax_websocket_url_accepts_full_websocket_endpoint() {
        assert_eq!(
            minimax_websocket_url("wss://api.minimaxi.com/ws/v1/t2a_v2"),
            "wss://api.minimaxi.com/ws/v1/t2a_v2"
        );
    }

    #[test]
    fn minimax_websocket_url_rewrites_docs_host_and_paths() {
        assert_eq!(
            minimax_websocket_url("https://platform.minimaxi.com/docs/guides/speech-t2a-websocket"),
            "wss://api.minimaxi.com/ws/v1/t2a_v2"
        );
    }
}

#[tauri::command]
pub async fn pet_siliconflow_synthesize(
    payload: PetSiliconFlowSynthesizePayload,
) -> Result<PetSiliconFlowSynthesizeResponse, String> {
    let api_key = payload.api_key.trim();
    if api_key.is_empty() {
        return Err("SiliconFlow API key is required".to_string());
    }

    let base_url = payload
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("https://api.siliconflow.cn/v1");
    let endpoint = format!("{}/audio/speech", base_url.trim_end_matches('/'));
    let model = payload
        .model
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("fnlp/MOSS-TTSD-v0.5");
    let voice = payload
        .voice
        .as_deref()
        .map(str::trim)
        .unwrap_or("fnlp/MOSS-TTSD-v0.5:anna");
    let speed = payload
        .speed
        .filter(|v| v.is_finite() && *v > 0.0)
        .unwrap_or(1.0);
    let gain = payload.gain.filter(|v| v.is_finite()).unwrap_or(0.0);

    let mut body = serde_json::json!({
        "model": model,
        "input": payload.text,
        "voice": voice,
        "response_format": "mp3",
        "speed": speed,
        "gain": gain,
        "stream": false
    });

    // voice �?references 互斥：有 references 时必须移�?voice 字段
    if let Some(refs) = &payload.references {
        if !refs.is_empty() {
            body.as_object_mut().and_then(|obj| obj.remove("voice"));
            body["references"] = serde_json::Value::Array(refs.clone());
        }
    }

    info!(
        model = model,
        voice = voice,
        endpoint = endpoint,
        has_references = payload
            .references
            .as_ref()
            .map(|r| !r.is_empty())
            .unwrap_or(false),
        "calling SiliconFlow TTS API"
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|error| format!("failed to build HTTP client: {}", error))?;

    let response = client
        .post(&endpoint)
        .bearer_auth(api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|error| format!("SiliconFlow TTS request failed: {}", error))?;

    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("audio/mpeg")
        .to_string();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("failed to read SiliconFlow response: {}", error))?;

    if !status.is_success() {
        let body_preview = String::from_utf8_lossy(&bytes);
        return Err(format!(
            "SiliconFlow TTS error: HTTP {} body={}",
            status, body_preview
        ));
    }

    if bytes.is_empty() {
        return Err("SiliconFlow TTS returned empty audio".to_string());
    }

    info!(
        model = model,
        audio_bytes = bytes.len(),
        content_type = content_type,
        "SiliconFlow TTS synthesis completed"
    );
    Ok(PetSiliconFlowSynthesizeResponse {
        base64_data: BASE64_STANDARD.encode(&bytes),
        content_type,
    })
}

fn build_pet_voice_assets(
    config_dir: &Path,
    config: &Config,
) -> Result<PetLoadedVoiceAssets, String> {
    let voice_dir = config_dir.join(PET_VOICE_DIR_NAME);
    let provider = normalize_tts_provider(&config.pet.tts_provider);
    Ok(PetLoadedVoiceAssets {
        active_voice: PetResolvedVoiceConfig {
            enabled: config.pet.tts_enabled,
            provider: provider.clone(),
            api_key: pet_tts_api_key_for_provider(&config.pet, &provider),
            openai_api_key: config.pet.tts_openai_api_key.clone(),
            siliconflow_api_key: config.pet.tts_siliconflow_api_key.clone(),
            minimax_api_key: config.pet.tts_minimax_api_key.clone(),
            base_url: config.pet.tts_base_url.clone(),
            model: config.pet.tts_model.clone(),
            voice_id: config.pet.tts_voice_id.clone(),
            reference_voice: config.pet.tts_reference_voice.clone(),
            reference_text: config.pet.tts_reference_text.clone(),
            speed: sanitized_pet_tts_speed(config.pet.tts_speed),
            volume: sanitized_pet_tts_volume(config.pet.tts_volume),
        },
        config_directory_path: config_dir.to_string_lossy().to_string(),
        voice_options: scan_pet_voice_files(config_dir)?,
        voice_directory_path: voice_dir.to_string_lossy().to_string(),
    })
}

fn scan_pet_voice_files(config_dir: &Path) -> Result<Vec<PetVoiceOption>, String> {
    let voice_root = config_dir.join(PET_VOICE_DIR_NAME);
    let mut options = Vec::new();
    if !voice_root.exists() {
        return Ok(options);
    }

    let mut stack = vec![voice_root];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory).map_err(|error| {
            format!(
                "failed to scan voice directory {}: {}",
                directory.display(),
                error
            )
        })? {
            let entry = entry.map_err(|error| {
                format!(
                    "failed to read voice directory entry in {}: {}",
                    directory.display(),
                    error
                )
            })?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|error| {
                format!("failed to inspect voice file {}: {}", path.display(), error)
            })?;
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            let file_name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
            if !is_pet_voice_file(file_name) {
                continue;
            }

            let relative_path = make_pet_voice_relative_path(config_dir, &path)?;
            let source = if relative_path.starts_with("voice_resource/custom/") {
                "custom"
            } else {
                "builtin"
            };

            options.push(PetVoiceOption {
                id: relative_path.clone(),
                label: derive_pet_voice_label(file_name),
                relative_path,
                source: source.to_string(),
            });
        }
    }
    options.sort_by(|left, right| left.label.cmp(&right.label));
    Ok(options)
}

fn pet_vrm_custom_models_dir(config_dir: &Path) -> PathBuf {
    config_dir
        .join(PET_VRM_DIR_NAME)
        .join(PET_VRM_MODELS_DIR_NAME)
        .join(PET_VRM_CUSTOM_DIR_NAME)
}

fn append_custom_vrm_models(
    config_dir: &Path,
    models: &mut Vec<VrmModelInfo>,
) -> Result<(), String> {
    let custom_dir = pet_vrm_custom_models_dir(config_dir);
    if !custom_dir.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(&custom_dir).map_err(|error| {
        format!(
            "failed to scan custom VRM directory {}: {}",
            custom_dir.display(),
            error
        )
    })? {
        let entry = entry.map_err(|error| {
            format!(
                "failed to read custom VRM directory entry in {}: {}",
                custom_dir.display(),
                error
            )
        })?;
        let path = entry.path();
        if path.extension().and_then(OsStr::to_str) != Some("vrm") {
            continue;
        }
        let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or("custom");
        models.push(VrmModelInfo {
            id: stem.to_string(),
            name: stem.to_string(),
            path: make_pet_vrm_relative_path(config_dir, &path)?,
            source: "custom".to_string(),
            thumbnail: None,
        });
    }
    Ok(())
}

fn normalize_pet_vrm_relative_path(relative_path: &str) -> Result<String, String> {
    let normalized = relative_path.trim().replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains('\0')
        || normalized
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || !normalized.starts_with("vrm/models/")
        || !normalized.to_lowercase().ends_with(".vrm")
    {
        return Err("VRM model path must stay under vrm/models and end with .vrm".to_string());
    }
    Ok(normalized)
}

fn make_pet_vrm_relative_path(config_dir: &Path, absolute_path: &Path) -> Result<String, String> {
    let relative_path = absolute_path.strip_prefix(config_dir).map_err(|error| {
        format!(
            "failed to create relative path from {} to {}: {}",
            config_dir.display(),
            absolute_path.display(),
            error
        )
    })?;
    normalize_pet_vrm_relative_path(&relative_path.to_string_lossy())
}

fn resolve_pet_vrm_model_file(config_dir: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let normalized = normalize_pet_vrm_relative_path(relative_path)?;
    let vrm_root = config_dir.join(PET_VRM_DIR_NAME);
    let candidate = config_dir.join(&normalized);
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|error| format!("failed to resolve VRM model path: {}", error))?;
    let canonical_vrm_root = vrm_root
        .canonicalize()
        .map_err(|error| format!("failed to resolve VRM resource directory: {}", error))?;

    if !canonical_candidate.starts_with(canonical_vrm_root) {
        return Err("VRM model file must be within vrm".to_string());
    }
    Ok(canonical_candidate)
}

fn sanitize_pet_vrm_file_name(file_name: &str) -> Result<String, String> {
    let trimmed = file_name.trim();
    let base_name = if trimmed.is_empty() {
        "custom-model.vrm"
    } else {
        trimmed
    };
    let sanitized: String = base_name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => c,
        })
        .collect();
    if !sanitized.to_lowercase().ends_with(".vrm") {
        return Err("only .vrm model files can be imported".to_string());
    }
    Ok(sanitized)
}

fn resolve_pet_voice_file(config_dir: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let normalized_relative_path = normalize_pet_voice_relative_path(relative_path)?;
    let voice_root = config_dir.join(PET_VOICE_DIR_NAME);
    let candidate = config_dir.join(&normalized_relative_path);
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|error| format!("failed to resolve voice file path: {}", error))?;
    let canonical_voice_root = voice_root
        .canonicalize()
        .map_err(|error| format!("failed to resolve voice resource directory: {}", error))?;

    if !canonical_candidate.starts_with(canonical_voice_root) {
        return Err("voice file must be within voice_resource".to_string());
    }
    Ok(canonical_candidate)
}

fn normalize_pet_voice_relative_path(relative_path: &str) -> Result<String, String> {
    let normalized = relative_path.trim().replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains('\0')
        || normalized
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || !normalized.starts_with("voice_resource/")
    {
        return Err("voice file path must stay under voice_resource".to_string());
    }
    Ok(normalized)
}

fn make_pet_voice_relative_path(config_dir: &Path, absolute_path: &Path) -> Result<String, String> {
    let relative_path = absolute_path.strip_prefix(config_dir).map_err(|error| {
        format!(
            "failed to create relative path from {} to {}: {}",
            config_dir.display(),
            absolute_path.display(),
            error
        )
    })?;
    normalize_pet_voice_relative_path(&relative_path.to_string_lossy())
}

fn sanitize_pet_voice_file_name(file_name: &str) -> String {
    let trimmed = file_name.trim();
    let base_name = if trimmed.is_empty() {
        "custom-voice.mp3"
    } else {
        trimmed
    };
    base_name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => c,
        })
        .collect()
}

fn is_pet_voice_file(file_name: &str) -> bool {
    let lower = file_name.to_lowercase();
    lower.ends_with(".mp3")
        || lower.ends_with(".wav")
        || lower.ends_with(".ogg")
        || lower.ends_with(".m4a")
        || lower.ends_with(".webm")
}

fn pet_voice_content_type(file_name: &str) -> &'static str {
    let lower = file_name.to_lowercase();
    if lower.ends_with(".wav") {
        "audio/wav"
    } else if lower.ends_with(".ogg") {
        "audio/ogg"
    } else if lower.ends_with(".m4a") {
        "audio/m4a"
    } else if lower.ends_with(".webm") {
        "audio/webm"
    } else {
        "audio/mpeg"
    }
}

fn derive_pet_voice_label(file_name: &str) -> String {
    file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name)
        .replace(['_', '-'], " ")
        .trim()
        .to_string()
}

#[cfg(test)]
mod pet_vrm_model_tests {
    use super::{
        append_custom_vrm_models, make_pet_vrm_relative_path, normalize_pet_vrm_relative_path,
        sanitize_pet_vrm_file_name,
    };

    #[test]
    fn normalizes_custom_vrm_relative_path() {
        let path = normalize_pet_vrm_relative_path("vrm\\models\\custom\\Alice.vrm").unwrap();
        assert_eq!(path, "vrm/models/custom/Alice.vrm");
    }

    #[test]
    fn rejects_invalid_vrm_relative_paths() {
        assert!(normalize_pet_vrm_relative_path("vrm/models/custom/../secret.vrm").is_err());
        assert!(normalize_pet_vrm_relative_path("/vrm/models/custom/model.vrm").is_err());
        assert!(normalize_pet_vrm_relative_path("vrm/models/custom/model.txt").is_err());
    }

    #[test]
    fn sanitizes_imported_vrm_file_names() {
        assert_eq!(
            sanitize_pet_vrm_file_name("bad:name?.vrm").unwrap(),
            "bad-name-.vrm"
        );
        assert!(sanitize_pet_vrm_file_name("bad.glb").is_err());
    }

    #[test]
    fn scans_custom_vrm_models_under_config_dir() {
        let temp = tempfile::tempdir().unwrap();
        let model_dir = temp.path().join("vrm").join("models").join("custom");
        std::fs::create_dir_all(&model_dir).unwrap();
        let model_path = model_dir.join("Custom.vrm");
        std::fs::write(&model_path, b"vrm").unwrap();
        std::fs::write(model_dir.join("ignored.txt"), b"no").unwrap();

        let relative = make_pet_vrm_relative_path(temp.path(), &model_path).unwrap();
        assert_eq!(relative, "vrm/models/custom/Custom.vrm");

        let mut models = Vec::new();
        append_custom_vrm_models(temp.path(), &mut models).unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "Custom");
        assert_eq!(models[0].source, "custom");
        assert_eq!(models[0].path, "vrm/models/custom/Custom.vrm");
    }
}

fn decode_hex_audio(value: &str) -> Result<Vec<u8>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.len() % 2 != 0 {
        return Err("MiniMax audio chunk is not valid hex".to_string());
    }

    let mut bytes = Vec::with_capacity(trimmed.len() / 2);
    for index in (0..trimmed.len()).step_by(2) {
        let byte = u8::from_str_radix(&trimmed[index..index + 2], 16)
            .map_err(|error| format!("failed to decode MiniMax audio chunk: {}", error))?;
        bytes.push(byte);
    }
    Ok(bytes)
}

fn normalize_tts_provider(provider: &str) -> String {
    match provider.trim().to_lowercase().as_str() {
        "openai" => "openai".to_string(),
        "siliconflow" => "siliconflow".to_string(),
        "minimax" => "minimax".to_string(),
        _ => "browser".to_string(),
    }
}

fn pet_tts_api_key_for_provider(
    config: &agent_diva_core::config::schema::PetConfig,
    provider: &str,
) -> Option<String> {
    match provider {
        "openai" => config.tts_openai_api_key.clone(),
        "siliconflow" => config.tts_siliconflow_api_key.clone(),
        "minimax" => config.tts_minimax_api_key.clone(),
        _ => None,
    }
}

fn sanitized_pet_tts_speed(speed: f64) -> f64 {
    if speed.is_finite() && speed > 0.0 {
        speed
    } else {
        1.0
    }
}

fn sanitized_pet_tts_volume(volume: f64) -> f64 {
    if volume.is_finite() && (0.0..=2.0).contains(&volume) {
        volume
    } else {
        1.0
    }
}

// ============================================================
// Token Usage Statistics Commands
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageTotal {
    pub total_input: i64,
    pub total_output: i64,
    pub total_tokens: i64,
    pub total_cache_creation: i64,
    pub total_cache_read: i64,
    pub request_count: u64,
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageSummary {
    pub group_key: String,
    pub total_input: i64,
    pub total_output: i64,
    pub total_tokens: i64,
    pub total_cache_creation: i64,
    pub total_cache_read: i64,
    pub request_count: u64,
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTimelinePoint {
    pub time_bucket: String,
    pub total_input: i64,
    pub total_output: i64,
    pub total_tokens: i64,
    pub request_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSessionUsage {
    pub session_id: String,
    pub total_input: i64,
    pub total_output: i64,
    pub total_tokens: i64,
    pub request_count: u64,
    pub total_cost: f64,
    pub primary_model: String,
    pub channel: Option<String>,
    pub last_activity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenModelDistribution {
    pub model: String,
    pub percentage: f64,
    pub total_tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInMemoryStats {
    pub total_tokens: i64,
    pub total_input: i64,
    pub total_output: i64,
    pub request_count: u64,
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiResponse<T> {
    status: String,
    data: Option<T>,
    message: Option<String>,
}

async fn fetch_token_stats<T: serde::de::DeserializeOwned>(
    state: &AgentState,
    endpoint: &str,
) -> Result<T, String> {
    let url = format!("{}{}", state.api_base_url(), endpoint);
    let response = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch token stats: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let api_response: ApiResponse<T> = response
        .json()
        .await
        .map_err(|e| format!("Invalid response payload: {}", e))?;

    if api_response.status != "ok" {
        return Err(api_response
            .message
            .unwrap_or_else(|| "Unknown error".to_string()));
    }

    api_response
        .data
        .ok_or_else(|| "No data in response".to_string())
}

#[tauri::command]
pub async fn get_token_usage_total(
    state: State<'_, AgentState>,
    period: String,
    tz_offset: Option<i32>,
) -> Result<TokenUsageTotal, String> {
    let mut endpoint = format!("/stats/tokens/total?period={}", period);
    if let Some(tz) = tz_offset {
        endpoint.push_str(&format!("&tz_offset={}", tz));
    }
    fetch_token_stats(&state, &endpoint).await
}

#[tauri::command]
pub async fn get_token_usage_summary(
    state: State<'_, AgentState>,
    period: String,
    group_by: String,
    tz_offset: Option<i32>,
) -> Result<Vec<TokenUsageSummary>, String> {
    let mut endpoint = format!(
        "/stats/tokens/summary?period={}&group_by={}",
        period, group_by
    );
    if let Some(tz) = tz_offset {
        endpoint.push_str(&format!("&tz_offset={}", tz));
    }
    fetch_token_stats(&state, &endpoint).await
}

#[tauri::command]
pub async fn get_token_usage_timeline(
    state: State<'_, AgentState>,
    period: String,
    interval: Option<String>,
    tz_offset: Option<i32>,
) -> Result<Vec<TokenTimelinePoint>, String> {
    let mut endpoint = match interval {
        Some(int) => format!("/stats/tokens/timeline?period={}&interval={}", period, int),
        None => format!("/stats/tokens/timeline?period={}", period),
    };
    if let Some(tz) = tz_offset {
        endpoint.push_str(&format!("&tz_offset={}", tz));
    }
    fetch_token_stats(&state, &endpoint).await
}

#[tauri::command]
pub async fn get_token_usage_sessions(
    state: State<'_, AgentState>,
    period: String,
    limit: u64,
    tz_offset: Option<i32>,
) -> Result<Vec<TokenSessionUsage>, String> {
    let mut endpoint = format!("/stats/tokens/sessions?period={}&limit={}", period, limit);
    if let Some(tz) = tz_offset {
        endpoint.push_str(&format!("&tz_offset={}", tz));
    }
    fetch_token_stats(&state, &endpoint).await
}

#[tauri::command]
pub async fn get_token_usage_models(
    state: State<'_, AgentState>,
    period: String,
    tz_offset: Option<i32>,
) -> Result<Vec<TokenModelDistribution>, String> {
    let mut endpoint = format!("/stats/tokens/models?period={}", period);
    if let Some(tz) = tz_offset {
        endpoint.push_str(&format!("&tz_offset={}", tz));
    }
    fetch_token_stats(&state, &endpoint).await
}

#[tauri::command]
pub async fn get_token_usage_realtime(
    state: State<'_, AgentState>,
) -> Result<TokenInMemoryStats, String> {
    fetch_token_stats(&state, "/stats/tokens/realtime").await
}

// ============================================================
// Sandbox Commands
// ============================================================

#[tauri::command]
pub fn get_sandbox_config() -> Result<serde_json::Value, String> {
    let loader = config_loader();
    let config = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    serde_json::to_value(&config.sandbox)
        .map_err(|e| format!("failed to serialize sandbox config: {}", e))
}

#[tauri::command]
pub fn save_sandbox_config(config: serde_json::Value) -> Result<(), String> {
    let sandbox_config: agent_diva_core::config::SandboxConfig =
        serde_json::from_value(config).map_err(|e| format!("Invalid sandbox config: {}", e))?;
    let loader = config_loader();
    let mut current = loader
        .load()
        .map_err(|e| format!("failed to load config: {}", e))?;
    current.sandbox = sandbox_config;
    loader
        .save(&current)
        .map_err(|e| format!("failed to save config: {}", e))
}

#[tauri::command]
pub fn get_gui_prefs(app: AppHandle) -> Result<GuiPrefs, String> {
    let store = app
        .store("settings.json")
        .map_err(|e| format!("Failed to get store: {}", e))?;

    let close_to_tray = store
        .get("closeToTray")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(GuiPrefs { close_to_tray })
}

#[tauri::command]
pub fn set_gui_prefs(app: AppHandle, prefs: GuiPrefs) -> Result<(), String> {
    let store = app
        .store("settings.json")
        .map_err(|e| format!("Failed to get store: {}", e))?;

    store.set("closeToTray", serde_json::json!(prefs.close_to_tray));
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn open_desktop_pet(app: AppHandle) -> Result<(), String> {
    // Show existing window or create a new one
    if let Some(window) = app.get_webview_window("desktop-pet") {
        window
            .set_ignore_cursor_events(false)
            .map_err(|e| format!("Failed to reset ignore cursor events: {}", e))?;
        let _ = app.emit_to("desktop-pet", "desktop-pet-render-resume", true);
        window
            .show()
            .map_err(|e| format!("Failed to show desktop-pet window: {}", e))?;
        if cfg!(debug_assertions) {
            window.open_devtools();
        }
    } else {
        // Calculate bottom-right position
        let (x, y) = app
            .available_monitors()
            .map_err(|e| e.to_string())?
            .into_iter()
            .next()
            .map(|m| {
                let size = m.size();
                let scale = m.scale_factor();
                let logical_w = size.width as f64 / scale;
                let logical_h = size.height as f64 / scale;
                let lx = (logical_w - 400.0 - 40.0).max(0.0);
                let ly = (logical_h - 600.0 - 60.0).max(0.0);
                (lx, ly)
            })
            .unwrap_or((100.0, 100.0));

        let window = WebviewWindowBuilder::new(
            &app,
            "desktop-pet",
            WebviewUrl::App("desktop-pet.html".into()),
        )
        .inner_size(400.0, 600.0)
        .position(x, y)
        .visible(true)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .build()
        .map_err(|e| format!("Failed to create desktop-pet window: {}", e))?;
        window
            .set_ignore_cursor_events(false)
            .map_err(|e| format!("Failed to reset ignore cursor events: {}", e))?;
        window
            .show()
            .map_err(|e| format!("Failed to show desktop-pet window: {}", e))?;
        if cfg!(debug_assertions) {
            window.open_devtools();
        }
        let _ = app.emit_to("desktop-pet", "desktop-pet-render-resume", true);
    }
    app.emit_to("main", "desktop-pet-active", true)
        .map_err(|e| format!("Failed to emit event: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn close_desktop_pet(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("desktop-pet")
        .ok_or("desktop-pet window not found")?;
    let _ = app.emit_to("desktop-pet", "desktop-pet-render-pause", true);
    window
        .set_ignore_cursor_events(false)
        .map_err(|e| format!("Failed to reset ignore cursor events: {}", e))?;
    window
        .hide()
        .map_err(|e| format!("Failed to hide desktop-pet window: {}", e))?;
    app.emit_to("main", "desktop-pet-inactive", false)
        .map_err(|e| format!("Failed to emit event: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn set_desktop_pet_ignore_mouse(app: AppHandle, ignore: bool) -> Result<(), String> {
    let window = app
        .get_webview_window("desktop-pet")
        .ok_or("desktop-pet window not found")?;
    window
        .set_ignore_cursor_events(ignore)
        .map_err(|e| format!("Failed to set ignore cursor events: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn set_desktop_pet_always_on_top(app: AppHandle, always_on_top: bool) -> Result<(), String> {
    let window = app
        .get_webview_window("desktop-pet")
        .ok_or("desktop-pet window not found")?;
    window
        .set_always_on_top(always_on_top)
        .map_err(|e| format!("Failed to set always_on_top: {}", e))
}

#[tauri::command]
pub fn minimize_desktop_pet(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("desktop-pet")
        .ok_or("desktop-pet window not found")?;
    window
        .minimize()
        .map_err(|e| format!("Failed to minimize desktop-pet window: {}", e))
}

// ============================================================
// Audit Log Commands
// ============================================================

/// DTO returned to the frontend for each parsed audit event.
///
/// Re-exports the shared type from `agent-diva-core` with camelCase serde
/// renaming for the JavaScript frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventDto {
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: String,
}

impl From<agent_diva_core::audit_parse::AuditEventDto> for AuditEventDto {
    fn from(dto: agent_diva_core::audit_parse::AuditEventDto) -> Self {
        Self {
            event_type: dto.event_type,
            data: dto.data,
            timestamp: dto.timestamp,
        }
    }
}

/// Parse audit events from gateway.log for a given date.
///
/// Reads ~/.diva/logs/gateway.log.YYYY-MM-DD, filters lines where
/// `target == "audit"`, and deserializes each line into an AuditEventDto.
#[tauri::command]
pub fn get_audit_events(date: String) -> Result<Vec<AuditEventDto>, String> {
    let log_path = resolve_audit_log_path(&date)?;
    if !log_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&log_path)
        .map_err(|e| format!("failed to read audit log for {}: {}", date, e))?;

    let events: Vec<AuditEventDto> = content
        .lines()
        .filter(|line| {
            line.contains(r#""target":"audit""#) || line.contains(r#""target": "audit""#)
        })
        .filter_map(|line| {
            agent_diva_core::audit_parse::parse_audit_event_from_json_line(line)
                .map(AuditEventDto::from)
        })
        .collect();

    Ok(events)
}

/// A sanitized console entry emitted by the desktop application's WebView.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiLogEntry {
    pub timestamp: String,
    pub level: String,
    pub source: String,
    pub message: String,
    pub args: serde_json::Value,
    pub window_label: String,
}

const GUI_LOG_VALUE_LIMIT: usize = 4_096;

fn valid_log_date(date: &str) -> Result<(), String> {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| format!("invalid log date: {date}"))
}

fn sanitize_gui_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    let sensitive = [
                        "apikey",
                        "api_key",
                        "token",
                        "password",
                        "authorization",
                        "secret",
                    ]
                    .iter()
                    .any(|needle| key.to_ascii_lowercase().contains(needle));
                    (
                        key,
                        if sensitive {
                            serde_json::Value::String("[REDACTED]".to_string())
                        } else {
                            sanitize_gui_value(value)
                        },
                    )
                })
                .collect(),
        ),
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(sanitize_gui_value).collect())
        }
        serde_json::Value::String(value) if value.chars().count() > GUI_LOG_VALUE_LIMIT => {
            serde_json::Value::String(
                value.chars().take(GUI_LOG_VALUE_LIMIT).collect::<String>() + "…[truncated]",
            )
        }
        value => value,
    }
}

fn gui_log_path(date: &str) -> Result<PathBuf, String> {
    valid_log_date(date)?;
    let loader = ConfigLoader::new();
    let config = loader.load().unwrap_or_default();
    Ok(
        resolve_configured_path(&config.logging.dir, loader.config_dir())
            .join(format!("gui.log.{date}")),
    )
}

/// Append sanitized GUI console records to the date-specific desktop log.
#[tauri::command]
pub fn append_gui_log(entries: Vec<GuiLogEntry>) -> Result<(), String> {
    use std::io::Write;
    for mut entry in entries {
        let date = entry
            .timestamp
            .get(..10)
            .ok_or("GUI log timestamp is missing a date")?;
        let path = gui_log_path(date)?;
        let parent = path.parent().ok_or("GUI log path has no parent")?;
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create GUI log directory: {e}"))?;
        entry.source = "gui".to_string();
        entry.message = sanitize_gui_value(serde_json::Value::String(entry.message))
            .as_str()
            .unwrap_or_default()
            .to_string();
        entry.args = sanitize_gui_value(entry.args);
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("failed to open GUI log file: {e}"))?;
        serde_json::to_writer(&mut file, &entry)
            .map_err(|e| format!("failed to encode GUI log: {e}"))?;
        file.write_all(b"\n")
            .map_err(|e| format!("failed to write GUI log: {e}"))?;
    }
    Ok(())
}

/// Read GUI console records for a date, returning only the requested tail.
#[tauri::command]
pub fn get_gui_log_lines(date: String, max_lines: u32) -> Result<Vec<String>, String> {
    let log_path = gui_log_path(&date)?;
    read_log_tail(&log_path, max_lines)
}

/// Read gateway backend log lines for a given date.
#[tauri::command]
pub fn get_gateway_log_lines(date: String, max_lines: u32) -> Result<Vec<String>, String> {
    let log_path = resolve_audit_log_path(&date)?;
    read_log_tail(&log_path, max_lines)
}

fn read_log_tail(log_path: &Path, max_lines: u32) -> Result<Vec<String>, String> {
    if !log_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(log_path)
        .map_err(|e| format!("failed to read log {}: {}", log_path.display(), e))?;

    let mut all_lines: Vec<String> = content.lines().map(ToString::to_string).collect();
    let keep = max_lines.max(1) as usize;
    if all_lines.len() > keep {
        all_lines = all_lines.split_off(all_lines.len().saturating_sub(keep));
    }
    Ok(all_lines)
}

fn resolve_audit_log_path(date: &str) -> Result<std::path::PathBuf, String> {
    valid_log_date(date)?;
    let loader = ConfigLoader::new();
    let config = loader.load().unwrap_or_default();

    // The logging dir is resolved relative to config dir (see lib.rs resolve_logging_config).
    // Default config.logging.dir = "logs" �?~/.agent-diva/logs
    let log_dir = resolve_configured_path(&config.logging.dir, loader.config_dir());

    // tracing_appender::rolling::daily(dir, "gateway.log") produces gateway.log.YYYY-MM-DD
    let log_path = log_dir.join(format!("gateway.log.{}", date));
    Ok(log_path)
}

#[cfg(test)]
mod gui_log_tests {
    use super::*;

    #[test]
    fn gui_log_date_must_be_a_calendar_date() {
        assert!(valid_log_date("2026-07-12").is_ok());
        assert!(valid_log_date("2026-99-99").is_err());
        assert!(valid_log_date("../../gateway.log").is_err());
    }

    #[test]
    fn gui_log_sanitization_redacts_nested_secrets_and_truncates_text() {
        let value = serde_json::json!({
            "token": "do-not-store",
            "nested": { "authorization": "Bearer secret" },
            "message": "x".repeat(GUI_LOG_VALUE_LIMIT + 1),
        });
        let sanitized = sanitize_gui_value(value);
        assert_eq!(sanitized["token"], "[REDACTED]");
        assert_eq!(sanitized["nested"]["authorization"], "[REDACTED]");
        assert!(sanitized["message"]
            .as_str()
            .unwrap()
            .ends_with("…[truncated]"));
    }

    #[test]
    fn read_log_tail_returns_last_requested_lines_and_handles_missing_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("gui.log.2026-07-12");
        assert!(read_log_tail(&path, 2).unwrap().is_empty());
        std::fs::write(&path, "one\ntwo\nthree\n").unwrap();
        assert_eq!(read_log_tail(&path, 2).unwrap(), vec!["two", "three"]);
    }
}

#[cfg(test)]
mod mask_tests {
    use super::*;
    use agent_diva_agent::mask::MaskFile;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    /// Global mutex to prevent parallel tests from clobbering `AGENT_DIVA_CONFIG_DIR`.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// RAII guard that removes the env var on drop.
    struct EnvGuard(String);

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            std::env::remove_var(&self.0);
        }
    }

    /// Test context that creates a temp directory with a minimal config and a
    /// `masks/` directory containing a `coder.md` test mask.
    struct TestContext {
        _dir: TempDir,
        _guard: EnvGuard,
    }

    fn setup() -> TestContext {
        let dir = TempDir::new().expect("temp dir");
        let workspace = dir.path().join("workspace");
        fs::create_dir_all(&workspace).expect("create workspace");

        let masks_dir = workspace.join("masks");
        fs::create_dir_all(&masks_dir).expect("create masks dir");

        // One user mask file so list_masks returns > 1 entry.
        fs::write(
            masks_dir.join("coder.md"),
            "---\nname: \"coder\"\nicon: \"💻\"\ndescription: \"Coding mode\"\n---\n\nYou are a coder.\n",
        )
        .expect("write coder.md");

        // Minimal config — Config::default() fills everything else.
        let config = serde_json::json!({
            "agents": {
                "defaults": {
                    "workspace": workspace.to_string_lossy()
                }
            }
        });
        fs::write(
            dir.path().join("config.json"),
            serde_json::to_string_pretty(&config).unwrap(),
        )
        .expect("write config.json");

        std::env::set_var("AGENT_DIVA_CONFIG_DIR", dir.path());

        TestContext {
            _dir: dir,
            _guard: EnvGuard("AGENT_DIVA_CONFIG_DIR".to_string()),
        }
    }

    // ------------------------------------------------------------------
    // Tests
    // ------------------------------------------------------------------

    #[test]
    fn test_list_masks_default() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _ctx = setup();

        let masks = list_masks().expect("list_masks should succeed");
        assert!(
            masks.iter().any(|m| m.name == MaskFile::DEFAULT_NAME),
            "default mask should be in the list"
        );
        assert!(
            masks.iter().any(|m| m.name == "coder"),
            "coder mask should be in the list"
        );
    }

    #[test]
    fn test_get_active_mask_none() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _ctx = setup();

        let active = get_active_mask().expect("get_active_mask should succeed");
        assert!(active.is_none(), "active mask should be None initially");
    }

    #[test]
    fn test_switch_mask() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _ctx = setup();

        let switched = switch_mask("coder".to_string()).expect("switch to coder");
        assert_eq!(switched.name, "coder");

        let active = get_active_mask()
            .expect("get_active_mask should succeed")
            .expect("active mask should be Some after switch");
        assert_eq!(active.name, "coder");
    }

    #[test]
    fn test_create_mask() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _ctx = setup();

        let payload = MaskPayload {
            id: None,
            name: "custom".to_string(),
            icon: Some("🛠️".to_string()),
            description: Some("Custom mask".to_string()),
            mode: None,
            model: None,
            subagent_defaults: Default::default(),
            tool_limits: Default::default(),
            body: Some("You are a custom assistant.".to_string()),
        };

        let created = create_or_update_mask(payload).expect("create mask");
        assert_eq!(created.name, "custom");

        let masks = list_masks().expect("list_masks should succeed");
        assert!(
            masks.iter().any(|m| m.name == "custom"),
            "new mask should appear in list"
        );
    }

    #[test]
    fn test_delete_mask() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _ctx = setup();

        // Switch to the coder mask first.
        switch_mask("coder".to_string()).expect("switch to coder");

        // Verify it's active.
        assert_eq!(
            get_active_mask()
                .expect("get_active_mask")
                .expect("coder should be active")
                .name,
            "coder"
        );

        // Delete the coder mask.
        delete_mask("coder".to_string()).expect("delete coder");

        // Active mask should now be cleared (back to default).
        let active = get_active_mask().expect("get_active_mask should succeed");
        assert!(
            active.is_none(),
            "active mask should be None after deleting the active mask"
        );
    }
}

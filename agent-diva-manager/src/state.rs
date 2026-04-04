use agent_diva_agent::AgentEvent;
use agent_diva_core::bus::{InboundMessage, MessageBus};
use agent_diva_core::config::schema::{
    ChannelsConfig, MCPServerConfig, WebFetchConfig, WebSearchConfig, WebToolsConfig,
};
use agent_diva_core::cron::{CreateCronJobRequest, CronJobDto, UpdateCronJobRequest};
use agent_diva_providers::{CustomProviderUpsert, ProviderModelCatalogView, ProviderView};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

use crate::mcp_service::{McpServerDto, McpServerUpsert};
use crate::skill_service::SkillDto;

#[derive(Clone)]
pub struct AppState {
    pub api_tx: mpsc::Sender<ManagerCommand>,
    pub bus: MessageBus,
}

pub enum ProviderCommand {
    GetProviders(oneshot::Sender<Vec<ProviderView>>),
    GetProvider(
        String,
        oneshot::Sender<Result<Option<ProviderView>, String>>,
    ),
    GetProviderModels(String, bool, oneshot::Sender<ProviderModelCatalogView>),
    ResolveProvider(String, Option<String>, oneshot::Sender<Option<String>>),
    AddProviderModel(String, String, oneshot::Sender<Result<(), String>>),
    DeleteProviderModel(String, String, oneshot::Sender<Result<(), String>>),
    CreateProvider(
        CustomProviderUpsert,
        oneshot::Sender<Result<Option<ProviderView>, String>>,
    ),
    UpdateProvider(
        String,
        CustomProviderUpsert,
        oneshot::Sender<Result<Option<ProviderView>, String>>,
    ),
    DeleteProvider(String, oneshot::Sender<Result<(), String>>),
}

pub enum ManagerCommand {
    // Core runtime control plane used by the formal CLI runtime.
    Chat(ApiRequest),
    StopChat(StopChatRequest, oneshot::Sender<Result<bool, String>>),
    ResetSession(ResetSessionRequest, oneshot::Sender<Result<bool, String>>),
    UpdateConfig(ConfigUpdate),
    UpdateChannel(ChannelUpdate),
    GetConfig(oneshot::Sender<ConfigResponse>),
    GetChannels(oneshot::Sender<ChannelsConfig>),
    GetTools(oneshot::Sender<ToolsConfigResponse>),
    UpdateTools(ToolsConfigUpdate),
    GetMcps(oneshot::Sender<Result<Vec<McpServerDto>, String>>),
    CreateMcp(
        McpServerUpsert,
        oneshot::Sender<Result<McpServerDto, String>>,
    ),
    UpdateMcp(
        String,
        McpServerUpsert,
        oneshot::Sender<Result<McpServerDto, String>>,
    ),
    DeleteMcp(String, oneshot::Sender<Result<(), String>>),
    SetMcpEnabled(String, bool, oneshot::Sender<Result<McpServerDto, String>>),
    RefreshMcpStatus(String, oneshot::Sender<Result<McpServerDto, String>>),
    GetSkills(oneshot::Sender<Result<Vec<SkillDto>, String>>),
    UploadSkill(
        SkillUploadRequest,
        oneshot::Sender<Result<SkillDto, String>>,
    ),
    DeleteSkill(String, oneshot::Sender<Result<(), String>>),
    GetSessions(oneshot::Sender<Result<Vec<agent_diva_core::session::SessionInfo>, String>>),
    GetSessionHistory(
        String,
        oneshot::Sender<Result<Option<agent_diva_core::session::store::Session>, String>>,
    ),
    DeleteSession(String, oneshot::Sender<Result<bool, String>>),
    ListCronJobs(oneshot::Sender<Result<Vec<CronJobDto>, String>>),
    GetCronJob(String, oneshot::Sender<Result<Option<CronJobDto>, String>>),
    CreateCronJob(
        CreateCronJobRequest,
        oneshot::Sender<Result<CronJobDto, String>>,
    ),
    UpdateCronJob(
        String,
        UpdateCronJobRequest,
        oneshot::Sender<Result<CronJobDto, String>>,
    ),
    DeleteCronJob(String, oneshot::Sender<Result<(), String>>),
    SetCronJobEnabled(String, bool, oneshot::Sender<Result<CronJobDto, String>>),
    RunCronJobNow(String, bool, oneshot::Sender<Result<CronJobDto, String>>),
    StopCronJobRun(
        String,
        oneshot::Sender<Result<agent_diva_core::cron::CronRunSnapshot, String>>,
    ),
    UploadFile(
        FileUploadRequest,
        oneshot::Sender<Result<agent_diva_core::attachment::FileAttachment, String>>,
    ),
    // Companion / HTTP management plane for GUI and remote administration.
    Provider(ProviderCommand),
}

pub struct ApiRequest {
    pub msg: InboundMessage,
    pub event_tx: mpsc::UnboundedSender<AgentEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopChatRequest {
    pub channel: Option<String>,
    pub chat_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetSessionRequest {
    pub channel: Option<String>,
    pub chat_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdate {
    pub api_base: Option<String>,
    pub api_key: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelUpdate {
    pub name: String,
    pub enabled: Option<bool>,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetCronJobEnabledRequest {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunCronJobRequest {
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub provider: Option<String>,
    pub api_base: Option<String>,
    pub model: String,
    // Don't return API key for security, or maybe masked
    pub has_api_key: bool,
}

#[derive(Debug, Clone)]
pub struct SkillUploadRequest {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct FileUploadRequest {
    pub file_name: String,
    pub bytes: Vec<u8>,
    pub channel: String,
    pub message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfigResponse {
    pub web: WebToolsConfigResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebToolsConfigResponse {
    pub search: WebSearchConfig,
    pub fetch: WebFetchConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfigUpdate {
    pub web: WebToolsConfigUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetMcpEnabledRequest {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRefreshRequest {
    #[serde(default)]
    pub reapply: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebToolsConfigUpdate {
    pub search: WebSearchConfig,
    pub fetch: WebFetchConfig,
}

impl From<WebToolsConfig> for WebToolsConfigResponse {
    fn from(value: WebToolsConfig) -> Self {
        Self {
            search: value.search,
            fetch: value.fetch,
        }
    }
}

pub fn active_mcp_servers(
    config: &agent_diva_core::config::schema::Config,
) -> HashMap<String, MCPServerConfig> {
    config.tools.active_mcp_servers()
}

use agent_diva_agent::AgentEvent;
use agent_diva_core::bus::{InboundMessage, MessageBus};
use agent_diva_core::config::schema::{
    ChannelsConfig, WebFetchConfig, WebSearchConfig, WebToolsConfig,
};
use agent_diva_core::cron::{CreateCronJobRequest, CronJobDto, UpdateCronJobRequest};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub struct AppState {
    pub api_tx: mpsc::Sender<ManagerCommand>,
    pub bus: MessageBus,
}

pub enum ManagerCommand {
    Chat(ApiRequest),
    StopChat(StopChatRequest, oneshot::Sender<Result<bool, String>>),
    ResetSession(ResetSessionRequest, oneshot::Sender<Result<bool, String>>),
    UpdateConfig(ConfigUpdate),
    UpdateChannel(ChannelUpdate),
    TestChannel(ChannelUpdate, oneshot::Sender<Result<(), String>>),
    GetConfig(oneshot::Sender<ConfigResponse>),
    GetChannels(oneshot::Sender<ChannelsConfig>),
    GetTools(oneshot::Sender<ToolsConfigResponse>),
    UpdateTools(ToolsConfigUpdate),
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
    pub api_base: Option<String>,
    pub model: String,
    // Don't return API key for security, or maybe masked
    pub has_api_key: bool,
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

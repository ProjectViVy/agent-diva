use tokio::sync::{mpsc, oneshot};
use agent_diva_agent::AgentEvent;
use agent_diva_core::bus::InboundMessage;
use agent_diva_core::config::schema::ChannelsConfig;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct AppState {
    pub api_tx: mpsc::Sender<ManagerCommand>,
}

pub enum ManagerCommand {
    Chat(ApiRequest),
    UpdateConfig(ConfigUpdate),
    UpdateChannel(ChannelUpdate),
    TestChannel(ChannelUpdate, oneshot::Sender<Result<(), String>>),
    GetConfig(oneshot::Sender<ConfigResponse>),
    GetChannels(oneshot::Sender<ChannelsConfig>),
}

pub struct ApiRequest {
    pub msg: InboundMessage,
    pub event_tx: mpsc::UnboundedSender<AgentEvent>,
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
pub struct ConfigResponse {
    pub api_base: Option<String>,
    pub model: String,
    // Don't return API key for security, or maybe masked
    pub has_api_key: bool,
}

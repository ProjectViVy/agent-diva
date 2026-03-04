use crate::tool_config::network::NetworkToolConfig;

#[derive(Debug)]
pub enum RuntimeControlCommand {
    UpdateNetwork(NetworkToolConfig),
    StopSession { session_key: String },
    ResetSession { session_key: String },
    GetSessions { reply_tx: tokio::sync::oneshot::Sender<Vec<agent_diva_core::session::SessionInfo>> },
    GetSession { session_key: String, reply_tx: tokio::sync::oneshot::Sender<Option<agent_diva_core::session::store::Session>> },
}

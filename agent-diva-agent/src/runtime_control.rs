use crate::tool_config::network::NetworkToolConfig;
use agent_diva_core::config::MCPServerConfig;
use agent_diva_swarm::CortexState;
use std::collections::HashMap;

#[derive(Debug)]
pub enum RuntimeControlCommand {
    UpdateNetwork(NetworkToolConfig),
    UpdateMcp {
        servers: HashMap<String, MCPServerConfig>,
    },
    StopSession {
        session_key: String,
    },
    ResetSession {
        session_key: String,
    },
    GetSessions {
        reply_tx: tokio::sync::oneshot::Sender<Vec<agent_diva_core::session::SessionInfo>>,
    },
    GetSession {
        session_key: String,
        reply_tx: tokio::sync::oneshot::Sender<Option<agent_diva_core::session::store::Session>>,
    },
    DeleteSession {
        session_key: String,
        reply_tx: tokio::sync::oneshot::Sender<Result<bool, String>>,
    },
    /// Story 6.4：HTTP / 管理面对齐 gateway 内 [`ProcessEventPipeline`] 所绑定的 [`CortexRuntime`]。
    GetCortexState {
        reply_tx: tokio::sync::oneshot::Sender<Result<CortexState, String>>,
    },
    SetCortexEnabled {
        enabled: bool,
        reply_tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
}

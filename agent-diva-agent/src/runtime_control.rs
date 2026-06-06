use crate::tool_config::mentle::MentleToolRuntimeConfig;
use crate::tool_config::network::NetworkToolConfig;
use agent_diva_core::config::MCPServerConfig;
use std::collections::HashMap;

#[derive(Debug)]
pub enum RuntimeControlCommand {
    UpdateNetwork(NetworkToolConfig),
    UpdateMentle {
        mentle: MentleToolRuntimeConfig,
        builtin_mentle: bool,
    },
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
    SetThinking {
        mode: agent_diva_core::reasoning::ThinkingMode,
    },
    /// Manually trigger context compaction for a session (/compact command).
    CompactSession {
        session_key: String,
        reply_tx: tokio::sync::oneshot::Sender<Result<String, String>>,
    },
}

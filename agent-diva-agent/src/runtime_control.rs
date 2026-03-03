use crate::tool_config::network::NetworkToolConfig;

#[derive(Debug, Clone)]
pub enum RuntimeControlCommand {
    UpdateNetwork(NetworkToolConfig),
    StopSession { session_key: String },
}

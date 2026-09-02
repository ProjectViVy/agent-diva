use crate::tool_config::network::NetworkToolConfig;
use agent_diva_core::channel::ChannelEnvelopeV1;
use agent_diva_core::config::MCPServerConfig;
use std::collections::HashMap;

#[derive(Debug)]
pub enum RuntimeControlCommand {
    /// Submit a typed Neuro-Link envelope to the bounded AgentLoop admission
    /// dispatcher.  The reply is sent at the first authoritative queued or
    /// running transition; turn execution continues on the session worker.
    StartChannelTurn {
        envelope: Box<ChannelEnvelopeV1>,
        reply_tx: tokio::sync::oneshot::Sender<
            Result<agent_diva_core::bus::SessionAdmissionObservation, String>,
        >,
    },
    /// Invalidate the machine Skill section for every cached Session. The
    /// next prompt assembly reloads the on-disk catalog; this is an internal
    /// control-plane event and never a user-visible Session message.
    ReloadMachineSkills {
        change_id: String,
    },
    /// Refresh the workspace-scoped Memory authority projection after a
    /// committed BML write. The change is internal control-plane state, not a
    /// user-visible Session message.
    RefreshMemoryAuthority {
        workspace_id: String,
        authority_revision: u64,
        change_id: String,
    },
    UpdateNetwork(NetworkToolConfig),
    UpdateMcp {
        servers: HashMap<String, MCPServerConfig>,
    },
    StopSession {
        session_key: String,
        request_id: Option<String>,
        reply_tx: tokio::sync::oneshot::Sender<agent_diva_core::bus::SessionControlOutcome>,
    },
    ResetSession {
        session_key: String,
        reply_tx: tokio::sync::oneshot::Sender<agent_diva_core::bus::SessionControlOutcome>,
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
    UpdateSessionTitle {
        session_key: String,
        title: Option<String>,
        reply_tx: tokio::sync::oneshot::Sender<Result<Option<String>, String>>,
    },
    GenerateSessionTitle {
        session_key: String,
        first_user_message: String,
        first_assistant_message: String,
        fallback_title: String,
        reply_tx: tokio::sync::oneshot::Sender<Result<(String, bool, bool), String>>,
    },
    SetThinking {
        mode: agent_diva_core::reasoning::ThinkingMode,
    },
    /// Update the orchestrator approval policy for subsequent tool calls.
    SetApprovalPolicy {
        policy: agent_diva_sandbox::AskForApproval,
    },
    /// Manually trigger context compaction for a session (/compact command).
    CompactSession {
        session_key: String,
        reply_tx: tokio::sync::oneshot::Sender<Result<String, String>>,
    },
    ApproveActivePlan {
        request: agent_diva_core::planning::ApprovalRequest,
        reply_tx:
            tokio::sync::oneshot::Sender<Result<agent_diva_core::bus::PlanApprovalResult, String>>,
    },
    ReturnActivePlanToDraft {
        reply_tx:
            tokio::sync::oneshot::Sender<Result<agent_diva_core::bus::PlanRuntimeState, String>>,
    },
}

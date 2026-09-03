use agent_diva_agent::AgentEvent;
use agent_diva_autodream::AutoDreamService;
use agent_diva_core::bus::AgentEventBus;
use agent_diva_core::channel::{ChannelEnvelopeV1, FabricHandle};
use agent_diva_core::config::schema::{
    ChannelsConfig, MCPServerConfig, SelfEvolutionConfig, WebFetchConfig, WebSearchConfig,
    WebToolsConfig,
};
use agent_diva_core::cron::{CreateCronJobRequest, CronJobDto, UpdateCronJobRequest};
use agent_diva_core::evolution::SkillHome;
use agent_diva_core::governance::ApprovalCoordinator;
use agent_diva_core::workspace::{WorkspaceContext, WorkspaceSource};
use agent_diva_laputa::{MemoryHome, PersonaService};
use agent_diva_providers::{CustomProviderUpsert, ProviderModelCatalogView, ProviderView};
use agent_diva_sandbox::CommandApprovalCoordinator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::OnceCell;
use tokio::sync::{mpsc, oneshot};

use crate::mcp_service::{McpServerDto, McpServerUpsert};
use crate::planning_service::{
    AppendPlanReportRevisionRequest, ApprovePlanReportRequest, CreatePlanReportRequest,
    UpdateExecutionTodoRequest,
};
use crate::skill_service::SkillDto;
use agent_diva_agent::runtime_control::RuntimeControlCommand;

#[derive(Clone)]
pub struct HealthSignals {
    cron: Arc<AtomicU8>,
    audit_sink_ready: bool,
}

impl HealthSignals {
    const UNKNOWN: u8 = 0;
    const READY: u8 = 1;

    fn new(audit_sink_ready: bool) -> Self {
        Self {
            cron: Arc::new(AtomicU8::new(Self::UNKNOWN)),
            audit_sink_ready,
        }
    }

    pub fn mark_cron_ready(&self) {
        self.cron.store(Self::READY, Ordering::Relaxed);
    }

    pub fn cron_ready(&self) -> Option<bool> {
        match self.cron.load(Ordering::Relaxed) {
            Self::READY => Some(true),
            _ => None,
        }
    }

    pub fn audit_sink_ready(&self) -> bool {
        self.audit_sink_ready
    }
}

#[derive(Clone)]
pub struct AppState {
    pub api_tx: mpsc::Sender<ManagerCommand>,
    pub bus: AgentEventBus,
    /// Authoritative runtime workspace snapshot used by operator-facing APIs.
    /// `workspace_root` remains as a compatibility projection for existing
    /// handlers and must always be copied from this context at construction.
    pub workspace_context: WorkspaceContext,
    pub workspace_root: PathBuf,
    pub config_dir: PathBuf,
    pub audit_root: PathBuf,
    pub autodream: AutoDreamService,
    pub persona: PersonaService,
    /// Machine-wide BML, ACTMEM, and MEMRULES authority.
    pub memory_home: MemoryHome,
    /// Machine-wide Skill and SkillProposal authority.
    pub skill_home: SkillHome,
    pub health: HealthSignals,
    /// Server start time, used for uptime calculation in the health endpoint.
    pub started_at: Instant,
    pub command_approvals: CommandApprovalCoordinator,
    /// Conversational ask-user coordinator shared with the agent loop and the
    /// ask-user HTTP endpoints.
    pub ask_user: agent_diva_core::ask_user::AskUserCoordinator,
    /// Process-wide durable governance authority in production.
    pub governance: Option<ApprovalCoordinator>,
    /// Canonical Plan service shared with the Manager command loop.
    pub planning_service: Option<Arc<crate::planning_service::PlanningService>>,
    /// Internal AgentLoop control channel for workspace-scoped authority
    /// projection refreshes. It is absent in isolated handler fixtures.
    pub runtime_control_tx: Option<mpsc::Sender<RuntimeControlCommand>>,
    /// Cloneable producer for every runtime ingress path.
    pub fabric_handle: Option<FabricHandle>,
    /// Shared attachment authority used to resolve HTTP references before Fabric admission.
    pub attachment_authority: Option<Arc<agent_diva_files::FileManager>>,
    /// Typed Neuro-Link ingress seam.  The gateway never reaches through the
    /// legacy AgentEventBus; production wiring can install an AgentLoop/Fabric
    /// implementation while isolated fixtures leave it unset.
    pub neuro_link_runtime: Option<Arc<dyn crate::neuro_link::NeuroLinkRuntime>>,
    /// Lazily opened durable Neuro-Link projection journal.  The lazy cell
    /// keeps existing isolated handler fixtures lightweight while production
    /// gateway traffic still shares one profile-local SQLite authority.
    pub projection_journal: Arc<OnceCell<Arc<crate::projection_journal::ProjectionJournal>>>,
    /// Process-wide AgentEvent projection fan-out shared by all Neuro-Link
    /// sockets.  The hub owns the only journal writer for live events.
    pub neuro_link_projection: Arc<crate::neuro_link_projection::NeuroLinkProjectionHub>,
}

impl AppState {
    pub fn new(
        api_tx: mpsc::Sender<ManagerCommand>,
        bus: AgentEventBus,
        workspace_root: impl Into<PathBuf>,
    ) -> anyhow::Result<Self> {
        Self::new_with_command_approvals(
            api_tx,
            bus,
            workspace_root,
            CommandApprovalCoordinator::default(),
        )
    }

    pub fn new_with_command_approvals(
        api_tx: mpsc::Sender<ManagerCommand>,
        bus: AgentEventBus,
        workspace_root: impl Into<PathBuf>,
        command_approvals: CommandApprovalCoordinator,
    ) -> anyhow::Result<Self> {
        Self::new_with_ask_user(
            api_tx,
            bus,
            workspace_root,
            command_approvals,
            agent_diva_core::ask_user::AskUserCoordinator::default(),
        )
    }

    pub fn new_with_ask_user(
        api_tx: mpsc::Sender<ManagerCommand>,
        bus: AgentEventBus,
        workspace_root: impl Into<PathBuf>,
        command_approvals: CommandApprovalCoordinator,
        ask_user: agent_diva_core::ask_user::AskUserCoordinator,
    ) -> anyhow::Result<Self> {
        Self::new_with_runtime_memory(api_tx, bus, workspace_root, command_approvals, ask_user)
    }

    pub fn new_with_runtime_memory(
        api_tx: mpsc::Sender<ManagerCommand>,
        bus: AgentEventBus,
        workspace_root: impl Into<PathBuf>,
        command_approvals: CommandApprovalCoordinator,
        ask_user: agent_diva_core::ask_user::AskUserCoordinator,
    ) -> anyhow::Result<Self> {
        Self::new_with_runtime_governance_inner(
            api_tx,
            bus,
            configured_workspace_context(workspace_root.into()),
            command_approvals,
            ask_user,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_runtime_governance(
        api_tx: mpsc::Sender<ManagerCommand>,
        bus: AgentEventBus,
        workspace_root: impl Into<PathBuf>,
        command_approvals: CommandApprovalCoordinator,
        ask_user: agent_diva_core::ask_user::AskUserCoordinator,
        governance: ApprovalCoordinator,
        planning_service: Arc<crate::planning_service::PlanningService>,
    ) -> anyhow::Result<Self> {
        Self::new_with_runtime_governance_inner(
            api_tx,
            bus,
            configured_workspace_context(workspace_root.into()),
            command_approvals,
            ask_user,
            Some(governance),
            Some(planning_service),
            None,
            None,
            None,
            None,
        )
    }

    /// Construct production AppState with the AgentLoop control channel used
    /// for post-commit Memory projection refresh notifications.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_runtime_governance_and_control(
        api_tx: mpsc::Sender<ManagerCommand>,
        bus: AgentEventBus,
        workspace_context: WorkspaceContext,
        config_dir: impl Into<PathBuf>,
        memory_home: MemoryHome,
        command_approvals: CommandApprovalCoordinator,
        ask_user: agent_diva_core::ask_user::AskUserCoordinator,
        governance: ApprovalCoordinator,
        planning_service: Arc<crate::planning_service::PlanningService>,
        runtime_control_tx: mpsc::Sender<RuntimeControlCommand>,
    ) -> anyhow::Result<Self> {
        Self::new_with_runtime_governance_inner(
            api_tx,
            bus,
            workspace_context,
            command_approvals,
            ask_user,
            Some(governance),
            Some(planning_service),
            Some(runtime_control_tx),
            Some(config_dir.into()),
            Some(memory_home),
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_runtime_governance_inner(
        api_tx: mpsc::Sender<ManagerCommand>,
        bus: AgentEventBus,
        workspace_context: WorkspaceContext,
        command_approvals: CommandApprovalCoordinator,
        ask_user: agent_diva_core::ask_user::AskUserCoordinator,
        governance: Option<ApprovalCoordinator>,
        planning_service: Option<Arc<crate::planning_service::PlanningService>>,
        runtime_control_tx: Option<mpsc::Sender<RuntimeControlCommand>>,
        config_dir: Option<PathBuf>,
        memory_home: Option<MemoryHome>,
        neuro_link_runtime: Option<Arc<dyn crate::neuro_link::NeuroLinkRuntime>>,
    ) -> anyhow::Result<Self> {
        let workspace_root = workspace_context.root.clone();
        let config_dir = config_dir.unwrap_or_else(|| workspace_root.clone());
        let audit_root = agent_diva_core::audit_sink::workspace_audit_dir(&workspace_root);
        std::fs::create_dir_all(&audit_root)?;
        let audit_sink_ready = agent_diva_core::audit_sink::get_sink().is_some()
            || agent_diva_core::audit_sink::ensure_workspace_jsonl_sink(&workspace_root).is_ok();
        let persona = PersonaService::open(config_dir.clone())?;
        let memory_home = memory_home.unwrap_or_else(|| MemoryHome::new(config_dir.clone()));
        let skill_home = SkillHome::new(
            &config_dir,
            agent_diva_agent::skills::SkillsLoader::default_builtin_skills_dir(),
        );
        skill_home.reconcile_pending_heads()?;
        let autodream =
            match crate::runtime::open_autodream_with_report_curation(workspace_root.clone()) {
                Ok(service) => service,
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        "failed to configure AutoDream providers; reflection remains unavailable"
                    );
                    AutoDreamService::open(workspace_root.clone())?
                }
            }
            .with_memory_home(memory_home.clone())
            .with_skill_home(skill_home.clone());
        let neuro_link_bus = bus.clone();
        let neuro_link_data_root = config_dir.clone();
        let state = Self {
            api_tx,
            bus,
            workspace_context,
            workspace_root,
            config_dir,
            audit_root,
            autodream,
            persona,
            memory_home,
            skill_home,
            health: HealthSignals::new(audit_sink_ready),
            started_at: Instant::now(),
            command_approvals,
            ask_user,
            governance,
            planning_service,
            runtime_control_tx,
            fabric_handle: None,
            attachment_authority: None,
            neuro_link_runtime,
            projection_journal: Arc::new(OnceCell::new()),
            neuro_link_projection: crate::neuro_link_projection::NeuroLinkProjectionHub::new(
                neuro_link_bus,
                neuro_link_data_root,
            ),
        };
        match state.autodream.resumable_runs() {
            Ok(runs) if !runs.is_empty() => {
                crate::handlers::autodream::spawn_autodream_runs(state.autodream.clone(), runs);
            }
            Ok(_) => {}
            Err(error) => tracing::error!(%error, "failed to recover AutoDream runs at startup"),
        }
        Ok(state)
    }

    /// Install the typed Neuro-Link runtime seam on an isolated or embedded
    /// gateway fixture.  This is intentionally additive; existing Manager
    /// constructors continue to use a service-unavailable default until the
    /// AgentLoop/Fabric production cutover lands.
    pub fn with_neuro_link_runtime(
        mut self,
        runtime: Arc<dyn crate::neuro_link::NeuroLinkRuntime>,
    ) -> Self {
        self.neuro_link_runtime = Some(runtime);
        self
    }

    /// Install the cloneable Fabric producer for typed HTTP/runtime ingress.
    pub fn with_fabric_handle(mut self, fabric_handle: FabricHandle) -> Self {
        self.fabric_handle = Some(fabric_handle);
        self
    }

    /// Install the shared file authority used while constructing typed content parts.
    pub fn with_attachment_authority(
        mut self,
        attachment_authority: Arc<agent_diva_files::FileManager>,
    ) -> Self {
        self.attachment_authority = Some(attachment_authority);
        self
    }

    /// Open the profile-local projection journal once and reuse it across all
    /// WebSocket connections and HTTP lifecycle handlers.
    pub async fn projection_journal(
        &self,
    ) -> Result<Arc<crate::projection_journal::ProjectionJournal>, String> {
        let data_root = self.config_dir.clone();
        self.projection_journal
            .get_or_try_init(|| async move {
                crate::projection_journal::ProjectionJournal::open(data_root)
                    .await
                    .map(Arc::new)
                    .map_err(|error| -> String { error.to_string() })
            })
            .await
            .map(Clone::clone)
    }
}

fn configured_workspace_context(root: PathBuf) -> WorkspaceContext {
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    WorkspaceContext {
        root,
        source: WorkspaceSource::Configured,
        agents_md: None,
    }
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
    Chat(Box<ApiRequest>),
    StopChat(
        StopChatRequest,
        oneshot::Sender<Result<agent_diva_core::bus::SessionControlOutcome, String>>,
    ),
    ResetSession(
        ResetSessionRequest,
        oneshot::Sender<Result<agent_diva_core::bus::SessionControlOutcome, String>>,
    ),
    UpdateConfig(ConfigUpdate),
    UpdateChannel(ChannelUpdate, oneshot::Sender<Result<(), String>>),
    GetChannelRuntime(oneshot::Sender<Vec<agent_diva_channels::runtime::ChannelRuntimeStatus>>),
    GetConfig(oneshot::Sender<ConfigResponse>),
    GetSelfEvolutionConfig(oneshot::Sender<Result<SelfEvolutionConfig, String>>),
    UpdateSelfEvolutionConfig(
        SelfEvolutionConfig,
        oneshot::Sender<Result<SelfEvolutionConfig, String>>,
    ),
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
    UpdateSessionTitle(
        String,                                          // session_key
        Option<String>,                                  // new title
        oneshot::Sender<Result<Option<String>, String>>, // returns updated title or None
    ),
    GenerateSessionTitle(
        String,
        GenerateSessionTitleRequest,
        oneshot::Sender<Result<GenerateSessionTitleResponse, String>>,
    ),
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
    // Plan report and execution commands
    ListPlanReports(
        oneshot::Sender<Result<Vec<agent_diva_core::planning::PlanReportDetail>, String>>,
    ),
    CreatePlanReport(
        CreatePlanReportRequest,
        oneshot::Sender<Result<agent_diva_core::planning::PlanReportDetail, String>>,
    ),
    AppendPlanReportRevision(
        String,
        AppendPlanReportRevisionRequest,
        oneshot::Sender<Result<agent_diva_core::planning::PlanReportDetail, String>>,
    ),
    ApprovePlanReport(
        String,
        ApprovePlanReportRequest,
        oneshot::Sender<Result<agent_diva_core::planning::ExecutionSession, String>>,
    ),
    GetActivePlanExecution(
        String,
        oneshot::Sender<Result<Option<agent_diva_core::planning::ExecutionSession>, String>>,
    ),
    ListExecutionTodos(
        String,
        oneshot::Sender<Result<Vec<agent_diva_core::planning::ExecutionTodo>, String>>,
    ),
    UpdateExecutionTodo(
        String,
        String,
        UpdateExecutionTodoRequest,
        oneshot::Sender<Result<agent_diva_core::planning::ExecutionTodo, String>>,
    ),
    // Companion / HTTP management plane for GUI and remote administration.
    Provider(ProviderCommand),
}

pub struct ApiRequest {
    pub envelope: ChannelEnvelopeV1,
    pub event_tx: mpsc::UnboundedSender<AgentEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopChatRequest {
    pub channel: Option<String>,
    pub chat_id: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetSessionRequest {
    pub channel: Option<String>,
    pub chat_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateSessionTitleRequest {
    pub first_user_message: String,
    pub first_assistant_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateSessionTitleResponse {
    pub title: String,
    pub title_generated: bool,
    pub title_manually_set: bool,
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
    pub budget: agent_diva_core::config::CompactionBudgetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebToolsConfigResponse {
    pub search: WebSearchConfig,
    pub fetch: WebFetchConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfigUpdate {
    pub web: WebToolsConfigUpdate,
    #[serde(default)]
    pub budget: agent_diva_core::config::CompactionBudgetConfig,
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

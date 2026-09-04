use crate::mask::{MaskFile, ToolPolicy};
use crate::planning::builtin_tool_capability;
use crate::tool_config::PlanningConfig;
use crate::tool_config::{builtin::BuiltInToolsConfig, network::NetworkToolConfig};
use agent_diva_core::ask_user::AskUserCoordinator;
use agent_diva_core::channel::ChannelRoute;
use agent_diva_core::config::schema::MaskConfig;
use agent_diva_core::config::MCPServerConfig;
use agent_diva_core::cron::CronService;
use agent_diva_core::memory::MemoryProvider;
use agent_diva_core::planning::model::PlanPhase;
use agent_diva_core::planning::policy::allows_for_phase;
use agent_diva_core::security::{SecurityConfig, SecurityLevel, SecurityPolicy};
use agent_diva_core::supervised::RunStore;
use agent_diva_core::tool_artifact::{ToolArtifactSecurityContext, ToolArtifactStore};
use agent_diva_files::FileManager;
use agent_diva_sandbox::{AskForApproval, CommandApprovalCoordinator};
use agent_diva_tooling::{
    ActiveDeferredToolsHandle, Tool, ToolError, ToolRegistry, ToolSchemaPartition,
};
use agent_diva_tools::{
    load_mcp_tools_sync, AskUserTool, BackgroundTaskContext, CronTool, EditFileTool,
    EnqueueBackgroundTaskTool, ExecTool, ExecutionTodoShowTool, ExecutionTodoWriteTool,
    ListDirTool, PersonaReadTool, PersonaRequestTool, PersonaUpdateTool, ReadAttachmentTool,
    ReadFileTool, ReadToolResultTool, SkillReadTool, SpawnTool, ToolSearchTool, UpdatePlanTool,
    WebFetchTool, WebSearchTool, WorldReadTool, WriteFileTool,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait SubagentSpawner: Send + Sync {
    async fn spawn(
        &self,
        task: String,
        label: Option<String>,
        route: ChannelRoute,
    ) -> Result<String, ToolError>;
}

pub struct ToolAssembly {
    workspace: PathBuf,
    persona_root: Option<PathBuf>,
    builtin_config: BuiltInToolsConfig,
    network_config: NetworkToolConfig,
    exec_timeout: u64,
    global_timeout_secs: u64,
    restrict_to_workspace: bool,
    mcp_servers: HashMap<String, MCPServerConfig>,
    cron_service: Option<Arc<CronService>>,
    custom_tools: Vec<Arc<dyn Tool>>,
    subagent_spawner: Option<Arc<dyn SubagentSpawner>>,
    file_manager: Option<Arc<FileManager>>,
    run_store: Option<Arc<RunStore>>,
    background_task_context: BackgroundTaskContext,
    mask_config: Option<MaskConfig>,
    planning_config: Option<PlanningConfig>,
    plan_phase: Option<PlanPhase>,
    execution_session_id: Option<String>,
    command_approvals: Option<CommandApprovalCoordinator>,
    approval_policy: AskForApproval,
    ask_user_coordinator: Option<AskUserCoordinator>,
    memory_provider: Option<Arc<dyn MemoryProvider>>,
    session_checkpoint_session: Option<String>,
    artifact_session: Option<String>,
    active_deferred_tools: Option<ActiveDeferredToolsHandle>,
}

impl ToolAssembly {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            persona_root: None,
            builtin_config: BuiltInToolsConfig::default(),
            network_config: NetworkToolConfig::default(),
            exec_timeout: 60,
            global_timeout_secs: 120,
            restrict_to_workspace: false,
            mcp_servers: HashMap::new(),
            cron_service: None,
            custom_tools: Vec::new(),
            subagent_spawner: None,
            file_manager: None,
            run_store: None,
            background_task_context: BackgroundTaskContext::default(),
            mask_config: None,
            planning_config: None,
            plan_phase: None,
            execution_session_id: None,
            command_approvals: None,
            approval_policy: AskForApproval::default(),
            ask_user_coordinator: None,
            memory_provider: None,
            session_checkpoint_session: None,
            artifact_session: None,
            active_deferred_tools: None,
        }
    }

    pub fn builtin(mut self, config: BuiltInToolsConfig) -> Self {
        self.builtin_config = config;
        self
    }

    pub fn with_network_config(mut self, config: NetworkToolConfig) -> Self {
        self.network_config = config;
        self
    }

    /// Bind Persona tools to the machine-wide config root, never the workspace.
    pub fn with_persona_root(mut self, config_dir: Option<PathBuf>) -> Self {
        self.persona_root = config_dir;
        self
    }

    pub fn with_exec_timeout(mut self, timeout: u64) -> Self {
        self.exec_timeout = timeout;
        self
    }

    pub fn with_global_timeout(mut self, timeout: u64) -> Self {
        self.global_timeout_secs = timeout;
        self
    }

    pub fn restrict_to_workspace(mut self, restrict: bool) -> Self {
        self.restrict_to_workspace = restrict;
        self
    }

    pub fn mcp_servers(mut self, servers: HashMap<String, MCPServerConfig>) -> Self {
        self.mcp_servers = servers;
        self
    }

    pub fn with_cron_service(mut self, service: Arc<CronService>) -> Self {
        self.cron_service = Some(service);
        self
    }

    pub fn with_subagent_spawner(mut self, spawner: Arc<dyn SubagentSpawner>) -> Self {
        self.subagent_spawner = Some(spawner);
        self
    }

    pub fn with_file_manager(mut self, file_manager: Arc<FileManager>) -> Self {
        self.file_manager = Some(file_manager);
        self
    }

    pub fn with_run_store(mut self, run_store: Arc<RunStore>) -> Self {
        self.run_store = Some(run_store);
        self
    }

    pub fn with_background_task_context(mut self, context: BackgroundTaskContext) -> Self {
        self.background_task_context = context;
        self
    }

    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.custom_tools.push(tool);
        self
    }

    pub fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.custom_tools.extend(tools);
        self
    }

    pub fn with_mask_config(mut self, mask_config: Option<MaskConfig>) -> Self {
        self.mask_config = mask_config;
        self
    }

    pub fn with_planning_config(mut self, config: Option<PlanningConfig>) -> Self {
        self.planning_config = config;
        self
    }

    /// Constrain this registry to the persisted phase active for the turn.
    pub fn with_plan_phase(mut self, phase: Option<PlanPhase>) -> Self {
        self.plan_phase = phase;
        self
    }

    pub fn with_execution_session(mut self, execution_session_id: Option<String>) -> Self {
        self.execution_session_id = execution_session_id;
        self
    }

    pub fn with_command_approvals(
        mut self,
        coordinator: Option<CommandApprovalCoordinator>,
    ) -> Self {
        self.command_approvals = coordinator;
        self
    }

    /// Shared conversational ask-user coordinator; `None` makes the tool
    /// report `unavailable` (headless runtimes).
    pub fn with_memory_provider(mut self, provider: Option<Arc<dyn MemoryProvider>>) -> Self {
        self.memory_provider = provider;
        self
    }

    /// Bind the active session key for the session checkpoint tool.
    pub fn with_session_checkpoint_session(mut self, session_id: Option<String>) -> Self {
        self.session_checkpoint_session = session_id;
        self
    }

    /// Bind opaque artifact reads to the runtime-owned session identity.
    pub fn with_artifact_session(mut self, session_id: Option<String>) -> Self {
        self.artifact_session = session_id;
        self
    }

    /// Reuse the task-local active deferred tools across same-turn registry
    /// rebuilds.
    pub fn with_active_deferred_tools(mut self, state: ActiveDeferredToolsHandle) -> Self {
        self.active_deferred_tools = Some(state);
        self
    }

    pub fn with_ask_user_coordinator(mut self, coordinator: Option<AskUserCoordinator>) -> Self {
        self.ask_user_coordinator = coordinator;
        self
    }

    /// Override the orchestrator's approval policy. Defaults to `OnFailure`.
    /// GUI modes map as: cautious → `OnRequest`, smart → `OnFailure`,
    /// trusted → `UnlessTrusted`.
    pub fn with_approval_policy(mut self, policy: AskForApproval) -> Self {
        self.approval_policy = policy;
        self
    }

    pub fn build(self) -> ToolRegistry {
        self.build_internal(false)
    }

    pub fn build_subagent_registry(mut self) -> ToolRegistry {
        self.builtin_config = self.builtin_config.for_subagent();
        self.subagent_spawner = None;
        self.cron_service = None;
        self.file_manager = None;
        self.run_store = None;
        self.background_task_context = BackgroundTaskContext::default();
        self.custom_tools.clear();
        self.build_internal(true)
    }

    fn build_internal(self, subagent_mode: bool) -> ToolRegistry {
        let mask_file = self.mask_config.clone().map(|frontmatter| MaskFile {
            frontmatter,
            body: String::new(),
        });
        let read_only_mode = mask_file
            .as_ref()
            .is_some_and(ToolPolicy::is_read_only_mode);
        // Plan exploration is a hard runtime read-only boundary.  It is not
        // represented by legacy planning-record tools.
        let action_restricted = read_only_mode || matches!(self.plan_phase, Some(PlanPhase::Plan));
        let mut registry = match self.active_deferred_tools {
            Some(state) => {
                ToolRegistry::with_active_deferred_tools(self.global_timeout_secs, state)
            }
            None => ToolRegistry::with_timeout(self.global_timeout_secs),
        };

        if self.builtin_config.tool_discovery {
            let activation = registry.deferred_tool_activation_handle();
            registry.register(Arc::new(ToolSearchTool::new(activation)));
        }

        if !subagent_mode {
            if let Some(config_dir) = self.persona_root.clone() {
                registry.register(Arc::new(SkillReadTool::with_config_dir(config_dir.clone())));
                registry.register(Arc::new(WorldReadTool::with_config_dir(config_dir.clone())));
                if self.plan_phase.is_none() {
                    registry.register_in_partition(
                        Arc::new(PersonaReadTool::with_config_dir(config_dir.clone())),
                        ToolSchemaPartition::Deferred,
                    );
                    if !action_restricted {
                        registry.register_in_partition(
                            Arc::new(PersonaRequestTool::with_config_dir(config_dir.clone())),
                            ToolSchemaPartition::Deferred,
                        );
                        registry.register_in_partition(
                            Arc::new(PersonaUpdateTool::with_config_dir(config_dir.clone())),
                            ToolSchemaPartition::Deferred,
                        );
                    }
                }
                if self.builtin_config.memory && !action_restricted && self.plan_phase.is_none() {
                    registry.register_in_partition(
                        Arc::new(agent_diva_tools::MemoryDistillTool::with_config_dir(
                            config_dir,
                            self.session_checkpoint_session.clone(),
                        )),
                        ToolSchemaPartition::Deferred,
                    );
                }
            }
        }

        if let Some(session_id) = self.artifact_session.as_deref() {
            registry.register(Arc::new(ReadToolResultTool::new(
                Arc::new(ToolArtifactStore::new(&self.workspace)),
                ToolArtifactSecurityContext::new(&self.workspace, session_id),
            )));
        }

        if self.builtin_config.filesystem {
            let security_config = if self.restrict_to_workspace {
                SecurityConfig {
                    level: SecurityLevel::Standard,
                    workspace_only: true,
                    ..SecurityConfig::default()
                }
            } else {
                SecurityConfig::default()
            };
            let security_config = if action_restricted {
                SecurityConfig {
                    level: SecurityLevel::Paranoid,
                    workspace_only: true,
                    read_only: Some(true),
                    ..security_config
                }
            } else {
                security_config
            };
            let security = Arc::new(SecurityPolicy::with_config(
                self.workspace.clone(),
                security_config,
            ));
            registry.register(Arc::new(ReadFileTool::new(security.clone())));
            registry.register(Arc::new(ListDirTool::new(security.clone())));
            if !action_restricted {
                registry.register(Arc::new(WriteFileTool::new(security.clone())));
                registry.register(Arc::new(EditFileTool::new(security)));
            }
        }

        if self.builtin_config.attachment {
            if let Some(file_manager) = self.file_manager {
                registry.register(Arc::new(ReadAttachmentTool::new(file_manager)));
            }
        }

        if self.builtin_config.shell && !action_restricted {
            registry.register(Arc::new(
                ExecTool::with_config(
                    self.exec_timeout,
                    Some(self.workspace.clone()),
                    self.restrict_to_workspace,
                )
                .with_approval_backend(self.command_approvals.clone(), self.approval_policy),
            ));
        }

        if self.builtin_config.web_search
            && self.network_config.web.search.enabled
            && self.plan_phase.as_ref().map_or(true, |phase| {
                allows_for_phase(phase, builtin_tool_capability("web_search"))
            })
        {
            registry.register(Arc::new(WebSearchTool::with_provider_and_max_results(
                self.network_config.web.search.provider.clone(),
                self.network_config.web.search.api_key.clone(),
                self.network_config.web.search.normalized_max_results(),
            )));
        }

        if self.builtin_config.web_fetch
            && self.network_config.web.fetch.enabled
            && self.plan_phase.as_ref().map_or(true, |phase| {
                allows_for_phase(phase, builtin_tool_capability("web_fetch"))
            })
        {
            registry.register(Arc::new(WebFetchTool::new()));
        }

        if self.builtin_config.ask_user && !subagent_mode && self.execution_session_id.is_none() {
            // Execution sessions stay tool-focused in the MVP; ask_user is a
            // conversational clarify surface (subagents are disabled via
            // `for_subagent` in `build_subagent_registry`).
            match &self.ask_user_coordinator {
                Some(coordinator) => {
                    registry.register(Arc::new(AskUserTool::with_coordinator(coordinator.clone())));
                }
                None => {
                    registry.register(Arc::new(AskUserTool::new()));
                }
            }
        }

        if self.builtin_config.memory && !subagent_mode {
            match &self.memory_provider {
                Some(provider) => {
                    let workspace = self.workspace.clone();
                    registry.register(Arc::new(agent_diva_tools::MemorySearchTool::with_provider(
                        provider.clone(),
                        workspace.clone(),
                    )));
                    registry.register(Arc::new(agent_diva_tools::MemoryGetTool::with_provider(
                        provider.clone(),
                        workspace.clone(),
                    )));
                    registry.register(Arc::new(agent_diva_tools::ActmemTool::with_provider(
                        provider.clone(),
                    )));
                    if !action_restricted {
                        registry.register(Arc::new(
                            agent_diva_tools::MemoryAddTool::with_provider(
                                provider.clone(),
                                workspace.clone(),
                            ),
                        ));
                        registry.register_in_partition(
                            Arc::new(agent_diva_tools::MemoryListTool::with_provider(
                                provider.clone(),
                                workspace.clone(),
                            )),
                            ToolSchemaPartition::Deferred,
                        );
                        registry.register(Arc::new(
                            agent_diva_tools::MemoryUpdateTool::with_provider(
                                provider.clone(),
                                workspace.clone(),
                            ),
                        ));
                        registry.register(Arc::new(
                            agent_diva_tools::MemoryRemoveTool::with_provider(
                                provider.clone(),
                                workspace,
                            ),
                        ));
                        registry.register_in_partition(
                            Arc::new(agent_diva_tools::ActmemEditWorkTool::with_provider(
                                provider.clone(),
                            )),
                            ToolSchemaPartition::Deferred,
                        );
                        registry.register_in_partition(
                            Arc::new(agent_diva_tools::ActmemCompleteTool::with_provider(
                                provider.clone(),
                            )),
                            ToolSchemaPartition::Deferred,
                        );
                        registry.register_in_partition(
                            Arc::new(agent_diva_tools::ActmemDropTool::with_provider(
                                provider.clone(),
                            )),
                            ToolSchemaPartition::Deferred,
                        );
                    }
                }
                None => {
                    registry.register(Arc::new(agent_diva_tools::MemorySearchTool::new()));
                    registry.register(Arc::new(agent_diva_tools::MemoryGetTool::new()));
                    registry.register(Arc::new(agent_diva_tools::ActmemTool::new()));
                    if !action_restricted {
                        for tool in [
                            Arc::new(agent_diva_tools::MemoryAddTool::new()) as Arc<dyn Tool>,
                            Arc::new(agent_diva_tools::MemoryUpdateTool::new()),
                            Arc::new(agent_diva_tools::MemoryRemoveTool::new()),
                        ] {
                            registry.register(tool);
                        }
                        for tool in [
                            Arc::new(agent_diva_tools::MemoryListTool::new()) as Arc<dyn Tool>,
                            Arc::new(agent_diva_tools::ActmemEditWorkTool::new()),
                            Arc::new(agent_diva_tools::ActmemCompleteTool::new()),
                            Arc::new(agent_diva_tools::ActmemDropTool::new()),
                        ] {
                            registry.register_in_partition(tool, ToolSchemaPartition::Deferred);
                        }
                    }
                }
            }
        }

        if self.builtin_config.working_memory
            && self.builtin_config.memory
            && !subagent_mode
            && !action_restricted
        {
            match &self.memory_provider {
                Some(provider) => {
                    registry.register_in_partition(
                        Arc::new(
                            agent_diva_tools::SessionCheckpointTool::with_provider(
                                provider.clone(),
                                self.workspace.clone(),
                            )
                            .with_session(self.session_checkpoint_session.clone()),
                        ),
                        ToolSchemaPartition::Deferred,
                    );
                }
                None => {
                    registry.register_in_partition(
                        Arc::new(agent_diva_tools::SessionCheckpointTool::new()),
                        ToolSchemaPartition::Deferred,
                    );
                }
            }
        }

        if self.builtin_config.spawn && !subagent_mode && !action_restricted {
            if let Some(spawner) = self.subagent_spawner {
                let route = self
                    .background_task_context
                    .route
                    .clone()
                    .unwrap_or_else(|| {
                        ChannelRoute::new(
                            agent_diva_core::channel::ChannelAddress::new("cli", "direct"),
                            agent_diva_core::channel::Correlation::new("cli:direct"),
                            agent_diva_core::channel::ChannelOrigin::OwnerFrontend,
                        )
                    });
                registry.register(Arc::new(
                    SpawnTool::new(move |task, label, route| {
                        let spawner = spawner.clone();
                        async move { spawner.spawn(task, label, route).await }
                    })
                    .with_context(route),
                ));
            }
        }

        if self.builtin_config.mcp
            && !self.mcp_servers.is_empty()
            && !action_restricted
            && self.plan_phase.is_none()
        {
            for tool in load_mcp_tools_sync(&self.mcp_servers) {
                registry.register_in_partition(tool, ToolSchemaPartition::Deferred);
            }
        }

        if self.builtin_config.cron && !subagent_mode && !action_restricted {
            if let Some(cron_service) = self.cron_service {
                registry.register(Arc::new(CronTool::new(cron_service)));
            }
        }

        if self.builtin_config.enqueue_background_task && !subagent_mode && !action_restricted {
            if let Some(run_store) = self.run_store {
                registry.register(Arc::new(EnqueueBackgroundTaskTool::with_context(
                    (*run_store).clone(),
                    self.background_task_context,
                )));
            }
        }

        if self.plan_phase.is_none() {
            for tool in self.custom_tools {
                registry.register_in_partition(tool, ToolSchemaPartition::Deferred);
            }
        }

        // Register the lightweight `update_plan` tool only in normal chat mode.
        // It is excluded from Plan mode (any plan_phase) and from approved Plan
        // Execution mode (where execution_session_id is set).
        if self.builtin_config.update_plan
            && self.plan_phase.is_none()
            && self.execution_session_id.is_none()
        {
            registry.register(Arc::new(UpdatePlanTool::new()));
        }

        if let (Some(planning), Some(execution_session_id)) =
            (self.planning_config, self.execution_session_id)
        {
            registry.register(Arc::new(ExecutionTodoShowTool::new(
                planning.registry.clone(),
                execution_session_id.clone(),
            )));
            registry.register(Arc::new(ExecutionTodoWriteTool::new(
                planning.registry,
                execution_session_id,
            )));
        }

        if read_only_mode {
            for tool_name in registry.tool_names() {
                if !ToolPolicy::is_read_only_tool(&tool_name) {
                    registry.unregister(&tool_name);
                }
            }
        }

        // Planning is a capability boundary, not a UI hint. Apply the same
        // closed mapping used by invocation checks after every registration
        // path so built-ins, custom tools, and future registrations cannot
        // bypass a persisted phase.
        if let Some(phase) = self.plan_phase.as_ref() {
            for tool_name in registry.tool_names() {
                if !allows_for_phase(phase, builtin_tool_capability(&tool_name)) {
                    registry.unregister(&tool_name);
                }
            }
        }

        // Apply mask-level tool_limits (allow/deny) regardless of mode.
        // This is intentionally done after the Assist-mode read-only filter so
        // explicit allow/deny lists act as an additional guard.
        if let Some(mask_cfg) = &self.mask_config {
            let tool_names: Vec<String> = registry.tool_names();
            let effective = ToolPolicy::resolve(&tool_names, &mask_cfg.tool_limits);
            let effective_set: std::collections::BTreeSet<&str> =
                effective.iter().map(|s| s.as_str()).collect();
            for name in &tool_names {
                if !effective_set.contains(name.as_str()) {
                    registry.unregister(name);
                }
            }
        }

        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NamedTool {
        name: &'static str,
    }

    struct SlowTool;

    #[async_trait::async_trait]
    impl Tool for NamedTool {
        fn name(&self) -> &str {
            self.name
        }

        fn description(&self) -> &str {
            "test tool"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            })
        }

        async fn execute(&self, _args: serde_json::Value) -> agent_diva_tooling::Result<String> {
            Ok("ok".to_string())
        }
    }

    #[async_trait::async_trait]
    impl Tool for SlowTool {
        fn name(&self) -> &str {
            "slow_tool"
        }

        fn description(&self) -> &str {
            "slow test tool"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            })
        }

        async fn execute(&self, _args: serde_json::Value) -> agent_diva_tooling::Result<String> {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            Ok("late".to_string())
        }
    }

    #[test]
    fn test_tool_assembly_minimal() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::minimal())
            .build();

        assert!(registry.has("read_file"));
        assert!(registry.has("write_file"));
        assert!(registry.has("edit_file"));
        assert!(registry.has("list_dir"));
        assert!(!registry.has("exec"));
        assert!(!registry.has("web_search"));
        assert!(!registry.has("web_fetch"));
    }

    #[test]
    fn persona_tools_keep_world_core_and_writes_deferred() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::minimal())
            .with_persona_root(Some(PathBuf::from("/tmp/config")))
            .build();

        let core_names = registry
            .get_definitions()
            .into_iter()
            .filter_map(|definition| definition["function"]["name"].as_str().map(str::to_string))
            .collect::<Vec<_>>();
        assert!(core_names.contains(&"world_read".to_string()));
        assert!(!core_names.contains(&"persona_read".to_string()));

        let deferred_names = registry
            .deferred_tools()
            .into_iter()
            .map(|tool| tool.name().to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            deferred_names,
            vec!["persona_read", "persona_request", "persona_update"]
        );
        assert!(!registry.has("laputa_propose_section_write"));
    }

    #[test]
    fn subagents_do_not_receive_persona_authority_tools() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::minimal())
            .with_persona_root(Some(PathBuf::from("/tmp/config")))
            .build_subagent_registry();

        assert!(!registry.has("world_read"));
        assert!(!registry.has("persona_read"));
        assert!(!registry.has("persona_request"));
        assert!(!registry.has("persona_update"));
    }

    #[test]
    fn c1b_tool_assembly_rebuilds_with_identical_schema_bytes() {
        let first = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::minimal())
            .build();
        let second = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::minimal())
            .build();

        assert_eq!(
            serde_json::to_vec(&first.get_definitions()).unwrap(),
            serde_json::to_vec(&second.get_definitions()).unwrap()
        );
        assert_eq!(first.len(), second.len());

        let names = first
            .get_definitions()
            .into_iter()
            .filter_map(|definition| definition["function"]["name"].as_str().map(str::to_string))
            .collect::<Vec<_>>();
        let mut sorted_names = names.clone();
        sorted_names.sort();
        assert_eq!(names, sorted_names);
    }

    #[test]
    fn custom_tools_are_deferred_until_search_activation() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig {
                tool_discovery: true,
                ..BuiltInToolsConfig::minimal()
            })
            .with_tool(Arc::new(NamedTool { name: "aaa_custom" }))
            .build();

        let hidden_names = registry
            .get_definitions()
            .into_iter()
            .filter_map(|definition| definition["function"]["name"].as_str().map(str::to_string))
            .collect::<Vec<_>>();
        assert!(!hidden_names.iter().any(|name| name == "aaa_custom"));
        registry.search_deferred("custom", 8);
        let names = registry
            .get_definitions()
            .into_iter()
            .filter_map(|definition| definition["function"]["name"].as_str().map(str::to_string))
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "edit_file",
                "list_dir",
                "read_file",
                "tool_search",
                "write_file",
                "aaa_custom",
            ]
        );
    }

    #[test]
    fn test_tool_assembly_none() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::none())
            .build();

        assert!(registry.is_empty());
    }

    #[test]
    fn test_tool_assembly_respects_split_web_flags() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig {
                web_search: true,
                web_fetch: false,
                ..BuiltInToolsConfig::none()
            })
            .with_network_config(NetworkToolConfig::default())
            .build();

        assert!(registry.has("web_search"));
        assert!(!registry.has("web_fetch"));
    }

    #[test]
    fn test_tool_assembly_subagent_mode_disables_spawn_and_attachment() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig {
                filesystem: true,
                spawn: true,
                attachment: true,
                ..BuiltInToolsConfig::none()
            })
            .build_subagent_registry();

        assert!(registry.has("read_file"));
        assert!(!registry.has("spawn"));
        assert!(!registry.has("read_attachment"));
    }

    #[tokio::test]
    async fn test_tool_assembly_plan_mode_is_read_only_without_legacy_planning_tools() {
        let temp_dir = tempfile::tempdir().unwrap();
        let planning = PlanningConfig::open_workspace(temp_dir.path())
            .await
            .unwrap();
        let registry = ToolAssembly::new(temp_dir.path().to_path_buf())
            .builtin(BuiltInToolsConfig::all())
            .with_planning_config(Some(planning))
            .with_tool(Arc::new(NamedTool {
                name: "custom_status",
            }))
            .with_plan_phase(Some(PlanPhase::Plan))
            .build();

        assert!(registry.has("read_file"));
        assert!(registry.has("list_dir"));
        assert!(!registry.has("plan_create"));
        assert!(!registry.has("plan_show"));
        assert!(!registry.has("plan_submit"));
        assert!(!registry.has("todo_write"));
        assert!(!registry.has("plan_approve"));
        assert!(!registry.has("write_file"));
        assert!(!registry.has("edit_file"));
        assert!(!registry.has("exec"));
        assert!(!registry.has("spawn"));
        assert!(!registry.has("cron"));
        assert!(!registry.has("web_search"));
        assert!(!registry.has("web_fetch"));
        assert!(!registry.has("custom_status"));
    }

    #[tokio::test]
    async fn memory_tools_are_registered_when_provider_is_available() {
        let workspace = tempfile::tempdir().unwrap();
        let assembly =
            ToolAssembly::new(workspace.path().to_path_buf()).with_memory_provider(Some(Arc::new(
                agent_diva_laputa::MemoryHome::new(workspace.path()),
            )));
        let registry = assembly.build();
        for name in [
            "memory_add",
            "memory_list",
            "memory_search",
            "memory_get",
            "memory_update",
            "memory_remove",
            "actmem",
            "actmem_edit_work",
            "actmem_complete",
            "actmem_drop",
        ] {
            assert!(
                registry.tool_names().iter().any(|n| n == name),
                "{name} should be registered when a memory provider is configured"
            );
        }
        let definitions = registry.get_definition_set();
        let core_names = definitions.definitions[..definitions.core_count]
            .iter()
            .filter_map(|definition| definition["function"]["name"].as_str().map(str::to_string))
            .collect::<Vec<_>>();
        for name in [
            "memory_add",
            "memory_search",
            "memory_get",
            "memory_update",
            "memory_remove",
            "actmem",
        ] {
            assert!(
                core_names.iter().any(|candidate| candidate == name),
                "{name}"
            );
        }
        let deferred_names = registry
            .deferred_tools()
            .into_iter()
            .map(|tool| tool.name().to_string())
            .collect::<Vec<_>>();
        for name in [
            "memory_list",
            "actmem_edit_work",
            "actmem_complete",
            "actmem_drop",
        ] {
            assert!(
                deferred_names.iter().any(|candidate| candidate == name),
                "{name}"
            );
        }
        assert!(!registry
            .tool_names()
            .iter()
            .any(|name| name == "memory_distill"));
    }

    #[test]
    fn skill_read_is_core_and_distill_is_deferred_only_for_main_agent() {
        let config = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let registry = ToolAssembly::new(workspace.path().to_path_buf())
            .with_persona_root(Some(config.path().to_path_buf()))
            .with_session_checkpoint_session(Some("session-1".into()))
            .build();
        let definitions = registry.get_definition_set();
        let core_names = definitions.definitions[..definitions.core_count]
            .iter()
            .filter_map(|definition| definition["function"]["name"].as_str())
            .collect::<Vec<_>>();
        assert!(core_names.contains(&"skill_read"));
        assert!(!core_names.contains(&"memory_distill"));
        assert!(registry
            .deferred_tools()
            .iter()
            .any(|tool| tool.name() == "memory_distill"));

        let subagent = ToolAssembly::new(workspace.path().to_path_buf())
            .with_persona_root(Some(config.path().to_path_buf()))
            .build_subagent_registry();
        assert!(!subagent.has("skill_read"));
        assert!(!subagent.has("memory_distill"));
    }

    #[tokio::test]
    async fn working_checkpoint_tool_registers_with_session_binding() {
        let workspace = tempfile::tempdir().unwrap();
        let registry = ToolAssembly::new(workspace.path().to_path_buf())
            .with_memory_provider(Some(Arc::new(agent_diva_laputa::MemoryHome::new(
                workspace.path(),
            ))))
            .with_session_checkpoint_session(Some("channel:42".to_string()))
            .build();
        assert!(
            registry
                .tool_names()
                .iter()
                .any(|n| n == "session_checkpoint"),
            "checkpoint tool should be registered when memory + working_memory gates are on"
        );
        registry
            .execute(
                "tool_search",
                serde_json::json!({"query": "session_checkpoint"}),
            )
            .await
            .unwrap();
        let result = registry
            .execute(
                "session_checkpoint",
                serde_json::json!({"key_info": "in-flight state"}),
            )
            .await
            .unwrap();
        assert!(result.contains("\"status\""));
    }

    #[tokio::test]
    async fn session_checkpoint_tool_is_gated_by_legacy_config_flag() {
        let workspace = tempfile::tempdir().unwrap();
        let registry = ToolAssembly::new(workspace.path().to_path_buf())
            .builtin(BuiltInToolsConfig::minimal())
            .build();
        assert!(
            !registry
                .tool_names()
                .iter()
                .any(|n| n == "session_checkpoint"),
            "checkpoint tool must be absent when working_memory gate is off"
        );
    }

    #[tokio::test]
    async fn memory_tools_register_without_provider_and_report_failed() {
        let workspace = tempfile::tempdir().unwrap();
        let registry = ToolAssembly::new(workspace.path().to_path_buf()).build();
        for name in [
            "memory_add",
            "memory_list",
            "memory_search",
            "memory_get",
            "memory_update",
            "memory_remove",
            "actmem",
            "actmem_edit_work",
            "actmem_complete",
            "actmem_drop",
        ] {
            assert!(
                registry.tool_names().iter().any(|n| n == name),
                "{name} should be registered (unavailable) even without a provider"
            );
        }
        let result = registry
            .execute("memory_add", serde_json::json!({"content": "x"}))
            .await
            .unwrap();
        assert!(result.contains("\"status\":\"failed\""));
    }

    #[test]
    fn subagents_receive_no_memory_or_actmem_tools() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::all())
            .build_subagent_registry();
        assert!(!registry.tool_names().iter().any(|name| {
            name.starts_with("memory_")
                || name.starts_with("actmem")
                || name == "session_checkpoint"
        }));
    }

    #[tokio::test]
    async fn planning_phase_filters_registry_with_the_core_capability_policy() {
        let temp_dir = tempfile::tempdir().unwrap();
        let planning = PlanningConfig::open_workspace(temp_dir.path())
            .await
            .unwrap();

        for (phase, inspect, write, execute, external) in [
            (PlanPhase::Explore, true, false, false, false),
            (PlanPhase::Plan, true, false, false, false),
            (PlanPhase::AwaitingApproval, true, false, false, false),
            (PlanPhase::Execute, true, true, true, true),
            (PlanPhase::Verify, true, false, true, false),
            (PlanPhase::Completed, false, false, false, false),
            (PlanPhase::Failed, false, false, false, false),
            (PlanPhase::Partial, false, false, false, false),
        ] {
            let registry = ToolAssembly::new(temp_dir.path().to_path_buf())
                .builtin(BuiltInToolsConfig::all())
                .with_planning_config(Some(planning.clone()))
                .with_tool(Arc::new(NamedTool {
                    name: "custom_tool",
                }))
                .with_plan_phase(Some(phase.clone()))
                .build();

            assert_eq!(registry.has("read_file"), inspect, "{phase}");
            assert_eq!(registry.has("list_dir"), inspect, "{phase}");
            assert!(!registry.has("plan_create"), "{phase}");
            assert!(!registry.has("plan_transition"), "{phase}");
            assert_eq!(registry.has("write_file"), write, "{phase}");
            assert_eq!(registry.has("edit_file"), write, "{phase}");
            assert_eq!(registry.has("exec"), execute, "{phase}");
            assert!(!registry.has("plan_submit"), "{phase}");
            assert!(!registry.has("todo_write"), "{phase}");
            assert_eq!(registry.has("web_search"), external, "{phase}");
            assert!(!registry.has("custom_tool"), "{phase}");
        }
    }

    #[tokio::test]
    async fn execution_session_registers_execution_todo_tools_only() {
        let temp_dir = tempfile::tempdir().unwrap();
        let planning = PlanningConfig::open_workspace(temp_dir.path())
            .await
            .unwrap();
        let registry = ToolAssembly::new(temp_dir.path().to_path_buf())
            .builtin(BuiltInToolsConfig::all())
            .with_planning_config(Some(planning))
            .with_plan_phase(Some(PlanPhase::Execute))
            .with_execution_session(Some("execution-1".to_string()))
            .build();

        assert!(registry.has("todo_show"));
        assert!(registry.has("todo_write"));
        assert!(!registry.has("plan_create"));
        assert!(!registry.has("plan_submit"));
        assert!(!registry.has("plan_transition"));
    }

    #[test]
    fn tool_assembly_normal_chat_has_update_plan() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::all())
            .build();

        assert!(registry.has("update_plan"));
    }

    #[tokio::test]
    async fn tool_assembly_plan_mode_no_update_plan() {
        let temp_dir = tempfile::tempdir().unwrap();
        let planning = PlanningConfig::open_workspace(temp_dir.path())
            .await
            .unwrap();

        for phase in [
            PlanPhase::Explore,
            PlanPhase::Plan,
            PlanPhase::AwaitingApproval,
            PlanPhase::Execute,
            PlanPhase::Verify,
            PlanPhase::Completed,
            PlanPhase::Failed,
            PlanPhase::Partial,
        ] {
            let registry = ToolAssembly::new(temp_dir.path().to_path_buf())
                .builtin(BuiltInToolsConfig::all())
                .with_planning_config(Some(planning.clone()))
                .with_plan_phase(Some(phase.clone()))
                .build();

            assert!(!registry.has("update_plan"), "{phase}");
        }
    }

    #[tokio::test]
    async fn tool_assembly_execution_mode_no_update_plan() {
        let temp_dir = tempfile::tempdir().unwrap();
        let planning = PlanningConfig::open_workspace(temp_dir.path())
            .await
            .unwrap();
        let registry = ToolAssembly::new(temp_dir.path().to_path_buf())
            .builtin(BuiltInToolsConfig::all())
            .with_planning_config(Some(planning))
            .with_plan_phase(Some(PlanPhase::Execute))
            .with_execution_session(Some("execution-1".to_string()))
            .build();

        assert!(!registry.has("update_plan"));
        assert!(registry.has("todo_write"));
    }

    #[test]
    fn test_tool_assembly_enqueue_background_task_when_config_true() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig {
                enqueue_background_task: true,
                ..BuiltInToolsConfig::none()
            })
            .build();

        // Without run_store, the tool is not registered
        assert!(!registry.has("enqueue_background_task"));
    }

    #[tokio::test]
    async fn test_tool_assembly_enqueue_background_task_with_run_store() {
        let temp_dir = tempfile::tempdir().unwrap();
        let run_store = Arc::new(
            RunStore::new(temp_dir.path())
                .await
                .expect("store creation"),
        );
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig {
                enqueue_background_task: true,
                ..BuiltInToolsConfig::none()
            })
            .with_run_store(run_store)
            .build();

        assert!(registry.has("enqueue_background_task"));
    }

    #[tokio::test]
    async fn test_tool_assembly_subagent_mode_excludes_enqueue_background_task() {
        let temp_dir = tempfile::tempdir().unwrap();
        let run_store = Arc::new(
            RunStore::new(temp_dir.path())
                .await
                .expect("store creation"),
        );
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig {
                enqueue_background_task: true,
                ..BuiltInToolsConfig::none()
            })
            .with_run_store(run_store)
            .build_subagent_registry();

        assert!(!registry.has("enqueue_background_task"));
    }

    #[test]
    fn test_tool_assembly_assist_mode_is_read_only() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::all())
            .with_mask_config(Some(MaskConfig {
                name: "Reviewer".to_string(),
                mode: Some(agent_diva_core::config::schema::AgentMode::Assist),
                ..Default::default()
            }))
            .build();

        assert!(registry.has("read_file"));
        assert!(registry.has("list_dir"));
        assert!(!registry.has("write_file"));
        assert!(!registry.has("edit_file"));
        assert!(!registry.has("exec"));
        assert!(!registry.has("spawn"));
        assert!(!registry.has("cron"));
    }

    #[tokio::test]
    async fn test_tool_assembly_applies_global_timeout_to_registry() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig {
                tool_discovery: true,
                ..BuiltInToolsConfig::none()
            })
            .with_global_timeout(1)
            .with_tool(Arc::new(SlowTool))
            .build();

        registry.search_deferred("slow", 8);
        let result = registry.execute("slow_tool", serde_json::json!({})).await;
        assert!(matches!(
            result,
            Err(agent_diva_tooling::ToolError::Timeout { secs: 1 })
        ));
    }

    #[test]
    fn tool_assembly_mask_limits_deny_hides_tool() {
        use agent_diva_core::config::schema::ToolLimits;

        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::all())
            .with_mask_config(Some(MaskConfig {
                name: "DenyWrite".to_string(),
                tool_limits: ToolLimits {
                    allow: vec![],
                    deny: vec!["write_file".to_string()],
                },
                ..Default::default()
            }))
            .build();

        assert!(registry.has("read_file"));
        assert!(!registry.has("write_file"));
        assert!(registry.has("list_dir"));
    }

    #[test]
    fn tool_assembly_mask_limits_allow_restricts_tools() {
        use agent_diva_core::config::schema::ToolLimits;

        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::all())
            .with_mask_config(Some(MaskConfig {
                name: "AllowReadOnly".to_string(),
                tool_limits: ToolLimits {
                    allow: vec!["read_file".to_string(), "list_dir".to_string()],
                    deny: vec![],
                },
                ..Default::default()
            }))
            .build();

        assert!(registry.has("read_file"));
        assert!(registry.has("list_dir"));
        assert!(!registry.has("write_file"));
        assert!(!registry.has("exec"));
    }

    #[test]
    fn tool_assembly_assist_mode_plus_explicit_deny() {
        use agent_diva_core::config::schema::{AgentMode, ToolLimits};

        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::all())
            .with_mask_config(Some(MaskConfig {
                name: "AssistNoRead".to_string(),
                mode: Some(AgentMode::Assist),
                tool_limits: ToolLimits {
                    allow: vec![],
                    deny: vec!["read_file".to_string()],
                },
                ..Default::default()
            }))
            .build();

        // Assist mode already restricts to read-only tools; explicit deny removes read_file.
        assert!(!registry.has("read_file"));
        assert!(!registry.has("write_file"));
        assert!(registry.has("list_dir"));
    }

    #[test]
    fn tool_assembly_registers_ask_user_tool() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::all())
            .build();

        assert!(registry.has("ask_user"));
    }

    #[tokio::test]
    async fn tool_assembly_headless_ask_user_reports_unavailable() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::all())
            .build();

        let result = registry
            .execute("ask_user", serde_json::json!({"question": "Pick?"}))
            .await;
        assert!(result.unwrap().contains("\"unavailable\""));
    }

    #[tokio::test]
    async fn tool_assembly_subagent_mode_excludes_ask_user() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::all())
            .build_subagent_registry();

        assert!(!registry.has("ask_user"));
    }

    #[tokio::test]
    async fn tool_assembly_execution_session_excludes_ask_user() {
        let temp_dir = tempfile::tempdir().unwrap();
        let planning = PlanningConfig::open_workspace(temp_dir.path())
            .await
            .unwrap();
        let registry = ToolAssembly::new(temp_dir.path().to_path_buf())
            .builtin(BuiltInToolsConfig::all())
            .with_planning_config(Some(planning))
            .with_plan_phase(Some(PlanPhase::Execute))
            .with_execution_session(Some("execution-1".to_string()))
            .build();

        assert!(!registry.has("ask_user"));
    }

    #[tokio::test]
    async fn tool_assembly_ask_user_answers_through_coordinator() {
        let coordinator = agent_diva_core::ask_user::AskUserCoordinator::default();
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig::all())
            .with_ask_user_coordinator(Some(coordinator.clone()))
            .build();

        let execute = tokio::spawn({
            let registry = Arc::new(registry);
            async move {
                registry
                    .execute(
                        "ask_user",
                        serde_json::json!({
                            "question": "Which option?",
                            "choices": ["A", "B"]
                        }),
                    )
                    .await
            }
        });
        let question_id = loop {
            if let Some(question) = coordinator.pending().await.into_iter().next() {
                break question.question_id;
            }
            tokio::task::yield_now().await;
        };
        coordinator
            .answer(&question_id, Some(1), None)
            .await
            .unwrap();
        let result = execute.await.unwrap().unwrap();
        assert!(result.contains("\"selected\":\"B\""));
    }
}

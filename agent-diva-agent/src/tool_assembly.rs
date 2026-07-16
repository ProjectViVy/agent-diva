use crate::mask::{MaskFile, ToolPolicy};
use crate::planning::builtin_tool_capability;
use crate::tool_config::PlanningConfig;
use crate::tool_config::{builtin::BuiltInToolsConfig, network::NetworkToolConfig};
use agent_diva_core::config::schema::MaskConfig;
use agent_diva_core::config::MCPServerConfig;
use agent_diva_core::cron::CronService;
use agent_diva_core::planning::model::PlanPhase;
use agent_diva_core::planning::policy::allows_for_phase;
use agent_diva_core::security::{SecurityConfig, SecurityLevel, SecurityPolicy};
use agent_diva_core::supervised::RunStore;
use agent_diva_files::FileManager;
use agent_diva_tooling::{Tool, ToolError, ToolRegistry};
use agent_diva_tools::{
    load_mcp_tools_sync, BackgroundTaskContext, CronTool, EditFileTool, EnqueueBackgroundTaskTool,
    ExecTool, ExecutionTodoShowTool, ExecutionTodoWriteTool, ListDirTool, ReadAttachmentTool,
    ReadFileTool, SpawnTool, UpdatePlanTool, WebFetchTool, WebSearchTool, WriteFileTool,
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
        channel: String,
        chat_id: String,
    ) -> Result<String, ToolError>;
}

pub struct ToolAssembly {
    workspace: PathBuf,
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
}

impl ToolAssembly {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
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
        let mut registry = ToolRegistry::with_timeout(self.global_timeout_secs);

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
            registry.register(Arc::new(ExecTool::with_config(
                self.exec_timeout,
                Some(self.workspace.clone()),
                self.restrict_to_workspace,
            )));
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

        if self.builtin_config.spawn && !subagent_mode && !action_restricted {
            if let Some(spawner) = self.subagent_spawner {
                registry.register(Arc::new(SpawnTool::new(
                    move |task, label, channel, chat_id| {
                        let spawner = spawner.clone();
                        async move { spawner.spawn(task, label, channel, chat_id).await }
                    },
                )));
            }
        }

        if self.builtin_config.mcp
            && !self.mcp_servers.is_empty()
            && !action_restricted
            && self.plan_phase.is_none()
        {
            for tool in load_mcp_tools_sync(&self.mcp_servers) {
                registry.register(tool);
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
                registry.register(tool);
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

    #[test]
    fn test_tool_assembly_subagent_mode_excludes_mentle_custom_tools() {
        let registry = ToolAssembly::new(PathBuf::from("/tmp/test"))
            .builtin(BuiltInToolsConfig {
                filesystem: true,
                mentle: true,
                ..BuiltInToolsConfig::all()
            })
            .with_tool(Arc::new(NamedTool {
                name: "memtle_status",
            }))
            .build_subagent_registry();

        assert!(registry.has("read_file"));
        assert!(!registry.has("memtle_status"));
        assert!(!registry.has("spawn"));
        assert!(!registry.has("cron"));
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
                name: "memtle_status",
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
        assert!(!registry.has("memtle_status"));
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
            .builtin(BuiltInToolsConfig::none())
            .with_global_timeout(1)
            .with_tool(Arc::new(SlowTool))
            .build();

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
}

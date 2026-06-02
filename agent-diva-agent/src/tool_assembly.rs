use crate::tool_config::{builtin::BuiltInToolsConfig, network::NetworkToolConfig};
use agent_diva_core::config::MCPServerConfig;
use agent_diva_core::cron::CronService;
use agent_diva_core::security::{SecurityConfig, SecurityLevel, SecurityPolicy};
use agent_diva_files::FileManager;
use agent_diva_tooling::{Tool, ToolError, ToolRegistry};
use agent_diva_tools::{
    load_mcp_tools_sync, CronTool, EditFileTool, ExecTool, ListDirTool, ReadAttachmentTool,
    ReadFileTool, SpawnTool, WebFetchTool, WebSearchTool, WriteFileTool,
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
    restrict_to_workspace: bool,
    mcp_servers: HashMap<String, MCPServerConfig>,
    cron_service: Option<Arc<CronService>>,
    custom_tools: Vec<Arc<dyn Tool>>,
    subagent_spawner: Option<Arc<dyn SubagentSpawner>>,
    file_manager: Option<Arc<FileManager>>,
}

impl ToolAssembly {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            builtin_config: BuiltInToolsConfig::default(),
            network_config: NetworkToolConfig::default(),
            exec_timeout: 60,
            restrict_to_workspace: false,
            mcp_servers: HashMap::new(),
            cron_service: None,
            custom_tools: Vec::new(),
            subagent_spawner: None,
            file_manager: None,
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

    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.custom_tools.push(tool);
        self
    }

    pub fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.custom_tools.extend(tools);
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
        self.build_internal(true)
    }

    fn build_internal(self, subagent_mode: bool) -> ToolRegistry {
        let mut registry = ToolRegistry::new();

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
            let security = Arc::new(SecurityPolicy::with_config(
                self.workspace.clone(),
                security_config,
            ));
            registry.register(Arc::new(ReadFileTool::new(security.clone())));
            registry.register(Arc::new(WriteFileTool::new(security.clone())));
            registry.register(Arc::new(EditFileTool::new(security.clone())));
            registry.register(Arc::new(ListDirTool::new(security)));
        }

        if self.builtin_config.attachment {
            if let Some(file_manager) = self.file_manager {
                registry.register(Arc::new(ReadAttachmentTool::new(file_manager)));
            }
        }

        if self.builtin_config.shell {
            registry.register(Arc::new(ExecTool::with_config(
                self.exec_timeout,
                Some(self.workspace.clone()),
                self.restrict_to_workspace,
            )));
        }

        if self.builtin_config.web_search && self.network_config.web.search.enabled {
            registry.register(Arc::new(WebSearchTool::with_provider_and_max_results(
                self.network_config.web.search.provider.clone(),
                self.network_config.web.search.api_key.clone(),
                self.network_config.web.search.normalized_max_results(),
            )));
        }

        if self.builtin_config.web_fetch && self.network_config.web.fetch.enabled {
            registry.register(Arc::new(WebFetchTool::new()));
        }

        if self.builtin_config.spawn && !subagent_mode {
            if let Some(spawner) = self.subagent_spawner {
                registry.register(Arc::new(SpawnTool::new(
                    move |task, label, channel, chat_id| {
                        let spawner = spawner.clone();
                        async move { spawner.spawn(task, label, channel, chat_id).await }
                    },
                )));
            }
        }

        if self.builtin_config.mcp && !self.mcp_servers.is_empty() {
            for tool in load_mcp_tools_sync(&self.mcp_servers) {
                registry.register(tool);
            }
        }

        if self.builtin_config.cron && !subagent_mode {
            if let Some(cron_service) = self.cron_service {
                registry.register(Arc::new(CronTool::new(cron_service)));
            }
        }

        for tool in self.custom_tools {
            registry.register(tool);
        }

        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

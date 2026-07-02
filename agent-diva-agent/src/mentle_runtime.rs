//! Internal Mentle runtime assembly for AgentLoop.

use agent_diva_core::memory::MemoryProvider;
use agent_diva_tooling::{Tool, ToolError};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::warn;

use crate::tool_config::mentle::MentleToolRuntimeConfig;

/// Runtime state shared by Mentle-backed memory and dynamic Mentle tools.
pub(super) struct MentleRuntime {
    #[allow(dead_code)]
    toolkit: Arc<Mutex<memtle::toolkit::MemtleToolkit>>,
    memory_provider: Arc<dyn MemoryProvider>,
    custom_tools: Vec<Arc<dyn Tool>>,
    active: bool,
}

impl MentleRuntime {
    /// Build the embedded Mentle runtime for one AgentLoop workspace.
    pub(super) async fn try_build(
        workspace: &Path,
        tool_config: &MentleToolRuntimeConfig,
    ) -> Option<Self> {
        if !tool_config.is_active_request() {
            return None;
        }

        let db_path = workspace.join("memory").join("palace.db");
        if let Some(parent) = db_path.parent() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                let mapped = map_mentle_transport_error(
                    MentleErrorPhase::StartupOpen,
                    MentleFallbackAction::DisableMentle,
                    err,
                );
                warn!(
                    phase = mapped.phase.as_str(),
                    category = mapped.category.as_str(),
                    fallback_action = mapped.fallback_action.as_str(),
                    db_path = %db_path.display(),
                    error = %mapped.message,
                    "Mentle disabled: failed to create memory dir"
                );
                return None;
            }
        }

        let toolkit = match memtle::toolkit::MemtleToolkit::open(&db_path).await {
            Ok(toolkit) => toolkit,
            Err(err) => {
                let mapped = map_mentle_transport_error(
                    MentleErrorPhase::StartupOpen,
                    MentleFallbackAction::DisableMentle,
                    err,
                );
                warn!(
                    phase = mapped.phase.as_str(),
                    category = mapped.category.as_str(),
                    fallback_action = mapped.fallback_action.as_str(),
                    db_path = %db_path.display(),
                    error = %mapped.message,
                    "Mentle disabled: failed to open palace database"
                );
                return None;
            }
        };

        let toolkit = Arc::new(Mutex::new(toolkit));
        let file_manager = Arc::new(agent_diva_core::memory::MemoryManager::new(workspace));
        let memory_provider: Arc<dyn MemoryProvider> = Arc::new(
            agent_diva_core::memory::HybridMemoryProvider::new(file_manager, toolkit.clone()).await,
        );

        let tool_defs = toolkit.lock().await.tool_definitions();
        let custom_tools = filter_mentle_tools(
            mentle_tools_from_definitions(tool_defs, toolkit.clone()),
            tool_config,
        );

        Some(Self::from_parts(toolkit, memory_provider, custom_tools))
    }

    #[must_use]
    pub(super) fn memory_provider(&self) -> Arc<dyn MemoryProvider> {
        self.memory_provider.clone()
    }

    #[must_use]
    pub(super) fn custom_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.custom_tools.clone()
    }

    #[must_use]
    pub(super) const fn active(&self) -> bool {
        self.active
    }

    fn from_parts(
        toolkit: Arc<Mutex<memtle::toolkit::MemtleToolkit>>,
        memory_provider: Arc<dyn MemoryProvider>,
        custom_tools: Vec<Arc<dyn Tool>>,
    ) -> Self {
        let active = custom_tools
            .iter()
            .any(|tool| tool.name() == "memtle_status");

        Self {
            toolkit,
            memory_provider,
            custom_tools,
            active,
        }
    }

    #[cfg(test)]
    pub(super) fn from_parts_for_test(
        toolkit: Arc<Mutex<memtle::toolkit::MemtleToolkit>>,
        memory_provider: Arc<dyn MemoryProvider>,
        custom_tools: Vec<Arc<dyn Tool>>,
    ) -> Self {
        Self::from_parts(toolkit, memory_provider, custom_tools)
    }
}

pub(super) struct MentleToolkitTool {
    pub(super) name: String,
    pub(super) description: String,
    pub(super) parameters: serde_json::Value,
    pub(super) toolkit: Arc<Mutex<memtle::toolkit::MemtleToolkit>>,
}

#[derive(Clone, Copy)]
enum MentleErrorPhase {
    StartupOpen,
    ToolDefinition,
    ToolCallTransport,
    ToolCallPayload,
}

impl MentleErrorPhase {
    const fn as_str(self) -> &'static str {
        match self {
            Self::StartupOpen => "startup_open",
            Self::ToolDefinition => "tool_definition",
            Self::ToolCallTransport => "tool_call_transport",
            Self::ToolCallPayload => "tool_call_payload",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MentleErrorCategory {
    Io,
    Database,
    Json,
    Config,
    InvalidArguments,
    UnknownTool,
    NotFound,
    InvalidDefinition,
    ToolPayload,
    Internal,
}

impl MentleErrorCategory {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Io => "io",
            Self::Database => "database",
            Self::Json => "json",
            Self::Config => "config",
            Self::InvalidArguments => "invalid_arguments",
            Self::UnknownTool => "unknown_tool",
            Self::NotFound => "not_found",
            Self::InvalidDefinition => "invalid_definition",
            Self::ToolPayload => "tool_payload",
            Self::Internal => "internal",
        }
    }
}

#[derive(Clone, Copy)]
enum MentleFallbackAction {
    DisableMentle,
    SkipTool,
    ReturnToolError,
}

impl MentleFallbackAction {
    const fn as_str(self) -> &'static str {
        match self {
            Self::DisableMentle => "disable_mentle",
            Self::SkipTool => "skip_tool",
            Self::ReturnToolError => "return_tool_error",
        }
    }
}

struct MentleMappedError {
    phase: MentleErrorPhase,
    category: MentleErrorCategory,
    fallback_action: MentleFallbackAction,
    message: String,
}

fn classify_mentle_error_message(message: &str) -> MentleErrorCategory {
    let lower = message.to_ascii_lowercase();
    if lower.contains("unknown tool") {
        MentleErrorCategory::UnknownTool
    } else if lower.contains("not found") {
        MentleErrorCategory::NotFound
    } else if lower.contains("argument") || lower.contains("must be") || lower.contains("invalid") {
        MentleErrorCategory::InvalidArguments
    } else if lower.starts_with("database error") || lower.contains("database") {
        MentleErrorCategory::Database
    } else if lower.starts_with("io error") || lower.contains("os error") {
        MentleErrorCategory::Io
    } else if lower.starts_with("json error") || lower.contains("json") {
        MentleErrorCategory::Json
    } else if lower.contains("config") {
        MentleErrorCategory::Config
    } else {
        MentleErrorCategory::Internal
    }
}

fn map_mentle_transport_error(
    phase: MentleErrorPhase,
    fallback_action: MentleFallbackAction,
    error: impl std::fmt::Display,
) -> MentleMappedError {
    let message = error.to_string();
    MentleMappedError {
        phase,
        category: classify_mentle_error_message(&message),
        fallback_action,
        message,
    }
}

fn map_mentle_payload_error(
    phase: MentleErrorPhase,
    fallback_action: MentleFallbackAction,
    value: &serde_json::Value,
) -> Option<MentleMappedError> {
    let error = value
        .get("error")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let success_false = value.get("success").and_then(serde_json::Value::as_bool) == Some(false);

    if error.is_none() && !success_false {
        return None;
    }

    let message =
        error.unwrap_or_else(|| "Mentle tool returned an unsuccessful payload".to_string());
    let mut category = classify_mentle_error_message(&message);
    if category == MentleErrorCategory::Internal {
        category = MentleErrorCategory::ToolPayload;
    }

    Some(MentleMappedError {
        phase,
        category,
        fallback_action,
        message,
    })
}

fn mentle_execution_failed(mapped: MentleMappedError) -> ToolError {
    ToolError::ExecutionFailed(mapped.message)
}

#[async_trait::async_trait]
impl Tool for MentleToolkitTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> serde_json::Value {
        self.parameters.clone()
    }

    async fn execute(&self, args: serde_json::Value) -> agent_diva_tooling::Result<String> {
        let toolkit = self.toolkit.lock().await;
        let result = toolkit.call_json(&self.name, args).await.map_err(|err| {
            let mapped = map_mentle_transport_error(
                MentleErrorPhase::ToolCallTransport,
                MentleFallbackAction::ReturnToolError,
                err,
            );
            warn!(
                phase = mapped.phase.as_str(),
                category = mapped.category.as_str(),
                fallback_action = mapped.fallback_action.as_str(),
                tool_name = %self.name,
                error = %mapped.message,
                "Mentle toolkit call failed"
            );
            mentle_execution_failed(mapped)
        })?;

        if let Some(mapped) = map_mentle_payload_error(
            MentleErrorPhase::ToolCallPayload,
            MentleFallbackAction::ReturnToolError,
            &result,
        ) {
            warn!(
                phase = mapped.phase.as_str(),
                category = mapped.category.as_str(),
                fallback_action = mapped.fallback_action.as_str(),
                tool_name = %self.name,
                error = %mapped.message,
                result = %result,
                "Mentle toolkit returned an error payload"
            );
            return Err(mentle_execution_failed(mapped));
        }

        if let Some(text) = result.as_str() {
            Ok(text.to_string())
        } else {
            serde_json::to_string_pretty(&result)
                .map_err(|err| ToolError::ExecutionFailed(err.to_string()))
        }
    }
}

pub(super) fn mentle_tool_metadata_from_definition(
    def: &serde_json::Value,
) -> Option<(String, String, serde_json::Value)> {
    let name = match def.get("name").and_then(|value| value.as_str()) {
        Some(name) => name,
        None => {
            warn_invalid_mentle_tool_definition(def, "name");
            return None;
        }
    };
    let description = match def.get("description").and_then(|value| value.as_str()) {
        Some(description) => description,
        None => {
            warn_invalid_mentle_tool_definition(def, "description");
            return None;
        }
    };
    let parameters = match def.get("inputSchema") {
        Some(parameters) => parameters,
        None => {
            warn_invalid_mentle_tool_definition(def, "inputSchema");
            return None;
        }
    };
    if !parameters.is_object() {
        warn_invalid_mentle_tool_definition(def, "inputSchema");
        return None;
    }

    Some((
        name.to_string(),
        description.to_string(),
        parameters.clone(),
    ))
}

fn warn_invalid_mentle_tool_definition(def: &serde_json::Value, field: &str) {
    warn!(
        phase = MentleErrorPhase::ToolDefinition.as_str(),
        category = MentleErrorCategory::InvalidDefinition.as_str(),
        fallback_action = MentleFallbackAction::SkipTool.as_str(),
        field,
        definition = %def,
        "Skipping invalid Mentle tool definition"
    );
}

pub(super) fn mentle_tool_from_definition(
    def: &serde_json::Value,
    toolkit: Arc<Mutex<memtle::toolkit::MemtleToolkit>>,
) -> Option<Arc<dyn Tool>> {
    let (name, description, parameters) = mentle_tool_metadata_from_definition(def)?;

    Some(Arc::new(MentleToolkitTool {
        name,
        description,
        parameters,
        toolkit,
    }) as Arc<dyn Tool>)
}

pub(super) fn mentle_tools_from_definitions(
    tool_defs: impl IntoIterator<Item = serde_json::Value>,
    toolkit: Arc<Mutex<memtle::toolkit::MemtleToolkit>>,
) -> Vec<Arc<dyn Tool>> {
    let mut tools = Vec::new();
    for def in tool_defs {
        if let Some(tool) = mentle_tool_from_definition(&def, toolkit.clone()) {
            tools.push(tool);
        }
    }

    tools
}

pub(super) fn filter_mentle_tools(
    tools: Vec<Arc<dyn Tool>>,
    config: &MentleToolRuntimeConfig,
) -> Vec<Arc<dyn Tool>> {
    tools
        .into_iter()
        .filter(|tool| config.allows_tool(tool.name()))
        .collect()
}

/// Discover `memtle_*` tool names from the workspace toolkit metadata.
pub async fn discover_mentle_tool_names(workspace: &Path) -> Vec<String> {
    #[cfg(feature = "mentle")]
    {
        let db_path = workspace.join("memory").join("palace.db");
        if let Some(parent) = db_path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                return Vec::new();
            }
        }

        let toolkit = match memtle::toolkit::MemtleToolkit::open(&db_path).await {
            Ok(toolkit) => toolkit,
            Err(_) => return Vec::new(),
        };

        let mut names = Vec::new();
        for def in toolkit.tool_definitions() {
            if let Some((name, _, _)) = mentle_tool_metadata_from_definition(&def) {
                if name.starts_with("memtle_") {
                    names.push(name);
                }
            }
        }
        names.sort();
        names.dedup();
        names
    }

    #[cfg(not(feature = "mentle"))]
    {
        let _ = workspace;
        Vec::new()
    }
}

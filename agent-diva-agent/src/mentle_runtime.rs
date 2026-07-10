//! Internal Mentle runtime assembly for AgentLoop.

use agent_diva_core::memory::MemoryProvider;
use agent_diva_tooling::{Tool, ToolError};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::tool_config::mentle::MentleToolRuntimeConfig;

/// Stack reserved for the dedicated Mentle open thread.
///
/// Windows default stacks are too small for turso/simsimd capability probes
/// (`simsimd_wsum_u8` has been observed to `STATUS_STACK_OVERFLOW` on the
/// main/Tokio worker stack). Opening on a large dedicated stack keeps the
/// native path available instead of permanently disabling Mentle on Windows.
const MENTLE_OPEN_STACK_BYTES: usize = 16 * 1024 * 1024;

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

        // Must be applied before the embedded turso connection is opened. The
        // CLI applies this before creating Tokio; this call also covers
        // embedded Manager users that already own a runtime.
        memtle::init_process_defaults();

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

        // Open the palace, warm HybridMemoryProvider, and collect tool defs on a
        // dedicated large-stack thread. Post-open status/graph queries also hit
        // turso/simsimd and have overflowed the main Tokio stack on Windows.
        let workspace = workspace.to_path_buf();
        let assembled = match assemble_mentle_runtime_isolated(db_path.clone(), workspace).await {
            Ok(parts) => {
                info!(
                    db_path = %db_path.display(),
                    tool_count = parts.tool_defs.len(),
                    "Mentle runtime assembled on isolated large-stack thread"
                );
                parts
            }
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
                    "Mentle disabled: failed to assemble palace runtime"
                );
                return None;
            }
        };

        let custom_tools = filter_mentle_tools(
            mentle_tools_from_definitions(assembled.tool_defs, assembled.toolkit.clone()),
            tool_config,
        );

        Some(Self::from_parts(
            assembled.toolkit,
            assembled.memory_provider,
            custom_tools,
        ))
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

struct AssembledMentleRuntime {
    toolkit: Arc<Mutex<memtle::toolkit::MemtleToolkit>>,
    memory_provider: Arc<dyn MemoryProvider>,
    tool_defs: Vec<serde_json::Value>,
}

/// Run a Mentle startup future on a dedicated large-stack thread.
///
/// `MemtleToolkit::open` and early palace queries pull in turso + simsimd. On
/// Windows those native libraries have overflowed the default main/Tokio worker
/// stack (`simsimd_wsum_u8` → `STATUS_STACK_OVERFLOW`). Isolating startup keeps
/// the native Mentle path available instead of permanently disabling it.
async fn run_on_mentle_large_stack<T, F, Fut>(label: &'static str, work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, String>> + Send + 'static,
{
    memtle::init_process_defaults();

    let join = tokio::task::spawn_blocking(move || {
        let builder = std::thread::Builder::new()
            .name(label.to_string())
            .stack_size(MENTLE_OPEN_STACK_BYTES);

        let handle = builder
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|err| format!("failed to build {label} runtime: {err}"))?;
                runtime.block_on(work())
            })
            .map_err(|err| format!("failed to spawn {label} thread: {err}"))?;

        match handle.join() {
            Ok(result) => result,
            Err(panic_payload) => {
                let message = panic_payload
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_string())
                    .or_else(|| panic_payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| format!("{label} thread panicked"));
                Err(message)
            }
        }
    })
    .await
    .map_err(|err| format!("{label} blocking task failed: {err}"))?;

    join
}

/// Open a Mentle palace database on a dedicated large-stack thread.
async fn open_memtle_toolkit_isolated(
    db_path: PathBuf,
) -> Result<memtle::toolkit::MemtleToolkit, String> {
    run_on_mentle_large_stack("mentle-open", move || async move {
        memtle::toolkit::MemtleToolkit::open(&db_path)
            .await
            .map_err(|err| err.to_string())
    })
    .await
}

/// Open the palace and warm HybridMemoryProvider on a dedicated large-stack thread.
async fn assemble_mentle_runtime_isolated(
    db_path: PathBuf,
    workspace: PathBuf,
) -> Result<AssembledMentleRuntime, String> {
    run_on_mentle_large_stack("mentle-assemble", move || async move {
        let toolkit = memtle::toolkit::MemtleToolkit::open(&db_path)
            .await
            .map_err(|err| err.to_string())?;
        let tool_defs = toolkit.tool_definitions();
        let toolkit = Arc::new(Mutex::new(toolkit));
        let file_manager = Arc::new(agent_diva_core::memory::MemoryManager::new(&workspace));
        let memory_provider: Arc<dyn MemoryProvider> = Arc::new(
            agent_diva_core::memory::HybridMemoryProvider::new(file_manager, toolkit.clone()).await,
        );
        Ok(AssembledMentleRuntime {
            toolkit,
            memory_provider,
            tool_defs,
        })
    })
    .await
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

        let toolkit = match open_memtle_toolkit_isolated(db_path).await {
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

#[cfg(test)]
mod open_isolation_tests {
    use super::{assemble_mentle_runtime_isolated, open_memtle_toolkit_isolated, MentleRuntime};
    use crate::tool_config::mentle::{MentleToolMode, MentleToolRuntimeConfig};
    use tempfile::tempdir;

    #[tokio::test]
    async fn try_build_returns_none_when_inactive() {
        let temp = tempdir().expect("tempdir");
        let config = MentleToolRuntimeConfig {
            enabled: false,
            mode: MentleToolMode::Off,
            allowed_tools: Vec::new(),
        };
        assert!(MentleRuntime::try_build(temp.path(), &config)
            .await
            .is_none());
    }

    #[tokio::test]
    async fn isolated_open_creates_palace_database() {
        let temp = tempdir().expect("tempdir");
        let db_path = temp.path().join("memory").join("palace.db");
        std::fs::create_dir_all(db_path.parent().expect("parent")).expect("mkdir");

        let toolkit = open_memtle_toolkit_isolated(db_path.clone())
            .await
            .expect("open palace on isolated stack");
        let defs = toolkit.tool_definitions();
        assert!(
            defs.iter().any(|def| {
                def.get("name")
                    .and_then(|v| v.as_str())
                    .is_some_and(|name| name == "memtle_status")
            }),
            "expected memtle_status in toolkit definitions"
        );
        assert!(db_path.exists(), "palace.db should exist after open");
    }

    #[tokio::test]
    async fn isolated_assemble_warms_hybrid_provider() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().to_path_buf();
        let db_path = workspace.join("memory").join("palace.db");
        std::fs::create_dir_all(db_path.parent().expect("parent")).expect("mkdir");

        let assembled = assemble_mentle_runtime_isolated(db_path, workspace)
            .await
            .expect("assemble palace runtime on isolated stack");
        assert!(
            assembled
                .tool_defs
                .iter()
                .any(|def| def.get("name").and_then(|v| v.as_str()) == Some("memtle_status")),
            "expected memtle_status after assemble"
        );
        // Hybrid provider must be usable without panicking.
        let _ = assembled.memory_provider.as_ref();
    }

    #[tokio::test]
    async fn try_build_activates_full_runtime() {
        let temp = tempdir().expect("tempdir");
        let config = MentleToolRuntimeConfig {
            enabled: true,
            mode: MentleToolMode::Full,
            allowed_tools: Vec::new(),
        };
        let runtime = MentleRuntime::try_build(temp.path(), &config)
            .await
            .expect("active mentle runtime");
        assert!(runtime.active());
        assert!(runtime
            .custom_tools()
            .iter()
            .any(|tool| tool.name() == "memtle_status"));
    }
}

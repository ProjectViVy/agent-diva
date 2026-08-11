//! Tool registry.

use crate::{Tool, ToolError};
use agent_diva_core::audit::{self, AuditEvent};
use agent_diva_core::error_context::{find_problematic_chars, ErrorContext};
use agent_diva_core::security::sanitize_tool_output;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tracing::{error, warn};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolExecutionOutput {
    pub content: String,
    pub char_count: usize,
    pub byte_count: usize,
    pub status: String,
}

/// Stable placement of a tool schema in the provider `tools` array.
///
/// Core schemas form the cache-friendly prefix. Deferred schemas are emitted
/// as a bounded suffix after the model activates them through `tool_search`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolSchemaPartition {
    /// Always-visible, cache-stable tool schemas.
    Core,
    /// MCP, extension, or dynamically activated tool schemas.
    Deferred,
}

/// Canonical provider-facing tool schemas plus the stable CORE prefix length.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolDefinitionSet {
    pub definitions: Vec<Value>,
    pub core_count: usize,
}

/// Maximum number of deferred schemas exposed after one search.
pub const MAX_ACTIVE_DEFERRED_TOOLS: usize = 8;

/// Task-local deferred tool activation.
///
/// This state is intentionally not serializable. The original transcript is
/// durable, while activation is a bounded provider-surface optimization that
/// is reclaimed at the next turn boundary.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActiveDeferredTools {
    names: BTreeSet<String>,
}

impl ActiveDeferredTools {
    pub fn names(&self) -> Vec<String> {
        self.names.iter().cloned().collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn replace<I>(&mut self, names: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.names = names
            .into_iter()
            .filter(|name| !name.trim().is_empty())
            .take(MAX_ACTIVE_DEFERRED_TOOLS)
            .collect();
    }

    pub fn clear(&mut self) {
        self.names.clear();
    }

    fn remove(&mut self, name: &str) {
        self.names.remove(name);
    }
}

/// Shared handle used by all registry instances for one task turn.
pub type ActiveDeferredToolsHandle = Arc<RwLock<ActiveDeferredTools>>;

/// A deterministic result returned by `tool_search`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolSearchResult {
    pub name: String,
    pub description: String,
    pub score: u32,
}

#[derive(Clone)]
pub struct DeferredToolActivationHandle {
    state: ActiveDeferredToolsHandle,
    catalog: Arc<RwLock<BTreeMap<String, CatalogEntry>>>,
}

#[derive(Clone)]
struct CatalogEntry {
    description: String,
    schema_keywords: String,
}

impl DeferredToolActivationHandle {
    /// Create an empty authorized catalog around a shared activation handle.
    pub fn new(state: ActiveDeferredToolsHandle) -> Self {
        Self {
            state,
            catalog: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    /// Create a fresh task-local activation handle.
    pub fn new_state() -> ActiveDeferredToolsHandle {
        Arc::new(RwLock::new(ActiveDeferredTools::default()))
    }

    /// Return the state handle so a runtime can persist or inspect it.
    pub fn state(&self) -> ActiveDeferredToolsHandle {
        Arc::clone(&self.state)
    }

    /// Return a lock-independent snapshot for a same-turn rebuild.
    pub fn snapshot(&self) -> ActiveDeferredTools {
        self.state
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Replace activation during a same-turn rebuild.
    pub fn restore(&self, state: ActiveDeferredTools) {
        *self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = state;
    }

    pub fn active_names(&self) -> Vec<String> {
        self.snapshot().names()
    }

    /// Search the currently authorized deferred catalog and replace the
    /// bounded active set for the next provider call.
    pub fn search(&self, query: &str, limit: usize) -> Vec<ToolSearchResult> {
        let query = query.trim();
        let query_lower = query.to_lowercase();
        let query_tokens = keyword_tokens(query);
        let entries = self
            .catalog
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let mut results = entries
            .into_iter()
            .map(|(name, entry)| {
                let score = score_catalog_entry(
                    &name,
                    &entry.description,
                    &entry.schema_keywords,
                    &query_lower,
                    &query_tokens,
                );
                ToolSearchResult {
                    name,
                    description: entry.description,
                    score,
                }
            })
            .filter(|result| query_lower.is_empty() || result.score > 0)
            .collect::<Vec<_>>();
        results.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.name.cmp(&right.name))
        });
        results.truncate(limit.clamp(1, 20));

        let mut state = self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.replace(results.iter().map(|result| result.name.clone()));
        results
    }

    fn set_catalog_entry(&self, name: String, tool: &dyn Tool) {
        self.catalog
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(
                name,
                CatalogEntry {
                    description: tool.description().to_string(),
                    schema_keywords: schema_keywords(&tool.parameters()),
                },
            );
    }

    fn remove_catalog_entry(&self, name: &str) {
        self.catalog
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(name);
    }

    fn is_active(&self, name: &str) -> bool {
        self.state
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains(name)
    }

    fn remove_active(&self, name: &str) {
        self.state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(name);
    }
}

fn keyword_tokens(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in value.chars() {
        if character.is_alphanumeric() {
            current.extend(character.to_lowercase());
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens.sort();
    tokens.dedup();
    tokens
}

fn schema_keywords(schema: &Value) -> String {
    fn visit(value: &Value, output: &mut Vec<String>) {
        match value {
            Value::Object(object) => {
                for (key, value) in object {
                    output.extend(keyword_tokens(key));
                    visit(value, output);
                }
            }
            Value::Array(values) => {
                for value in values {
                    visit(value, output);
                }
            }
            Value::String(string) => output.extend(keyword_tokens(string)),
            _ => {}
        }
    }

    let mut keywords = Vec::new();
    visit(schema, &mut keywords);
    keywords.sort();
    keywords.dedup();
    keywords.join(" ")
}

fn score_catalog_entry(
    name: &str,
    description: &str,
    schema: &str,
    query_lower: &str,
    query_tokens: &[String],
) -> u32 {
    if query_lower.is_empty() {
        return 0;
    }

    let name_lower = name.to_lowercase();
    let name_tokens = keyword_tokens(name);
    let description_tokens = keyword_tokens(description);
    let schema_tokens = keyword_tokens(schema);
    if name_lower == query_lower {
        return 600;
    }
    if name_lower.starts_with(query_lower) {
        return 500;
    }
    if name_lower.contains(query_lower) {
        return 400;
    }

    let name_hits = query_tokens
        .iter()
        .filter(|token| name_tokens.binary_search(token).is_ok())
        .count();
    if name_hits > 0 {
        return 300 + name_hits.min(99) as u32;
    }
    let description_hits = query_tokens
        .iter()
        .filter(|token| description_tokens.binary_search(token).is_ok())
        .count();
    if description_hits > 0 {
        return 200 + description_hits.min(99) as u32;
    }
    let schema_hits = query_tokens
        .iter()
        .filter(|token| schema_tokens.binary_search(token).is_ok())
        .count();
    if schema_hits > 0 {
        return 100 + schema_hits.min(99) as u32;
    }
    0
}

impl ToolDefinitionSet {
    /// Retain matching schemas while preserving the CORE/DEFERRED boundary.
    pub fn retain(&mut self, mut predicate: impl FnMut(&Value) -> bool) {
        let mut retained_core = 0usize;
        self.definitions = std::mem::take(&mut self.definitions)
            .into_iter()
            .enumerate()
            .filter_map(|(index, definition)| {
                predicate(&definition).then(|| {
                    if index < self.core_count {
                        retained_core += 1;
                    }
                    definition
                })
            })
            .collect();
        self.core_count = retained_core;
    }

    /// Apply a schema transform without changing partition membership.
    pub fn map(mut self, mut transform: impl FnMut(Value) -> Value) -> Self {
        self.definitions = self.definitions.into_iter().map(&mut transform).collect();
        self
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }
}

struct RegisteredTool {
    tool: Arc<dyn Tool>,
    schema_partition: ToolSchemaPartition,
}

/// Registry of available tools.
pub struct ToolRegistry {
    tools: HashMap<String, RegisteredTool>,
    global_timeout_secs: u64,
    activation: DeferredToolActivationHandle,
}

impl ToolRegistry {
    /// Create a new tool registry with default global timeout (120s).
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            global_timeout_secs: 120,
            activation: DeferredToolActivationHandle::new(DeferredToolActivationHandle::new_state()),
        }
    }

    /// Create a new tool registry with a custom global timeout.
    pub fn with_timeout(global_timeout_secs: u64) -> Self {
        Self {
            tools: HashMap::new(),
            global_timeout_secs,
            activation: DeferredToolActivationHandle::new(DeferredToolActivationHandle::new_state()),
        }
    }

    /// Create a registry that reuses one task-local activation state.
    pub fn with_active_deferred_tools(
        global_timeout_secs: u64,
        state: ActiveDeferredToolsHandle,
    ) -> Self {
        Self {
            tools: HashMap::new(),
            global_timeout_secs,
            activation: DeferredToolActivationHandle::new(state),
        }
    }

    /// Return the shared activation handle used by `tool_search`.
    pub fn deferred_tool_activation_handle(&self) -> DeferredToolActivationHandle {
        self.activation.clone()
    }

    /// Return the current task-local activation snapshot.
    pub fn active_deferred_tools_snapshot(&self) -> ActiveDeferredTools {
        self.activation.snapshot()
    }

    /// Restore activation after a same-turn registry rebuild.
    pub fn restore_active_deferred_tools(&self, state: ActiveDeferredTools) {
        self.activation.restore(state);
    }

    /// Search the authorized deferred catalog.
    pub fn search_deferred(&self, query: &str, limit: usize) -> Vec<ToolSearchResult> {
        self.activation.search(query, limit)
    }

    /// Search alias for the authorized deferred catalog.
    pub fn search_catalog(&self, query: &str, limit: usize) -> Vec<ToolSearchResult> {
        self.search_deferred(query, limit)
    }

    /// Return currently active deferred names that are still authorized in
    /// this registry rebuild.
    pub fn active_deferred_tool_names(&self) -> Vec<String> {
        let mut names = self
            .tools
            .iter()
            .filter(|(_, registered)| registered.schema_partition == ToolSchemaPartition::Deferred)
            .filter(|(name, _)| self.activation.is_active(name))
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    /// Return deferred tool handles so an agent rebuilt for a turn can retain
    /// programmatically supplied custom tools while reapplying authorization.
    pub fn deferred_tools(&self) -> Vec<Arc<dyn Tool>> {
        let mut tools = self
            .tools
            .iter()
            .filter(|(_, registered)| registered.schema_partition == ToolSchemaPartition::Deferred)
            .map(|(name, registered)| (name.clone(), Arc::clone(&registered.tool)))
            .collect::<Vec<_>>();
        tools.sort_by(|left, right| left.0.cmp(&right.0));
        tools.into_iter().map(|(_, tool)| tool).collect()
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.register_in_partition(tool, ToolSchemaPartition::Core);
    }

    /// Register a tool in an explicit schema partition.
    pub fn register_in_partition(
        &mut self,
        tool: Arc<dyn Tool>,
        schema_partition: ToolSchemaPartition,
    ) {
        let name = tool.name().to_string();
        self.activation.remove_catalog_entry(&name);
        if schema_partition == ToolSchemaPartition::Deferred {
            self.activation
                .set_catalog_entry(name.clone(), tool.as_ref());
        }
        self.tools.insert(
            name,
            RegisteredTool {
                tool,
                schema_partition,
            },
        );
    }

    /// Unregister a tool by name.
    pub fn unregister(&mut self, name: &str) {
        self.tools.remove(name);
        self.activation.remove_catalog_entry(name);
        self.activation.remove_active(name);
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools
            .get(name)
            .map(|registered| Arc::clone(&registered.tool))
    }

    /// Check if a tool is registered.
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get all tool definitions in OpenAI format.
    pub fn get_definitions(&self) -> Vec<Value> {
        self.get_definition_set().definitions
    }

    /// Get canonical tool definitions with the stable CORE prefix boundary.
    pub fn get_definition_set(&self) -> ToolDefinitionSet {
        let mut tools = self.tools.iter().collect::<Vec<_>>();
        tools.sort_by(|(left_name, left), (right_name, right)| {
            let left_visible = left.schema_partition == ToolSchemaPartition::Core
                || self.activation.is_active(left_name);
            let right_visible = right.schema_partition == ToolSchemaPartition::Core
                || self.activation.is_active(right_name);
            right_visible
                .cmp(&left_visible)
                .then_with(|| left.schema_partition.cmp(&right.schema_partition))
                .then_with(|| left_name.cmp(right_name))
        });

        let core_count = tools
            .iter()
            .take_while(|(_, registered)| registered.schema_partition == ToolSchemaPartition::Core)
            .count();
        let definitions = tools
            .into_iter()
            .filter(|(name, registered)| {
                registered.schema_partition == ToolSchemaPartition::Core
                    || self.activation.is_active(name)
            })
            .map(|(_, registered)| canonicalize_json(registered.tool.to_schema()))
            .collect();
        ToolDefinitionSet {
            definitions,
            core_count,
        }
    }

    /// Execute a tool by name with given parameters.
    pub async fn execute(&self, name: &str, params: Value) -> crate::Result<String> {
        self.execute_structured(name, params)
            .await
            .map(|output| output.content)
    }

    /// Execute without prompt truncation so the caller can persist the complete sanitized result.
    pub async fn execute_structured(
        &self,
        name: &str,
        params: Value,
    ) -> crate::Result<ToolExecutionOutput> {
        let tool = match self.tools.get(name) {
            Some(registered) => &registered.tool,
            None => {
                if self.activation.is_active(name) {
                    audit::emit(AuditEvent::ToolExecuted {
                        tool_name: name.to_string(),
                        duration_ms: 0,
                        result_size: 0,
                        status: "unavailable".to_string(),
                    });
                    return Err(ToolError::ToolUnavailable {
                        name: name.to_string(),
                    });
                }
                audit::emit(AuditEvent::ToolExecuted {
                    tool_name: name.to_string(),
                    duration_ms: 0,
                    result_size: 0,
                    status: "not_found".to_string(),
                });
                let ctx = ErrorContext::new("tool_lookup", format!("Tool '{}' not found", name))
                    .with_metadata("tool_name", name.to_string())
                    .with_metadata("available_tools", self.tool_names().join(", "));
                warn!("{}", ctx.to_detailed_string());
                return Err(ToolError::Error(format!("Tool '{}' not found", name)));
            }
        };

        if self
            .tools
            .get(name)
            .is_some_and(|registered| registered.schema_partition == ToolSchemaPartition::Deferred)
            && !self.activation.is_active(name)
        {
            audit::emit(AuditEvent::ToolExecuted {
                tool_name: name.to_string(),
                duration_ms: 0,
                result_size: 0,
                status: "not_active".to_string(),
            });
            return Err(ToolError::ToolUnavailable {
                name: name.to_string(),
            });
        }

        let errors = tool.validate_params(&params);
        if !errors.is_empty() {
            let validation_message = errors.join("; ");
            audit::emit(AuditEvent::ToolExecuted {
                tool_name: name.to_string(),
                duration_ms: 0,
                result_size: validation_message.len() as u32,
                status: "invalid_params".to_string(),
            });
            let params_str = serde_json::to_string(&params).unwrap_or_default();
            let problems = find_problematic_chars(&params_str);
            let ctx = ErrorContext::new("tool_validation", validation_message.clone())
                .with_content(&params_str)
                .with_metadata("tool_name", name.to_string());
            let ctx_str = ctx.to_detailed_string();
            if problems.is_empty() {
                warn!("{}", ctx_str);
            } else {
                warn!(
                    "{}\n  Problematic characters found:\n    - {}",
                    ctx_str,
                    problems.join("\n    - ")
                );
            }
            return Err(ToolError::InvalidParams(format!(
                "Invalid parameters for tool '{}': {}",
                name, validation_message,
            )));
        }

        let timeout_secs = tool.timeout_secs().unwrap_or(self.global_timeout_secs);
        let start = Instant::now();
        let inner = tool.execute(params.clone());

        let result = tokio::time::timeout(Duration::from_secs(timeout_secs), inner).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(output)) => {
                let sanitized_output = sanitize_tool_output(&output);
                let result_size = sanitized_output.len() as u32;
                audit::emit(AuditEvent::ToolExecuted {
                    tool_name: name.to_string(),
                    duration_ms,
                    result_size,
                    status: "ok".to_string(),
                });
                Ok(ToolExecutionOutput {
                    char_count: sanitized_output.chars().count(),
                    byte_count: sanitized_output.len(),
                    content: sanitized_output,
                    status: "ok".to_string(),
                })
            }
            Ok(Err(tool_err)) => {
                let error_msg = tool_err.to_string();
                let result_size = error_msg.len() as u32;
                audit::emit(AuditEvent::ToolExecuted {
                    tool_name: name.to_string(),
                    duration_ms,
                    result_size,
                    status: "error".to_string(),
                });
                let params_str = serde_json::to_string(&params).unwrap_or_default();
                let problems = find_problematic_chars(&params_str);
                let ctx = ErrorContext::new("tool_execution", error_msg)
                    .with_content(&params_str)
                    .with_metadata("tool_name", name.to_string());
                let ctx_str = ctx.to_detailed_string();
                if problems.is_empty() {
                    error!("{}", ctx_str);
                } else {
                    error!(
                        "{}\n  Problematic characters found:\n    - {}",
                        ctx_str,
                        problems.join("\n    - ")
                    );
                }
                Err(tool_err)
            }
            Err(_) => {
                let error_msg = format!("Tool execution timed out after {}s", timeout_secs);
                audit::emit(AuditEvent::ToolExecuted {
                    tool_name: name.to_string(),
                    duration_ms,
                    result_size: error_msg.len() as u32,
                    status: "error".to_string(),
                });
                let params_str = serde_json::to_string(&params).unwrap_or_default();
                let problems = find_problematic_chars(&params_str);
                let ctx = ErrorContext::new("tool_execution_timeout", error_msg)
                    .with_content(&params_str)
                    .with_metadata("tool_name", name.to_string())
                    .with_metadata("timeout_secs", timeout_secs.to_string());
                let ctx_str = ctx.to_detailed_string();
                if problems.is_empty() {
                    error!("{}", ctx_str);
                } else {
                    error!(
                        "{}\n  Problematic characters found:\n    - {}",
                        ctx_str,
                        problems.join("\n    - ")
                    );
                }
                Err(ToolError::Timeout { secs: timeout_secs })
            }
        }
    }

    /// Get list of registered tool names.
    pub fn tool_names(&self) -> Vec<String> {
        let mut names = self.tools.keys().cloned().collect::<Vec<_>>();
        names.sort();
        names
    }

    /// Get number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

fn canonicalize_json(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut entries = object.into_iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));

            let mut canonical = serde_json::Map::new();
            for (key, value) in entries {
                canonical.insert(key, canonicalize_json(value));
            }
            Value::Object(canonical)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(canonicalize_json).collect()),
        scalar => scalar,
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::security::MAX_TOOL_RESULT_CHARS;
    use async_trait::async_trait;

    struct MockTool;

    struct NamedSchemaTool {
        name: &'static str,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            "mock"
        }

        fn description(&self) -> &str {
            "A mock tool"
        }

        fn parameters(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            })
        }

        async fn execute(&self, _args: Value) -> crate::Result<String> {
            Ok("mock result".to_string())
        }
    }

    #[async_trait]
    impl Tool for NamedSchemaTool {
        fn name(&self) -> &str {
            self.name
        }

        fn description(&self) -> &str {
            "schema ordering fixture"
        }

        fn parameters(&self) -> Value {
            serde_json::json!({
                "required": ["zeta", "alpha"],
                "properties": {
                    "zeta": {"type": "string"},
                    "alpha": {
                        "type": "object",
                        "properties": {
                            "zulu": {"type": "boolean"},
                            "alpha": {"type": "boolean"}
                        }
                    }
                },
                "type": "object"
            })
        }

        async fn execute(&self, _args: Value) -> crate::Result<String> {
            Ok("ok".to_string())
        }
    }

    /// A mock tool that requires a `name` parameter.
    struct MockParamTool;

    #[async_trait]
    impl Tool for MockParamTool {
        fn name(&self) -> &str {
            "mock_params"
        }

        fn description(&self) -> &str {
            "A mock tool with required params"
        }

        fn parameters(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"]
            })
        }

        async fn execute(&self, _args: Value) -> crate::Result<String> {
            Ok("mock param result".to_string())
        }
    }

    /// A mock tool that always fails.
    struct MockFailingTool;

    #[async_trait]
    impl Tool for MockFailingTool {
        fn name(&self) -> &str {
            "mock_fail"
        }

        fn description(&self) -> &str {
            "A mock tool that always fails"
        }

        fn parameters(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            })
        }

        async fn execute(&self, _args: Value) -> crate::Result<String> {
            Err(ToolError::ExecutionFailed("mock failure".to_string()))
        }
    }

    #[test]
    fn test_register_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool));
        assert_eq!(registry.len(), 1);
        assert!(registry.has("mock"));
    }

    #[test]
    fn tool_definitions_are_partitioned_sorted_and_byte_stable() {
        fn register_fixture(registry: &mut ToolRegistry, reverse: bool) {
            let fixtures = [
                ("zeta_core", ToolSchemaPartition::Core),
                ("alpha_deferred", ToolSchemaPartition::Deferred),
                ("alpha_core", ToolSchemaPartition::Core),
                ("zeta_deferred", ToolSchemaPartition::Deferred),
            ];
            let indices: &[usize] = if reverse {
                &[3, 2, 1, 0]
            } else {
                &[0, 1, 2, 3]
            };

            for index in indices {
                let (name, partition) = fixtures[*index];
                registry.register_in_partition(Arc::new(NamedSchemaTool { name }), partition);
            }
        }

        let mut first = ToolRegistry::new();
        register_fixture(&mut first, false);
        first.search_deferred("", 20);
        let first_definitions = first.get_definitions();
        assert_eq!(first.get_definition_set().core_count, 2);
        let first_names = first_definitions
            .iter()
            .filter_map(|definition| definition["function"]["name"].as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            first_names,
            vec!["alpha_core", "zeta_core", "alpha_deferred", "zeta_deferred"]
        );

        let first_bytes = serde_json::to_vec(&first_definitions).unwrap();
        assert_eq!(
            first_bytes,
            serde_json::to_vec(&first.get_definitions()).unwrap()
        );

        let mut second = ToolRegistry::new();
        register_fixture(&mut second, true);
        second.search_deferred("", 20);
        assert_eq!(
            first_bytes,
            serde_json::to_vec(&second.get_definitions()).unwrap()
        );
    }

    #[test]
    fn retained_definition_set_recomputes_core_boundary() {
        let mut registry = ToolRegistry::new();
        registry.register_in_partition(
            Arc::new(NamedSchemaTool { name: "alpha" }),
            ToolSchemaPartition::Core,
        );
        registry.register_in_partition(
            Arc::new(NamedSchemaTool { name: "beta" }),
            ToolSchemaPartition::Core,
        );
        registry.register_in_partition(
            Arc::new(NamedSchemaTool { name: "gamma" }),
            ToolSchemaPartition::Deferred,
        );

        registry.search_deferred("gamma", 8);
        let mut set = registry.get_definition_set();
        set.retain(|definition| definition["function"]["name"] != "alpha");
        assert_eq!(set.core_count, 1);
        assert_eq!(set.definitions.len(), 2);
        assert_eq!(set.definitions[0]["function"]["name"], "beta");
        assert_eq!(set.definitions[1]["function"]["name"], "gamma");
    }

    #[test]
    fn tool_schema_canonicalization_sorts_objects_but_preserves_arrays() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(NamedSchemaTool { name: "schema" }));

        let definitions = registry.get_definitions();
        let parameters = &definitions[0]["function"]["parameters"];
        assert_eq!(parameters["required"], serde_json::json!(["zeta", "alpha"]));

        let serialized = serde_json::to_string(parameters).unwrap();
        assert!(serialized.find("\"alpha\"").unwrap() < serialized.find("\"zeta\"").unwrap());

        let nested = serde_json::to_string(&parameters["properties"]["alpha"]).unwrap();
        assert!(nested.find("\"alpha\"").unwrap() < nested.find("\"zulu\"").unwrap());
    }

    #[test]
    fn test_unregister_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool));
        registry.unregister("mock");
        assert_eq!(registry.len(), 0);
        assert!(!registry.has("mock"));
    }

    #[tokio::test]
    async fn deferred_execution_errors_are_stable_and_source_removal_is_unavailable() {
        let mut registry = ToolRegistry::new();
        registry.register_in_partition(
            Arc::new(NamedSchemaTool {
                name: "deferred_fixture",
            }),
            ToolSchemaPartition::Deferred,
        );

        assert!(matches!(
            registry
                .execute("deferred_fixture", serde_json::json!({}))
                .await,
            Err(ToolError::ToolUnavailable { .. })
        ));
        registry.search_deferred("fixture", 8);
        let valid_arguments = serde_json::json!({
            "zeta": true,
            "alpha": {"zulu": true, "alpha": true}
        });
        assert_eq!(
            registry
                .execute("deferred_fixture", valid_arguments.clone())
                .await
                .unwrap(),
            "ok"
        );

        registry.unregister("deferred_fixture");
        assert!(matches!(
            registry
                .execute("deferred_fixture", serde_json::json!({}))
                .await,
            Err(ToolError::Error(_))
        ));
        assert!(matches!(
            registry
                .execute("unknown_fixture", serde_json::json!({}))
                .await,
            Err(ToolError::Error(_))
        ));
    }

    #[test]
    fn discovery_search_is_case_insensitive_bounded_and_deterministic() {
        let mut registry = ToolRegistry::new();
        for name in ["calendar_lookup", "calendar_create", "weather_read"] {
            registry.register_in_partition(
                Arc::new(NamedSchemaTool { name }),
                ToolSchemaPartition::Deferred,
            );
        }

        let first = registry.search_deferred("CALENDAR", 1);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].score, 500);
        assert_eq!(first[0].name, "calendar_create");
        assert!(first[0].description.contains("schema"));
        assert_eq!(
            registry.active_deferred_tools_snapshot().names(),
            vec!["calendar_create"]
        );

        let second = registry.search_deferred("does-not-exist", 20);
        assert!(second.is_empty());
        let empty = registry.search_deferred("", 20);
        assert_eq!(
            empty
                .iter()
                .map(|result| result.name.as_str())
                .collect::<Vec<_>>(),
            vec!["calendar_create", "calendar_lookup", "weather_read"]
        );
    }

    #[test]
    fn search_replaces_activation_and_caps_it_at_eight_tools() {
        let mut registry = ToolRegistry::new();
        for index in 0..10 {
            let name: &'static str = Box::leak(format!("deferred_{index}").into_boxed_str());
            registry.register_in_partition(
                Arc::new(NamedSchemaTool { name }),
                ToolSchemaPartition::Deferred,
            );
        }

        let results = registry.search_deferred("", 20);
        assert_eq!(results.len(), 10);
        assert_eq!(registry.active_deferred_tools_snapshot().len(), 8);

        registry.search_deferred("9", 20);
        assert_eq!(
            registry.active_deferred_tools_snapshot().names(),
            vec!["deferred_9"]
        );
    }

    #[test]
    fn active_deferred_tools_are_isolated_and_restorable_across_rebuilds() {
        let state = DeferredToolActivationHandle::new_state();
        let mut first = ToolRegistry::with_active_deferred_tools(120, state.clone());
        first.register_in_partition(
            Arc::new(NamedSchemaTool {
                name: "persisted_tool",
            }),
            ToolSchemaPartition::Deferred,
        );
        first.search_deferred("persisted", 8);

        let mut rebuilt = ToolRegistry::with_active_deferred_tools(120, state);
        rebuilt.register_in_partition(
            Arc::new(NamedSchemaTool {
                name: "persisted_tool",
            }),
            ToolSchemaPartition::Deferred,
        );
        assert!(rebuilt
            .get_definitions()
            .iter()
            .any(|definition| definition["function"]["name"] == "persisted_tool"));

        let isolated = ToolRegistry::new();
        assert!(!isolated
            .active_deferred_tools_snapshot()
            .contains("persisted_tool"));
        assert!(rebuilt
            .active_deferred_tools_snapshot()
            .contains("persisted_tool"));
    }

    #[tokio::test]
    async fn test_execute_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool));
        let result = registry.execute("mock", serde_json::json!({})).await;
        assert_eq!(result.unwrap(), "mock result");
    }

    #[tokio::test]
    async fn test_execute_unknown_tool() {
        let registry = ToolRegistry::new();
        let result = registry.execute("nonexistent", serde_json::json!({})).await;
        assert!(result.is_err());
        match result {
            Err(ToolError::Error(msg)) => {
                assert!(msg.contains("Tool 'nonexistent' not found"));
            }
            _ => panic!("Expected ToolError::Error"),
        }
    }

    #[tokio::test]
    async fn test_execute_invalid_params_returns_err() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockParamTool));
        let result = registry.execute("mock_params", serde_json::json!({})).await;
        assert!(result.is_err());
        match result {
            Err(ToolError::InvalidParams(msg)) => {
                assert!(msg.contains("Invalid parameters for tool 'mock_params'"));
            }
            _ => panic!("Expected ToolError::InvalidParams"),
        }
    }

    #[tokio::test]
    async fn test_execute_tool_failure_returns_err() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockFailingTool));
        let result = registry.execute("mock_fail", serde_json::json!({})).await;
        assert!(result.is_err());
        match result {
            Err(ToolError::ExecutionFailed(msg)) => {
                assert_eq!(msg, "mock failure");
            }
            _ => panic!("Expected ToolError::ExecutionFailed"),
        }
    }

    #[tokio::test]
    async fn test_execute_success_no_truncation() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool));
        let result = registry.execute("mock", serde_json::json!({})).await;
        assert_eq!(result.unwrap(), "mock result");
    }

    #[tokio::test]
    async fn test_execute_success_preserves_complete_sanitized_output() {
        let mut registry = ToolRegistry::new();
        let large_tool = MockLargeOutputTool;
        registry.register(Arc::new(large_tool));
        let result = registry.execute("mock_large", serde_json::json!({})).await;
        let output = result.unwrap();
        assert!(output.chars().count() >= MAX_TOOL_RESULT_CHARS + 5_000);
        assert!(!output.contains("Result truncated"));
        assert!(!output.contains("Result truncated"));
    }

    /// A mock tool that produces output exceeding MAX_TOOL_RESULT_CHARS.
    struct MockLargeOutputTool;

    #[async_trait]
    impl Tool for MockLargeOutputTool {
        fn name(&self) -> &str {
            "mock_large"
        }

        fn description(&self) -> &str {
            "A mock tool with oversized output"
        }

        fn parameters(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            })
        }

        async fn execute(&self, _args: Value) -> crate::Result<String> {
            Ok("x".repeat(MAX_TOOL_RESULT_CHARS + 5000))
        }
    }

    // ------------------------------------------------------------------
    //  Global timeout tests
    // ------------------------------------------------------------------

    /// A mock tool that sleeps for a long time (simulates slow execution).
    struct MockSlowTool;

    #[async_trait]
    impl Tool for MockSlowTool {
        fn name(&self) -> &str {
            "mock_slow"
        }

        fn description(&self) -> &str {
            "A mock tool that sleeps"
        }

        fn parameters(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            })
        }

        async fn execute(&self, _args: Value) -> crate::Result<String> {
            tokio::time::sleep(Duration::from_secs(300)).await;
            Ok("should not reach here".to_string())
        }
    }

    /// Global timeout fires when a tool takes too long.
    #[tokio::test]
    async fn test_global_timeout_fires() {
        let mut registry = ToolRegistry::with_timeout(1);
        registry.register(Arc::new(MockSlowTool));
        let result = registry.execute("mock_slow", serde_json::json!({})).await;
        match result {
            Err(ToolError::Timeout { secs }) => {
                assert_eq!(secs, 1);
            }
            other => panic!("Expected ToolError::Timeout, got: {:?}", other),
        }
    }

    /// Normal execution completes within the global timeout.
    #[tokio::test]
    async fn test_global_timeout_normal_execution() {
        let mut registry = ToolRegistry::with_timeout(120);
        registry.register(Arc::new(MockTool));
        let result = registry.execute("mock", serde_json::json!({})).await;
        assert_eq!(result.unwrap(), "mock result");
    }

    /// Default registry has 120s timeout — fast tools still work.
    #[tokio::test]
    async fn test_default_registry_timeout() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool));
        let result = registry.execute("mock", serde_json::json!({})).await;
        assert_eq!(result.unwrap(), "mock result");
    }

    // ------------------------------------------------------------------
    //  ToolExecutionTap audit tests
    // ------------------------------------------------------------------

    use std::sync::{Arc as StdArc, Mutex};
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

    /// A tracing layer that captures audit events by their type name.
    struct AuditCaptureLayer {
        events: StdArc<Mutex<Vec<String>>>,
    }

    impl<S> Layer<S> for AuditCaptureLayer
    where
        S: tracing::Subscriber,
    {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            if event.metadata().target() == "audit" {
                let mut visitor = AuditEventVisitor::default();
                event.record(&mut visitor);
                if let Some(type_name) = visitor.type_name {
                    self.events.lock().unwrap().push(type_name);
                }
            }
        }
    }

    #[derive(Default)]
    struct AuditEventVisitor {
        type_name: Option<String>,
    }

    impl tracing::field::Visit for AuditEventVisitor {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "event" {
                let s = format!("{:?}", value);
                // Extract the variant name from the debug output, e.g.:
                // ToolExecuted { tool_name: "...", ... }
                if let Some(end) = s.find(" {") {
                    self.type_name = Some(s[..end].to_string());
                } else if let Some(end) = s.find("(") {
                    self.type_name = Some(s[..end].to_string());
                } else {
                    self.type_name = Some(s);
                }
            }
        }
    }

    /// Helper to run an async test with an audit-capture layer installed.
    async fn with_audit_capture<F, Fut>(f: F) -> Vec<String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let events = StdArc::new(Mutex::new(Vec::new()));
        let layer = AuditCaptureLayer {
            events: events.clone(),
        };

        let subscriber = tracing_subscriber::registry().with(layer);
        let _guard = subscriber.set_default();

        f().await;

        drop(_guard);
        let result = events.lock().unwrap().clone();
        result
    }

    #[tokio::test]
    async fn test_tool_executed_audit_event_on_success() {
        let captured = with_audit_capture(|| async {
            let mut registry = ToolRegistry::new();
            registry.register(std::sync::Arc::new(MockTool));
            let _ = registry.execute("mock", serde_json::json!({})).await;
        })
        .await;

        let tool_events: Vec<_> = captured
            .iter()
            .filter(|e| e == &&"ToolExecuted".to_string())
            .collect();

        assert_eq!(
            tool_events.len(),
            1,
            "expected exactly one ToolExecuted audit event, got: {:?}",
            captured
        );
    }

    #[tokio::test]
    async fn test_tool_executed_audit_event_on_failure() {
        let captured = with_audit_capture(|| async {
            let mut registry = ToolRegistry::new();
            registry.register(std::sync::Arc::new(MockFailingTool));
            let _ = registry.execute("mock_fail", serde_json::json!({})).await;
        })
        .await;

        let tool_events: Vec<_> = captured
            .iter()
            .filter(|e| e == &&"ToolExecuted".to_string())
            .collect();

        assert_eq!(
            tool_events.len(),
            1,
            "expected exactly one ToolExecuted audit event, got: {:?}",
            captured
        );
    }

    #[tokio::test]
    async fn test_tool_executed_audit_event_on_timeout() {
        let captured = with_audit_capture(|| async {
            let mut registry = ToolRegistry::with_timeout(1);
            registry.register(std::sync::Arc::new(MockSlowTool));
            let _ = registry.execute("mock_slow", serde_json::json!({})).await;
        })
        .await;

        let tool_events: Vec<_> = captured
            .iter()
            .filter(|e| e == &&"ToolExecuted".to_string())
            .collect();

        assert_eq!(
            tool_events.len(),
            1,
            "expected exactly one ToolExecuted audit event on timeout, got: {:?}",
            captured
        );
    }
}

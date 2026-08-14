//! Persona authority tools backed by the config-rooted Markdown workspace.

use std::path::PathBuf;
use std::str::FromStr;

use agent_diva_laputa::persona::{PersonaError, PersonaKind, PersonaRequestActor, PersonaService};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

fn service(config_dir: &Option<PathBuf>) -> Result<PersonaService, ToolError> {
    let Some(config_dir) = config_dir else {
        return Err(ToolError::ExecutionFailed(
            "persona_unavailable: Persona config root is unavailable".to_string(),
        ));
    };
    PersonaService::open(config_dir)
        .map_err(|error| ToolError::ExecutionFailed(format!("{}: {error}", error.code())))
}

fn map_error(error: PersonaError) -> ToolError {
    ToolError::ExecutionFailed(format!("{}: {error}", error.code()))
}

fn parse_kind(value: &str) -> Result<PersonaKind, ToolError> {
    PersonaKind::from_str(value)
        .map_err(|error| ToolError::InvalidArguments(format!("{}: {error}", error.code())))
}

/// Read the user-owned WORLD authority without exposing filesystem paths.
pub struct WorldReadTool {
    config_dir: Option<PathBuf>,
}

impl WorldReadTool {
    pub fn new() -> Self {
        Self { config_dir: None }
    }

    pub fn with_config_dir(config_dir: PathBuf) -> Self {
        Self {
            config_dir: Some(config_dir),
        }
    }
}

impl Default for WorldReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WorldReadTool {
    fn name(&self) -> &str {
        "world_read"
    }

    fn description(&self) -> &str {
        "Read the current user-owned WORLD Persona authority, including its revision and content hash."
    }

    fn parameters(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        if !args.is_object() {
            return Err(ToolError::InvalidArguments(
                "arguments must be an object".to_string(),
            ));
        }
        let document = service(&self.config_dir)?
            .get_document(PersonaKind::World)
            .map_err(map_error)?;
        serde_json::to_string(&document)
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))
    }
}

/// Read one Persona authority document by registered kind.
pub struct PersonaReadTool {
    config_dir: Option<PathBuf>,
}

impl PersonaReadTool {
    pub fn new() -> Self {
        Self { config_dir: None }
    }

    pub fn with_config_dir(config_dir: PathBuf) -> Self {
        Self {
            config_dir: Some(config_dir),
        }
    }
}

impl Default for PersonaReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct PersonaReadArgs {
    kind: String,
}

#[async_trait]
impl Tool for PersonaReadTool {
    fn name(&self) -> &str {
        "persona_read"
    }

    fn description(&self) -> &str {
        "Read a registered Persona Markdown authority document and its CAS metadata. Use world_read for WORLD when only world context is needed."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "kind": {
                    "type": "string",
                    "enum": ["identity", "relationship", "redline", "user", "world", "dream", "dark"]
                }
            },
            "required": ["kind"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let args: PersonaReadArgs = serde_json::from_value(args)
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
        let document = service(&self.config_dir)?
            .get_document(parse_kind(&args.kind)?)
            .map_err(map_error)?;
        serde_json::to_string(&document)
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))
    }
}

/// Submit a governed P5 change request for a user-reviewable Persona authority.
pub struct PersonaRequestTool {
    config_dir: Option<PathBuf>,
}

impl PersonaRequestTool {
    pub fn new() -> Self {
        Self { config_dir: None }
    }

    pub fn with_config_dir(config_dir: PathBuf) -> Self {
        Self {
            config_dir: Some(config_dir),
        }
    }
}

impl Default for PersonaRequestTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct PersonaRequestArgs {
    kind: String,
    base_revision: u64,
    base_hash: String,
    proposed_markdown: String,
    reason: String,
}

#[async_trait]
impl Tool for PersonaRequestTool {
    fn name(&self) -> &str {
        "persona_request"
    }

    fn description(&self) -> &str {
        "Create one governed Persona change request for user review. The base revision and hash must come from persona_read or world_read; this tool never applies the proposal directly."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "kind": {
                    "type": "string",
                    "enum": ["identity", "relationship", "redline", "user", "world"]
                },
                "base_revision": {"type": "integer", "minimum": 1},
                "base_hash": {"type": "string"},
                "proposed_markdown": {"type": "string"},
                "reason": {"type": "string"}
            },
            "required": ["kind", "base_revision", "base_hash", "proposed_markdown", "reason"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let args: PersonaRequestArgs = serde_json::from_value(args)
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
        let request = service(&self.config_dir)?
            .create_request(
                parse_kind(&args.kind)?,
                args.base_revision,
                &args.base_hash,
                &args.proposed_markdown,
                PersonaRequestActor::Agent,
                &args.reason,
            )
            .map_err(map_error)?;
        serde_json::to_string(&request)
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))
    }
}

/// Apply the narrow P16 agent-owned write scopes directly.
pub struct PersonaUpdateTool {
    config_dir: Option<PathBuf>,
}

impl PersonaUpdateTool {
    pub fn new() -> Self {
        Self { config_dir: None }
    }

    pub fn with_config_dir(config_dir: PathBuf) -> Self {
        Self {
            config_dir: Some(config_dir),
        }
    }
}

impl Default for PersonaUpdateTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct PersonaUpdateArgs {
    kind: String,
    content: String,
    reason: String,
}

#[async_trait]
impl Tool for PersonaUpdateTool {
    fn name(&self) -> &str {
        "persona_update"
    }

    fn description(&self) -> &str {
        "Directly update only the agent-owned P16 scopes: DREAM, DARK, or the Observations section of USER. All other Persona writes require persona_request."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "kind": {"type": "string", "enum": ["dream", "dark", "user"]},
                "content": {"type": "string"},
                "reason": {"type": "string"}
            },
            "required": ["kind", "content", "reason"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let args: PersonaUpdateArgs = serde_json::from_value(args)
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
        let outcome = service(&self.config_dir)?
            .save_agent_p16(parse_kind(&args.kind)?, &args.content, &args.reason)
            .map_err(map_error)?;
        serde_json::to_string(&outcome)
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_laputa::persona::PersonaInitialization;

    fn initialized_tool_root() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        PersonaService::open(root.path())
            .unwrap()
            .initialize(PersonaInitialization {
                identity: "Identity".to_string(),
                relationship: "Relationship".to_string(),
                redline: "Redline".to_string(),
                user: "Likes concise answers".to_string(),
                world: "The user owns WORLD.".to_string(),
            })
            .unwrap();
        root
    }

    #[tokio::test]
    async fn world_read_uses_config_root_authority() {
        let root = initialized_tool_root();
        let value: Value = serde_json::from_str(
            &WorldReadTool::with_config_dir(root.path().to_path_buf())
                .execute(json!({}))
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(value["kind"], "world");
        assert_eq!(value["content"], "The user owns WORLD.");
    }

    #[tokio::test]
    async fn request_rejects_agent_owned_kind() {
        let root = initialized_tool_root();
        let document = PersonaService::open(root.path())
            .unwrap()
            .get_document(PersonaKind::Dream)
            .unwrap();
        let error = PersonaRequestTool::with_config_dir(root.path().to_path_buf())
            .execute(json!({
                "kind": "dream",
                "base_revision": document.revision,
                "base_hash": document.content_hash,
                "proposed_markdown": "dream",
                "reason": "test"
            }))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("persona_kind_forbidden"));
    }

    #[tokio::test]
    async fn update_rejects_governed_kind() {
        let root = initialized_tool_root();
        let error = PersonaUpdateTool::with_config_dir(root.path().to_path_buf())
            .execute(json!({"kind": "identity", "content": "new", "reason": "test"}))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("persona_kind_forbidden"));
    }

    #[tokio::test]
    async fn missing_config_root_is_explicit() {
        let error = WorldReadTool::new().execute(json!({})).await.unwrap_err();
        assert!(error.to_string().contains("persona_unavailable"));
    }
}

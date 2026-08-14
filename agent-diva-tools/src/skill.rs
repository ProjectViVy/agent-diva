//! Read-only access to enabled machine-wide Skills.

use std::path::PathBuf;

use agent_diva_core::evolution::{SkillHome, SkillHomeError};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

pub struct SkillReadTool {
    config_dir: Option<PathBuf>,
    builtin_dir: PathBuf,
}

impl SkillReadTool {
    pub fn new() -> Self {
        Self {
            config_dir: None,
            builtin_dir: default_builtin_dir(),
        }
    }

    pub fn with_config_dir(config_dir: PathBuf) -> Self {
        Self {
            config_dir: Some(config_dir),
            builtin_dir: default_builtin_dir(),
        }
    }

    #[cfg(test)]
    fn with_roots(config_dir: PathBuf, builtin_dir: PathBuf) -> Self {
        Self {
            config_dir: Some(config_dir),
            builtin_dir,
        }
    }
}

impl Default for SkillReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct SkillReadArgs {
    slug: String,
}

#[async_trait]
impl Tool for SkillReadTool {
    fn name(&self) -> &str {
        "skill_read"
    }

    fn description(&self) -> &str {
        "Read the complete Markdown for one enabled Skill by slug. This is the only runtime Skill file reader and never exposes disk paths."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {"slug": {"type": "string"}},
            "required": ["slug"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let args: SkillReadArgs = serde_json::from_value(args)
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
        let Some(config_dir) = &self.config_dir else {
            return Err(ToolError::ExecutionFailed(
                "skill_unavailable: machine config root is unavailable".into(),
            ));
        };
        let document = SkillHome::new(config_dir, self.builtin_dir.clone())
            .read_enabled(&args.slug)
            .map_err(map_error)?;
        serde_json::to_string(&json!({
            "slug": document.summary.slug,
            "description": document.summary.description,
            "content_hash": document.summary.content_hash,
            "markdown": document.markdown,
        }))
        .map_err(|error| ToolError::ExecutionFailed(error.to_string()))
    }
}

fn default_builtin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("skills")
}

fn map_error(error: SkillHomeError) -> ToolError {
    ToolError::ExecutionFailed(format!("{}: {error}", error.code()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn reads_enabled_home_without_returning_path() {
        let config = tempfile::tempdir().unwrap();
        let builtin = tempfile::tempdir().unwrap();
        fs::create_dir_all(config.path().join("skills/demo")).unwrap();
        fs::write(
            config.path().join("skills/demo/SKILL.md"),
            "---\ndescription: Demo\nenabled: true\n---\nInstructions",
        )
        .unwrap();
        let output =
            SkillReadTool::with_roots(config.path().to_path_buf(), builtin.path().to_path_buf())
                .execute(json!({"slug": "demo"}))
                .await
                .unwrap();
        assert!(output.contains("Instructions"));
        assert!(!output.contains(config.path().to_string_lossy().as_ref()));
    }

    #[tokio::test]
    async fn disabled_skill_is_not_readable() {
        let config = tempfile::tempdir().unwrap();
        let builtin = tempfile::tempdir().unwrap();
        fs::create_dir_all(config.path().join("skills/demo")).unwrap();
        fs::write(
            config.path().join("skills/demo/SKILL.md"),
            "---\ndescription: Demo\nenabled: false\n---\nInstructions",
        )
        .unwrap();
        let error =
            SkillReadTool::with_roots(config.path().to_path_buf(), builtin.path().to_path_buf())
                .execute(json!({"slug": "demo"}))
                .await
                .unwrap_err();
        assert!(error.to_string().contains("skill_not_found"));
    }
}

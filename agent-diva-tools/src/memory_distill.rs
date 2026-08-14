//! `memory_distill`: create a governed Skill request, never a Skill head.

use std::path::PathBuf;

use agent_diva_core::evolution::{
    CreateSkillProposal, SkillEvidence, SkillHome, SkillProposalSource,
};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

pub struct MemoryDistillTool {
    config_dir: Option<PathBuf>,
    builtin_dir: PathBuf,
    session_key: Option<String>,
}

impl MemoryDistillTool {
    pub fn new() -> Self {
        Self {
            config_dir: None,
            builtin_dir: default_builtin_dir(),
            session_key: None,
        }
    }

    pub fn with_config_dir(config_dir: PathBuf, session_key: Option<String>) -> Self {
        Self {
            config_dir: Some(config_dir),
            builtin_dir: default_builtin_dir(),
            session_key,
        }
    }

    #[cfg(test)]
    fn with_roots(config_dir: PathBuf, builtin_dir: PathBuf, session_key: String) -> Self {
        Self {
            config_dir: Some(config_dir),
            builtin_dir,
            session_key: Some(session_key),
        }
    }
}

impl Default for MemoryDistillTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct DistillArgs {
    slug: String,
    title: String,
    description: String,
    content: String,
    reason: String,
    #[serde(default)]
    actmem_pointer: Option<String>,
    #[serde(default)]
    artifact: Option<String>,
}

#[async_trait]
impl Tool for MemoryDistillTool {
    fn name(&self) -> &str {
        "memory_distill"
    }

    fn description(&self) -> &str {
        "Propose reusable, action-backed task experience as a complete Skill for user review. This tool creates a pending request only; it never writes or enables a Skill."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "slug": {"type": "string", "description": "kebab-case Skill slug"},
                "title": {"type": "string"},
                "description": {"type": "string", "description": "single-line Skill index description"},
                "content": {"type": "string", "description": "reusable Markdown instructions"},
                "reason": {"type": "string"},
                "actmem_pointer": {"type": "string"},
                "artifact": {"type": "string"}
            },
            "required": ["slug", "title", "description", "content", "reason"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let args: DistillArgs = serde_json::from_value(args)
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
        let Some(config_dir) = &self.config_dir else {
            return Ok(json!({"status": "failed", "code": "skill_unavailable"}).to_string());
        };
        let Some(session_key) = self
            .session_key
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        else {
            return Ok(json!({"status": "failed", "code": "skill_evidence_required"}).to_string());
        };
        let home = SkillHome::new(config_dir, self.builtin_dir.clone());
        let base_hash = home
            .read(&args.slug)
            .map(|document| document.summary.content_hash)
            .unwrap_or_else(|_| "0".into());
        let markdown = build_markdown(&args)?;
        match home.create_request(CreateSkillProposal {
            slug: args.slug,
            title: args.title,
            proposed_markdown: markdown,
            evidence: vec![SkillEvidence {
                session_key: Some(session_key.to_string()),
                actmem_pointer: args.actmem_pointer,
                autodream_run_id: None,
                tool: Some("memory_distill".into()),
                artifact: args.artifact,
            }],
            attestation: None,
            base_hash,
            source: SkillProposalSource::Distill,
            reason: args.reason,
        }) {
            Ok(request) => {
                crate::distill_guard::mark_distill_ran();
                Ok(json!({"status": "request_created", "request_id": request.id}).to_string())
            }
            Err(error) => Ok(json!({"status": "failed", "code": error.code()}).to_string()),
        }
    }
}

fn build_markdown(args: &DistillArgs) -> Result<String, ToolError> {
    let description = serde_yaml::to_string(&args.description)
        .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
    let description = description.trim_end_matches("\n").trim_end_matches("...");
    Ok(format!(
        "---\nname: {}\ndescription: {}\nenabled: true\nalways: false\n---\n{}",
        args.slug,
        description,
        args.content.trim()
    ))
}

fn default_builtin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("skills")
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::evolution::SkillProposalStatus;

    fn args() -> Value {
        json!({
            "slug": "retry-http",
            "title": "Retry HTTP",
            "description": "Bounded retry for idempotent HTTP calls",
            "content": "## Steps\n\nRetry only verified idempotent requests.",
            "reason": "Repeated successful workflow"
        })
    }

    #[tokio::test]
    async fn creates_request_with_session_evidence_without_writing_skill() {
        let config = tempfile::tempdir().unwrap();
        let builtin = tempfile::tempdir().unwrap();
        let output = MemoryDistillTool::with_roots(
            config.path().to_path_buf(),
            builtin.path().to_path_buf(),
            "session-1".into(),
        )
        .execute(args())
        .await
        .unwrap();
        assert!(output.contains("request_created"));
        assert!(!config.path().join("skills/retry-http/SKILL.md").exists());
        let requests = SkillHome::new(config.path(), builtin.path())
            .list_requests()
            .unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].status, SkillProposalStatus::Pending);
        assert_eq!(
            requests[0].evidence[0].session_key.as_deref(),
            Some("session-1")
        );
    }

    #[tokio::test]
    async fn unavailable_tool_has_bounded_failed_result() {
        let output = MemoryDistillTool::new().execute(args()).await.unwrap();
        assert_eq!(output, r#"{"code":"skill_unavailable","status":"failed"}"#);
    }
}

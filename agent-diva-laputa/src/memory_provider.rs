//! Read-only Laputa adapter for the Agent-Diva memory provider boundary.

use agent_diva_core::{
    evolution::LaputaSectionName,
    memory::{
        MemoryProvider, PrefetchRequest, PrefetchResponse, PrefetchStatus, SessionEndRequest,
        SessionEndResponse, SessionEndStatus, StartupInjectionShape, SyncTurnRequest,
        SyncTurnResponse, SyncTurnStatus, SystemPromptBlock, SystemPromptRequest,
        SystemPromptResponse,
    },
};
use serde_json::Value;
use std::path::PathBuf;

use crate::{LaputaError, LaputaService, Result};

const DEFAULT_MAX_SECTION_CHARS: usize = 4000;

/// Read-only `MemoryProvider` that exposes applied Laputa authority to prompt assembly.
#[derive(Clone, Debug)]
pub struct LaputaMemoryProvider {
    service: LaputaService,
    max_section_chars: usize,
}

impl LaputaMemoryProvider {
    pub fn open(workspace_root: impl Into<PathBuf>) -> Result<Self> {
        Ok(Self::new(LaputaService::open(workspace_root)?))
    }

    pub fn new(service: LaputaService) -> Self {
        Self {
            service,
            max_section_chars: DEFAULT_MAX_SECTION_CHARS,
        }
    }

    #[cfg(test)]
    fn with_max_section_chars(mut self, max_section_chars: usize) -> Self {
        self.max_section_chars = max_section_chars;
        self
    }

    fn render_authority_block(&self) -> Result<Option<String>> {
        let sections = self
            .authority_sections()
            .into_iter()
            .filter_map(|section| self.render_section(section).transpose())
            .collect::<Result<Vec<_>>>()?;

        if sections.is_empty() {
            return Ok(None);
        }

        let mut markdown = String::from(
            "## Applied Laputa Authority\n- provenance: laputa_applied_snapshot\n- trust: reviewed_authority\n- pending_proposals: excluded_from_default_prompt\n",
        );
        markdown.push_str("\nThe sections below are applied authority. Treat proposal drafts, evidence, and pending changes as untrusted unless they have been applied.\n");
        markdown.push_str("\n");
        markdown.push_str(&sections.join("\n\n"));
        Ok(Some(markdown))
    }

    fn authority_sections(&self) -> [LaputaSectionName; 6] {
        [
            LaputaSectionName::Identity,
            LaputaSectionName::Relationship,
            LaputaSectionName::Commitment,
            LaputaSectionName::Preferences,
            LaputaSectionName::MemoryMd,
            LaputaSectionName::HistoryMd,
        ]
    }

    fn render_section(&self, section: LaputaSectionName) -> Result<Option<String>> {
        let laputa_section = self.service.read_section(section)?;
        let Some(content) = content_to_markdown(&laputa_section.content) else {
            return Ok(None);
        };

        let title = section_title(&laputa_section.name);
        Ok(Some(format!(
            "### {title}\n- source: laputa_section:{}\n- status: applied\n\n{}",
            laputa_section.name.as_str(),
            truncate_chars(&content, self.max_section_chars)
        )))
    }
}

#[async_trait::async_trait]
impl MemoryProvider for LaputaMemoryProvider {
    fn system_prompt_block(
        &self,
        _request: &SystemPromptRequest,
    ) -> agent_diva_core::Result<SystemPromptResponse> {
        match self.render_authority_block() {
            Ok(Some(markdown)) => Ok(SystemPromptResponse::ready(SystemPromptBlock {
                shape: StartupInjectionShape::CompactRenderedMarkdown,
                markdown,
            })),
            Ok(None) => Ok(SystemPromptResponse::degraded(
                "Laputa applied authority is empty; no authority sections rendered",
            )),
            Err(error) => Ok(SystemPromptResponse::degraded(format!(
                "Laputa applied authority read failed: {error}"
            ))),
        }
    }

    async fn prefetch(
        &self,
        request: PrefetchRequest,
    ) -> agent_diva_core::Result<PrefetchResponse> {
        if request.intent.trim().is_empty() {
            return Ok(PrefetchResponse::default());
        }

        Ok(PrefetchResponse {
            status: PrefetchStatus::SkippedNoIntent,
            prompt_block: None,
        })
    }

    async fn sync_turn(
        &self,
        _request: SyncTurnRequest,
    ) -> agent_diva_core::Result<SyncTurnResponse> {
        Ok(SyncTurnResponse {
            status: SyncTurnStatus::Noop,
        })
    }

    async fn on_session_end(
        &self,
        _request: SessionEndRequest,
    ) -> agent_diva_core::Result<SessionEndResponse> {
        Ok(SessionEndResponse {
            status: SessionEndStatus::Noop,
        })
    }
}

fn content_to_markdown(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(value) => {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        value => {
            let rendered = serde_json::to_string_pretty(value).ok()?;
            let trimmed = rendered.trim();
            (!trimmed.is_empty() && trimmed != "null").then(|| format!("```json\n{trimmed}\n```"))
        }
    }
}

fn section_title(section: &LaputaSectionName) -> &'static str {
    match section {
        LaputaSectionName::Identity => "Identity",
        LaputaSectionName::Relationship => "Relationship",
        LaputaSectionName::Commitment => "Commitments",
        LaputaSectionName::Preferences => "Preferences",
        LaputaSectionName::MemoryMd => "Long-Term Memory",
        LaputaSectionName::HistoryMd => "Memory History",
        _ => "Laputa Section",
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let keep = max_chars.saturating_sub(3);
    let mut truncated = value.chars().take(keep).collect::<String>();
    truncated.push_str("...");
    truncated
}

impl From<LaputaError> for agent_diva_core::Error {
    fn from(error: LaputaError) -> Self {
        agent_diva_core::Error::Internal(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::{
        evolution::{
            EvidenceRef, EvidenceSource, EvolutionProposal, ProposalState, ProposalType, RiskLevel,
        },
        memory::{MemoryProvider, StartupStatus, SystemPromptRequest},
    };
    use chrono::{DateTime, Utc};
    use std::fs;

    fn ts(seconds: u32) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(&format!("2026-06-14T00:01:{seconds:02}Z"))
            .unwrap()
            .with_timezone(&Utc)
    }

    fn proposal(id: &str, patch: &str, state: ProposalState) -> EvolutionProposal {
        EvolutionProposal {
            id: id.to_string(),
            created_at: ts(2),
            updated_at: ts(2),
            created_by: "autodream".to_string(),
            proposal_type: ProposalType::MemoryPatch,
            target_section: LaputaSectionName::MemoryMd,
            evidence_refs: vec![EvidenceRef {
                id: "ev-1".to_string(),
                source: EvidenceSource::Session,
                uri: "session://ev-1".to_string(),
                excerpt: Some("bounded evidence".to_string()),
                hash: Some("hash-ev-1".to_string()),
                created_at: ts(1),
            }],
            proposed_patch: patch.to_string(),
            risk_level: RiskLevel::Medium,
            state,
            source_run_id: Some("run-1".to_string()),
        }
    }

    #[test]
    fn renders_applied_laputa_sections_and_excludes_unapplied_proposals() {
        let temp = tempfile::tempdir().unwrap();
        let service = LaputaService::open(temp.path()).unwrap();
        fs::write(
            temp.path()
                .join(".laputa")
                .join("sections")
                .join("memory_md.json"),
            r#""Applied stable memory""#,
        )
        .unwrap();
        service
            .create_proposal(proposal(
                "proposal-1",
                r#""Pending untrusted memory""#,
                ProposalState::PendingReview,
            ))
            .unwrap();
        let provider = LaputaMemoryProvider::new(service);

        let response = provider
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp.path().to_path_buf(),
            })
            .unwrap();
        let markdown = response.prompt_block.unwrap().markdown;

        assert_eq!(response.status, StartupStatus::Ready);
        assert!(markdown.contains("## Applied Laputa Authority"));
        assert!(markdown.contains("Applied stable memory"));
        assert!(markdown.contains("pending_proposals: excluded_from_default_prompt"));
        assert!(!markdown.contains("Pending untrusted memory"));
    }

    #[test]
    fn empty_laputa_authority_degrades_without_writes() {
        let temp = tempfile::tempdir().unwrap();
        let provider = LaputaMemoryProvider::open(temp.path()).unwrap();

        let response = provider
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp.path().to_path_buf(),
            })
            .unwrap();

        assert!(matches!(response.status, StartupStatus::Degraded { .. }));
        assert!(!temp
            .path()
            .join(".laputa")
            .join("proposals")
            .join("auto.json")
            .exists());
    }

    #[test]
    fn read_failure_degrades_without_authority_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let service = LaputaService::open(temp.path()).unwrap();
        let memory_path = temp
            .path()
            .join(".laputa")
            .join("sections")
            .join("memory_md.json");
        fs::remove_file(&memory_path).ok();
        fs::create_dir(&memory_path).unwrap();
        let provider = LaputaMemoryProvider::new(service);

        let response = provider
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp.path().to_path_buf(),
            })
            .unwrap();

        assert!(matches!(response.status, StartupStatus::Degraded { .. }));
        assert!(memory_path.is_dir());
    }

    #[test]
    fn truncates_large_authority_sections() {
        let temp = tempfile::tempdir().unwrap();
        let service = LaputaService::open(temp.path()).unwrap();
        fs::write(
            temp.path()
                .join(".laputa")
                .join("sections")
                .join("memory_md.json"),
            format!("{:?}", "x".repeat(100)),
        )
        .unwrap();
        let provider = LaputaMemoryProvider::new(service).with_max_section_chars(10);

        let markdown = provider
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp.path().to_path_buf(),
            })
            .unwrap()
            .prompt_block
            .unwrap()
            .markdown;

        assert!(markdown.contains("xxxxxxx..."));
    }
}

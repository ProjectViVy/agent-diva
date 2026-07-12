//! LLM-backed report narrative generation (no tools).

use std::{sync::Arc, time::Instant};

use agent_diva_core::config::LlmCurationConfig;
use agent_diva_core::reports::{
    validate_curated_narrative, CoverageStatus, CuratedReportNarrative, ReportFactBundle,
    ReportGenerationMetadata, ReportNarrativeError, ReportNarrativeGenerator,
    ReportNarrativeOptions, PROMPT_VERSION,
};
use async_trait::async_trait;
use serde_json::Value;
use tracing::{debug, warn};

use crate::base::{LLMProvider, Message, ToolChoiceMode};

/// Provider-backed report narrative generator.
pub struct LlmReportNarrativeGenerator {
    provider: Arc<dyn LLMProvider>,
    config: LlmCurationConfig,
    /// Resolved model id (raw; never rewritten here).
    model: String,
}

impl LlmReportNarrativeGenerator {
    pub fn new(
        provider: Arc<dyn LLMProvider>,
        config: LlmCurationConfig,
        model: impl Into<String>,
    ) -> Self {
        Self {
            provider,
            config,
            model: model.into(),
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    fn build_messages(&self, bundle: &ReportFactBundle, options: &ReportNarrativeOptions) -> Vec<Message> {
        let system = format!(
            r#"你是 Agent Diva 的周期报告归纳器。只根据用户消息中提供的 JSON 事实生成中文结构化报告。
硬性规则：
1. 只能使用提供的 facts；不得编造项目、完成项、决定、时间或外部事件。
2. 事实不足时明确写「信息不足」或「未观察到明确的工作推进」。
3. 每个 themes/accomplishments/decisions/risks_or_blockers/next_actions/coverage_notes 条目必须包含至少一个 evidence_ids，且只能引用输入中的 evidence_id。
4. 不要输出 session id、不要输出思维过程、不要输出 markdown，只输出一个 JSON 对象。
5. 语言：{language}
6. prompt_version={prompt_version}

JSON schema:
{{
  "executive_summary": "string",
  "themes": [{{"text":"string","evidence_ids":["id"]}}],
  "accomplishments": [{{"text":"string","evidence_ids":["id"]}}],
  "decisions": [{{"text":"string","evidence_ids":["id"]}}],
  "risks_or_blockers": [{{"text":"string","evidence_ids":["id"]}}],
  "next_actions": [{{"text":"string","evidence_ids":["id"]}}],
  "coverage_notes": [{{"text":"string","evidence_ids":["id"]}}]
}}
"#,
            language = options.language,
            prompt_version = PROMPT_VERSION,
        );

        let user_payload = serde_json::json!({
            "period": bundle.window.period.as_str(),
            "window_key": bundle.window.key,
            "coverage": bundle.coverage,
            "facts": bundle.facts.iter().map(|fact| serde_json::json!({
                "id": fact.id,
                "kind": fact.kind,
                "source_label": fact.source_label,
                "content": fact.content,
                "evidence_id": fact.evidence_id,
            })).collect::<Vec<_>>(),
            "allowed_evidence_ids": bundle.evidence_refs.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
        });

        vec![
            Message::system(system),
            Message::user(format!(
                "请根据下列事实生成报告 JSON：\n{}",
                user_payload
            )),
        ]
    }
}

#[async_trait]
impl ReportNarrativeGenerator for LlmReportNarrativeGenerator {
    async fn generate(
        &self,
        bundle: &ReportFactBundle,
        options: &ReportNarrativeOptions,
    ) -> Result<(CuratedReportNarrative, ReportGenerationMetadata), ReportNarrativeError> {
        if !self.config.enabled {
            return Err(ReportNarrativeError::Disabled);
        }
        if bundle.facts.is_empty() {
            return Err(ReportNarrativeError::Empty);
        }

        let messages = self.build_messages(bundle, options);
        let max_tokens = options
            .max_output_tokens
            .min(self.config.max_output_tokens)
            .max(1) as i32;
        let timeout = std::time::Duration::from_secs(
            options.timeout_secs.max(1).min(self.config.timeout_secs.max(1)),
        );

        let started = Instant::now();
        let chat_future = self.provider.chat(
            messages,
            None,
            ToolChoiceMode::Disabled,
            Some(self.model.clone()),
            max_tokens,
            0.2,
        );

        let response = match tokio::time::timeout(timeout, chat_future).await {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => {
                warn!(error = %error, "report narrative provider failed");
                return Err(ReportNarrativeError::Provider);
            }
            Err(_) => {
                warn!("report narrative generation timed out");
                return Err(ReportNarrativeError::Timeout);
            }
        };

        let duration_ms = started.elapsed().as_millis() as u64;
        let content = response
            .content
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(ReportNarrativeError::Empty)?;

        let narrative = parse_narrative_json(content)?;
        let allowed = bundle.evidence_ids();
        validate_curated_narrative(&narrative, &allowed).map_err(|error| {
            debug!(error = %error, "report narrative evidence validation failed");
            ReportNarrativeError::EvidenceValidation
        })?;

        let coverage = if bundle.coverage.truncated_fact_count > 0
            || bundle.coverage.missing_daily_dates_count > 0
        {
            CoverageStatus::Partial
        } else {
            CoverageStatus::Complete
        };

        let input_tokens = response
            .usage
            .get("prompt_tokens")
            .or_else(|| response.usage.get("input_tokens"))
            .copied()
            .map(|v| v as u64);
        let output_tokens = response
            .usage
            .get("completion_tokens")
            .or_else(|| response.usage.get("output_tokens"))
            .copied()
            .map(|v| v as u64);

        let metadata = ReportGenerationMetadata::llm_curated(
            Some(self.model.clone()),
            coverage,
            input_tokens,
            output_tokens,
            Some(duration_ms),
        );
        Ok((narrative, metadata))
    }
}

fn parse_narrative_json(content: &str) -> Result<CuratedReportNarrative, ReportNarrativeError> {
    let json_text = extract_json_object(content).ok_or(ReportNarrativeError::InvalidJson)?;
    let value: Value =
        serde_json::from_str(json_text).map_err(|_| ReportNarrativeError::InvalidJson)?;
    // Reject unknown top-level fields for safety.
    if let Some(obj) = value.as_object() {
        const ALLOWED: &[&str] = &[
            "executive_summary",
            "themes",
            "accomplishments",
            "decisions",
            "risks_or_blockers",
            "next_actions",
            "coverage_notes",
        ];
        for key in obj.keys() {
            if !ALLOWED.contains(&key.as_str()) {
                return Err(ReportNarrativeError::InvalidJson);
            }
        }
    }
    serde_json::from_value(value).map_err(|_| ReportNarrativeError::InvalidJson)
}

fn extract_json_object(content: &str) -> Option<&str> {
    let trimmed = content.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed);
    }
    // Fenced ```json ... ```
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end > start {
                return Some(&trimmed[start..=end]);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::evolution::{EvidenceRef, EvidenceSource};
    use agent_diva_core::reports::{
        build_daily_fact_bundle, FactBundleLimits, SessionDigestItem, SessionWindowDigest,
    };
    use std::collections::HashMap;

    struct MockProvider {
        content: String,
        model_seen: std::sync::Mutex<Option<String>>,
    }

    #[async_trait]
    impl LLMProvider for MockProvider {
        async fn chat(
            &self,
            _messages: Vec<Message>,
            tools: Option<Vec<serde_json::Value>>,
            tool_choice: ToolChoiceMode,
            model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> crate::base::ProviderResult<crate::base::LLMResponse> {
            assert!(tools.is_none());
            assert!(matches!(tool_choice, ToolChoiceMode::Disabled));
            *self.model_seen.lock().unwrap() = model.clone();
            Ok(crate::base::LLMResponse {
                content: Some(self.content.clone()),
                tool_calls: vec![],
                finish_reason: "stop".to_string(),
                usage: HashMap::from([
                    ("prompt_tokens".to_string(), 100),
                    ("completion_tokens".to_string(), 50),
                ]),
                reasoning_content: None,
            })
        }

        fn get_default_model(&self) -> String {
            "deepseek-chat".to_string()
        }
    }

    fn sample_bundle() -> ReportFactBundle {
        let created_at = chrono::Utc::now();
        let evidence = EvidenceRef {
            id: "session-a".to_string(),
            source: EvidenceSource::Session,
            uri: "session://a".to_string(),
            excerpt: Some("完成了报告系统改造".to_string()),
            hash: None,
            created_at,
        };
        let digest = SessionWindowDigest {
            items: vec![SessionDigestItem {
                session_key: "a".to_string(),
                first_timestamp: evidence.created_at,
                summary: "完成了报告系统改造与 evidence 校验".to_string(),
                message_count: 4,
                estimated_tokens: 30,
                evidence,
            }],
            session_count: 1,
            estimated_tokens: 30,
            message_count: 4,
        };
        build_daily_fact_bundle(
            chrono::NaiveDate::from_ymd_opt(2026, 7, 12).unwrap(),
            &digest,
            "zh-CN",
            FactBundleLimits::default(),
        )
    }

    #[tokio::test]
    async fn parses_valid_json_and_keeps_raw_model_id() {
        let bundle = sample_bundle();
        let evidence_id = bundle.facts[0].evidence_id.clone();
        let content = format!(
            r#"{{
              "executive_summary": "完成了报告系统改造。",
              "themes": [{{"text":"报告归纳","evidence_ids":["{evidence_id}"]}}],
              "accomplishments": [{{"text":"完成 evidence 校验","evidence_ids":["{evidence_id}"]}}],
              "decisions": [],
              "risks_or_blockers": [],
              "next_actions": [{{"text":"接入周报","evidence_ids":["{evidence_id}"]}}],
              "coverage_notes": []
            }}"#
        );
        let mock = Arc::new(MockProvider {
            content,
            model_seen: std::sync::Mutex::new(None),
        });
        let generator = LlmReportNarrativeGenerator::new(
            mock.clone(),
            LlmCurationConfig {
                enabled: true,
                ..LlmCurationConfig::default()
            },
            "deepseek-chat",
        );
        let (narrative, metadata) = generator
            .generate(&bundle, &ReportNarrativeOptions::default())
            .await
            .unwrap();
        assert!(narrative.executive_summary.contains("报告系统"));
        assert_eq!(metadata.model.as_deref(), Some("deepseek-chat"));
        assert_eq!(
            mock.model_seen.lock().unwrap().as_deref(),
            Some("deepseek-chat")
        );
    }

    #[tokio::test]
    async fn rejects_unknown_evidence_and_invalid_json() {
        let bundle = sample_bundle();
        let mock = Arc::new(MockProvider {
            content: r#"{"executive_summary":"x","themes":[{"text":"t","evidence_ids":["nope"]}],"accomplishments":[],"decisions":[],"risks_or_blockers":[],"next_actions":[],"coverage_notes":[]}"#.to_string(),
            model_seen: std::sync::Mutex::new(None),
        });
        let generator = LlmReportNarrativeGenerator::new(
            mock,
            LlmCurationConfig {
                enabled: true,
                ..LlmCurationConfig::default()
            },
            "deepseek-chat",
        );
        let err = generator
            .generate(&bundle, &ReportNarrativeOptions::default())
            .await
            .unwrap_err();
        assert_eq!(err, ReportNarrativeError::EvidenceValidation);

        let mock2 = Arc::new(MockProvider {
            content: "not-json".to_string(),
            model_seen: std::sync::Mutex::new(None),
        });
        let generator2 = LlmReportNarrativeGenerator::new(
            mock2,
            LlmCurationConfig {
                enabled: true,
                ..LlmCurationConfig::default()
            },
            "deepseek-chat",
        );
        let err2 = generator2
            .generate(&bundle, &ReportNarrativeOptions::default())
            .await
            .unwrap_err();
        assert_eq!(err2, ReportNarrativeError::InvalidJson);
    }
}

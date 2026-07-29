use super::{
    fact_bundle::ReportFactBundle,
    narrative::{
        CoverageStatus, CuratedReportNarrative, GenerationMode, NarrativeItem,
        ReportGenerationMetadata,
    },
    period::ReportPeriod,
};

/// Rendered report body pieces used by autodream writers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedReportBody {
    pub title: String,
    pub summary: String,
    pub sections: Vec<String>,
    pub source: String,
    pub generation: ReportGenerationMetadata,
}

/// Render a curated narrative into markdown sections (Chinese headings).
pub fn render_curated_body(
    bundle: &ReportFactBundle,
    narrative: &CuratedReportNarrative,
    metadata: ReportGenerationMetadata,
) -> RenderedReportBody {
    let title = title_for_window(bundle);
    let summary = narrative.executive_summary.trim().to_string();
    let mut sections = Vec::new();

    sections.push(format!("## 摘要\n\n{}", summary));

    if let Some(section) = render_item_section("重点进展/主题", &narrative.themes) {
        sections.push(section);
    }
    if let Some(section) = render_item_section("关键决策与产出", &narrative.accomplishments)
    {
        sections.push(section);
    }
    // decisions are separate when present
    if let Some(section) = render_item_section("关键决策", &narrative.decisions) {
        sections.push(section);
    }
    if let Some(section) = render_item_section("风险、阻塞与待确认", &narrative.risks_or_blockers)
    {
        sections.push(section);
    }
    if narrative.next_actions.is_empty() {
        sections.push("## 下一步\n\n- 暂无明确后续动作".to_string());
    } else if let Some(section) = render_item_section("下一步", &narrative.next_actions) {
        sections.push(section);
    }
    sections.push(render_coverage_section(
        bundle,
        &metadata,
        &narrative.coverage_notes,
    ));

    RenderedReportBody {
        title,
        summary,
        sections,
        source: source_for_period(bundle.window.period),
        generation: metadata,
    }
}

/// Deterministic fallback narrative from the fact bundle (no LLM).
pub fn render_deterministic_fallback(
    bundle: &ReportFactBundle,
    reason: impl Into<String>,
) -> RenderedReportBody {
    let reason = reason.into();
    let coverage = if bundle.coverage.truncated_fact_count > 0
        || bundle.coverage.missing_daily_dates_count > 0
    {
        CoverageStatus::Partial
    } else {
        CoverageStatus::Fallback
    };
    let metadata = ReportGenerationMetadata::deterministic_fallback(reason, coverage);
    let title = title_for_window(bundle);

    let summary = match bundle.window.period {
        ReportPeriod::Daily => format!(
            "汇总 {} 个会话、{} 条消息（确定性降级）。",
            bundle.coverage.session_count, bundle.coverage.message_count
        ),
        ReportPeriod::Weekly | ReportPeriod::Monthly => format!(
            "聚合 {} 份日报与 {} 个缺口会话（确定性降级）。",
            bundle.coverage.daily_inputs_count, bundle.coverage.fallback_session_count
        ),
    };

    let mut highlight_lines = Vec::new();
    for fact in bundle.facts.iter().take(12) {
        // Avoid leaking raw session keys in body; use source_label only.
        highlight_lines.push(format!("- {}: {}", fact.source_label, fact.content));
    }
    if highlight_lines.is_empty() {
        highlight_lines.push("- 本周期未收集到可归纳事实。".to_string());
    }

    let sections = vec![
        format!("## 摘要\n\n{summary}"),
        format!("## 重点进展/主题\n\n{}", highlight_lines.join("\n")),
        "## 下一步\n\n- 暂无明确后续动作（确定性降级未推断行动项）".to_string(),
        render_coverage_section(bundle, &metadata, &[]),
    ];

    RenderedReportBody {
        title,
        summary,
        sections,
        source: source_for_period(bundle.window.period),
        generation: metadata,
    }
}

fn render_item_section(title: &str, items: &[NarrativeItem]) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    let mut body = format!("## {title}\n\n");
    for item in items {
        body.push_str(&format!("- {}\n", item.text.trim()));
    }
    Some(body)
}

fn render_coverage_section(
    bundle: &ReportFactBundle,
    metadata: &ReportGenerationMetadata,
    coverage_notes: &[NarrativeItem],
) -> String {
    let mode = metadata.generation_mode.as_str();
    let status = metadata.coverage_status.as_str();
    let mut section = String::from("## 数据口径与覆盖\n\n");
    section.push_str(&format!("- **周期**: {}\n", bundle.window.key));
    section.push_str(&format!(
        "- **会话数**: {}\n",
        bundle.coverage.session_count
    ));
    section.push_str(&format!(
        "- **消息数**: {}\n",
        bundle.coverage.message_count
    ));
    section.push_str(&format!(
        "- **估算 token**: {}\n",
        bundle.coverage.estimated_tokens
    ));
    if matches!(
        bundle.window.period,
        ReportPeriod::Weekly | ReportPeriod::Monthly
    ) {
        section.push_str(&format!(
            "- **日报输入数**: {}\n",
            bundle.coverage.daily_inputs_count
        ));
        section.push_str(&format!(
            "- **缺失日报天数**: {}\n",
            bundle.coverage.missing_daily_dates_count
        ));
        section.push_str(&format!(
            "- **缺口恢复会话**: {}\n",
            bundle.coverage.fallback_session_count
        ));
    }
    if bundle.coverage.truncated_fact_count > 0 {
        section.push_str(&format!(
            "- **事实截断数**: {}\n",
            bundle.coverage.truncated_fact_count
        ));
    }
    if bundle.coverage.filtered_noise_count > 0 {
        section.push_str(&format!(
            "- **低信息量过滤**: {}\n",
            bundle.coverage.filtered_noise_count
        ));
    }
    section.push_str(&format!("- **生成模式**: `{mode}`\n"));
    section.push_str(&format!("- **覆盖状态**: `{status}`\n"));
    if let Some(reason) = metadata.failure_reason.as_deref() {
        section.push_str(&format!("- **降级原因**: `{reason}`\n"));
    }
    if metadata.generation_mode == GenerationMode::LlmCurated {
        if let Some(model) = metadata.model.as_deref() {
            section.push_str(&format!("- **模型（审计）**: `{model}`\n"));
        }
    }
    for note in coverage_notes {
        section.push_str(&format!("- {}\n", note.text.trim()));
    }
    section
}

fn title_for_window(bundle: &ReportFactBundle) -> String {
    match bundle.window.period {
        ReportPeriod::Daily => format!("Daily Report {}", bundle.window.key),
        ReportPeriod::Weekly => format!("Weekly Report {}", bundle.window.key),
        ReportPeriod::Monthly => format!("Monthly Report {}", bundle.window.key),
    }
}

fn source_for_period(period: ReportPeriod) -> String {
    match period {
        ReportPeriod::Daily => "session_aggregate".to_string(),
        ReportPeriod::Weekly | ReportPeriod::Monthly => "daily_aggregate".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evolution::{EvidenceRef, EvidenceSource};
    use crate::reports::{
        build_daily_fact_bundle, FactBundleLimits, SessionDigestItem, SessionWindowDigest,
    };
    use chrono::{NaiveDate, TimeZone, Utc};

    #[test]
    fn deterministic_fallback_avoids_session_key_prefix() {
        let evidence = EvidenceRef {
            id: "session-telegram-1".to_string(),
            source: EvidenceSource::Session,
            uri: "session://telegram%3A1".to_string(),
            excerpt: Some("实现报告归纳".to_string()),
            hash: None,
            created_at: Utc.with_ymd_and_hms(2026, 7, 12, 1, 0, 0).unwrap(),
        };
        let digest = SessionWindowDigest {
            items: vec![SessionDigestItem {
                session_key: "telegram:1".to_string(),
                first_timestamp: evidence.created_at,
                summary: "实现报告归纳".to_string(),
                message_count: 3,
                estimated_tokens: 12,
                evidence,
            }],
            session_count: 1,
            estimated_tokens: 12,
            message_count: 3,
        };
        let bundle = build_daily_fact_bundle(
            NaiveDate::from_ymd_opt(2026, 7, 12).unwrap(),
            &digest,
            "zh-CN",
            FactBundleLimits::default(),
        );
        let rendered = render_deterministic_fallback(&bundle, "disabled");
        assert_eq!(
            rendered.generation.generation_mode,
            GenerationMode::DeterministicFallback
        );
        let joined = rendered.sections.join("\n");
        assert!(!joined.contains("`telegram:1`"));
        assert!(joined.contains("确定性降级"));
    }
}

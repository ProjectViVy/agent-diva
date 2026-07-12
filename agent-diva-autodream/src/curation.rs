//! Shared LLM-curation orchestration for rhythm reports.

use std::sync::Arc;

use agent_diva_core::config::LlmCurationConfig;
use agent_diva_core::reports::{
    render_curated_body, render_deterministic_fallback, ReportFactBundle, ReportNarrativeError,
    ReportNarrativeGenerator, ReportNarrativeOptions, ReportPeriod, RenderedReportBody,
};
use tracing::{info, warn};

use crate::RhythmReportContent;

/// Resolve narrative for a fact bundle, falling back on any failure.
pub async fn resolve_report_body(
    bundle: &ReportFactBundle,
    generator: Option<&Arc<dyn ReportNarrativeGenerator>>,
    curation: &LlmCurationConfig,
) -> RenderedReportBody {
    if !curation.enabled || generator.is_none() {
        return render_deterministic_fallback(bundle, "disabled");
    }

    let generator = generator.expect("checked above");
    let options = ReportNarrativeOptions {
        period: bundle.window.period,
        language: curation.language.clone(),
        max_output_tokens: curation.max_output_tokens,
        timeout_secs: curation.timeout_secs,
    };

    match generator.generate(bundle, &options).await {
        Ok((narrative, metadata)) => {
            info!(
                period = bundle.window.period.as_str(),
                key = %bundle.window.key,
                mode = metadata.generation_mode.as_str(),
                "report narrative curated by llm"
            );
            render_curated_body(bundle, &narrative, metadata)
        }
        Err(error) => {
            warn!(
                period = bundle.window.period.as_str(),
                key = %bundle.window.key,
                reason = error.category(),
                "report narrative fell back to deterministic"
            );
            render_deterministic_fallback(bundle, map_error_category(&error))
        }
    }
}

fn map_error_category(error: &ReportNarrativeError) -> &'static str {
    error.category()
}

/// Convert a rendered body + bundle stats into autodream write content.
pub fn content_from_rendered(
    bundle: &ReportFactBundle,
    rendered: RenderedReportBody,
) -> RhythmReportContent {
    let generation = &rendered.generation;
    RhythmReportContent {
        title: rendered.title,
        summary: rendered.summary,
        sections: rendered.sections,
        evidence_refs: bundle.evidence_refs.clone(),
        source: Some(rendered.source),
        session_count: Some(bundle.coverage.session_count),
        token_used: Some(bundle.coverage.estimated_tokens),
        fallback_used: Some(
            bundle.coverage.fallback_session_count > 0
                || bundle.coverage.missing_daily_dates_count > 0,
        ),
        daily_inputs_count: Some(bundle.coverage.daily_inputs_count),
        missing_daily_dates_count: Some(bundle.coverage.missing_daily_dates_count),
        generation_mode: Some(generation.generation_mode.as_str().to_string()),
        narrative_schema_version: Some(generation.narrative_schema_version),
        prompt_version: Some(generation.prompt_version.clone()),
        coverage_status: Some(generation.coverage_status.as_str().to_string()),
        fallback_reason: generation.failure_reason.clone(),
    }
}

/// Monthly markdown rendering shared with report-system frontmatter conventions.
pub fn render_monthly_markdown(
    month_key: &str,
    generated_at: &str,
    generated_by: &str,
    content: &RhythmReportContent,
) -> String {
    let mut markdown = String::new();
    markdown.push_str("---\n");
    markdown.push_str("period: monthly\n");
    markdown.push_str(&format!("month: {month_key}\n"));
    markdown.push_str(&format!("generated_at: {generated_at}\n"));
    markdown.push_str(&format!("generated_by: {generated_by}\n"));
    if let Some(source) = content.source.as_deref() {
        markdown.push_str(&format!("source: {source}\n"));
    }
    if let Some(session_count) = content.session_count {
        markdown.push_str(&format!("session_count: {session_count}\n"));
    }
    if let Some(token_used) = content.token_used {
        markdown.push_str(&format!("token_used: {token_used}\n"));
    }
    if let Some(fallback_used) = content.fallback_used {
        markdown.push_str(&format!("fallback_used: {fallback_used}\n"));
    }
    if let Some(daily_inputs_count) = content.daily_inputs_count {
        markdown.push_str(&format!("daily_inputs_count: {daily_inputs_count}\n"));
    }
    if let Some(missing_daily_dates_count) = content.missing_daily_dates_count {
        markdown.push_str(&format!(
            "missing_daily_dates_count: {missing_daily_dates_count}\n"
        ));
    }
    if let Some(generation_mode) = content.generation_mode.as_deref() {
        markdown.push_str(&format!("generation_mode: {generation_mode}\n"));
    }
    if let Some(narrative_schema_version) = content.narrative_schema_version {
        markdown.push_str(&format!(
            "narrative_schema_version: {narrative_schema_version}\n"
        ));
    }
    if let Some(prompt_version) = content.prompt_version.as_deref() {
        markdown.push_str(&format!("prompt_version: {prompt_version}\n"));
    }
    if let Some(coverage_status) = content.coverage_status.as_deref() {
        markdown.push_str(&format!("coverage_status: {coverage_status}\n"));
    }
    if let Some(fallback_reason) = content.fallback_reason.as_deref() {
        markdown.push_str(&format!("fallback_reason: {fallback_reason}\n"));
    }
    markdown.push_str("schema_version: 1\n");
    markdown.push_str("---\n\n");
    markdown.push_str(&format!("# {}\n\n", content.title.trim()));
    markdown.push_str(content.summary.trim());
    markdown.push_str("\n\n");
    for section in content
        .sections
        .iter()
        .filter(|section| !section.trim().is_empty())
    {
        markdown.push_str(section.trim());
        markdown.push_str("\n\n");
    }
    markdown.push_str("## Evidence References\n\n");
    for evidence in content.evidence_refs.iter().take(50) {
        markdown.push_str(&format!("- id: `{}`\n", evidence.id));
        markdown.push_str(&format!("  source: `{:?}`\n", evidence.source));
        markdown.push_str(&format!("  uri: `{}`\n", evidence.uri));
        if let Some(excerpt) = &evidence.excerpt {
            let bounded: String = excerpt.chars().take(240).collect();
            markdown.push_str(&format!("  excerpt: {}\n", bounded.replace('\n', " ")));
        }
    }
    markdown.push_str(&format!(
        "\n---\n*Generated by {generated_by} at {generated_at}*\n"
    ));
    let _ = ReportPeriod::Monthly;
    markdown
}

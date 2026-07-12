use std::collections::HashSet;

use super::narrative::{CuratedReportNarrative, NarrativeItem, NARRATIVE_SCHEMA_VERSION};

const MAX_SUMMARY_CHARS: usize = 800;
const MAX_ITEM_TEXT_CHARS: usize = 400;
const MAX_ITEMS_PER_SECTION: usize = 12;
const MAX_EVIDENCE_IDS_PER_ITEM: usize = 8;

/// Validation failure for curated narratives.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NarrativeValidationError {
    #[error("executive_summary is empty")]
    EmptySummary,
    #[error("executive_summary exceeds max length")]
    SummaryTooLong,
    #[error("section `{0}` has too many items")]
    TooManyItems(String),
    #[error("item text is empty")]
    EmptyItemText,
    #[error("item text exceeds max length")]
    ItemTextTooLong,
    #[error("item is missing evidence_ids")]
    MissingEvidenceIds,
    #[error("item has too many evidence_ids")]
    TooManyEvidenceIds,
    #[error("unknown evidence_id: {0}")]
    UnknownEvidenceId(String),
}

/// Validate a curated narrative against the allowed evidence set.
pub fn validate_curated_narrative(
    narrative: &CuratedReportNarrative,
    allowed_evidence_ids: &HashSet<String>,
) -> Result<(), NarrativeValidationError> {
    let summary = narrative.executive_summary.trim();
    if summary.is_empty() {
        return Err(NarrativeValidationError::EmptySummary);
    }
    if summary.chars().count() > MAX_SUMMARY_CHARS {
        return Err(NarrativeValidationError::SummaryTooLong);
    }

    validate_section("themes", &narrative.themes, allowed_evidence_ids)?;
    validate_section(
        "accomplishments",
        &narrative.accomplishments,
        allowed_evidence_ids,
    )?;
    validate_section("decisions", &narrative.decisions, allowed_evidence_ids)?;
    validate_section(
        "risks_or_blockers",
        &narrative.risks_or_blockers,
        allowed_evidence_ids,
    )?;
    validate_section(
        "next_actions",
        &narrative.next_actions,
        allowed_evidence_ids,
    )?;
    validate_section(
        "coverage_notes",
        &narrative.coverage_notes,
        allowed_evidence_ids,
    )?;
    let _ = NARRATIVE_SCHEMA_VERSION;
    Ok(())
}

fn validate_section(
    name: &str,
    items: &[NarrativeItem],
    allowed_evidence_ids: &HashSet<String>,
) -> Result<(), NarrativeValidationError> {
    if items.len() > MAX_ITEMS_PER_SECTION {
        return Err(NarrativeValidationError::TooManyItems(name.to_string()));
    }
    for item in items {
        let text = item.text.trim();
        if text.is_empty() {
            return Err(NarrativeValidationError::EmptyItemText);
        }
        if text.chars().count() > MAX_ITEM_TEXT_CHARS {
            return Err(NarrativeValidationError::ItemTextTooLong);
        }
        if item.evidence_ids.is_empty() {
            return Err(NarrativeValidationError::MissingEvidenceIds);
        }
        if item.evidence_ids.len() > MAX_EVIDENCE_IDS_PER_ITEM {
            return Err(NarrativeValidationError::TooManyEvidenceIds);
        }
        for evidence_id in &item.evidence_ids {
            if !allowed_evidence_ids.contains(evidence_id) {
                return Err(NarrativeValidationError::UnknownEvidenceId(
                    evidence_id.clone(),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::narrative::NarrativeItem;

    #[test]
    fn rejects_unknown_evidence_id() {
        let narrative = CuratedReportNarrative {
            executive_summary: "今天推进了报告归纳。".to_string(),
            themes: vec![NarrativeItem {
                text: "主题".to_string(),
                evidence_ids: vec!["missing".to_string()],
            }],
            accomplishments: vec![],
            decisions: vec![],
            risks_or_blockers: vec![],
            next_actions: vec![],
            coverage_notes: vec![],
        };
        let allowed = HashSet::from(["session-a".to_string()]);
        let err = validate_curated_narrative(&narrative, &allowed).unwrap_err();
        assert!(matches!(
            err,
            NarrativeValidationError::UnknownEvidenceId(_)
        ));
    }

    #[test]
    fn accepts_valid_narrative() {
        let narrative = CuratedReportNarrative {
            executive_summary: "完成了校验逻辑。".to_string(),
            themes: vec![],
            accomplishments: vec![NarrativeItem {
                text: "完成 evidence 校验".to_string(),
                evidence_ids: vec!["session-a".to_string()],
            }],
            decisions: vec![],
            risks_or_blockers: vec![],
            next_actions: vec![],
            coverage_notes: vec![],
        };
        let allowed = HashSet::from(["session-a".to_string()]);
        validate_curated_narrative(&narrative, &allowed).unwrap();
    }
}

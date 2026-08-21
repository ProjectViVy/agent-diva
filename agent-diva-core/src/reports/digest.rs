use std::{
    collections::{BTreeSet, HashMap},
    fs,
    path::Path,
};

use chrono::{DateTime, Local, NaiveDate, Utc};

use crate::{
    error::{Error, Result},
    evolution::{EvidenceRef, EvidenceSource},
    session::SessionManager,
};

use super::frontmatter::{RhythmReportDocument, RhythmReportFrontmatter};

const MAX_SUMMARY_CHARS: usize = 240;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionDigestItem {
    pub session_key: String,
    pub first_timestamp: DateTime<Utc>,
    pub summary: String,
    pub message_count: usize,
    pub estimated_tokens: u64,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionWindowDigest {
    pub items: Vec<SessionDigestItem>,
    pub session_count: u64,
    pub estimated_tokens: u64,
    pub message_count: usize,
}

pub fn parse_rhythm_report(markdown: &str) -> Result<RhythmReportDocument> {
    let (frontmatter, body) = split_frontmatter(markdown)?;
    let title = extract_title(body).unwrap_or_else(|| "Untitled report".to_string());
    let summary = extract_summary(body).unwrap_or_else(|| title.clone());
    Ok(RhythmReportDocument {
        frontmatter,
        title,
        summary,
        body: body.to_string(),
    })
}

pub fn read_rhythm_report(path: &Path) -> Result<RhythmReportDocument> {
    let markdown = fs::read_to_string(path)?;
    parse_rhythm_report(&markdown)
}

pub fn collect_session_window_digest_for_dates(
    workspace: &Path,
    dates: &BTreeSet<NaiveDate>,
) -> Result<SessionWindowDigest> {
    if dates.is_empty() {
        return Ok(SessionWindowDigest::default());
    }

    let mut manager = SessionManager::new(workspace);
    let mut by_session: HashMap<String, Vec<(DateTime<Utc>, String)>> = HashMap::new();

    for info in manager.list_sessions() {
        let Some(session) = manager.get_or_load(&info.key) else {
            continue;
        };
        let mut matched = Vec::new();
        for message in &session.messages {
            let local_date = message.timestamp.with_timezone(&Local).date_naive();
            if !dates.contains(&local_date) {
                continue;
            }
            if !matches!(message.role.as_str(), "user" | "assistant" | "tool") {
                continue;
            }
            let content = message.content.trim();
            if content.is_empty() {
                continue;
            }
            matched.push((message.timestamp, content.to_string()));
        }
        if !matched.is_empty() {
            by_session.insert(info.key.clone(), matched);
        }
    }

    let mut items = by_session
        .into_iter()
        .map(|(session_key, messages)| {
            let first_timestamp = messages
                .iter()
                .map(|(timestamp, _)| *timestamp)
                .min()
                .unwrap_or_else(Utc::now);
            let combined = messages
                .iter()
                .map(|(_, content)| content.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let summary = truncate_chars(&combined.replace('\n', " "), MAX_SUMMARY_CHARS);
            let estimated_tokens = estimate_tokens(&combined);
            let message_count = messages.len();
            let evidence = EvidenceRef {
                id: format!("session-{}", sanitize_id_component(&session_key)),
                source: EvidenceSource::Session,
                uri: format!("session://{}", urlencoding::encode(&session_key)),
                excerpt: Some(summary.clone()),
                hash: None,
                created_at: first_timestamp,
            };
            SessionDigestItem {
                session_key,
                first_timestamp,
                summary,
                message_count,
                estimated_tokens,
                evidence,
            }
        })
        .collect::<Vec<_>>();
    items.sort_by_key(|item| item.first_timestamp);

    let estimated_tokens = items.iter().map(|item| item.estimated_tokens).sum();
    let message_count = items.iter().map(|item| item.message_count).sum();
    Ok(SessionWindowDigest {
        session_count: items.len() as u64,
        estimated_tokens,
        message_count,
        items,
    })
}

pub fn estimate_tokens(text: &str) -> u64 {
    let chars = text.chars().count();
    chars.div_ceil(4) as u64
}

pub(crate) fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut iter = value.chars();
    let truncated: String = iter.by_ref().take(max_chars).collect();
    if iter.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

pub(crate) fn sanitize_id_component(value: &str) -> String {
    let mut sanitized = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            sanitized.push(ch);
        } else {
            sanitized.push('-');
        }
    }
    sanitized.trim_matches('-').to_string()
}

fn split_frontmatter(markdown: &str) -> Result<(RhythmReportFrontmatter, &str)> {
    if !markdown.starts_with("---\n") {
        return Ok((RhythmReportFrontmatter::default(), markdown));
    }
    let rest = &markdown[4..];
    let Some(end) = rest.find("\n---\n") else {
        return Err(Error::Validation(
            "invalid rhythm report frontmatter fence".to_string(),
        ));
    };
    let yaml = &rest[..end];
    let body = &rest[end + 5..];
    let frontmatter =
        serde_yaml::from_str(yaml).map_err(|error| Error::Serialization(error.to_string()))?;
    Ok((frontmatter, body))
}

fn extract_title(body: &str) -> Option<String> {
    body.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_summary(body: &str) -> Option<String> {
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .find(|line| !line.starts_with('#') && !line.starts_with("```"))
        .map(ToOwned::to_owned)
}
